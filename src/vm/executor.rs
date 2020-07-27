use super::codegen::ContextKind;
use crate::*;

#[cfg(feature = "perf")]
use super::perf::*;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, SyncSender};
use vm_inst::*;

pub type ValueTable = FxHashMap<IdentId, Value>;
pub type VMResult = Result<Value, RubyError>;

#[derive(Debug)]
pub struct VM {
    // Global info
    pub globals: GlobalsRef,
    pub root_path: Vec<PathBuf>,
    // VM state
    fiber_state: FiberState,
    exec_context: Vec<ContextRef>,
    class_context: Vec<(Value, DefineMode)>,
    exec_stack: Vec<Value>,
    temp_stack: Vec<Value>,
    exception: bool,
    pc: usize,
    //gc_counter: usize,
    pub parent_fiber: Option<ParentFiberInfo>,
    #[cfg(feature = "perf")]
    pub perf: Perf,
}

pub type VMRef = Ref<VM>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FiberState {
    Created,
    Running,
    Dead,
}

#[derive(Debug)]
pub struct ParentFiberInfo {
    pub parent: VMRef,
    pub tx: SyncSender<VMResult>,
    pub rx: Receiver<FiberMsg>,
}

impl ParentFiberInfo {
    fn new(parent: VMRef, tx: SyncSender<VMResult>, rx: Receiver<FiberMsg>) -> Self {
        ParentFiberInfo { parent, tx, rx }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefineMode {
    module_function: bool,
}

impl DefineMode {
    pub fn default() -> Self {
        DefineMode {
            module_function: false,
        }
    }
}

// API's

impl GC for VM {
    fn mark(&self, alloc: &mut Allocator) {
        self.exec_context.iter().for_each(|c| c.mark(alloc));
        self.class_context.iter().for_each(|(v, _)| v.mark(alloc));
        self.exec_stack.iter().for_each(|v| v.mark(alloc));
        self.temp_stack.iter().for_each(|v| v.mark(alloc));
        if let Some(ParentFiberInfo { parent, .. }) = self.parent_fiber {
            parent.mark(alloc)
        }
    }
}

impl VM {
    pub fn new(globals: GlobalsRef) -> Self {
        let vm = VM {
            globals,
            root_path: vec![],
            fiber_state: FiberState::Created,
            class_context: vec![(Value::nil(), DefineMode::default())],
            exec_context: vec![],
            exec_stack: vec![],
            temp_stack: vec![],
            exception: false,
            pc: 0,
            //gc_counter: 0,
            parent_fiber: None,
            #[cfg(feature = "perf")]
            perf: Perf::new(),
        };
        vm
    }

    pub fn create_fiber(&mut self, tx: SyncSender<VMResult>, rx: Receiver<FiberMsg>) -> Self {
        let vm = VM {
            globals: self.globals,
            root_path: self.root_path.clone(),
            fiber_state: FiberState::Created,
            exec_context: vec![],
            temp_stack: vec![],
            class_context: self.class_context.clone(),
            exec_stack: vec![],
            exception: false,
            pc: 0,
            parent_fiber: Some(ParentFiberInfo::new(VMRef::from_ref(self), tx, rx)),
            #[cfg(feature = "perf")]
            perf: Perf::new(),
        };
        self.globals.fibers.push(VMRef::from_ref(&vm));
        vm
    }

    /// Set ALLOC to Globals' Allocator for Fiber.
    /// This method should be called in the thread where `self` is to be run.
    pub fn set_allocator(&self) {
        ALLOC.with(|a| {
            *a.borrow_mut() = Some(self.globals.allocator.clone());
        })
    }

    pub fn current_context(&self) -> ContextRef {
        self.exec_context.last().unwrap().to_owned()
    }

    pub fn latest_context(&self) -> Option<ContextRef> {
        self.exec_context.last().cloned()
    }

    pub fn source_info(&self) -> SourceInfoRef {
        match self.current_context().iseq_ref {
            Some(iseq) => iseq.source_info,
            None => SourceInfoRef::empty(),
        }
    }

    pub fn fiberstate_created(&mut self) {
        self.fiber_state = FiberState::Created;
    }

    pub fn fiberstate_running(&mut self) {
        self.fiber_state = FiberState::Running;
    }

    pub fn fiberstate_dead(&mut self) {
        self.fiber_state = FiberState::Dead;
    }

    pub fn fiberstate(&self) -> FiberState {
        self.fiber_state
    }

    pub fn is_dead(&self) -> bool {
        self.fiber_state == FiberState::Dead
    }

    pub fn is_running(&self) -> bool {
        self.fiber_state == FiberState::Running
    }

    pub fn stack_push(&mut self, val: Value) {
        self.exec_stack.push(val)
    }

    pub fn stack_pop(&mut self) -> Value {
        self.exec_stack.pop().unwrap()
    }

    pub fn stack_top(&mut self) -> Value {
        self.exec_stack.last().unwrap().clone()
    }

    pub fn stack_len(&self) -> usize {
        self.exec_stack.len()
    }

    pub fn set_stack_len(&mut self, len: usize) {
        self.exec_stack.truncate(len);
    }

    /// Push an object to the temporary area.
    pub fn temp_push(&mut self, v: Value) {
        self.temp_stack.push(v);
    }

    pub fn temp_push_args(&mut self, args: &Args) {
        for a in args.iter() {
            self.temp_stack.push(*a);
        }
    }

    /// Push objects to the temporary area.
    pub fn temp_push_vec(&mut self, vec: &mut Vec<Value>) {
        self.temp_stack.append(vec);
    }

    pub fn context_push(&mut self, ctx: ContextRef) {
        self.exec_context.push(ctx);
    }

    pub fn context_pop(&mut self) -> Option<ContextRef> {
        self.exec_context.pop()
    }

    pub fn clear(&mut self) {
        self.exec_stack.clear();
        self.class_context = vec![(Value::nil(), DefineMode::default())];
        self.exec_context.clear();
    }

    pub fn class_push(&mut self, val: Value) {
        self.class_context.push((val, DefineMode::default()));
    }

    pub fn class_pop(&mut self) {
        self.class_context.pop().unwrap();
    }

    pub fn classref(&self) -> ClassRef {
        let (class, _) = self.class_context.last().unwrap();
        if class.is_nil() {
            self.globals.object_class
        } else {
            class.as_module().unwrap()
        }
    }

    pub fn class(&self) -> Value {
        let (class, _) = self.class_context.last().unwrap();
        if class.is_nil() {
            self.globals.builtins.object
        } else {
            *class
        }
    }

    pub fn define_mode(&self) -> &DefineMode {
        &self.class_context.last().unwrap().1
    }

    #[cfg(not(tarpaulin_include))]
    pub fn define_mode_mut(&mut self) -> &mut DefineMode {
        &mut self.class_context.last_mut().unwrap().1
    }

    pub fn module_function(&mut self, flag: bool) {
        self.class_context.last_mut().unwrap().1.module_function = flag;
    }

    pub fn jump_pc(&mut self, inst_offset: i64, disp: i64) {
        self.pc = ((self.pc as i64) + inst_offset + disp) as usize;
    }

    fn read16(&self, iseq: &ISeq, offset: usize) -> u16 {
        let pc = self.pc + offset;
        let ptr = iseq[pc..pc + 1].as_ptr() as *const u16;
        unsafe { *ptr }
    }

    fn read32(&self, iseq: &ISeq, offset: usize) -> u32 {
        let pc = self.pc + offset;
        let ptr = iseq[pc..pc + 1].as_ptr() as *const u32;
        unsafe { *ptr }
    }

    fn read_usize(&self, iseq: &ISeq, offset: usize) -> usize {
        self.read32(iseq, offset) as usize
    }

    fn read_id(&self, iseq: &ISeq, offset: usize) -> IdentId {
        IdentId::from(self.read32(iseq, offset))
    }

    fn read_lvar_id(&self, iseq: &ISeq, offset: usize) -> LvarId {
        LvarId::from_usize(self.read_usize(iseq, offset))
    }

    fn read_methodref(&self, iseq: &ISeq, offset: usize) -> MethodRef {
        MethodRef::from(self.read32(iseq, offset))
    }

    fn read64(&self, iseq: &ISeq, offset: usize) -> u64 {
        let pc = self.pc + offset;
        let ptr = iseq[pc..pc + 1].as_ptr() as *const u64;
        unsafe { *ptr }
    }

    fn read8(&self, iseq: &ISeq, offset: usize) -> u8 {
        iseq[self.pc + offset]
    }

    fn read_disp(&self, iseq: &ISeq, offset: usize) -> i64 {
        self.read32(iseq, offset) as i32 as i64
    }

    pub fn parse_program(&mut self, path: PathBuf, program: &str) -> Result<MethodRef, RubyError> {
        let parser = Parser::new();
        //std::mem::swap(&mut parser.ident_table, &mut self.globals.ident_table);
        let result = parser.parse_program(path, program)?;
        //self.globals.ident_table = result.ident_table;

        #[cfg(feature = "perf")]
        self.perf.set_prev_inst(Perf::INVALID);

        let methodref = Codegen::new(result.source_info).gen_iseq(
            &mut self.globals,
            &vec![],
            &result.node,
            &result.lvar_collector,
            true,
            ContextKind::Method,
            None,
        )?;
        Ok(methodref)
    }

    pub fn parse_program_eval(
        &mut self,
        path: PathBuf,
        program: &str,
    ) -> Result<MethodRef, RubyError> {
        let parser = Parser::new();
        //std::mem::swap(&mut parser.ident_table, &mut self.globals.ident_table);
        let ext_lvar = self.current_context().iseq_ref.unwrap().lvar.clone();
        let result = parser.parse_program_eval(path, program, ext_lvar.clone())?;
        //self.globals.ident_table = result.ident_table;

        #[cfg(feature = "perf")]
        self.perf.set_prev_inst(Perf::INVALID);

        let mut codegen = Codegen::new(result.source_info);
        codegen.context_push(ext_lvar);
        let method = codegen.gen_iseq(
            &mut self.globals,
            &vec![],
            &result.node,
            &result.lvar_collector,
            true,
            ContextKind::Eval,
            None,
        )?;
        Ok(method)
    }

    pub fn run(&mut self, path: PathBuf, program: &str) -> VMResult {
        let method = self.parse_program(path, program)?;
        let self_value = self.globals.main_object;
        let arg = Args::new0();
        let val = self.eval_send(method, self_value, &arg)?;
        #[cfg(feature = "perf")]
        self.perf.get_perf(Perf::INVALID);

        let stack_len = self.exec_stack.len();
        if stack_len != 0 {
            eprintln!("Error: stack length is illegal. {}", stack_len);
        };

        Ok(val)
    }

    #[cfg(not(tarpaulin_include))]
    pub fn run_repl(&mut self, result: &ParseResult, mut context: ContextRef) -> VMResult {
        #[cfg(feature = "perf")]
        self.perf.set_prev_inst(Perf::CODEGEN);

        let methodref = Codegen::new(result.source_info).gen_iseq(
            &mut self.globals,
            &vec![],
            &result.node,
            &result.lvar_collector,
            true,
            ContextKind::Method,
            None,
        )?;
        let iseq = self.get_iseq(methodref)?;
        context.iseq_ref = Some(iseq);
        context.adjust_lvar_size();
        //context.pc = 0;

        let val = self.run_context(context)?;
        #[cfg(feature = "perf")]
        self.perf.get_perf(Perf::INVALID);

        let stack_len = self.exec_stack.len();
        if stack_len != 0 {
            eprintln!("Error: stack length is illegal. {}", stack_len);
        };

        Ok(val)
    }

    #[allow(dead_code)]
    #[cfg(not(tarpaulin_include))]
    pub fn dump_context(&self) {
        fn dump_single_context(context: ContextRef) {
            eprintln!("self: {:#?}", context.self_value);
            match context.iseq_ref {
                Some(iseq_ref) => {
                    for i in 0..iseq_ref.lvars {
                        let id = LvarId::from_usize(i);
                        let (k, _) = iseq_ref
                            .lvar
                            .table()
                            .iter()
                            .find(|(_, v)| **v == id)
                            .unwrap();
                        eprintln!("lvar({}): {:?} {:#?}", id.as_u32(), k, context[id]);
                    }
                }
                None => {}
            }
        }
        eprintln!("---dump");
        for (i, context) in self.exec_context.iter().rev().enumerate() {
            eprintln!("context: {}", i);
            dump_single_context(*context);
        }
        for v in &self.exec_stack {
            eprintln!("stack: {:#?}", *v);
        }
        eprintln!("---dump end");
    }
}

impl VM {
    fn gc(&mut self) {
        //self.gc_counter += 1;
        if !self.globals.gc_enabled {
            return;
        }
        if self.globals.allocator.is_allocated() {
            #[cfg(feature = "perf")]
            self.perf.get_perf(Perf::GC);
            self.globals.gc();
        };
    }

    fn unwind_context(&mut self, err: &mut RubyError) {
        self.context_pop().unwrap();
        if self.latest_context().is_some() {
            err.info.push((self.source_info(), self.get_loc()));
        };
    }

    fn handle_error(&mut self, mut err: RubyError) -> VMResult {
        let res = match err.kind {
            RubyErrorKind::MethodReturn(val) => {
                // Catch MethodReturn if this context is the target.
                let iseq = self.current_context().iseq_ref;
                if iseq.is_some() && iseq.unwrap().is_method() {
                    #[cfg(feature = "trace")]
                    println!("<--- METHOD_RETURN Ok({:?})", val);
                    Ok(val)
                } else {
                    self.unwind_context(&mut err);
                    #[cfg(feature = "trace")]
                    println!("<--- METHOD_RETURN");
                    Err(err)
                }
            }
            _ => {
                //self.dump_context();
                self.unwind_context(&mut err);
                #[cfg(feature = "trace")]
                println!("<--- Err({:?})", err.kind);
                Err(err)
            }
        };
        return res;
    }

    /// Evaluate expr, and return the value.
    fn try_get(&mut self, expr: VMResult) -> VMResult {
        match expr {
            Ok(val) => Ok(val),
            Err(err) => match err.kind {
                RubyErrorKind::BlockReturn(val) => Ok(val),
                _ => self.handle_error(err),
            },
        }
    }

    /// Evaluate expr. Stack is not changed.
    fn try_eval(&mut self, expr: Result<(), RubyError>) -> Result<(), RubyError> {
        match expr {
            Ok(_) => Ok(()),
            Err(err) => match err.kind {
                RubyErrorKind::BlockReturn(_) => Ok(()),
                _ => match self.handle_error(err) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(err),
                },
            },
        }
    }

    fn jmp_cond(&mut self, iseq: &ISeq, cond: bool, inst_offset: i64, dest_offset: usize) {
        if cond {
            self.jump_pc(inst_offset, 0);
        } else {
            let disp = self.read_disp(iseq, dest_offset);
            self.jump_pc(inst_offset, disp);
        }
    }

    pub fn run_context(&mut self, context: ContextRef) -> VMResult {
        let stack_len = self.exec_stack.len();
        let pc = self.pc;
        self.context_push(context);
        self.pc = 0;
        match self.run_context_main(context) {
            Ok(val) => {
                self.context_pop().unwrap();
                #[cfg(debug_assertions)]
                assert_eq!(stack_len, self.exec_stack.len());
                self.pc = pc;
                Ok(val)
            }
            Err(err) => {
                self.exec_stack.truncate(stack_len);
                self.pc = pc;
                Err(err)
            }
        }
    }

    /// Main routine for VM execution.
    fn run_context_main(&mut self, context: ContextRef) -> VMResult {
        #[cfg(feature = "trace")]
        {
            if self.parent_fiber.is_some() {
                print!("===>");
            } else {
                print!("--->");
            }
            println!(" {:?} {:?}", context.iseq_ref.unwrap().method, context.kind);
        }

        let iseq = &context.iseq_ref.unwrap().iseq;
        let self_oref = context.self_value.rvalue_mut();
        self.gc();

        /// Evaluate expr, and push return value to stack.
        macro_rules! try_push {
            ($eval:expr) => {
                match $eval {
                    Ok(val) => self.stack_push(val),
                    Err(err) => match err.kind {
                        RubyErrorKind::BlockReturn(val) => self.stack_push(val),
                        _ => return self.handle_error(err),
                    },
                };
            };
        }

        /// Evaluate expr, and return value.
        macro_rules! try_get_bool {
            ($eval:expr) => {
                match $eval {
                    Ok(b) => b,
                    Err(err) => match err.kind {
                        RubyErrorKind::BlockReturn(val) => self.val_to_bool(val),
                        _ => match self.handle_error(err) {
                            Ok(res) => self.val_to_bool(res),
                            Err(err) => return Err(err),
                        },
                    },
                };
            };
        }

        loop {
            #[cfg(feature = "perf")]
            self.perf.get_perf(iseq[self.pc]);
            #[cfg(feature = "trace")]
            {
                println!(
                    "{:>4x}:{:<15} stack:{}",
                    self.pc,
                    Inst::inst_info(&self.globals, context.iseq_ref.unwrap(), self.pc),
                    self.exec_stack.len()
                );
            }
            match iseq[self.pc] {
                Inst::END => {
                    // reached the end of the method or block.
                    // - the end of a method or block.
                    // - `return` in method.
                    // - `next` in block AND outer of loops.
                    let val = self.stack_pop();
                    #[cfg(feature = "trace")]
                    println!("<--- Ok({:?})", val);
                    return Ok(val);
                }
                Inst::RETURN => {
                    // - `break`  in block or eval AND outer of loops.
                    #[cfg(debug_assertions)]
                    assert!(context.kind == ISeqKind::Block || context.kind == ISeqKind::Other);
                    let val = self.stack_pop();
                    let err = self.error_block_return(val);
                    self.context_pop().unwrap();
                    #[cfg(feature = "trace")]
                    println!("<--- Err({:?})", err.kind);
                    return Err(err);
                }
                Inst::MRETURN => {
                    // - `return` in block
                    #[cfg(debug_assertions)]
                    assert_eq!(context.kind, ISeqKind::Block);
                    let val = self.stack_pop();
                    let err = self.error_method_return(val);
                    self.context_pop().unwrap();
                    #[cfg(feature = "trace")]
                    println!("<--- Err({:?})", err.kind);
                    return Err(err);
                }
                Inst::PUSH_NIL => {
                    self.stack_push(Value::nil());
                    self.pc += 1;
                }
                Inst::PUSH_TRUE => {
                    self.stack_push(Value::true_val());
                    self.pc += 1;
                }
                Inst::PUSH_FALSE => {
                    self.stack_push(Value::false_val());
                    self.pc += 1;
                }
                Inst::PUSH_SELF => {
                    self.stack_push(context.self_value);
                    self.pc += 1;
                }
                Inst::PUSH_FIXNUM => {
                    let num = self.read64(iseq, 1);
                    self.pc += 9;
                    self.stack_push(Value::fixnum(num as i64));
                }
                Inst::PUSH_FLONUM => {
                    let num = f64::from_bits(self.read64(iseq, 1));
                    self.pc += 9;
                    self.stack_push(Value::flonum(num));
                }
                Inst::PUSH_SYMBOL => {
                    let id = self.read_id(iseq, 1);
                    self.stack_push(Value::symbol(id));
                    self.pc += 5;
                }
                Inst::ADD => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let cache = self.read32(iseq, 1);
                    try_push!(self.eval_add(lhs, rhs, cache));
                    self.pc += 5;
                }
                Inst::ADDI => {
                    let lhs = self.stack_pop();
                    let i = self.read32(iseq, 1) as i32;
                    try_push!(self.eval_addi(lhs, i));
                    self.pc += 5;
                }
                Inst::SUB => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let cache = self.read32(iseq, 1);
                    try_push!(self.eval_sub(lhs, rhs, cache));
                    self.pc += 5;
                }
                Inst::SUBI => {
                    let lhs = self.stack_pop();
                    let i = self.read32(iseq, 1) as i32;
                    try_push!(self.eval_subi(lhs, i));
                    self.pc += 5;
                }
                Inst::MUL => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let cache = self.read32(iseq, 1);
                    try_push!(self.eval_mul(lhs, rhs, cache));
                    self.pc += 5;
                }
                Inst::POW => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    try_push!(self.eval_exp(lhs, rhs));
                    self.pc += 1;
                }
                Inst::DIV => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let cache = self.read32(iseq, 1);
                    try_push!(self.eval_div(lhs, rhs, cache));
                    self.pc += 5;
                }
                Inst::REM => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    try_push!(self.eval_rem(lhs, rhs));
                    self.pc += 1;
                }
                Inst::SHR => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    try_push!(self.eval_shr(lhs, rhs));
                    self.pc += 1;
                }
                Inst::SHL => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let cache = self.read32(iseq, 1);
                    try_push!(self.eval_shl(lhs, rhs, cache));
                    self.pc += 5;
                }
                Inst::BAND => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    try_push!(self.eval_bitand(lhs, rhs));
                    self.pc += 1;
                }
                Inst::B_ANDI => {
                    let lhs = self.stack_pop();
                    let i = self.read32(iseq, 1) as i32;
                    try_push!(self.eval_bitandi(lhs, i));
                    self.pc += 5;
                }
                Inst::BOR => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    try_push!(self.eval_bitor(lhs, rhs));
                    self.pc += 1;
                }
                Inst::B_ORI => {
                    let lhs = self.stack_pop();
                    let i = self.read32(iseq, 1) as i32;
                    try_push!(self.eval_bitori(lhs, i));
                    self.pc += 5;
                }
                Inst::BXOR => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    try_push!(self.eval_bitxor(lhs, rhs));
                    self.pc += 1;
                }
                Inst::BNOT => {
                    let lhs = self.stack_pop();
                    try_push!(self.eval_bitnot(lhs));
                    self.pc += 1;
                }

                Inst::EQ => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = Value::bool(self.eval_eq(rhs, lhs));
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::EQI => {
                    let lhs = self.stack_pop();
                    let i = self.read32(iseq, 1) as i32;
                    let val = Value::bool(self.eval_eqi(lhs, i));
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::NE => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = Value::bool(!self.eval_eq(rhs, lhs));
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::NEI => {
                    let lhs = self.stack_pop();
                    let i = self.read32(iseq, 1) as i32;
                    let val = Value::bool(!self.eval_eqi(lhs, i));
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::TEQ => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let res = self.eval_teq(rhs, lhs)?;
                    let val = Value::bool(res);
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::GT => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    try_push!(self.eval_gt(rhs, lhs).map(|x| Value::bool(x)));
                    self.pc += 1;
                }
                Inst::GTI => {
                    let lhs = self.stack_pop();
                    let i = self.read32(iseq, 1) as i32;
                    try_push!(self.eval_gti(lhs, i).map(|x| Value::bool(x)));
                    self.pc += 5;
                }
                Inst::GE => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    try_push!(self.eval_ge(rhs, lhs).map(|x| Value::bool(x)));
                    self.pc += 1;
                }
                Inst::GEI => {
                    let lhs = self.stack_pop();
                    let i = self.read32(iseq, 1) as i32;
                    try_push!(self.eval_gei(lhs, i).map(|x| Value::bool(x)));
                    self.pc += 5;
                }
                Inst::LT => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    try_push!(self.eval_lt(rhs, lhs).map(|x| Value::bool(x)));
                    self.pc += 1;
                }
                Inst::LTI => {
                    let lhs = self.stack_pop();
                    let i = self.read32(iseq, 1) as i32;
                    try_push!(self.eval_lti(lhs, i).map(|x| Value::bool(x)));
                    self.pc += 5;
                }
                Inst::LE => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    try_push!(self.eval_le(rhs, lhs).map(|x| Value::bool(x)));
                    self.pc += 1;
                }
                Inst::LEI => {
                    let lhs = self.stack_pop();
                    let i = self.read32(iseq, 1) as i32;
                    try_push!(self.eval_lei(lhs, i).map(|x| Value::bool(x)));
                    self.pc += 5;
                }
                Inst::CMP => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    try_push!(self.eval_cmp(rhs, lhs));
                    self.pc += 1;
                }
                Inst::NOT => {
                    let lhs = self.stack_pop();
                    let val = Value::bool(!self.val_to_bool(lhs));
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::CONCAT_STRING => {
                    let val = match self.read32(iseq, 1) as usize {
                        0 => Value::string(&self.globals.builtins, "".to_string()),
                        i => {
                            let mut res = match self.stack_pop().as_string() {
                                Some(s) => s.to_owned(),
                                None => unreachable!("Illegal CONCAT_STRING arguments."),
                            };
                            for _ in 0..i - 1 {
                                res = match self.stack_pop().as_string() {
                                    Some(lhs) => format!("{}{}", lhs, res),
                                    None => unreachable!("Illegal CONCAT_STRING arguments."),
                                };
                            }
                            Value::string(&self.globals.builtins, res)
                        }
                    };

                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::SET_LOCAL => {
                    let id = self.read_lvar_id(iseq, 1);
                    let val = self.stack_pop();
                    self.current_context()[id] = val;
                    self.pc += 5;
                }
                Inst::GET_LOCAL => {
                    let id = self.read_lvar_id(iseq, 1);
                    let val = self.current_context()[id];
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::LVAR_ADDI => {
                    let id = self.read_lvar_id(iseq, 1);
                    let i = self.read32(iseq, 5) as i32;
                    let val = self.current_context()[id];
                    self.current_context()[id] = self.eval_addi(val, i)?;
                    self.pc += 9;
                }
                Inst::SET_DYNLOCAL => {
                    let id = self.read_lvar_id(iseq, 1);
                    let outer = self.read32(iseq, 5);
                    let val = self.stack_pop();
                    let mut cref = self.get_outer_context(outer);
                    cref[id] = val;
                    self.pc += 9;
                }
                Inst::GET_DYNLOCAL => {
                    let id = self.read_lvar_id(iseq, 1);
                    let outer = self.read32(iseq, 5);
                    let cref = self.get_outer_context(outer);
                    let val = cref[id];
                    self.stack_push(val);
                    self.pc += 9;
                }
                Inst::CHECK_LOCAL => {
                    let id = self.read_lvar_id(iseq, 1);
                    let outer = self.read32(iseq, 5);
                    let cref = self.get_outer_context(outer);
                    let val = cref[id].is_uninitialized();
                    self.stack_push(Value::bool(val));
                    self.pc += 9;
                }
                Inst::SET_CONST => {
                    let id = self.read_id(iseq, 1);
                    let mut parent = match self.stack_pop() {
                        v if v.is_nil() => self.class(),
                        v => v,
                    };
                    let val = self.stack_pop();
                    match val.as_module() {
                        Some(mut cref) => {
                            if cref.name == None {
                                cref.name = Some(id);
                            }
                        }
                        None => {}
                    }
                    parent.set_var(id, val);
                    self.pc += 5;
                }
                Inst::GET_CONST => {
                    let id = self.read_id(iseq, 1);
                    let val = match self.get_env_const(id) {
                        Some(val) => val,
                        None => self.get_super_const(self.class(), id)?,
                    };
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::GET_CONST_TOP => {
                    let id = self.read_id(iseq, 1);
                    let class = self.globals.builtins.object;
                    let val = self.get_super_const(class, id)?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::GET_SCOPE => {
                    let parent = self.stack_pop();
                    let id = self.read_id(iseq, 1);
                    let val = self.get_super_const(parent, id)?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::SET_IVAR => {
                    let var_id = self.read_id(iseq, 1);
                    let new_val = self.stack_pop();
                    self_oref.set_var(var_id, new_val);
                    self.pc += 5;
                }
                Inst::GET_IVAR => {
                    let var_id = self.read_id(iseq, 1);
                    let val = match self_oref.get_var(var_id) {
                        Some(val) => val.clone(),
                        None => Value::nil(),
                    };
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::IVAR_ADDI => {
                    let var_id = self.read_id(iseq, 1);
                    let i = self.read32(iseq, 5) as i32;
                    let v = self_oref
                        .var_table_mut()
                        .entry(var_id)
                        .or_insert(Value::nil());
                    *v = self.eval_addi(*v, i)?;

                    self.pc += 9;
                }
                Inst::SET_GVAR => {
                    let var_id = self.read_id(iseq, 1);
                    let new_val = self.stack_pop();
                    self.set_global_var(var_id, new_val);
                    self.pc += 5;
                }
                Inst::GET_GVAR => {
                    let var_id = self.read_id(iseq, 1);
                    let val = self.get_global_var(var_id);
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::SET_INDEX => {
                    let arg_num = self.read_usize(iseq, 1);
                    let res = self.set_index(arg_num);
                    self.try_eval(res)?;
                    self.pc += 5;
                }
                Inst::GET_INDEX => {
                    let arg_num = self.read_usize(iseq, 1);
                    try_push!(self.get_index(arg_num));
                    self.pc += 5;
                }
                Inst::OPT_SET_INDEX => {
                    let idx = self.read32(iseq, 1);
                    let res = self.opt_set_index(idx);
                    self.try_eval(res)?;
                    self.pc += 5;
                }
                Inst::OPT_GET_INDEX => {
                    let idx = self.read32(iseq, 1);
                    try_push!(self.opt_get_index(idx));
                    self.pc += 5;
                }
                Inst::SPLAT => {
                    let val = self.stack_pop();
                    let res = Value::splat(&self.globals, val);
                    self.stack_push(res);
                    self.pc += 1;
                }
                Inst::CONST_VAL => {
                    let id = self.read_usize(iseq, 1);
                    let val = self.globals.const_values.get(id);
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::CREATE_RANGE => {
                    let start = self.stack_pop();
                    let end = self.stack_pop();
                    if !start.is_packed_fixnum() || !end.is_packed_fixnum() {
                        return Err(self.error_argument("Bad value for range."));
                    };
                    let exclude_val = self.stack_pop();
                    let exclude_end = self.val_to_bool(exclude_val);
                    let range = Value::range(&self.globals, start, end, exclude_end);
                    self.stack_push(range);
                    self.pc += 1;
                }
                Inst::CREATE_ARRAY => {
                    let arg_num = self.read_usize(iseq, 1);
                    let elems = self.pop_args_to_ary(arg_num).into_vec();
                    let array = Value::array_from(&self.globals, elems);
                    self.stack_push(array);
                    self.pc += 5;
                }
                Inst::CREATE_PROC => {
                    let method = self.read_methodref(iseq, 1);
                    let proc_obj = self.create_proc(method)?;
                    self.stack_push(proc_obj);
                    self.pc += 5;
                }
                Inst::CREATE_HASH => {
                    let arg_num = self.read_usize(iseq, 1);
                    let key_value = self.pop_key_value_pair(arg_num);
                    let hash = Value::hash_from_map(&self.globals, key_value);
                    self.stack_push(hash);
                    self.pc += 5;
                }
                Inst::CREATE_REGEXP => {
                    let arg = self.stack_pop();
                    let regexp = self.create_regexp(arg)?;
                    self.stack_push(regexp);
                    self.pc += 1;
                }
                Inst::JMP => {
                    let disp = self.read_disp(iseq, 1);
                    if 0 < disp {
                        self.gc();
                    }
                    self.jump_pc(5, disp);
                }
                Inst::JMP_F => {
                    let val = self.stack_pop();
                    let b = self.val_to_bool(val);
                    self.jmp_cond(iseq, b, 5, 1);
                }
                Inst::JMP_T => {
                    let val = self.stack_pop();
                    let b = !self.val_to_bool(val);
                    self.jmp_cond(iseq, b, 5, 1);
                }

                Inst::JMP_F_EQ => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let b = self.eval_eq(rhs, lhs);
                    self.jmp_cond(iseq, b, 5, 1);
                }
                Inst::JMP_F_NE => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let b = !self.eval_eq(rhs, lhs);
                    self.jmp_cond(iseq, b, 5, 1);
                }
                Inst::JMP_F_GT => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let b = try_get_bool!(self.eval_gt(rhs, lhs));
                    self.jmp_cond(iseq, b, 5, 1);
                }
                Inst::JMP_F_GE => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let b = try_get_bool!(self.eval_ge(rhs, lhs));
                    self.jmp_cond(iseq, b, 5, 1);
                }
                Inst::JMP_F_LT => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let b = try_get_bool!(self.eval_lt(rhs, lhs));
                    self.jmp_cond(iseq, b, 5, 1);
                }
                Inst::JMP_F_LE => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let b = try_get_bool!(self.eval_le(rhs, lhs));
                    self.jmp_cond(iseq, b, 5, 1);
                }

                Inst::JMP_F_EQI => {
                    let lhs = self.stack_pop();
                    let i = self.read32(iseq, 1) as i32;
                    let b = self.eval_eqi(lhs, i);
                    self.jmp_cond(iseq, b, 9, 5);
                }
                Inst::JMP_F_NEI => {
                    let lhs = self.stack_pop();
                    let i = self.read32(iseq, 1) as i32;
                    let b = !self.eval_eqi(lhs, i);
                    self.jmp_cond(iseq, b, 9, 5);
                }
                Inst::JMP_F_GTI => {
                    let lhs = self.stack_pop();
                    let i = self.read32(iseq, 1) as i32;
                    let b = try_get_bool!(self.eval_gti(lhs, i));
                    self.jmp_cond(iseq, b, 9, 5);
                }
                Inst::JMP_F_GEI => {
                    let lhs = self.stack_pop();
                    let i = self.read32(iseq, 1) as i32;
                    let b = try_get_bool!(self.eval_gei(lhs, i));
                    self.jmp_cond(iseq, b, 9, 5);
                }
                Inst::JMP_F_LTI => {
                    let lhs = self.stack_pop();
                    let i = self.read32(iseq, 1) as i32;
                    let b = try_get_bool!(self.eval_lti(lhs, i));
                    self.jmp_cond(iseq, b, 9, 5);
                }
                Inst::JMP_F_LEI => {
                    let lhs = self.stack_pop();
                    let i = self.read32(iseq, 1) as i32;
                    let b = try_get_bool!(self.eval_lei(lhs, i));
                    self.jmp_cond(iseq, b, 9, 5);
                }

                Inst::OPT_CASE => {
                    let val = self.stack_pop();
                    let map = self.globals.case_dispatch.get_entry(self.read32(iseq, 1));
                    let disp = match map.get(&val) {
                        Some(disp) => *disp as i64,
                        None => self.read_disp(iseq, 5),
                    };
                    self.jump_pc(9, disp);
                }
                Inst::SEND => {
                    let receiver = self.stack_pop();
                    try_push!(self.vm_send(iseq, receiver));
                    self.pc += 17;
                }
                Inst::SEND_SELF => {
                    let receiver = context.self_value;
                    try_push!(self.vm_send(iseq, receiver));
                    self.pc += 17;
                }
                Inst::OPT_SEND => {
                    let receiver = self.stack_pop();
                    try_push!(self.vm_opt_send(iseq, receiver));
                    self.pc += 11;
                }
                Inst::OPT_SEND_SELF => {
                    let receiver = context.self_value;
                    try_push!(self.vm_opt_send(iseq, receiver));
                    self.pc += 11;
                }
                Inst::YIELD => {
                    let args_num = self.read32(iseq, 1) as usize;
                    let args = self.pop_args_to_ary(args_num);
                    try_push!(self.eval_yield(&args));
                    self.pc += 5;
                }
                Inst::DEF_CLASS => {
                    let is_module = self.read8(iseq, 1) == 1;
                    let id = self.read_id(iseq, 2);
                    let method = self.read_methodref(iseq, 6);
                    let super_val = self.stack_pop();
                    let res = self.define_class(id, is_module, super_val);
                    let val = self.try_get(res)?;

                    self.class_push(val);
                    let mut iseq = self.get_iseq(method)?;
                    iseq.class_defined = self.get_class_defined(val);
                    let arg = Args::new0();
                    try_push!(self.eval_send(method, val, &arg));
                    self.pc += 10;
                    self.class_pop();
                }
                Inst::DEF_METHOD => {
                    let id = self.read_id(iseq, 1);
                    let method = self.read_methodref(iseq, 5);
                    let mut iseq = self.get_iseq(method)?;
                    iseq.class_defined = self.get_class_defined(None);
                    self.define_method(context.self_value, id, method);
                    if self.define_mode().module_function {
                        self.define_singleton_method(context.self_value, id, method)?;
                    };
                    self.pc += 9;
                }
                Inst::DEF_SMETHOD => {
                    let id = self.read_id(iseq, 1);
                    let method = self.read_methodref(iseq, 5);
                    let mut iseq = self.get_iseq(method)?;
                    iseq.class_defined = self.get_class_defined(None);
                    //let _singleton = self.stack_pop();
                    self.define_singleton_method(context.self_value, id, method)?;
                    if self.define_mode().module_function {
                        self.define_method(context.self_value, id, method);
                    };
                    self.pc += 9;
                }
                Inst::TO_S => {
                    let val = self.stack_pop();
                    let s = self.val_to_s(val);
                    let res = Value::string(&self.globals.builtins, s);
                    self.stack_push(res);
                    self.pc += 1;
                }
                Inst::POP => {
                    self.stack_pop();
                    self.pc += 1;
                }
                Inst::DUP => {
                    let len = self.read_usize(iseq, 1);
                    let stack_len = self.exec_stack.len();
                    for i in stack_len - len..stack_len {
                        let val = self.exec_stack[i];
                        self.stack_push(val);
                    }
                    self.pc += 5;
                }
                Inst::TAKE => {
                    let len = self.read_usize(iseq, 1);
                    let val = self.stack_pop();
                    match val.as_array() {
                        Some(info) => {
                            let elem = &info.elements;
                            let ary_len = elem.len();
                            if len <= ary_len {
                                for i in 0..len {
                                    self.stack_push(elem[i]);
                                }
                            } else {
                                for i in 0..ary_len {
                                    self.stack_push(elem[i]);
                                }
                                for _ in ary_len..len {
                                    self.stack_push(Value::nil());
                                }
                            }
                        }
                        None => {
                            self.stack_push(val);
                            for _ in 0..len - 1 {
                                self.stack_push(Value::nil());
                            }
                        }
                    }

                    self.pc += 5;
                }
                _ => return Err(self.error_unimplemented("Unimplemented instruction.")),
            }
        }
    }
}

impl VM {
    pub fn error_nomethod(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(
            RuntimeErrKind::NoMethod,
            msg.into(),
            self.source_info(),
            loc,
        )
    }

    pub fn error_undefined_op(
        &self,
        method_name: impl Into<String>,
        rhs: Value,
        lhs: Value,
    ) -> RubyError {
        self.error_nomethod(format!(
            "undefined method `{}' {} for {}",
            method_name.into(),
            self.globals.get_class_name(rhs),
            self.globals.get_class_name(lhs)
        ))
    }

    pub fn error_undefined_method(&self, method: IdentId, receiver: Value) -> RubyError {
        self.error_nomethod(format!(
            "undefined method `{:?}' for {}",
            method,
            self.globals.get_class_name(receiver)
        ))
    }

    pub fn error_unimplemented(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(
            RuntimeErrKind::Unimplemented,
            msg.into(),
            self.source_info(),
            loc,
        )
    }

    pub fn error_internal(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(
            RuntimeErrKind::Internal,
            msg.into(),
            self.source_info(),
            loc,
        )
    }

    pub fn error_name(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(RuntimeErrKind::Name, msg.into(), self.source_info(), loc)
    }

    pub fn error_type(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(RuntimeErrKind::Type, msg.into(), self.source_info(), loc)
    }

    pub fn error_argument(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(
            RuntimeErrKind::Argument,
            msg.into(),
            self.source_info(),
            loc,
        )
    }

    pub fn error_regexp(&self, err: fancy_regex::Error) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(
            RuntimeErrKind::Regexp,
            format!("Invalid string for a regular expression. {:?}", err),
            self.source_info(),
            loc,
        )
    }

    pub fn error_index(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(RuntimeErrKind::Index, msg.into(), self.source_info(), loc)
    }

    pub fn error_fiber(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(RuntimeErrKind::Fiber, msg.into(), self.source_info(), loc)
    }

    pub fn error_stop_iteration(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(
            RuntimeErrKind::StopIteration,
            msg.into(),
            self.source_info(),
            loc,
        )
    }

    pub fn error_method_return(&self, val: Value) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_method_return(val, self.source_info(), loc)
    }

    pub fn error_local_jump(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(
            RuntimeErrKind::LocalJump,
            msg.into(),
            self.source_info(),
            loc,
        )
    }

    pub fn error_block_return(&self, val: Value) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_block_return(val, self.source_info(), loc)
    }

    pub fn check_args_num(&self, len: usize, num: usize) -> Result<(), RubyError> {
        if len == num {
            Ok(())
        } else {
            Err(self.error_argument(format!(
                "Wrong number of arguments. (given {}, expected {})",
                len, num
            )))
        }
    }

    pub fn check_args_range(&self, len: usize, min: usize, max: usize) -> Result<(), RubyError> {
        if min <= len && len <= max {
            Ok(())
        } else {
            Err(self.error_argument(format!(
                "Wrong number of arguments. (given {}, expected {}..{})",
                len, min, max
            )))
        }
    }

    pub fn check_args_min(&self, len: usize, min: usize) -> Result<(), RubyError> {
        if min <= len {
            Ok(())
        } else {
            Err(self.error_argument(format!(
                "Wrong number of arguments. (given {}, expected {}+)",
                len, min
            )))
        }
    }
}

impl VM {
    pub fn expect_block(&self, block: Option<MethodRef>) -> Result<MethodRef, RubyError> {
        match block {
            Some(method) => Ok(method),
            None => return Err(self.error_argument("Currently, needs block.")),
        }
    }

    pub fn expect_integer(&mut self, val: Value, msg: &str) -> Result<i64, RubyError> {
        val.as_fixnum().ok_or_else(|| {
            let inspect = self.val_inspect(val);
            self.error_type(format!("{} must be Integer. (given:{})", msg, inspect))
        })
    }

    pub fn expect_flonum(&mut self, val: Value, msg: &str) -> Result<f64, RubyError> {
        val.as_flonum().ok_or_else(|| {
            let inspect = self.val_inspect(val);
            self.error_type(format!("{} must be Float. (given:{})", msg, inspect))
        })
    }

    /// Returns `ClassRef` if `self` is a Class.
    /// When `self` is not a Class, returns `TypeError`.
    pub fn expect_class(&mut self, val: Value, msg: &str) -> Result<ClassRef, RubyError> {
        val.is_class().ok_or_else(|| {
            let val = self.val_inspect(val);
            self.error_type(format!("{} must be Class. (given:{})", msg, val))
        })
    }

    pub fn expect_module(&mut self, val: Value) -> Result<ClassRef, RubyError> {
        val.as_module().ok_or_else(|| {
            let val = self.val_inspect(val);
            self.error_type(format!("Must be Module or Class. (given:{})", val))
        })
    }
}

impl VM {
    fn get_loc(&self) -> Loc {
        match self.current_context().iseq_ref {
            None => Loc(1, 1),
            Some(iseq) => {
                iseq.iseq_sourcemap
                    .iter()
                    .find(|x| x.0 == ISeqPos::from(self.pc))
                    .unwrap_or(&(ISeqPos::from(0), Loc(0, 0)))
                    .1
            }
        }
    }

    /// Get class list from the nearest exec context.
    fn get_nearest_class_stack(&self) -> Option<ClassListRef> {
        for context in self.exec_context.iter().rev() {
            match context.iseq_ref.unwrap().class_defined {
                Some(class_list) => return Some(class_list),
                None => {}
            }
        }
        None
    }

    /// Get class list in the current context.
    ///
    /// At first, this method searches the class list of outer context,
    /// and adds a class given as an argument `new_class` on the top of the list.
    /// return None in top-level.
    fn get_class_defined(&self, new_class: impl Into<Option<Value>>) -> Option<ClassListRef> {
        let new_class = new_class.into();
        match new_class {
            Some(class) => {
                let outer = self.get_nearest_class_stack();
                let class_list = ClassList::new(outer, class);
                Some(ClassListRef::new(class_list))
            }
            None => self.get_nearest_class_stack(),
        }
    }

    // Search class stack for the constant.
    fn get_env_const(&self, id: IdentId) -> Option<Value> {
        let mut class_list = match self.get_nearest_class_stack() {
            Some(list) => list,
            None => return None,
        };
        loop {
            match class_list.class.get_var(id) {
                Some(val) => return Some(val),
                None => {}
            }
            class_list = match class_list.outer {
                Some(class) => class,
                None => return None,
            };
        }
    }

    /// Search class inheritance chain for the constant.
    pub fn get_super_const(&self, mut class: Value, id: IdentId) -> VMResult {
        loop {
            match class.get_var(id) {
                Some(val) => {
                    return Ok(val);
                }
                None => match class.superclass() {
                    Some(superclass) => {
                        class = superclass;
                    }
                    None => {
                        return Err(self.error_name(format!("Uninitialized constant {:?}.", id)));
                    }
                },
            }
        }
    }

    pub fn get_global_var(&self, id: IdentId) -> Value {
        match self.globals.global_var.get(&id) {
            Some(val) => val.clone(),
            None => Value::nil(),
        }
    }

    pub fn set_global_var(&mut self, id: IdentId, val: Value) {
        self.globals.global_var.insert(id, val);
    }
}

// Utilities for method call

impl VM {
    /// Get a method from the method cache if saved in it.
    /// Otherwise, search a class chain for the method.
    fn get_method_from_cache(
        &mut self,
        cache_slot: u32,
        rec_class: Value,
        method_id: IdentId,
    ) -> Result<MethodRef, RubyError> {
        /*if rec_class.is_nil() {
            return Err(self.error_unimplemented("receiver's class is nil."));
        };*/
        match self
            .globals
            .get_method_from_inline_cache(cache_slot, rec_class)
        {
            Some(method) => Ok(method),
            _ => {
                let method = self.get_instance_method(rec_class, method_id)?;
                self.globals
                    .set_inline_cache_entry(cache_slot, rec_class, method);
                Ok(method)
            }
        }
    }

    fn fallback(&mut self, method_id: IdentId, receiver: Value, args: &Args) -> VMResult {
        match self.get_method(receiver, method_id) {
            Ok(mref) => {
                let val = self.eval_send(mref, receiver, args)?;
                Ok(val)
            }
            Err(_) => Err(self.error_undefined_method(method_id, receiver)),
        }
    }

    fn fallback_for_binop(&mut self, method: IdentId, lhs: Value, rhs: Value) -> VMResult {
        match self.get_method(lhs, method) {
            Ok(mref) => {
                let arg = Args::new1(rhs);
                let val = self.eval_send(mref, lhs, &arg)?;
                Ok(val)
            }
            Err(_) => {
                let name = IdentId::get_ident_name(method);
                Err(self.error_undefined_op(name, rhs, lhs))
            }
        }
    }

    fn fallback_cache_for_binop(
        &mut self,
        lhs: Value,
        rhs: Value,
        method: IdentId,
        cache: u32,
    ) -> VMResult {
        let rec_class = lhs.get_class_object_for_method(&self.globals);
        let methodref = self.get_method_from_cache(cache, rec_class, method)?;
        let arg = Args::new1(rhs);
        self.eval_send(methodref, lhs, &arg)
    }
}

macro_rules! eval_op_i {
    ($vm:ident, $iseq:ident, $lhs:expr, $i:ident, $op:ident, $id:expr) => {
        if $lhs.is_packed_fixnum() {
            return Ok(Value::fixnum($lhs.as_packed_fixnum().$op($i as i64)));
        } else if $lhs.is_packed_num() {
            return Ok(Value::flonum($lhs.as_packed_flonum().$op($i as f64)));
        }
        let val = match $lhs.unpack() {
            RV::Integer(lhs) => Value::fixnum(lhs.$op($i as i64)),
            RV::Float(lhs) => Value::flonum(lhs.$op($i as f64)),
            _ => return $vm.fallback_for_binop($id, $lhs, Value::fixnum($i as i64)),
        };
        return Ok(val);
    };
}

macro_rules! eval_op {
    ($vm:ident, $cache:expr, $rhs:expr, $lhs:expr, $op:ident, $id:expr) => {
        if $lhs.is_packed_fixnum() {
            let lhs = $lhs.as_packed_fixnum();
            if $rhs.is_packed_fixnum() {
                let rhs = $rhs.as_packed_fixnum();
                return Ok(Value::fixnum(lhs.$op(rhs)));
            } else if $rhs.is_packed_num() {
                let rhs = $rhs.as_packed_flonum();
                return Ok(Value::flonum((lhs as f64).$op(rhs)));
            }
        } else if $lhs.is_packed_num() {
            let lhs = $lhs.as_packed_flonum();
            if $rhs.is_packed_fixnum() {
                let rhs = $rhs.as_packed_fixnum();
                return Ok(Value::flonum(lhs.$op(rhs as f64)));
            } else if $rhs.is_packed_num() {
                let rhs = $rhs.as_packed_flonum();
                return Ok(Value::flonum(lhs.$op(rhs)));
            }
        }
        let val = match ($lhs.unpack(), $rhs.unpack()) {
            (RV::Integer(lhs), RV::Integer(rhs)) => Value::fixnum(lhs.$op(rhs)),
            (RV::Integer(lhs), RV::Float(rhs)) => Value::flonum((lhs as f64).$op(rhs)),
            (RV::Float(lhs), RV::Integer(rhs)) => Value::flonum(lhs.$op(rhs as f64)),
            (RV::Float(lhs), RV::Float(rhs)) => Value::flonum(lhs.$op(rhs)),
            _ => {
                return $vm.fallback_cache_for_binop($lhs, $rhs, $id, $cache);
            }
        };
        return Ok(val);
    };
}

impl VM {
    fn eval_add(&mut self, rhs: Value, lhs: Value, cache: u32) -> VMResult {
        use std::ops::Add;
        eval_op!(self, cache, rhs, lhs, add, IdentId::_ADD);
    }

    fn eval_sub(&mut self, rhs: Value, lhs: Value, cache: u32) -> VMResult {
        use std::ops::Sub;
        eval_op!(self, cache, rhs, lhs, sub, IdentId::_SUB);
    }

    fn eval_mul(&mut self, rhs: Value, lhs: Value, cache: u32) -> VMResult {
        use std::ops::Mul;
        eval_op!(self, cache, rhs, lhs, mul, IdentId::_MUL);
    }

    fn eval_addi(&mut self, lhs: Value, i: i32) -> VMResult {
        use std::ops::Add;
        eval_op_i!(self, iseq, lhs, i, add, IdentId::_ADD);
    }

    fn eval_subi(&mut self, lhs: Value, i: i32) -> VMResult {
        use std::ops::Sub;
        eval_op_i!(self, iseq, lhs, i, sub, IdentId::_SUB);
    }

    fn eval_div(&mut self, rhs: Value, lhs: Value, cache: u32) -> VMResult {
        use std::ops::Div;
        eval_op!(self, cache, rhs, lhs, div, IdentId::_DIV);
    }

    fn eval_rem(&mut self, rhs: Value, lhs: Value) -> VMResult {
        fn rem_floorf64(self_: f64, other: f64) -> f64 {
            if self_ > 0.0 && other < 0.0 {
                ((self_ - 1.0) % other) + other + 1.0
            } else if self_ < 0.0 && other > 0.0 {
                ((self_ + 1.0) % other) + other - 1.0
            } else {
                self_ % other
            }
        }
        use divrem::*;
        let val = match (lhs.unpack(), rhs.unpack()) {
            (RV::Integer(lhs), RV::Integer(rhs)) => Value::fixnum(lhs.rem_floor(rhs)),
            (RV::Integer(lhs), RV::Float(rhs)) => Value::flonum(rem_floorf64(lhs as f64, rhs)),
            (RV::Float(lhs), RV::Integer(rhs)) => Value::flonum(rem_floorf64(lhs, rhs as f64)),
            (RV::Float(lhs), RV::Float(rhs)) => Value::flonum(rem_floorf64(lhs, rhs)),
            (_, _) => return self.fallback_for_binop(IdentId::_REM, lhs, rhs),
        };
        Ok(val)
    }

    fn eval_exp(&mut self, rhs: Value, lhs: Value) -> VMResult {
        let val = match (lhs.unpack(), rhs.unpack()) {
            (RV::Integer(lhs), RV::Integer(rhs)) => {
                if 0 <= rhs && rhs <= std::u32::MAX as i64 {
                    Value::fixnum(lhs.pow(rhs as u32))
                } else {
                    Value::flonum((lhs as f64).powf(rhs as f64))
                }
            }
            (RV::Integer(lhs), RV::Float(rhs)) => Value::flonum((lhs as f64).powf(rhs)),
            (RV::Float(lhs), RV::Integer(rhs)) => Value::flonum(lhs.powf(rhs as f64)),
            (RV::Float(lhs), RV::Float(rhs)) => Value::flonum(lhs.powf(rhs)),
            _ => {
                return self.fallback_for_binop(IdentId::_POW, lhs, rhs);
            }
        };
        Ok(val)
    }

    fn eval_shl(&mut self, rhs: Value, mut lhs: Value, cache: u32) -> VMResult {
        if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            return Ok(Value::fixnum(
                lhs.as_packed_fixnum() << rhs.as_packed_fixnum(),
            ));
        }
        match lhs.as_mut_rvalue() {
            None => match lhs.unpack() {
                RV::Integer(lhs) => {
                    match rhs.as_fixnum() {
                        Some(rhs) => return Ok(Value::fixnum(lhs << rhs)),
                        _ => {}
                    };
                }
                _ => {}
            },
            Some(lhs_o) => match lhs_o.kind {
                ObjKind::Array(ref mut aref) => {
                    aref.elements.push(rhs);
                    return Ok(lhs);
                }
                _ => {}
            },
        };
        let val = self.fallback_cache_for_binop(lhs, rhs, IdentId::_SHL, cache)?;
        Ok(val)
    }

    fn eval_shr(&mut self, rhs: Value, lhs: Value) -> VMResult {
        if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            return Ok(Value::fixnum(
                lhs.as_packed_fixnum() >> rhs.as_packed_fixnum(),
            ));
        }
        match (lhs.unpack(), rhs.unpack()) {
            (RV::Integer(lhs), RV::Integer(rhs)) => Ok(Value::fixnum(lhs >> rhs)),
            (_, _) => return Err(self.error_undefined_op(">>", rhs, lhs)),
        }
    }

    fn eval_bitand(&mut self, rhs: Value, lhs: Value) -> VMResult {
        if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            return Ok(Value::fixnum(
                lhs.as_packed_fixnum() & rhs.as_packed_fixnum(),
            ));
        }
        match (lhs.unpack(), rhs.unpack()) {
            (RV::Integer(lhs), RV::Integer(rhs)) => Ok(Value::fixnum(lhs & rhs)),
            (_, _) => return Err(self.error_undefined_op("&", rhs, lhs)),
        }
    }

    fn eval_bitandi(&mut self, lhs: Value, i: i32) -> VMResult {
        let i = i as i64;
        if lhs.is_packed_fixnum() {
            return Ok(Value::fixnum(lhs.as_packed_fixnum() & i));
        }
        match lhs.unpack() {
            RV::Integer(lhs) => Ok(Value::fixnum(lhs & i)),
            _ => return Err(self.error_undefined_op("&", Value::fixnum(i), lhs)),
        }
    }

    fn eval_bitor(&mut self, rhs: Value, lhs: Value) -> VMResult {
        if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            return Ok(Value::fixnum(
                lhs.as_packed_fixnum() | rhs.as_packed_fixnum(),
            ));
        }
        match (lhs.unpack(), rhs.unpack()) {
            (RV::Integer(lhs), RV::Integer(rhs)) => Ok(Value::fixnum(lhs | rhs)),
            (_, _) => return Err(self.error_undefined_op("|", rhs, lhs)),
        }
    }

    fn eval_bitori(&mut self, lhs: Value, i: i32) -> VMResult {
        let i = i as i64;
        if lhs.is_packed_fixnum() {
            return Ok(Value::fixnum(lhs.as_packed_fixnum() | i));
        }
        match lhs.unpack() {
            RV::Integer(lhs) => Ok(Value::fixnum(lhs | i)),
            _ => return Err(self.error_undefined_op("|", Value::fixnum(i), lhs)),
        }
    }

    fn eval_bitxor(&mut self, rhs: Value, lhs: Value) -> VMResult {
        if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            return Ok(Value::fixnum(
                lhs.as_packed_fixnum() ^ rhs.as_packed_fixnum(),
            ));
        }
        match (lhs.unpack(), rhs.unpack()) {
            (RV::Integer(lhs), RV::Integer(rhs)) => Ok(Value::fixnum(lhs ^ rhs)),
            (_, _) => return Err(self.error_undefined_op("^", rhs, lhs)),
        }
    }

    fn eval_bitnot(&mut self, lhs: Value) -> VMResult {
        match lhs.unpack() {
            RV::Integer(lhs) => Ok(Value::fixnum(!lhs)),
            _ => Err(self.error_nomethod("NoMethodError: '~'")),
        }
    }
}

macro_rules! eval_cmp_sub {
    ($vm:ident, $rhs:expr, $lhs:expr, $op:ident, $id:expr) => {
        match ($lhs.unpack(), $rhs.unpack()) {
            (RV::Integer(lhs), RV::Integer(rhs)) => Ok(lhs.$op(&rhs)),
            (RV::Float(lhs), RV::Integer(rhs)) => Ok(lhs.$op(&(rhs as f64))),
            (RV::Integer(lhs), RV::Float(rhs)) => Ok((lhs as f64).$op(&rhs)),
            (RV::Float(lhs), RV::Float(rhs)) => Ok(lhs.$op(&rhs)),
            (_, _) => {
                let res = $vm.fallback_for_binop($id, $lhs, $rhs);
                res.map(|x| $vm.val_to_bool(x))
            }
        }
    };
}

macro_rules! eval_cmp {
    ($vm:ident, $rhs:expr, $lhs:expr, $op:ident, $id:expr) => {
        if $lhs.is_packed_fixnum() {
            let lhs = $lhs.as_packed_fixnum();
            if $rhs.is_packed_fixnum() {
                let rhs = $rhs.as_packed_fixnum();
                Ok(lhs.$op(&rhs))
            } else if $rhs.is_packed_num() {
                let rhs = $rhs.as_packed_flonum();
                Ok((lhs as f64).$op(&rhs))
            } else {
                eval_cmp_sub!($vm, $rhs, $lhs, $op, $id)
            }
        } else if $lhs.is_packed_num() {
            let lhs = $lhs.as_packed_flonum();
            if $rhs.is_packed_fixnum() {
                let rhs = $rhs.as_packed_fixnum();
                Ok(lhs.$op(&(rhs as f64)))
            } else if $rhs.is_packed_num() {
                let rhs = $rhs.as_packed_flonum();
                Ok(lhs.$op(&rhs))
            } else {
                eval_cmp_sub!($vm, $rhs, $lhs, $op, $id)
            }
        } else {
            eval_cmp_sub!($vm, $rhs, $lhs, $op, $id)
        }
    };
}

macro_rules! eval_cmp_i {
    ($vm:ident, $lhs:expr, $i:expr, $op:ident, $id:expr) => {
        if $lhs.is_packed_fixnum() {
            let i = $i as i64;
            Ok($lhs.as_packed_fixnum().$op(&i))
        } else if $lhs.is_packed_num() {
            let i = $i as f64;
            Ok($lhs.as_packed_flonum().$op(&i))
        } else {
            match $lhs.unpack() {
                RV::Integer(lhs) => Ok(lhs.$op(&($i as i64))),
                RV::Float(lhs) => Ok(lhs.$op(&($i as f64))),
                _ => {
                    let res = $vm.fallback_for_binop($id, $lhs, Value::fixnum($i as i64));
                    res.map(|x| $vm.val_to_bool(x))
                }
            }
        }
    };
}

impl VM {
    pub fn eval_eq(&self, rhs: Value, lhs: Value) -> bool {
        lhs.equal(rhs)
    }

    pub fn eval_eqi(&self, lhs: Value, i: i32) -> bool {
        lhs.equal_i(i)
    }

    pub fn eval_teq(&mut self, rhs: Value, lhs: Value) -> Result<bool, RubyError> {
        match lhs.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Class(_) => {
                    let res = rhs.get_class_object(&self.globals).id() == lhs.id();
                    Ok(res)
                }
                ObjKind::Regexp(re) => {
                    let given = match rhs.unpack() {
                        RV::Symbol(sym) => IdentId::get_name(sym),
                        RV::Object(_) => match rhs.as_string() {
                            Some(s) => s.to_owned(),
                            None => return Ok(false),
                        },
                        _ => return Ok(false),
                    };
                    let res = RegexpInfo::find_one(self, &*re, &given)?.is_some();
                    Ok(res)
                }
                _ => Ok(self.eval_eq(lhs, rhs)),
            },
            None => Ok(self.eval_eq(lhs, rhs)),
        }
    }

    fn eval_ge(&mut self, rhs: Value, lhs: Value) -> Result<bool, RubyError> {
        eval_cmp!(self, rhs, lhs, ge, IdentId::_GE)
    }
    pub fn eval_gt(&mut self, rhs: Value, lhs: Value) -> Result<bool, RubyError> {
        eval_cmp!(self, rhs, lhs, gt, IdentId::_GT)
    }
    fn eval_le(&mut self, rhs: Value, lhs: Value) -> Result<bool, RubyError> {
        eval_cmp!(self, rhs, lhs, le, IdentId::_LE)
    }
    fn eval_lt(&mut self, rhs: Value, lhs: Value) -> Result<bool, RubyError> {
        eval_cmp!(self, rhs, lhs, lt, IdentId::_LT)
    }

    fn eval_gei(&mut self, lhs: Value, i: i32) -> Result<bool, RubyError> {
        eval_cmp_i!(self, lhs, i, ge, IdentId::_GE)
    }
    fn eval_gti(&mut self, lhs: Value, i: i32) -> Result<bool, RubyError> {
        eval_cmp_i!(self, lhs, i, gt, IdentId::_GT)
    }
    fn eval_lei(&mut self, lhs: Value, i: i32) -> Result<bool, RubyError> {
        eval_cmp_i!(self, lhs, i, le, IdentId::_LE)
    }
    fn eval_lti(&mut self, lhs: Value, i: i32) -> Result<bool, RubyError> {
        eval_cmp_i!(self, lhs, i, lt, IdentId::_LT)
    }

    pub fn eval_cmp(&mut self, rhs: Value, lhs: Value) -> VMResult {
        let res = match lhs.unpack() {
            RV::Integer(lhs) => match rhs.unpack() {
                RV::Integer(rhs) => lhs.partial_cmp(&rhs),
                RV::Float(rhs) => (lhs as f64).partial_cmp(&rhs),
                _ => return Ok(Value::nil()),
            },
            RV::Float(lhs) => match rhs.unpack() {
                RV::Integer(rhs) => lhs.partial_cmp(&(rhs as f64)),
                RV::Float(rhs) => lhs.partial_cmp(&rhs),
                _ => return Ok(Value::nil()),
            },
            _ => {
                let id = IdentId::get_id("<=>");
                return self.fallback_for_binop(id, lhs, rhs);
            }
        };
        match res {
            Some(ord) => Ok(Value::fixnum(ord as i64)),
            None => Ok(Value::nil()),
        }
    }

    fn set_index(&mut self, arg_num: usize) -> Result<(), RubyError> {
        let mut args = self.pop_args_to_ary(arg_num);
        let mut receiver = self.stack_pop();
        let val = self.stack_pop();
        match receiver.as_mut_rvalue() {
            Some(oref) => {
                match oref.kind {
                    ObjKind::Array(ref mut aref) => {
                        args.push(val);
                        aref.set_elem(self, &args)?;
                    }
                    ObjKind::Hash(ref mut href) => href.insert(args[0], val),
                    _ => return Err(self.error_undefined_method(IdentId::_INDEX_ASSIGN, receiver)),
                };
            }
            None => return Err(self.error_undefined_method(IdentId::_INDEX_ASSIGN, receiver)),
        }
        Ok(())
    }

    fn opt_set_index(&mut self, idx: u32) -> Result<(), RubyError> {
        let mut receiver = self.stack_pop();
        let val = self.stack_pop();
        match receiver.as_mut_rvalue() {
            Some(oref) => {
                match oref.kind {
                    ObjKind::Array(ref mut aref) => {
                        aref.set_elem_imm(idx, val);
                    }
                    ObjKind::Hash(ref mut href) => href.insert(Value::fixnum(idx as i64), val),
                    _ => return Err(self.error_undefined_method(IdentId::_INDEX_ASSIGN, receiver)),
                };
            }
            None => return Err(self.error_undefined_method(IdentId::_INDEX_ASSIGN, receiver)),
        }
        Ok(())
    }

    fn get_index(&mut self, arg_num: usize) -> VMResult {
        let args = self.pop_args_to_ary(arg_num);
        let arg_num = args.len();
        let receiver = self.stack_top();
        let val = match receiver.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Array(aref) => aref.get_elem(self, &args)?,
                ObjKind::Hash(href) => {
                    self.check_args_range(arg_num, 1, 1)?;
                    match href.get(&args[0]) {
                        Some(val) => *val,
                        None => Value::nil(),
                    }
                }
                ObjKind::Method(mref) => self.eval_send(mref.method, mref.receiver, &args)?,
                _ => self.fallback(IdentId::_INDEX, receiver, &args)?,
            },
            None if receiver.is_packed_fixnum() => {
                let i = receiver.as_packed_fixnum();
                self.check_args_range(arg_num, 1, 1)?;
                let index = args[0].expect_integer(&self, "Index")?;
                let val = if index < 0 || 63 < index {
                    0
                } else {
                    (i >> index) & 1
                };
                Value::fixnum(val)
            }
            _ => return Err(self.error_undefined_method(IdentId::_INDEX, receiver)),
        };
        self.stack_pop();
        Ok(val)
    }

    fn opt_get_index(&mut self, idx: u32) -> VMResult {
        let receiver = self.stack_top();
        let val = match receiver.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Array(aref) => aref.get_elem_imm(idx),
                ObjKind::Hash(href) => match href.get(&Value::fixnum(idx as i64)) {
                    Some(val) => *val,
                    None => Value::nil(),
                },
                ObjKind::Method(mref) => {
                    let args = Args::new1(Value::fixnum(idx as i64));
                    self.eval_send(mref.method, mref.receiver, &args)?
                }
                _ => {
                    let args = Args::new1(Value::fixnum(idx as i64));
                    self.fallback(IdentId::_INDEX, receiver, &args)?
                }
            },
            None if receiver.is_packed_fixnum() => {
                let i = receiver.as_packed_fixnum();
                let val = if 63 < idx { 0 } else { (i >> idx) & 1 };
                Value::fixnum(val)
            }
            _ => return Err(self.error_undefined_method(IdentId::_INDEX, receiver)),
        };
        self.stack_pop();
        Ok(val)
    }

    fn define_class(&mut self, id: IdentId, is_module: bool, super_val: Value) -> VMResult {
        let val = match self.globals.builtins.object.get_var(id) {
            Some(val) => {
                if val.is_module().is_some() != is_module {
                    return Err(self.error_type(format!(
                        "{:?} is not {}.",
                        id,
                        if is_module { "module" } else { "class" },
                    )));
                };
                let classref = self.expect_module(val.clone())?;
                if !super_val.is_nil() && classref.superclass.id() != super_val.id() {
                    return Err(
                        self.error_type(format!("superclass mismatch for class {:?}.", id,))
                    );
                };
                val.clone()
            }
            None => {
                let super_val = if super_val.is_nil() {
                    self.globals.builtins.object
                } else {
                    self.expect_class(super_val, "Superclass")?;
                    super_val
                };
                let classref = ClassRef::from(id, super_val);
                let val = if is_module {
                    Value::module(&mut self.globals, classref)
                } else {
                    Value::class(&mut self.globals, classref)
                };
                self.class().set_var(id, val);
                val
            }
        };
        Ok(val)
    }

    pub fn sort_array(&mut self, vec: &mut Vec<Value>) -> Result<(), RubyError> {
        if vec.len() > 0 {
            let val = vec[0];
            for i in 1..vec.len() {
                match self.eval_cmp(vec[i], val)? {
                    v if v.is_nil() => {
                        let lhs = self.globals.get_class_name(val);
                        let rhs = self.globals.get_class_name(vec[i]);
                        return Err(self.error_argument(format!(
                            "Comparison of {} with {} failed.",
                            lhs, rhs
                        )));
                    }
                    _ => {}
                }
            }
            vec.sort_by(|a, b| self.eval_cmp(*b, *a).unwrap().to_ordering());
        }
        Ok(())
    }
}

impl VM {
    fn create_regexp(&mut self, arg: Value) -> VMResult {
        let mut arg = match arg.as_string() {
            Some(arg) => arg.clone(),
            None => return Err(self.error_argument("Illegal argument for CREATE_REGEXP")),
        };
        match arg.pop().unwrap() {
            'i' => arg.insert_str(0, "(?mi)"),
            'm' => arg.insert_str(0, "(?ms)"),
            'x' => arg.insert_str(0, "(?mx)"),
            'o' => arg.insert_str(0, "(?mo)"),
            '-' => arg.insert_str(0, "(?m)"),
            _ => return Err(self.error_internal("Illegal internal regexp expression.")),
        };
        self.create_regexp_from_string(&arg)
    }
}

// API's for handling values.

impl VM {
    pub fn val_to_bool(&self, val: Value) -> bool {
        !val.is_nil() && !val.is_false_val() && !val.is_uninitialized()
    }

    pub fn val_to_s(&mut self, val: Value) -> String {
        match val.unpack() {
            RV::Uninitialized => "[Uninitialized]".to_string(),
            RV::Nil => "".to_string(),
            RV::Bool(b) => match b {
                true => "true".to_string(),
                false => "false".to_string(),
            },
            RV::Integer(i) => i.to_string(),
            RV::Float(f) => {
                if f.fract() == 0.0 {
                    format!("{:.1}", f)
                } else {
                    f.to_string()
                }
            }
            RV::Symbol(i) => format!("{:?}", i),
            RV::Object(oref) => match &oref.kind {
                ObjKind::Invalid => panic!("Invalid rvalue. (maybe GC problem) {:?}", *oref),
                ObjKind::String(s) => s.to_s(),
                ObjKind::Class(cref) => match cref.name {
                    Some(id) => format! {"{:?}", id},
                    None => format! {"#<Class:0x{:x}>", cref.id()},
                },
                ObjKind::Ordinary => oref.to_s(),
                ObjKind::Array(aref) => aref.to_s(self),
                ObjKind::Range(rinfo) => rinfo.to_s(self),
                ObjKind::Regexp(rref) => format!("({})", rref.as_str().to_string()),
                ObjKind::Hash(href) => href.to_s(self),
                _ => format!("{:?}", oref.kind),
            },
        }
    }

    pub fn val_inspect(&mut self, val: Value) -> String {
        match val.unpack() {
            RV::Uninitialized => "[Uninitialized]".to_string(),
            RV::Nil => "nil".to_string(),
            RV::Bool(b) => match b {
                true => "true".to_string(),
                false => "false".to_string(),
            },
            RV::Integer(i) => i.to_string(),
            RV::Float(f) => {
                if f.fract() == 0.0 {
                    format!("{:.1}", f)
                } else {
                    f.to_string()
                }
            }
            RV::Symbol(sym) => format!(":{:?}", sym),
            RV::Object(oref) => match &oref.kind {
                ObjKind::Invalid => "[Invalid]".to_string(),
                ObjKind::String(s) => s.inspect(),
                ObjKind::Range(rinfo) => rinfo.inspect(self),
                ObjKind::Class(cref) => match cref.name {
                    Some(id) => format! {"{:?}", id},
                    None => format! {"#<Class:0x{:x}>", cref.id()},
                },
                ObjKind::Module(cref) => match cref.name {
                    Some(id) => format! {"{:?}", id},
                    None => format! {"#<Module:0x{:x}>", cref.id()},
                },
                ObjKind::Array(aref) => aref.to_s(self),
                ObjKind::Regexp(rref) => format!("/{}/", rref.as_str().to_string()),
                ObjKind::Ordinary => oref.inspect(self),
                ObjKind::Proc(pref) => format!("#<Proc:0x{:x}>", pref.context.id()),
                ObjKind::Hash(href) => href.to_s(self),
                _ => {
                    let id = IdentId::get_id("inspect");
                    self.send0(val, id)
                        .unwrap()
                        .as_string()
                        .unwrap()
                        .to_string()
                }
            },
        }
    }

    pub fn send0(&mut self, receiver: Value, method_id: IdentId) -> VMResult {
        let method = self.get_method(receiver, method_id)?;
        let args = Args::new0();
        let val = self.eval_send(method, receiver, &args)?;
        Ok(val)
    }
}

impl VM {
    fn vm_send(&mut self, iseq: &ISeq, receiver: Value) -> VMResult {
        let method_id = self.read_id(iseq, 1);
        let args_num = self.read16(iseq, 5);
        let flag = self.read16(iseq, 7);
        let cache_slot = self.read32(iseq, 9);
        let block = self.read32(iseq, 13);
        let rec_class = receiver.get_class_object_for_method(&self.globals);
        let methodref = self.get_method_from_cache(cache_slot, rec_class, method_id)?;

        let keyword = if flag & 0b01 == 1 {
            let val = self.stack_pop();
            Some(val)
        } else {
            None
        };

        let block = if block != 0 {
            Some(MethodRef::from(block))
        } else if flag & 0b10 == 2 {
            let val = self.stack_pop();
            let method = val
                .as_proc()
                .ok_or_else(|| {
                    self.error_argument(format!("Block argument must be Proc. given:{:?}", val))
                })?
                .context
                .iseq_ref
                .unwrap()
                .method;
            Some(method)
        } else {
            None
        };
        let mut args = self.pop_args_to_ary(args_num as usize);
        args.block = block;
        args.kw_arg = keyword;
        let val = self.eval_send(methodref, receiver, &args)?;
        Ok(val)
    }

    fn vm_opt_send(&mut self, iseq: &ISeq, receiver: Value) -> VMResult {
        // No block nor keyword/block/splat arguments for OPT_SEND.
        let method_id = self.read_id(iseq, 1);
        let args_num = self.read16(iseq, 5);
        let cache_slot = self.read32(iseq, 7);
        let rec_class = receiver.get_class_object_for_method(&self.globals);
        let methodref = self.get_method_from_cache(cache_slot, rec_class, method_id)?;
        //let args = self.pop_args_to_ary(args_num as usize);
        let mut args = Args::new(0);
        for _ in 0..args_num {
            let val = self.stack_pop();
            args.push(val);
        }
        let val = self.eval_send(methodref, receiver, &args)?;
        Ok(val)
    }
}

impl VM {
    /// Evaluate method with given `self_val`, `args` and no outer context.
    pub fn eval_send(&mut self, methodref: MethodRef, self_val: Value, args: &Args) -> VMResult {
        self.eval_method(methodref, self_val, None, args)
    }

    /// Evaluate method with self_val of current context, current context as outer context, and given `args`.
    pub fn eval_block(&mut self, methodref: MethodRef, args: &Args) -> VMResult {
        let context = self.current_context();
        self.eval_method(methodref, context.self_value, Some(context), args)
    }

    /// Evaluate method with self_val of current context, caller context as outer context, and given `args`.
    pub fn eval_yield(&mut self, args: &Args) -> VMResult {
        let mut context = self.current_context();
        loop {
            if let ISeqKind::Method(_) = context.kind {
                break;
            }
            context = context
                .outer
                .ok_or_else(|| self.error_unimplemented("No block given."))?;
        }
        let method = context
            .block
            .ok_or_else(|| self.error_unimplemented("No block given."))?;

        let res = self.eval_method(
            method,
            self.current_context().self_value,
            context.caller,
            &args,
        )?;
        Ok(res)
    }

    /// Evaluate method with given `self_val`, `outer` context, and `args`.
    pub fn eval_method(
        &mut self,
        methodref: MethodRef,
        mut self_val: Value,
        outer: Option<ContextRef>,
        args: &Args,
    ) -> VMResult {
        let info = self.globals.get_method_info(methodref);
        #[cfg(feature = "perf")]
        let mut _inst: u8;
        #[cfg(feature = "perf")]
        {
            _inst = self.perf.get_prev_inst();
        }
        let val = match info {
            MethodInfo::BuiltinFunc { func, .. } => {
                let func = func.to_owned();
                #[cfg(feature = "perf")]
                self.perf.get_perf(Perf::EXTERN);

                let len = self.temp_stack.len();
                self.temp_push(self_val);
                self.temp_push_args(args);
                let res = func(self, self_val, args);
                self.temp_stack.truncate(len);

                #[cfg(feature = "perf")]
                self.perf.get_perf_no_count(_inst);
                res?
            }
            MethodInfo::AttrReader { id } => match self_val.as_rvalue() {
                Some(oref) => match oref.get_var(*id) {
                    Some(v) => v,
                    None => Value::nil(),
                },
                None => unreachable!("AttrReader must be used only for class instance."),
            },
            MethodInfo::AttrWriter { id } => match self_val.as_mut_rvalue() {
                Some(oref) => {
                    oref.set_var(*id, args[0]);
                    args[0]
                }
                None => unreachable!("AttrReader must be used only for class instance."),
            },
            MethodInfo::RubyFunc { iseq } => {
                let iseq = *iseq;
                let context =
                    Context::from_args(self, self_val, iseq, args, outer, self.latest_context())?;
                let res = self.run_context(ContextRef::from_local(&context));
                #[cfg(feature = "perf")]
                self.perf.get_perf_no_count(_inst);
                res?
            }
        };
        Ok(val)
    }
}

// API's for handling instance/singleton methods.

impl VM {
    /// Define a method on `target_obj`.
    /// If `target_obj` is not Class, use Class of it.
    pub fn define_method(&mut self, target_obj: Value, id: IdentId, method: MethodRef) {
        let mut class = match target_obj.as_module() {
            Some(mref) => mref,
            None => target_obj
                .get_class_object(&self.globals)
                .as_module()
                .unwrap(),
        };
        class.add_method(&mut self.globals, id, method);
    }

    /// Define a method on a singleton class of `target_obj`.
    pub fn define_singleton_method(
        &mut self,
        target_obj: Value,
        id: IdentId,
        method: MethodRef,
    ) -> Result<(), RubyError> {
        let singleton = self.get_singleton_class(target_obj)?;
        singleton
            .as_class()
            .add_method(&mut self.globals, id, method);
        Ok(())
    }

    /// Get method(MethodRef) for receiver.
    pub fn get_method(
        &mut self,
        receiver: Value,
        method_id: IdentId,
    ) -> Result<MethodRef, RubyError> {
        let rec_class = receiver.get_class_object_for_method(&self.globals);
        let method = self.get_instance_method(rec_class, method_id)?;
        Ok(method)
    }

    /// Get corresponding instance method(MethodRef) for the class object `class` and `method`.
    ///
    /// If an entry for `class` and `method` exists in global method cache and the entry is not outdated,
    /// return MethodRef of the entry.
    /// If not, search `method` by scanning a class chain.
    /// `class` must be a Class.
    pub fn get_instance_method(
        &mut self,
        class: Value,
        method: IdentId,
    ) -> Result<MethodRef, RubyError> {
        match self.globals.get_method_cache_entry(class, method) {
            Some(MethodCacheEntry { version, method }) => {
                if *version == self.globals.class_version {
                    return Ok(*method);
                }
            }
            None => {}
        };
        let mut temp_class = class;
        let mut singleton_flag = class.as_class().is_singleton;
        loop {
            match temp_class.get_instance_method(method) {
                Some(methodref) => {
                    self.globals
                        .add_method_cache_entry(class, method, methodref);
                    return Ok(methodref);
                }
                None => match temp_class.superclass() {
                    Some(superclass) => temp_class = superclass,
                    None => {
                        if singleton_flag {
                            singleton_flag = false;
                            temp_class = class.rvalue().class();
                        } else {
                            return Err(self.error_nomethod(format!(
                                "no method `{:?}' found for {:?}",
                                method, class
                            )));
                        }
                    }
                },
            };
        }
    }

    pub fn get_singleton_class(&mut self, mut obj: Value) -> VMResult {
        obj.get_singleton_class(&self.globals)
            .map_err(|_| self.error_type("Can not define singleton."))
    }
}

impl VM {
    /// Yield args to parent fiber. (execute Fiber.yield)
    pub fn fiber_yield(&mut self, args: &Args) -> VMResult {
        let val = match args.len() {
            0 => Value::nil(),
            1 => args[0],
            _ => Value::array_from(&self.globals, args.to_vec()),
        };
        match &self.parent_fiber {
            None => return Err(self.error_fiber("Can not yield from main fiber.")),
            Some(ParentFiberInfo { tx, rx, .. }) => {
                #[cfg(feature = "perf")]
                let mut _inst: u8;
                #[cfg(feature = "perf")]
                {
                    _inst = self.perf.get_prev_inst();
                }
                #[cfg(feature = "perf")]
                self.perf.get_perf(Perf::INVALID);
                #[cfg(feature = "trace")]
                println!("<=== yield Ok({:?})", val);

                tx.send(Ok(val)).unwrap();
                // Wait for fiber's response
                rx.recv().unwrap();
                #[cfg(feature = "perf")]
                self.perf.get_perf_no_count(_inst);
                // TODO: this return value is not correct. The arg og Fiber#resume should be returned.
                Ok(Value::nil())
            }
        }
    }

    /// Get local variable table.
    fn get_outer_context(&mut self, outer: u32) -> ContextRef {
        let mut context = self.current_context();
        for _ in 0..outer {
            context = context.outer.unwrap();
        }
        context
    }

    /// Calculate array index.
    pub fn get_array_index(&self, index: i64, len: usize) -> Result<usize, RubyError> {
        if index < 0 {
            let i = len as i64 + index;
            if i < 0 {
                return Err(self.error_unimplemented("Index out of range."));
            };
            Ok(i as usize)
        } else {
            Ok(index as usize)
        }
    }

    fn pop_key_value_pair(&mut self, arg_num: usize) -> FxHashMap<HashKey, Value> {
        let mut hash = FxHashMap::default();
        for _ in 0..arg_num {
            let value = self.stack_pop();
            let key = self.stack_pop();
            hash.insert(HashKey(key), value);
        }
        hash
    }

    fn pop_args_to_ary(&mut self, arg_num: usize) -> Args {
        let mut args = Args::new(0);
        for _ in 0..arg_num {
            let val = self.stack_pop();
            match val.as_splat() {
                Some(inner) => match inner.as_rvalue() {
                    None => args.push(inner),
                    Some(obj) => match &obj.kind {
                        ObjKind::Array(aref) => {
                            for elem in &aref.elements {
                                args.push(*elem);
                            }
                        }
                        ObjKind::Range(rref) => {
                            let start = if rref.start.is_packed_fixnum() {
                                rref.start.as_packed_fixnum()
                            } else {
                                unimplemented!("Range start not fixnum.")
                            };
                            let end = if rref.end.is_packed_fixnum() {
                                rref.end.as_packed_fixnum()
                            } else {
                                unimplemented!("Range end not fixnum.")
                            } + if rref.exclude { 0 } else { 1 };
                            for i in start..end {
                                args.push(Value::fixnum(i));
                            }
                        }
                        _ => args.push(inner),
                    },
                },
                None => args.push(val),
            };
        }
        args
    }

    /// Create new Proc object from `method`,
    /// moving outer `Context`s on stack to heap.
    pub fn create_proc(&mut self, method: MethodRef) -> VMResult {
        self.move_outer_to_heap();
        let context = self.create_block_context(method)?;
        Ok(Value::procobj(&self.globals, context))
    }

    pub fn create_enum_info(
        &mut self,
        method_id: IdentId,
        receiver: Value,
        args: Args,
    ) -> FiberInfo {
        let (tx0, rx0) = std::sync::mpsc::sync_channel(0);
        let (tx1, rx1) = std::sync::mpsc::sync_channel(0);
        let fiber_vm = self.create_fiber(tx0, rx1);
        //self.globals.fibers.push(VMRef::from_ref(&fiber_vm));
        //let context = ContextRef::new(Context::new_noiseq());
        //fiber_vm.context_push(context);
        FiberInfo::new_internal(fiber_vm, receiver, method_id, args, rx0, tx1)
    }

    pub fn dup_enum(&mut self, eref: &FiberInfo) -> FiberInfo {
        let (receiver, method_id, args) = match &eref.kind {
            FiberKind::Enum(receiver, method_id, args) => (*receiver, *method_id, args.clone()),
            _ => unreachable!(),
        };
        self.create_enum_info(method_id, receiver, args)
    }

    pub fn create_enumerator(
        &mut self,
        method_id: IdentId,
        receiver: Value,
        args: Args,
    ) -> VMResult {
        let fiber = self.create_enum_info(method_id, receiver, args);
        Ok(Value::enumerator(&self.globals, fiber))
    }

    /// Move outer execution contexts on the stack to the heap.
    fn move_outer_to_heap(&mut self) {
        let mut prev_ctx: Option<ContextRef> = None;

        for context in self.exec_context.iter_mut().rev() {
            if !context.on_stack {
                break;
            };
            let mut heap_context = context.dup();
            heap_context.on_stack = false;
            *context = heap_context;
            if let Some(mut ctx) = prev_ctx {
                ctx.outer = Some(heap_context);
            };
            if heap_context.outer.is_none() {
                break;
            }
            prev_ctx = Some(heap_context);
        }
    }

    /// Create a new execution context for a block.
    pub fn create_block_context(&mut self, method: MethodRef) -> Result<ContextRef, RubyError> {
        self.move_outer_to_heap();
        let iseq = self.get_iseq(method)?;
        let outer = self.current_context();
        Ok(ContextRef::from(
            outer.self_value,
            None,
            iseq,
            Some(outer),
            None,
        ))
    }

    pub fn get_iseq(&self, method: MethodRef) -> Result<ISeqRef, RubyError> {
        self.globals.get_method_info(method).as_iseq(&self)
    }

    /// Create new Regexp object from `string`.
    /// Regular expression meta characters are handled as is.
    /// Returns RubyError if `string` was invalid regular expression.
    pub fn create_regexp_from_string(&mut self, string: &str) -> VMResult {
        let re = RegexpInfo::from_string(&mut self.globals, string)
            .map_err(|err| self.error_regexp(err))?;
        let regexp = Value::regexp(&self.globals, re);
        Ok(regexp)
    }

    /// Create fancy_regex::Regex from `string`.
    /// Escapes all regular expression meta characters in `string`.
    /// Returns RubyError if `string` was invalid regular expression.
    pub fn regexp_from_string(&mut self, string: &str) -> Result<RegexpInfo, RubyError> {
        RegexpInfo::from_escaped(&mut self.globals, string).map_err(|err| self.error_regexp(err))
    }
}

impl VM {
    pub fn load_file(
        &mut self,
        file_name: String,
    ) -> Result<(std::path::PathBuf, String), RubyError> {
        use crate::loader::*;
        match crate::loader::load_file(file_name.clone()) {
            Ok((path, program)) => Ok((path, program)),
            Err(err) => {
                let err_str = match err {
                    LoadError::NotFound(msg) => format!(
                        "LoadError: No such file or directory -- {}\n{}",
                        &file_name, msg
                    ),
                    LoadError::CouldntOpen(msg) => {
                        format!("Cannot open file. '{}'\n{}", &file_name, msg)
                    }
                };
                Err(self.error_internal(err_str))
            }
        }
    }

    pub fn exec_file(&mut self, file_name: impl Into<String>) {
        use crate::loader::*;
        let file_name = file_name.into();
        let (absolute_path, program) = match crate::loader::load_file(file_name.clone()) {
            Ok((path, program)) => (path, program),
            Err(err) => {
                match err {
                    LoadError::NotFound(msg) => eprintln!("LoadError: {}\n{}", &file_name, msg),
                    LoadError::CouldntOpen(msg) => eprintln!("LoadError: {}\n{}", &file_name, msg),
                };
                return;
            }
        };

        let root_path = absolute_path.clone();
        #[cfg(feature = "verbose")]
        eprintln!("load file: {:?}", root_path);
        self.root_path.push(root_path);
        self.exec_program(absolute_path, program);
        self.root_path.pop();
    }

    pub fn exec_program(&mut self, absolute_path: PathBuf, program: String) {
        //let absolute_path = PathBuf::default();
        match self.run(absolute_path, &program) {
            Ok(_) => {
                #[cfg(feature = "perf")]
                self.perf.print_perf();
                #[cfg(feature = "gc-debug")]
                self.globals.print_mark();
            }
            Err(err) => {
                err.show_err();
                for i in 0..err.info.len() {
                    eprint!("{}:", i);
                    err.show_loc(i);
                }
            }
        };
    }
}
