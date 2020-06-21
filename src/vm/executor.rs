use super::codegen::ContextKind;
use crate::*;

#[cfg(feature = "perf")]
use super::perf::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, SyncSender};
use vm_inst::*;

pub type ValueTable = HashMap<IdentId, Value>;
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
    gc_counter: usize,
    pub parent_fiber: Option<ParentFiberInfo>,
    #[cfg(feature = "perf")]
    #[cfg_attr(tarpaulin, skip)]
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
    parent: VMRef,
    tx: SyncSender<VMResult>,
    rx: Receiver<usize>,
}

impl ParentFiberInfo {
    fn new(parent: VMRef, tx: SyncSender<VMResult>, rx: Receiver<usize>) -> Self {
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
        //self.globals.mark(alloc);
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
            gc_counter: 0,
            parent_fiber: None,
            #[cfg(feature = "perf")]
            #[cfg_attr(tarpaulin, skip)]
            perf: Perf::new(),
        };
        vm
    }

    pub fn dup_fiber(&self, tx: SyncSender<VMResult>, rx: Receiver<usize>) -> Self {
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
            gc_counter: 0,
            parent_fiber: Some(ParentFiberInfo::new(VMRef::from_ref(self), tx, rx)),
            #[cfg(feature = "perf")]
            #[cfg_attr(tarpaulin, skip)]
            perf: Perf::new(),
        };
        vm
    }

    pub fn context(&self) -> ContextRef {
        *self.exec_context.last().unwrap()
    }

    pub fn caller_context(&self) -> ContextRef {
        let len = self.exec_context.len();
        if len < 2 {
            unreachable!("caller_context(): exec_context.len is {}", len)
        };
        self.exec_context[len - 2]
    }

    pub fn source_info(&self) -> SourceInfoRef {
        self.context().iseq_ref.source_info
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

    pub fn stack_push(&mut self, val: Value) {
        self.exec_stack.push(val)
    }

    pub fn stack_pop(&mut self) -> Value {
        self.exec_stack.pop().unwrap()
    }

    pub fn stack_top(&mut self) -> Value {
        self.exec_stack.last().unwrap().clone()
    }

    /// Push an object to the temporary area.
    pub fn temp_push(&mut self, v: Value) {
        self.temp_stack.push(v);
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

    pub fn define_mode_mut(&mut self) -> &mut DefineMode {
        &mut self.class_context.last_mut().unwrap().1
    }

    pub fn module_function(&mut self, flag: bool) {
        self.class_context.last_mut().unwrap().1.module_function = flag;
    }

    pub fn get_pc(&mut self) -> usize {
        self.pc
    }

    pub fn set_pc(&mut self, pc: usize) {
        self.pc = pc;
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
        #[cfg_attr(tarpaulin, skip)]
        {
            self.perf.set_prev_inst(Perf::INVALID);
        }
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
        let ext_lvar = self.context().iseq_ref.lvar.clone();
        let result = parser.parse_program_eval(path, program, ext_lvar.clone())?;
        //self.globals.ident_table = result.ident_table;

        #[cfg(feature = "perf")]
        #[cfg_attr(tarpaulin, skip)]
        {
            self.perf.set_prev_inst(Perf::INVALID);
        }
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

    pub fn run(&mut self, path: PathBuf, program: &str, self_value: Option<Value>) -> VMResult {
        let method = self.parse_program(path, program)?;
        let self_value = match self_value {
            Some(val) => val,
            None => self.globals.main_object,
        };
        let arg = Args::new0();
        let val = self.eval_send(method, self_value, &arg)?;
        #[cfg(feature = "perf")]
        #[cfg_attr(tarpaulin, skip)]
        {
            self.perf.get_perf(Perf::INVALID);
        }

        let stack_len = self.exec_stack.len();
        if stack_len != 0 {
            eprintln!("Error: stack length is illegal. {}", stack_len);
        };

        Ok(val)
    }

    #[cfg_attr(tarpaulin, skip)]
    pub fn run_repl(&mut self, result: &ParseResult, mut context: ContextRef) -> VMResult {
        #[cfg(feature = "perf")]
        #[cfg_attr(tarpaulin, skip)]
        {
            self.perf.set_prev_inst(Perf::CODEGEN);
        }
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
        context.iseq_ref = iseq;
        context.adjust_lvar_size();
        context.pc = 0;

        let val = self.run_context(context)?;
        #[cfg(feature = "perf")]
        #[cfg_attr(tarpaulin, skip)]
        {
            self.perf.get_perf(Perf::INVALID);
        }

        let stack_len = self.exec_stack.len();
        if stack_len != 0 {
            eprintln!("Error: stack length is illegal. {}", stack_len);
        };

        Ok(val)
    }

    #[allow(dead_code)]
    #[cfg_attr(tarpaulin, skip)]
    pub fn dump_context(&self) {
        eprintln!("---dump");
        for (i, context) in self.exec_context.iter().enumerate() {
            eprintln!("context: {}", i);
            eprintln!("self: {:#?}", context.self_value);
            for i in 0..context.iseq_ref.lvars {
                let id = LvarId::from_usize(i);
                let (k, _) = context
                    .iseq_ref
                    .lvar
                    .table()
                    .iter()
                    .find(|(_, v)| **v == id)
                    .unwrap();
                let name = IdentId::get_ident_name(*k);
                eprintln!("lvar({}): {} {:#?}", id.as_u32(), name, context[id]);
            }
        }
        for v in &self.exec_stack {
            eprintln!("stack: {:#?}", *v);
        }
        eprintln!("---dump end");
    }
}

macro_rules! try_err {
    ($self:ident, $eval:expr) => {
        match $eval {
            Ok(val) => $self.stack_push(val),
            Err(err) if err.kind == RubyErrorKind::BlockReturn => {}
            Err(mut err) => {
                let m = $self.context().iseq_ref.method;
                let res = if RubyErrorKind::MethodReturn(m) == err.kind {
                    let result = $self.stack_pop();
                    let prev_len = $self.context().stack_len;
                    $self.exec_stack.truncate(prev_len);
                    $self.unwind_context(&mut err);
                    #[cfg(feature = "trace")]
                    {
                        println!("<--- METHOD_RETURN Ok({:?})", result);
                    }
                    Ok(result)
                } else {
                    //$self.dump_context();
                    $self.unwind_context(&mut err);
                    #[cfg(feature = "trace")]
                    {
                        println!("<--- Err({:?})", err.kind);
                    }
                    Err(err)
                };
                $self.fiberstate_dead();
                $self.fiber_send_to_parent(res.clone());
                return res;
            }
        };
    };
}

impl VM {
    fn gc(&mut self) {
        self.gc_counter += 1;
        if !self.globals.gc_enabled || self.gc_counter % 32 != 0 {
            return;
        }
        if !ALLOC_THREAD.with(|m| m.borrow().is_allocated()) {
            return;
        };
        #[cfg(feature = "perf")]
        self.perf.get_perf(Perf::GC);
        self.globals.gc();
    }

    /// Main routine for VM execution.
    pub fn run_context(&mut self, context: ContextRef) -> VMResult {
        #[cfg(feature = "trace")]
        {
            if context.is_fiber {
                println!("===> {:?}", context.iseq_ref.method);
            } else {
                println!("---> {:?}", context.iseq_ref.method);
            }
        }
        if let Some(prev_context) = self.exec_context.last_mut() {
            prev_context.pc = self.pc;
            prev_context.stack_len = self.exec_stack.len();
        };
        self.context_push(context);
        self.pc = context.pc;
        let iseq = &context.iseq_ref.iseq;
        let self_oref = context.self_value.rvalue_mut();
        self.gc();

        loop {
            #[cfg(feature = "perf")]
            #[cfg_attr(tarpaulin, skip)]
            {
                self.perf.get_perf(iseq[self.pc]);
            }
            #[cfg(feature = "trace")]
            {
                println!(
                    "{:>4x}:{:<15} stack:{}",
                    self.pc,
                    Inst::inst_name(iseq[self.pc]),
                    self.exec_stack.len()
                );
            }
            match iseq[self.pc] {
                Inst::END => {
                    // reached the end of the method or block.
                    // - the end of the method or block.
                    // - `next` in block AND outer of loops.
                    if self.exec_context.len() == 1 {
                        // if in the final context, the fiber becomes DEAD.
                        self.fiberstate_dead();
                        self.fiber_send_to_parent(Err(self.error_fiber("Dead fiber called.")));
                    };
                    let _context = self.context_pop().unwrap();
                    let val = self.stack_pop();
                    #[cfg(feature = "trace")]
                    {
                        if _context.is_fiber {
                            println!("<=== Ok({:?})", val);
                        } else {
                            println!("<--- Ok({:?})", val);
                        }
                    }
                    if !self.exec_context.is_empty() {
                        self.pc = self.context().pc;
                    };
                    return Ok(val);
                }
                Inst::RETURN => {
                    // 'Inst::RETURN' is executed.
                    // - `return` in method.
                    // - `break` outer of loops.
                    let res = if let ISeqKind::Block(_) = context.kind {
                        // if in block context, exit with Err(BLOCK_RETURN).
                        let err = self.error_block_return();
                        #[cfg(feature = "trace")]
                        {
                            println!("<--- Err({:?})", err.kind);
                        }
                        Err(err)
                    } else {
                        // if in method context, exit with Ok(rerurn_value).
                        let val = self.stack_pop();
                        #[cfg(feature = "trace")]
                        {
                            println!("<--- Ok({:?})", val);
                        }
                        Ok(val)
                    };

                    self.context_pop().unwrap();
                    if !self.exec_context.is_empty() {
                        self.pc = self.context().pc;
                    }
                    return res;
                }
                Inst::MRETURN => {
                    // 'METHOD_RETURN' is executed.
                    // - `return` in block
                    let res = if let ISeqKind::Block(method) = context.kind {
                        // exit with Err(METHOD_RETURN).
                        let err = self.error_method_return(method);
                        #[cfg(feature = "trace")]
                        {
                            println!("<--- Err({:?})", err.kind);
                        }
                        Err(err)
                    } else {
                        unreachable!()
                    };
                    self.context_pop().unwrap();
                    if !self.exec_context.is_empty() {
                        self.pc = self.context().pc;
                    }
                    return res;
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
                Inst::PUSH_STRING => {
                    let id = self.read_id(iseq, 1);
                    let string = IdentId::get_ident_name(id);
                    self.stack_push(Value::string(&self.globals, string));
                    self.pc += 5;
                }
                Inst::PUSH_SYMBOL => {
                    let id = self.read_id(iseq, 1);
                    self.stack_push(Value::symbol(id));
                    self.pc += 5;
                }
                Inst::ADD => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    try_err!(self, self.eval_add(lhs, rhs, iseq));
                    self.pc += 5;
                }
                Inst::ADDI => {
                    let lhs = self.stack_pop();
                    let i = self.read32(iseq, 1) as i32;
                    try_err!(self, self.eval_addi(lhs, i));
                    self.pc += 5;
                }
                Inst::SUB => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    try_err!(self, self.eval_sub(lhs, rhs, iseq));
                    self.pc += 5;
                }
                Inst::SUBI => {
                    let lhs = self.stack_pop();
                    let i = self.read32(iseq, 1) as i32;
                    try_err!(self, self.eval_subi(lhs, i));
                    self.pc += 5;
                }
                Inst::MUL => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    try_err!(self, self.eval_mul(lhs, rhs, iseq));
                    self.pc += 5;
                }
                Inst::POW => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    try_err!(self, self.eval_exp(lhs, rhs));
                    self.pc += 1;
                }
                Inst::DIV => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    try_err!(self, self.eval_div(lhs, rhs, iseq));
                    self.pc += 5;
                }
                Inst::REM => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    try_err!(self, self.eval_rem(lhs, rhs));
                    self.pc += 1;
                }
                Inst::SHR => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    try_err!(self, self.eval_shr(lhs, rhs));
                    self.pc += 1;
                }
                Inst::SHL => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    try_err!(self, self.eval_shl(lhs, rhs, iseq));
                    self.pc += 5;
                }
                Inst::BIT_AND => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    try_err!(self, self.eval_bitand(lhs, rhs));
                    self.pc += 1;
                }
                Inst::BIT_OR => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    try_err!(self, self.eval_bitor(lhs, rhs));
                    self.pc += 1;
                }
                Inst::BIT_XOR => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    try_err!(self, self.eval_bitxor(lhs, rhs));
                    self.pc += 1;
                }
                Inst::BIT_NOT => {
                    let lhs = self.stack_pop();
                    try_err!(self, self.eval_bitnot(lhs));
                    self.pc += 1;
                }
                Inst::EQ => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = Value::bool(self.eval_eq(rhs, lhs));
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::NE => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = Value::bool(!self.eval_eq(rhs, lhs));
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::TEQ => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let res = self.eval_teq(rhs, lhs)?;
                    let val = Value::bool(res);
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::GT => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = self.eval_gt(lhs, rhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::GE => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = self.eval_ge(lhs, rhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::CMP => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = self.eval_cmp(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::NOT => {
                    let lhs = self.stack_pop();
                    let val = Value::bool(!self.val_to_bool(lhs));
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::CONCAT_STRING => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let val = match (lhs.as_string(), rhs.as_string()) {
                        (Some(lhs), Some(rhs)) => {
                            Value::string(&self.globals, format!("{}{}", lhs, rhs))
                        }
                        (_, _) => unreachable!("Illegal CAONCAT_STRING arguments."),
                    };
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::SET_LOCAL => {
                    let id = self.read_lvar_id(iseq, 1);
                    let outer = self.read32(iseq, 5);
                    let val = self.stack_pop();
                    let mut cref = self.get_outer_context(outer);
                    cref[id] = val;
                    self.pc += 9;
                }
                Inst::GET_LOCAL => {
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
                                _ => return Err(self.error_undefined_method("[]=", receiver)),
                            };
                        }
                        None => return Err(self.error_undefined_method("[]=", receiver)),
                    }

                    self.pc += 5;
                }
                Inst::GET_INDEX => {
                    let arg_num = self.read_usize(iseq, 1);
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
                            ObjKind::Method(mref) => {
                                self.eval_send(mref.method, mref.receiver, &args)?
                            }
                            _ => {
                                let id = IdentId::get_ident_id("[]");
                                match self.get_method(receiver, id) {
                                    Ok(mref) => self.eval_send(mref, receiver, &args)?,
                                    Err(_) => {
                                        return Err(self.error_undefined_method("[]", receiver))
                                    }
                                }
                            }
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
                        _ => return Err(self.error_undefined_method("[]", receiver)),
                    };
                    self.stack_pop();
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::SPLAT => {
                    let val = self.stack_pop();
                    let res = Value::splat(&self.globals, val);
                    self.stack_push(res);
                    self.pc += 1;
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
                Inst::JMP_IF_FALSE => {
                    let val = self.stack_pop();
                    if self.val_to_bool(val) {
                        self.jump_pc(5, 0);
                    } else {
                        let disp = self.read_disp(iseq, 1);
                        if 0 < disp {
                            self.gc();
                        }
                        self.jump_pc(5, disp);
                    }
                }
                Inst::OPT_CASE => {
                    let val = self.stack_pop();
                    let map = self.globals.get_case_dispatch_map(self.read32(iseq, 1));
                    let disp = match map.get(&val) {
                        Some(disp) => *disp as i64,
                        None => self.read_disp(iseq, 5),
                    };
                    self.jump_pc(9, disp);
                }
                Inst::SEND => {
                    let receiver = self.stack_pop();
                    try_err!(self, self.vm_send(iseq, receiver));
                    self.pc += 17;
                }
                Inst::SEND_SELF => {
                    let receiver = context.self_value;
                    try_err!(self, self.vm_send(iseq, receiver));
                    self.pc += 17;
                }
                Inst::OPT_SEND => {
                    let receiver = self.stack_pop();
                    try_err!(self, self.vm_opt_send(iseq, receiver));
                    self.pc += 11;
                }
                Inst::OPT_SEND_SELF => {
                    let receiver = context.self_value;
                    try_err!(self, self.vm_opt_send(iseq, receiver));
                    self.pc += 11;
                }
                Inst::YIELD => {
                    try_err!(self, self.eval_yield(iseq));
                    self.pc += 5;
                }
                Inst::DEF_CLASS => {
                    let is_module = self.read8(iseq, 1) == 1;
                    let id = self.read_id(iseq, 2);
                    let method = self.read_methodref(iseq, 6);
                    let super_val = self.stack_pop();
                    let val = match self.globals.builtins.object.get_var(id) {
                        Some(val) => {
                            if val.is_module().is_some() != is_module {
                                return Err(self.error_type(format!(
                                    "{} is not {}.",
                                    IdentId::get_ident_name(id),
                                    if is_module { "module" } else { "class" },
                                )));
                            };
                            let classref = self.expect_module(val.clone())?;
                            if !super_val.is_nil() && classref.superclass.id() != super_val.id() {
                                return Err(self.error_type(format!(
                                    "superclass mismatch for class {}.",
                                    IdentId::get_ident_name(id),
                                )));
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

                    self.class_push(val);
                    let mut iseq = self.get_iseq(method)?;
                    iseq.class_defined = self.gen_class_defined(val);
                    let arg = Args::new0();
                    try_err!(self, self.eval_send(method, val, &arg));
                    self.pc += 10;
                    self.class_pop();
                }
                Inst::DEF_METHOD => {
                    let id = self.read_id(iseq, 1);
                    let method = self.read_methodref(iseq, 5);
                    let mut iseq = self.get_iseq(method)?;
                    iseq.class_defined = self.gen_class_defined(None);
                    self.define_method(id, method);
                    if self.define_mode().module_function {
                        self.define_singleton_method(self.class(), id, method)?;
                    };
                    self.pc += 9;
                }
                Inst::DEF_SMETHOD => {
                    let id = self.read_id(iseq, 1);
                    let method = self.read_methodref(iseq, 5);
                    let mut iseq = self.get_iseq(method)?;
                    iseq.class_defined = self.gen_class_defined(None);
                    let singleton = self.stack_pop();
                    self.define_singleton_method(singleton, id, method)?;
                    if self.define_mode().module_function {
                        self.define_method(id, method);
                    };
                    self.pc += 9;
                }
                Inst::TO_S => {
                    let val = self.stack_pop();
                    let s = self.val_to_s(val);
                    let res = Value::string(&self.globals, s);
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
            RuntimeErrKind::NoMethod(msg.into()),
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

    pub fn error_undefined_method(
        &self,
        method_name: impl Into<String>,
        receiver: Value,
    ) -> RubyError {
        self.error_nomethod(format!(
            "undefined method `{}' for {}",
            method_name.into(),
            self.globals.get_class_name(receiver)
        ))
    }

    pub fn error_unimplemented(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(
            RuntimeErrKind::Unimplemented(msg.into()),
            self.source_info(),
            loc,
        )
    }

    pub fn error_internal(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(
            RuntimeErrKind::Internal(msg.into()),
            self.source_info(),
            loc,
        )
    }

    pub fn error_name(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(RuntimeErrKind::Name(msg.into()), self.source_info(), loc)
    }

    pub fn error_type(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(RuntimeErrKind::Type(msg.into()), self.source_info(), loc)
    }

    pub fn error_argument(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(
            RuntimeErrKind::Argument(msg.into()),
            self.source_info(),
            loc,
        )
    }

    pub fn error_regexp(&self, err: fancy_regex::Error) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(
            RuntimeErrKind::Regexp(format!(
                "Invalid string for a regular expression. {:?}",
                err
            )),
            self.source_info(),
            loc,
        )
    }

    pub fn error_index(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(RuntimeErrKind::Index(msg.into()), self.source_info(), loc)
    }

    pub fn error_fiber(&self, msg: impl Into<String>) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_runtime_err(RuntimeErrKind::Fiber(msg.into()), self.source_info(), loc)
    }

    pub fn error_method_return(&self, method: MethodRef) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_method_return(method, self.source_info(), loc)
    }

    pub fn error_block_return(&self) -> RubyError {
        let loc = self.get_loc();
        RubyError::new_block_return(self.source_info(), loc)
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

    pub fn expect_fiber(&self, val: Value, error_msg: &str) -> Result<FiberRef, RubyError> {
        match val.as_rvalue() {
            Some(oref) => match oref.kind {
                ObjKind::Fiber(f) => Ok(f),
                _ => Err(self.error_argument(error_msg)),
            },
            None => Err(self.error_argument(error_msg)),
        }
    }
}

impl VM {
    fn get_loc(&self) -> Loc {
        let sourcemap = &self.context().iseq_ref.iseq_sourcemap;
        sourcemap
            .iter()
            .find(|x| x.0 == ISeqPos::from(self.pc))
            .unwrap_or(&(ISeqPos::from(0), Loc(0, 0)))
            .1
    }

    fn get_nearest_class_stack(&self) -> Option<ClassListRef> {
        let mut class_stack = None;
        for context in self.exec_context.iter().rev() {
            match context.iseq_ref.class_defined {
                Some(class_list) => {
                    class_stack = Some(class_list);
                    break;
                }
                None => {}
            }
        }
        class_stack
    }

    /// Return None in top-level.
    fn gen_class_defined(&self, new_class: impl Into<Option<Value>>) -> Option<ClassListRef> {
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
                        let name = IdentId::get_ident_name(id);
                        return Err(self.error_name(format!("Uninitialized constant {}.", name)));
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
        receiver: Value,
        method_id: IdentId,
    ) -> Result<MethodRef, RubyError> {
        let rec_class = receiver.get_class_object_for_method(&self.globals);
        if rec_class.is_nil() {
            return Err(self.error_unimplemented("receiver's class is nil."));
        };
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

    fn fallback_to_method(&mut self, method: IdentId, lhs: Value, rhs: Value) -> VMResult {
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

    fn fallback_to_method_with_cache(
        &mut self,
        lhs: Value,
        rhs: Value,
        method: IdentId,
        cache: u32,
    ) -> VMResult {
        let methodref = self.get_method_from_cache(cache, lhs, method)?;
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
            _ => return $vm.fallback_to_method($id, $lhs, Value::fixnum($i as i64)),
        };
        return Ok(val);
    };
}

macro_rules! eval_op {
    ($vm:ident, $iseq:ident, $rhs:expr, $lhs:expr, $op:ident, $id:expr) => {
        let val = match ($lhs.unpack(), $rhs.unpack()) {
            (RV::Integer(lhs), RV::Integer(rhs)) => Value::fixnum(lhs.$op(rhs)),
            (RV::Integer(lhs), RV::Float(rhs)) => Value::flonum((lhs as f64).$op(rhs)),
            (RV::Float(lhs), RV::Integer(rhs)) => Value::flonum(lhs.$op(rhs as f64)),
            (RV::Float(lhs), RV::Float(rhs)) => Value::flonum(lhs.$op(rhs)),
            _ => {
                let cache = $vm.read32($iseq, 1);
                return $vm.fallback_to_method_with_cache($lhs, $rhs, $id, cache);
            }
        };
        return Ok(val);
    };
}

impl VM {
    fn eval_add(&mut self, rhs: Value, lhs: Value, iseq: &ISeq) -> VMResult {
        use std::ops::Add;
        eval_op!(self, iseq, rhs, lhs, add, IdentId::_ADD);
    }

    fn eval_sub(&mut self, rhs: Value, lhs: Value, iseq: &ISeq) -> VMResult {
        use std::ops::Sub;
        eval_op!(self, iseq, rhs, lhs, sub, IdentId::_SUB);
    }

    fn eval_mul(&mut self, rhs: Value, lhs: Value, iseq: &ISeq) -> VMResult {
        use std::ops::Mul;
        eval_op!(self, iseq, rhs, lhs, mul, IdentId::_MUL);
    }

    fn eval_addi(&mut self, lhs: Value, i: i32) -> VMResult {
        use std::ops::Add;
        eval_op_i!(self, iseq, lhs, i, add, IdentId::_ADD);
    }

    fn eval_subi(&mut self, lhs: Value, i: i32) -> VMResult {
        use std::ops::Sub;
        eval_op_i!(self, iseq, lhs, i, sub, IdentId::_SUB);
    }

    fn eval_div(&mut self, rhs: Value, lhs: Value, iseq: &ISeq) -> VMResult {
        use std::ops::Div;
        eval_op!(self, iseq, rhs, lhs, div, IdentId::_DIV);
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
            (_, _) => return self.fallback_to_method(IdentId::_REM, lhs, rhs),
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
                return self.fallback_to_method(IdentId::_POW, lhs, rhs);
            }
        };
        Ok(val)
    }

    fn eval_shl(&mut self, rhs: Value, mut lhs: Value, iseq: &ISeq) -> VMResult {
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
        let cache = self.read32(iseq, 1);
        let val = self.fallback_to_method_with_cache(lhs, rhs, IdentId::_SHL, cache)?;
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

macro_rules! eval_cmp {
    ($vm:ident, $rhs:expr, $lhs:expr, $op:ident, $id:expr) => {
        match ($lhs.unpack(), $rhs.unpack()) {
            (RV::Integer(lhs), RV::Integer(rhs)) => Ok(Value::bool(lhs.$op(&rhs))),
            (RV::Float(lhs), RV::Integer(rhs)) => Ok(Value::bool(lhs.$op(&(rhs as f64)))),
            (RV::Integer(lhs), RV::Float(rhs)) => Ok(Value::bool((lhs as f64).$op(&rhs))),
            (RV::Float(lhs), RV::Float(rhs)) => Ok(Value::bool(lhs.$op(&rhs))),
            (_, _) => return $vm.fallback_to_method($id, $lhs, $rhs),
        }
    };
}

impl VM {
    pub fn eval_eq(&self, rhs: Value, lhs: Value) -> bool {
        rhs.equal(lhs)
    }

    pub fn eval_teq(&mut self, rhs: Value, lhs: Value) -> Result<bool, RubyError> {
        match lhs.as_rvalue() {
            Some(oref) => match oref.kind {
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
                    let res = Regexp::find_one(self, &re.regexp, &given)?.is_some();
                    Ok(res)
                }
                _ => Ok(self.eval_eq(lhs, rhs)),
            },
            None => Ok(self.eval_eq(lhs, rhs)),
        }
    }

    fn eval_ge(&mut self, rhs: Value, lhs: Value) -> VMResult {
        eval_cmp!(self, rhs, lhs, ge, IdentId::_GE)
    }

    pub fn eval_gt(&mut self, rhs: Value, lhs: Value) -> VMResult {
        eval_cmp!(self, rhs, lhs, gt, IdentId::_GT)
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
                let id = IdentId::get_ident_id("<=>");
                return self.fallback_to_method(id, lhs, rhs);
            }
        };
        match res {
            Some(ord) => Ok(Value::fixnum(ord as i64)),
            None => Ok(Value::nil()),
        }
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
            RV::Symbol(i) => format!("{}", IdentId::get_ident_name(i)),
            RV::Object(oref) => match &oref.kind {
                ObjKind::Invalid => panic!("Invalid rvalue. (maybe GC problem) {:?}", *oref),
                ObjKind::String(s) => s.to_s(),
                ObjKind::Class(cref) => match cref.name {
                    Some(id) => format! {"{}", IdentId::get_ident_name(id)},
                    None => format! {"#<Class:0x{:x}>", cref.id()},
                },
                ObjKind::Ordinary => oref.to_s(),
                ObjKind::Array(aref) => aref.to_s(self),
                ObjKind::Range(rinfo) => rinfo.to_s(self),
                ObjKind::Regexp(rref) => format!("({})", rref.regexp.as_str().to_string()),
                ObjKind::Hash(href) => href.to_s(self),
                _ => format!("{:?}", oref.kind),
            },
        }
    }

    pub fn val_debug(&self, val: Value) -> String {
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
            RV::Symbol(sym) => format!(":{}", IdentId::get_ident_name(sym)),
            RV::Object(oref) => match &oref.kind {
                ObjKind::Invalid => "[Invalid]".to_string(),
                ObjKind::Ordinary => oref.debug(self),
                ObjKind::Class(cref) => match cref.name {
                    Some(id) => format! {"{}", IdentId::get_ident_name(id)},
                    None => format! {"#<Class:0x{:x}>", cref.id()},
                },
                ObjKind::Module(cref) => match cref.name {
                    Some(id) => format! {"{}", IdentId::get_ident_name(id)},
                    None => format! {"#<Module:0x{:x}>", cref.id()},
                },
                ObjKind::String(s) => s.inspect(),
                ObjKind::Array(aref) => aref.debug(self),
                ObjKind::Range(rinfo) => rinfo.debug(self),
                ObjKind::Splat(v) => self.val_debug(*v),
                ObjKind::Hash(href) => href.debug(self),
                ObjKind::Proc(pref) => format!("#<Proc:0x{:x}>", pref.context.id()),
                ObjKind::Regexp(rref) => format!("/{}/", rref.regexp.as_str().to_string()),
                ObjKind::Method(_) => "Method".to_string(),
                ObjKind::Fiber(_) => "Fiber".to_string(),
                ObjKind::Enumerator(_) => "Enumerator".to_string(),
                _ => "Not supported".to_string(),
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
            RV::Symbol(sym) => format!(":{}", IdentId::get_ident_name(sym)),
            RV::Object(oref) => match &oref.kind {
                ObjKind::Invalid => "[Invalid]".to_string(),
                ObjKind::String(s) => s.inspect(),
                ObjKind::Range(rinfo) => rinfo.inspect(self),
                ObjKind::Class(cref) => match cref.name {
                    Some(id) => format! {"{}", IdentId::get_ident_name(id)},
                    None => format! {"#<Class:0x{:x}>", cref.id()},
                },
                ObjKind::Module(cref) => match cref.name {
                    Some(id) => format! {"{}", IdentId::get_ident_name(id)},
                    None => format! {"#<Module:0x{:x}>", cref.id()},
                },
                ObjKind::Array(aref) => aref.to_s(self),
                ObjKind::Regexp(rref) => format!("/{}/", rref.regexp.as_str().to_string()),
                ObjKind::Ordinary => oref.inspect(self),
                ObjKind::Proc(pref) => format!("#<Proc:0x{:x}>", pref.context.id()),
                ObjKind::Hash(href) => href.to_s(self),
                _ => {
                    let id = IdentId::get_ident_id("inspect");
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
        let methodref = self.get_method_from_cache(cache_slot, receiver, method_id)?;

        let keyword = if flag & 0b01 == 1 {
            let val = self.stack_pop();
            Some(val)
        } else {
            None
        };
        let mut args = self.pop_args_to_ary(args_num as usize);
        let block = if block != 0 {
            Some(MethodRef::from(block))
        } else if flag & 0b10 == 2 {
            let val = self.stack_pop();
            let method = val
                .as_proc()
                .ok_or_else(|| self.error_argument("Block argument must be Proc."))?
                .context
                .iseq_ref
                .method;
            Some(method)
        } else {
            None
        };
        args.block = block;
        args.kw_arg = keyword;
        let val = self.eval_send(methodref, receiver, &args)?;
        Ok(val)
    }

    fn vm_opt_send(&mut self, iseq: &ISeq, receiver: Value) -> VMResult {
        let method_id = self.read_id(iseq, 1);
        let args_num = self.read16(iseq, 5);
        let cache_slot = self.read32(iseq, 7);
        let methodref = self.get_method_from_cache(cache_slot, receiver, method_id)?;

        let args = self.pop_args_to_ary(args_num as usize);
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
        let context = self.context();
        self.eval_method(methodref, context.self_value, Some(context), args)
    }

    /// Evaluate method with self_val of current context, caller context as outer context, and given `args`.
    fn eval_yield(&mut self, iseq: &ISeq) -> VMResult {
        let args_num = self.read32(iseq, 1) as usize;
        let args = self.pop_args_to_ary(args_num);
        let mut context = self.context();
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
            self.context().self_value,
            Some(self.caller_context()),
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
        if methodref.is_none() {
            let res = match args.len() {
                0 => Value::nil(),
                1 => args[0],
                _ => {
                    let ary = args.to_vec();
                    Value::array_from(&self.globals, ary)
                }
            };
            return Ok(res);
        };
        let info = self.globals.get_method_info(methodref);
        #[allow(unused_variables, unused_mut)]
        let mut inst: u8;
        #[cfg(feature = "perf")]
        #[cfg_attr(tarpaulin, skip)]
        {
            inst = self.perf.get_prev_inst();
        }
        let val = match info {
            MethodInfo::BuiltinFunc { func, .. } => {
                let func = func.to_owned();
                #[cfg(feature = "perf")]
                #[cfg_attr(tarpaulin, skip)]
                {
                    self.perf.get_perf(Perf::EXTERN);
                }

                let len = self.temp_stack.len();
                self.temp_push(self_val); // If func() returns Err, self_val remains on exec stack.
                self.temp_push_vec(&mut args.to_vec());
                let res = func(self, self_val, args);
                self.temp_stack.truncate(len);

                #[cfg(feature = "perf")]
                #[cfg_attr(tarpaulin, skip)]
                {
                    self.perf.get_perf_no_count(inst);
                }
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
                let context = Context::from_args(self, self_val, iseq, args, outer)?;
                let val = self.run_context(ContextRef::from_local(&context))?;
                #[cfg(feature = "perf")]
                #[cfg_attr(tarpaulin, skip)]
                {
                    self.perf.get_perf_no_count(inst);
                }
                val
            }
        };
        Ok(val)
    }
}

// API's for handling instance/singleton methods.

impl VM {
    pub fn define_method(&mut self, id: IdentId, method: MethodRef) {
        if self.exec_context.len() == 1 {
            // A method defined in "top level" is registered as an object method.
            self.add_object_method(id, method);
        } else {
            // A method defined in a class definition is registered as an instance method of the class.
            self.add_instance_method(self.class(), id, method);
        }
    }

    pub fn define_singleton_method(
        &mut self,
        obj: Value,
        id: IdentId,
        method: MethodRef,
    ) -> Result<(), RubyError> {
        if self.exec_context.len() == 1 {
            // A method defined in "top level" is registered as an object method.
            self.add_object_method(id, method);
            Ok(())
        } else {
            // A method defined in a class definition is registered as an instance method of the class.
            self.add_singleton_method(obj, id, method)
        }
    }

    pub fn add_singleton_method(
        &mut self,
        obj: Value,
        id: IdentId,
        info: MethodRef,
    ) -> Result<(), RubyError> {
        self.globals.class_version += 1;
        let singleton = self.get_singleton_class(obj)?;
        let mut singleton_class = singleton.as_class();
        singleton_class.method_table.insert(id, info);
        Ok(())
    }

    pub fn add_instance_method(
        &mut self,
        class_obj: Value,
        id: IdentId,
        info: MethodRef,
    ) -> Option<MethodRef> {
        self.globals.class_version += 1;
        class_obj.as_module().unwrap().method_table.insert(id, info)
    }

    pub fn add_object_method(&mut self, id: IdentId, info: MethodRef) {
        self.add_instance_method(self.globals.builtins.object, id, info);
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

    /// Get instance method(MethodRef) for the class object.
    pub fn get_instance_method(
        &mut self,
        mut class: Value,
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
        let original_class = class;
        let mut singleton_flag = original_class.as_class().is_singleton;
        loop {
            match class.get_instance_method(method) {
                Some(methodref) => {
                    self.globals
                        .add_method_cache_entry(original_class, method, methodref);
                    return Ok(methodref);
                }
                None => match class.superclass() {
                    Some(superclass) => class = superclass,
                    None => {
                        if singleton_flag {
                            singleton_flag = false;
                            class = original_class.rvalue().class();
                        } else {
                            let inspect = self.val_inspect(original_class);
                            let method_name = IdentId::get_ident_name(method);
                            return Err(self.error_nomethod(format!(
                                "no method `{}' found for {}",
                                method_name, inspect
                            )));
                        }
                    }
                },
            };
        }
    }

    pub fn get_singleton_class(&mut self, obj: Value) -> VMResult {
        self.globals
            .get_singleton_class(obj)
            .map_err(|_| self.error_type("Can not define singleton."))
    }
}

impl VM {
    fn unwind_context(&mut self, err: &mut RubyError) {
        self.context_pop().unwrap();
        if let Some(context) = self.exec_context.last_mut() {
            self.pc = context.pc;
            err.info.push((self.source_info(), self.get_loc()));
        };
    }

    pub fn fiber_send_to_parent(&self, val: VMResult) {
        match &self.parent_fiber {
            Some(ParentFiberInfo { tx, rx, .. }) => {
                tx.send(val).unwrap();
                rx.recv().unwrap();
            }
            None => return,
        };
        #[cfg(feature = "trace")]
        {
            match val {
                Ok(val) => println!("<=== yield Ok({:?})", val),
                Err(err) => println!("<=== yield Err({:?})", err.kind),
            }
        }
    }

    /// Get local variable table.
    fn get_outer_context(&mut self, outer: u32) -> ContextRef {
        let mut context = self.context();
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

    fn pop_key_value_pair(&mut self, arg_num: usize) -> HashMap<HashKey, Value> {
        let mut hash = HashMap::new();
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
        let outer = self.context();
        Ok(ContextRef::from(outer.self_value, None, iseq, Some(outer)))
    }

    pub fn get_iseq(&self, method: MethodRef) -> Result<ISeqRef, RubyError> {
        self.globals.get_method_info(method).as_iseq(&self)
    }

    /// Create new Regexp object from `string`.
    /// Regular expression meta characters are handled as is.
    /// Returns RubyError if `string` was invalid regular expression.
    pub fn create_regexp_from_string(&self, string: &str) -> VMResult {
        let re = RegexpRef::from_string(string).map_err(|err| self.error_regexp(err))?;
        let regexp = Value::regexp(&self.globals, re);
        Ok(regexp)
    }

    /// Create fancy_regex::Regex from `string`.
    /// Escapes all regular expression meta characters in `string`.
    /// Returns RubyError if `string` was invalid regular expression.
    pub fn regexp_from_string(&self, string: &str) -> Result<Regexp, RubyError> {
        match fancy_regex::Regex::new(&regex::escape(string)) {
            Ok(re) => Ok(Regexp::new(re)),
            Err(err) => Err(self.error_regexp(err)),
        }
    }
}
