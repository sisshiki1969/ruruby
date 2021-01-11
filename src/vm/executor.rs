use super::codegen::ContextKind;
use crate::*;

#[cfg(feature = "perf")]
use super::perf::*;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread;
use vm_inst::*;

pub type ValueTable = FxHashMap<IdentId, Value>;
pub type VMResult = Result<Value, RubyError>;

#[derive(Debug)]
pub struct VM {
    // Global info
    pub globals: GlobalsRef,
    //pub root_path: Vec<PathBuf>,
    // VM state
    fiber_state: FiberState,
    exec_context: Vec<ContextRef>,
    class_context: Vec<(Module, DefineMode)>,
    exec_stack: Vec<Value>,
    temp_stack: Vec<Value>,
    //exception: bool,
    pc: usize,
    pub parent_fiber: Option<ParentFiberInfo>,
    pub handle: Option<thread::JoinHandle<()>>,
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
    pub fn new(mut globals: GlobalsRef) -> Self {
        let mut vm = VM {
            globals,
            fiber_state: FiberState::Created,
            class_context: vec![(BuiltinClass::object(), DefineMode::default())],
            exec_context: vec![],
            exec_stack: vec![],
            temp_stack: vec![],
            pc: 0,
            parent_fiber: None,
            handle: None,
            #[cfg(feature = "perf")]
            perf: Perf::new(),
        };

        let load_path = include_str!(concat!(env!("OUT_DIR"), "/libpath.rb"));
        match vm.run(PathBuf::from("(startup)"), load_path) {
            Ok(val) => globals.set_global_var_by_str("$:", val),
            Err(_) => {}
        };

        match vm.run(
            PathBuf::from("ruruby/startup/startup.rb"),
            include_str!("../startup/startup.rb"),
        ) {
            Ok(_) => {}
            Err(err) => {
                err.show_err();
                err.show_loc(0);
                panic!("Error occured in executing startup.rb.");
            }
        };

        #[cfg(feature = "perf")]
        {
            vm.perf = Perf::new();
        }

        vm
    }

    pub fn create_fiber(&mut self, tx: SyncSender<VMResult>, rx: Receiver<FiberMsg>) -> Self {
        let vm = VM {
            globals: self.globals,
            fiber_state: FiberState::Created,
            exec_context: vec![],
            temp_stack: vec![],
            class_context: self.class_context.clone(),
            exec_stack: vec![],
            pc: 0,
            parent_fiber: Some(ParentFiberInfo::new(VMRef::from_ref(self), tx, rx)),
            handle: None,
            #[cfg(feature = "perf")]
            perf: Perf::new(),
        };
        self.globals.fibers.push(VMRef::from_ref(&vm));
        vm
    }

    pub fn current_context(&self) -> ContextRef {
        let ctx = self.exec_context.last().unwrap();
        assert!(!ctx.on_stack || ctx.moved_to_heap.is_none());
        ctx.to_owned()
    }

    fn get_method_context(&self) -> ContextRef {
        let mut context = self.current_context();
        loop {
            context = match context.outer {
                Some(context) => context,
                None => return context,
            };
        }
    }

    pub fn get_method_iseq(&self) -> ISeqRef {
        self.get_method_context().iseq_ref.unwrap()
    }

    pub fn latest_context(&self) -> Option<ContextRef> {
        self.exec_context.last().cloned()
    }

    pub fn source_info(&self) -> SourceInfoRef {
        match self.current_context().iseq_ref {
            Some(iseq) => iseq.source_info,
            None => SourceInfoRef::default(),
        }
    }

    pub fn get_source_path(&self) -> PathBuf {
        self.current_context()
            .iseq_ref
            .unwrap()
            .source_info
            .path
            .clone()
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

    pub fn is_method(&self) -> bool {
        self.current_context().iseq_ref.unwrap().is_method()
    }

    fn stack_push(&mut self, val: Value) {
        self.exec_stack.push(val)
    }

    fn stack_pop(&mut self) -> VMResult {
        match self.exec_stack.pop() {
            Some(val) => Ok(val),
            None => Err(RubyError::internal("exec stack is empty.")),
        }
    }

    fn stack_top(&mut self) -> Value {
        self.exec_stack.last().unwrap().clone()
    }

    fn stack_len(&self) -> usize {
        self.exec_stack.len()
    }

    fn set_stack_len(&mut self, len: usize) {
        self.exec_stack.truncate(len);
    }

    /// Push an object to the temporary area.
    pub fn temp_push(&mut self, v: Value) {
        self.temp_stack.push(v);
    }

    pub fn temp_push_args(&mut self, args: &Args) {
        self.temp_stack.extend_from_slice(args);
        self.temp_stack.push(args.kw_arg);
        if let Block::Proc(val) = args.block {
            self.temp_stack.push(val)
        }
    }

    /// Push objects to the temporary area.
    pub fn temp_push_vec(&mut self, slice: &[Value]) {
        self.temp_stack.extend_from_slice(slice);
    }

    pub fn context_push(&mut self, ctx: ContextRef) {
        self.exec_context.push(ctx);
    }

    pub fn context_pop(&mut self) -> Option<ContextRef> {
        self.exec_context.pop()
    }

    pub fn clear(&mut self) {
        self.exec_stack.clear();
        self.class_context = vec![(BuiltinClass::object(), DefineMode::default())];
        self.exec_context.clear();
    }

    pub fn class_push(&mut self, val: Module) {
        self.class_context.push((val, DefineMode::default()));
    }

    pub fn class_pop(&mut self) {
        self.class_context.pop().unwrap();
    }

    /// Get Class of current class context.
    pub fn class(&self) -> Module {
        self.class_context.last().unwrap().0
    }

    pub fn define_mode(&self) -> &DefineMode {
        &self.class_context.last().unwrap().1
    }

    pub fn define_mode_mut(&mut self) -> &mut DefineMode {
        &mut self.class_context.last_mut().unwrap().1
    }

    pub fn module_function(&mut self, flag: bool) {
        self.define_mode_mut().module_function = flag;
    }

    pub fn jump_pc(&mut self, inst_offset: usize, disp: i64) {
        self.pc = (((self.pc + inst_offset) as i64) + disp) as usize;
    }

    pub fn parse_program(&mut self, path: PathBuf, program: &str) -> Result<MethodRef, RubyError> {
        let parser = Parser::new();
        let result = parser.parse_program(path, program)?;

        #[cfg(feature = "perf")]
        self.perf.set_prev_inst(Perf::INVALID);

        let methodref = Codegen::new(result.source_info).gen_iseq(
            &mut self.globals,
            vec![],
            result.node,
            result.lvar_collector,
            true,
            ContextKind::Method,
            None,
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
        let extern_context = self.current_context();
        let result = parser.parse_program_eval(path, program, Some(extern_context))?;

        #[cfg(feature = "perf")]
        self.perf.set_prev_inst(Perf::INVALID);

        let mut codegen = Codegen::new(result.source_info);
        codegen.set_external_context(extern_context);
        let method = codegen.gen_iseq(
            &mut self.globals,
            vec![],
            result.node,
            result.lvar_collector,
            true,
            ContextKind::Eval,
            None,
            None,
        )?;
        Ok(method)
    }

    pub fn run(&mut self, path: PathBuf, program: &str) -> VMResult {
        let method = self.parse_program(path, program)?;
        let mut iseq = method.as_iseq();
        iseq.class_defined = self.get_class_defined();
        let self_value = self.globals.main_object;
        let val = self.eval_send(method, self_value, &Args::new0())?;
        #[cfg(feature = "perf")]
        self.perf.get_perf(Perf::INVALID);
        assert!(
            self.stack_len() == 0,
            "exec_stack length must be 0. actual:{}",
            self.stack_len()
        );
        Ok(val)
    }

    #[cfg(not(tarpaulin_include))]
    pub fn run_repl(&mut self, result: ParseResult, mut context: ContextRef) -> VMResult {
        #[cfg(feature = "perf")]
        self.perf.set_prev_inst(Perf::CODEGEN);

        let method = Codegen::new(result.source_info).gen_iseq(
            &mut self.globals,
            vec![],
            result.node,
            result.lvar_collector,
            true,
            ContextKind::Method,
            None,
            None,
        )?;
        let iseq = method.as_iseq();
        context.iseq_ref = Some(iseq);
        context.adjust_lvar_size();
        //context.pc = 0;

        let val = self.run_context(context)?;
        #[cfg(feature = "perf")]
        self.perf.get_perf(Perf::INVALID);

        let stack_len = self.stack_len();
        if stack_len != 0 {
            eprintln!("Error: stack length is illegal. {}", stack_len);
        };

        Ok(val)
    }

    #[allow(dead_code)]
    #[cfg(not(tarpaulin_include))]
    pub fn dump_context(&self) {
        eprintln!("---dump");
        for (i, context) in self.exec_context.iter().rev().enumerate() {
            eprintln!("context: {}", i);
            context.dump();
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
        if !self.globals.allocator.is_allocated() {
            return;
        };
        #[cfg(feature = "perf")]
        self.perf.get_perf(Perf::GC);
        self.globals.gc();
    }

    fn jmp_cond(&mut self, iseq: &ISeq, cond: bool, inst_offset: usize, dest_offset: usize) {
        if cond {
            self.pc += inst_offset;
        } else {
            let disp = iseq.read_disp(self.pc + dest_offset);
            self.jump_pc(inst_offset, disp);
        }
    }

    pub fn run_context(&mut self, context: impl Into<ContextRef>) -> VMResult {
        let context = context.into();
        let stack_len = self.stack_len();
        let pc = self.pc;
        self.context_push(context);
        self.pc = 0;
        #[cfg(feature = "trace")]
        {
            print!("--->");
            println!(" {:?} {:?}", context.iseq_ref.unwrap().method, context.kind);
            context.dump();
            eprintln!("  ------------------------------------------------------------------");
        }
        #[cfg(feature = "trace-func")]
        {
            print!("--->");
            println!(" {:?} {:?}", context.iseq_ref.unwrap().method, context.kind);
        }
        loop {
            match self.run_context_main(context) {
                Ok(val) => {
                    self.context_pop().unwrap();
                    debug_assert_eq!(stack_len, self.stack_len());
                    self.pc = pc;
                    #[cfg(any(feature = "trace", feature = "trace-func"))]
                    println!("<--- Ok({:?})", val);
                    return Ok(val);
                }
                Err(mut err) => {
                    err.info.push((self.source_info(), self.get_loc()));
                    //eprintln!("{:?}", iseq.exception_table);
                    if let RubyErrorKind::Internal(msg) = &err.kind {
                        eprintln!();
                        err.show_err();
                        err.show_all_loc();
                        unreachable!("{}", msg);
                    };
                    let iseq = self.current_context().iseq_ref.unwrap();
                    let catch = iseq.exception_table.iter().find(|x| x.include(self.pc));
                    if let Some(entry) = catch {
                        // Exception raised inside of begin-end with rescue clauses.
                        self.pc = entry.dest.to_usize();
                        self.set_stack_len(stack_len);
                        let val = err.to_exception_val(&self.globals);
                        self.stack_push(val);
                    } else {
                        // Exception raised outside of begin-end.
                        self.context_pop().unwrap();
                        self.set_stack_len(stack_len);
                        self.pc = pc;
                        #[cfg(any(feature = "trace", feature = "trace-func"))]
                        {
                            println!("<--- Err({:?})", err.kind);
                        }
                        return Err(err);
                    }
                }
            }
        }
    }

    /// Main routine for VM execution.
    fn run_context_main(&mut self, context: ContextRef) -> VMResult {
        let iseq = &mut context.iseq_ref.unwrap().iseq;
        let self_value = context.self_value;
        let self_oref = self_value.rvalue_mut();
        self.gc();
        for (i, (outer, lvar)) in context.iseq_ref.unwrap().forvars.iter().enumerate() {
            self.get_outer_context(*outer)[*lvar as usize] = context[i];
        }
        /// Evaluate expr, and push return value to stack.
        macro_rules! try_push {
            ($eval:expr) => {
                match $eval {
                    Ok(val) => self.stack_push(val),
                    Err(err) => match err.kind {
                        RubyErrorKind::BlockReturn(val) => self.stack_push(val),
                        RubyErrorKind::MethodReturn(val) if self.is_method() => return Ok(val),
                        _ => return Err(err),
                    },
                };
            };
        }

        /// Evaluate expr, and discard return value.
        macro_rules! try_no_push {
            ($eval:expr) => {
                match $eval {
                    Ok(_) => {}
                    Err(err) => match err.kind {
                        RubyErrorKind::BlockReturn(_) => {}
                        RubyErrorKind::MethodReturn(val) if self.is_method() => return Ok(val),
                        _ => return Err(err),
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
                    "{:>4x}: {:<40} tmp: {:<4} stack: {:<3} top: {}",
                    self.pc,
                    Inst::inst_info(&self.globals, context.iseq_ref.unwrap(), self.pc),
                    self.temp_stack.len(),
                    self.stack_len(),
                    match self.exec_stack.last() {
                        Some(x) => format!("{:?}", x),
                        None => "".to_string(),
                    }
                );
            }
            match iseq[self.pc] {
                Inst::RETURN => {
                    // - reached the end of the method or block.
                    // - `return` in method.
                    // - `next` in block AND outer of loops.
                    let val = self.stack_pop()?;
                    return Ok(val);
                }
                Inst::BREAK => {
                    // - `break`  in block or eval AND outer of loops.
                    #[cfg(debug_assertions)]
                    assert!(context.kind == ISeqKind::Block || context.kind == ISeqKind::Other);
                    let val = self.stack_pop()?;
                    let err = RubyError::block_return(val);
                    return Err(err);
                }
                Inst::MRETURN => {
                    // - `return` in block
                    #[cfg(debug_assertions)]
                    assert_eq!(context.kind, ISeqKind::Block);
                    let val = self.stack_pop()?;
                    let err = RubyError::method_return(val);
                    return Err(err);
                }
                Inst::THROW => {
                    // - raise error
                    let val = self.stack_pop()?;
                    return Err(RubyError::value(val));
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
                    self.stack_push(self_value);
                    self.pc += 1;
                }
                Inst::PUSH_FIXNUM => {
                    let num = iseq.read64(self.pc + 1);
                    self.pc += 9;
                    self.stack_push(Value::integer(num as i64));
                }
                Inst::PUSH_FLONUM => {
                    let num = f64::from_bits(iseq.read64(self.pc + 1));
                    self.pc += 9;
                    self.stack_push(Value::float(num));
                }
                Inst::PUSH_SYMBOL => {
                    let id = iseq.read_id(self.pc + 1);
                    self.stack_push(Value::symbol(id));
                    self.pc += 5;
                }
                Inst::ADD => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let val = self.eval_add(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::ADDI => {
                    let lhs = self.stack_pop()?;
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = self.eval_addi(lhs, i)?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::SUB => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let val = self.eval_sub(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::SUBI => {
                    let lhs = self.stack_pop()?;
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = self.eval_subi(lhs, i)?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::MUL => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let val = self.eval_mul(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::POW => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let val = self.eval_exp(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::DIV => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let val = self.eval_div(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::REM => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let val = self.eval_rem(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::SHR => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let val = self.eval_shr(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::SHL => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let val = self.eval_shl(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::NEG => {
                    let lhs = self.stack_pop()?;
                    let val = self.eval_neg(lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::BAND => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let val = self.eval_bitand(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::B_ANDI => {
                    let lhs = self.stack_pop()?;
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = self.eval_bitandi(lhs, i)?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::BOR => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let val = self.eval_bitor(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::B_ORI => {
                    let lhs = self.stack_pop()?;
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = self.eval_bitori(lhs, i)?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::BXOR => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let val = self.eval_bitxor(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::BNOT => {
                    let lhs = self.stack_pop()?;
                    let val = self.eval_bitnot(lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }

                Inst::EQ => {
                    let lhs = self.stack_pop()?;
                    let rhs = self.stack_pop()?;
                    let val = Value::bool(self.eval_eq(rhs, lhs)?);
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::EQI => {
                    let lhs = self.stack_pop()?;
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = Value::bool(self.eval_eqi(lhs, i));
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::NE => {
                    let lhs = self.stack_pop()?;
                    let rhs = self.stack_pop()?;
                    let val = Value::bool(!self.eval_eq(rhs, lhs)?);
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::NEI => {
                    let lhs = self.stack_pop()?;
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = Value::bool(!self.eval_eqi(lhs, i));
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::TEQ => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let res = self.eval_teq(rhs, lhs)?;
                    let val = Value::bool(res);
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::GT => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let val = self.eval_gt(rhs, lhs).map(|x| Value::bool(x))?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::GTI => {
                    let lhs = self.stack_pop()?;
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = self.eval_gti(lhs, i).map(|x| Value::bool(x))?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::GE => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let val = self.eval_ge(rhs, lhs).map(|x| Value::bool(x))?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::GEI => {
                    let lhs = self.stack_pop()?;
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = self.eval_gei(lhs, i).map(|x| Value::bool(x))?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::LT => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let val = self.eval_lt(rhs, lhs).map(|x| Value::bool(x))?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::LTI => {
                    let lhs = self.stack_pop()?;
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = self.eval_lti(lhs, i).map(|x| Value::bool(x))?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::LE => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let val = self.eval_le(rhs, lhs).map(|x| Value::bool(x))?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::LEI => {
                    let lhs = self.stack_pop()?;
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = self.eval_lei(lhs, i).map(|x| Value::bool(x))?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::CMP => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let val = self.eval_compare(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::NOT => {
                    let lhs = self.stack_pop()?;
                    let val = Value::bool(!lhs.to_bool());
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::RESCUE => {
                    let len = iseq.read32(self.pc + 1) as usize;
                    let stack_len = self.exec_stack.len();
                    let val = self.exec_stack[stack_len - len - 1];
                    let ex = &self.exec_stack[stack_len - len..];
                    let b = self.eval_rescue(val, ex)?;
                    self.set_stack_len(stack_len - len - 1);
                    self.stack_push(Value::bool(b));
                    self.pc += 5;
                }
                Inst::CONCAT_STRING => {
                    let num = iseq.read32(self.pc + 1) as usize;
                    let stack_len = self.stack_len();
                    /*let mut res = String::new();
                    for v in self.exec_stack.drain(stack_len - num..stack_len) {
                        res += v.as_string().unwrap();
                    }*/
                    let res = self
                        .exec_stack
                        .drain(stack_len - num..)
                        .fold(String::new(), |acc, x| acc + x.as_string().unwrap());

                    let val = Value::string(res);
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::SET_LOCAL => {
                    let id = iseq.read_lvar_id(self.pc + 1);
                    let val = self.stack_pop()?;
                    self.current_context()[id] = val;
                    self.pc += 5;
                }
                Inst::GET_LOCAL => {
                    let id = iseq.read_lvar_id(self.pc + 1);
                    let val = self.current_context()[id];
                    if val.is_uninitialized() {
                        self.current_context()[id] = Value::nil();
                        self.stack_push(Value::nil());
                    } else {
                        self.stack_push(val);
                    }
                    self.pc += 5;
                }
                Inst::LVAR_ADDI => {
                    let id = iseq.read_lvar_id(self.pc + 1);
                    let i = iseq.read32(self.pc + 5) as i32;
                    let val = self.current_context()[id];
                    self.current_context()[id] = self.eval_addi(val, i)?;
                    self.pc += 9;
                }
                Inst::SET_DYNLOCAL => {
                    let id = iseq.read_lvar_id(self.pc + 1);
                    let outer = iseq.read32(self.pc + 5);
                    let val = self.stack_pop()?;
                    let mut cref = self.get_outer_context(outer);
                    cref[id] = val;
                    self.pc += 9;
                }
                Inst::GET_DYNLOCAL => {
                    let id = iseq.read_lvar_id(self.pc + 1);
                    let outer = iseq.read32(self.pc + 5);
                    let cref = self.get_outer_context(outer);
                    let val = cref[id];
                    self.stack_push(val);
                    self.pc += 9;
                }
                Inst::CHECK_LOCAL => {
                    let id = iseq.read_lvar_id(self.pc + 1);
                    let outer = iseq.read32(self.pc + 5);
                    let cref = self.get_outer_context(outer);
                    let val = cref[id].is_uninitialized();
                    self.stack_push(Value::bool(val));
                    self.pc += 9;
                }
                Inst::SET_CONST => {
                    let id = iseq.read_id(self.pc + 1);
                    let parent = match self.stack_pop()? {
                        v if v.is_nil() => match self.get_method_iseq().class_defined.last() {
                            Some(class) => *class,
                            None => self.globals.builtins.object,
                        }, //self.class(),
                        v => v.expect_mod_class()?,
                    };
                    let val = self.stack_pop()?;
                    self.globals.set_const(parent, id, val);
                    self.pc += 5;
                }
                Inst::GET_CONST => {
                    let id = iseq.read_id(self.pc + 1);
                    let slot = iseq.read32(self.pc + 5);
                    let val = match self.globals.find_const_cache(slot) {
                        Some(val) => val,
                        None => {
                            let val = match self.get_env_const(id) {
                                Some(val) => val,
                                None => VM::get_super_const(self.class(), id)?,
                            };
                            self.globals.set_const_cache(slot, val);
                            val
                        }
                    };

                    self.stack_push(val);
                    self.pc += 9;
                }
                Inst::GET_CONST_TOP => {
                    let id = iseq.read_id(self.pc + 1);
                    let parent = self.globals.builtins.object;
                    let val = self.get_const(parent, id)?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::GET_SCOPE => {
                    let parent = self.stack_pop()?.expect_mod_class()?;
                    let id = iseq.read_id(self.pc + 1);
                    let val = self.get_const(parent, id)?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::SET_IVAR => {
                    let var_id = iseq.read_id(self.pc + 1);
                    let new_val = self.stack_pop()?;
                    self_oref.set_var(var_id, new_val);
                    self.pc += 5;
                }
                Inst::GET_IVAR => {
                    let var_id = iseq.read_id(self.pc + 1);
                    let val = match self_oref.get_var(var_id) {
                        Some(val) => val,
                        None => Value::nil(),
                    };
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::CHECK_IVAR => {
                    let var_id = iseq.read_id(self.pc + 1);
                    let val = match self_oref.get_var(var_id) {
                        Some(_) => Value::false_val(),
                        None => Value::true_val(),
                    };
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::IVAR_ADDI => {
                    let var_id = iseq.read_id(self.pc + 1);
                    let i = iseq.read32(self.pc + 5) as i32;
                    let v = self_oref
                        .var_table_mut()
                        .entry(var_id)
                        .or_insert(Value::nil());
                    *v = self.eval_addi(*v, i)?;

                    self.pc += 9;
                }
                Inst::SET_GVAR => {
                    let var_id = iseq.read_id(self.pc + 1);
                    let new_val = self.stack_pop()?;
                    self.set_global_var(var_id, new_val);
                    self.pc += 5;
                }
                Inst::GET_GVAR => {
                    let var_id = iseq.read_id(self.pc + 1);
                    let val = self.get_global_var(var_id).unwrap_or(Value::nil());
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::CHECK_GVAR => {
                    let var_id = iseq.read_id(self.pc + 1);
                    let val = match self.get_global_var(var_id) {
                        Some(_) => Value::false_val(),
                        None => Value::true_val(),
                    };
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::SET_CVAR => {
                    let var_id = iseq.read_id(self.pc + 1);
                    let new_val = self.stack_pop()?;
                    self.set_class_var(var_id, new_val)?;
                    self.pc += 5;
                }
                Inst::GET_CVAR => {
                    let var_id = iseq.read_id(self.pc + 1);
                    let val = self.get_class_var(var_id)?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::SET_INDEX => {
                    self.set_index()?;
                    self.pc += 1;
                }
                Inst::GET_INDEX => {
                    let val = self.get_index()?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::SET_IDX_I => {
                    let idx = iseq.read32(self.pc + 1);
                    self.set_index_imm(idx)?;
                    self.pc += 5;
                }
                Inst::GET_IDX_I => {
                    let idx = iseq.read32(self.pc + 1);
                    let val = self.get_index_imm(idx)?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::SPLAT => {
                    let val = self.stack_pop()?;
                    let res = Value::splat(val);
                    self.stack_push(res);
                    self.pc += 1;
                }
                Inst::CONST_VAL => {
                    let id = iseq.read_usize(self.pc + 1);
                    let val = self.globals.const_values.get(id);
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::CREATE_RANGE => {
                    let start = self.stack_pop()?;
                    let end = self.stack_pop()?;
                    let exclude_end = self.stack_pop()?.to_bool();
                    let range = self.create_range(start, end, exclude_end)?;
                    self.stack_push(range);
                    self.pc += 1;
                }
                Inst::CREATE_ARRAY => {
                    let arg_num = iseq.read_usize(self.pc + 1);
                    let elems = self.pop_args_to_args(arg_num).into_vec();
                    let array = Value::array_from(elems);
                    self.stack_push(array);
                    self.pc += 5;
                }
                Inst::CREATE_PROC => {
                    let method = iseq.read_methodref(self.pc + 1);
                    let ctx = self.current_context();
                    let proc_obj = self.create_proc(&Block::Block(method, ctx))?;
                    self.stack_push(proc_obj);
                    self.pc += 9;
                }
                Inst::CREATE_HASH => {
                    let arg_num = iseq.read_usize(self.pc + 1);
                    let key_value = self.pop_key_value_pair(arg_num);
                    let hash = Value::hash_from_map(key_value);
                    self.stack_push(hash);
                    self.pc += 5;
                }
                Inst::CREATE_REGEXP => {
                    let arg = self.stack_pop()?;
                    let regexp = self.create_regexp(arg)?;
                    self.stack_push(regexp);
                    self.pc += 1;
                }
                Inst::JMP => {
                    let disp = iseq.read_disp(self.pc + 1);
                    self.jump_pc(5, disp);
                }
                Inst::JMP_BACK => {
                    let disp = iseq.read_disp(self.pc + 1);
                    self.gc();
                    self.jump_pc(5, disp);
                }
                Inst::JMP_F => {
                    let val = self.stack_pop()?;
                    let b = val.to_bool();
                    self.jmp_cond(iseq, b, 5, 1);
                }
                Inst::JMP_T => {
                    let val = self.stack_pop()?;
                    let b = !val.to_bool();
                    self.jmp_cond(iseq, b, 5, 1);
                }

                Inst::JMP_F_EQ => {
                    let lhs = self.stack_pop()?;
                    let rhs = self.stack_pop()?;
                    let b = self.eval_eq(rhs, lhs)?;
                    self.jmp_cond(iseq, b, 5, 1);
                }
                Inst::JMP_F_NE => {
                    let lhs = self.stack_pop()?;
                    let rhs = self.stack_pop()?;
                    let b = !self.eval_eq(rhs, lhs)?;
                    self.jmp_cond(iseq, b, 5, 1);
                }
                Inst::JMP_F_GT => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let b = self.eval_gt(rhs, lhs)?;
                    self.jmp_cond(iseq, b, 5, 1);
                }
                Inst::JMP_F_GE => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let b = self.eval_ge(rhs, lhs)?;
                    self.jmp_cond(iseq, b, 5, 1);
                }
                Inst::JMP_F_LT => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let b = self.eval_lt(rhs, lhs)?;
                    self.jmp_cond(iseq, b, 5, 1);
                }
                Inst::JMP_F_LE => {
                    let rhs = self.stack_pop()?;
                    let lhs = self.stack_pop()?;
                    let b = self.eval_le(rhs, lhs)?;
                    self.jmp_cond(iseq, b, 5, 1);
                }

                Inst::JMP_F_EQI => {
                    let lhs = self.stack_pop()?;
                    let i = iseq.read32(self.pc + 1) as i32;
                    let b = self.eval_eqi(lhs, i);
                    self.jmp_cond(iseq, b, 9, 5);
                }
                Inst::JMP_F_NEI => {
                    let lhs = self.stack_pop()?;
                    let i = iseq.read32(self.pc + 1) as i32;
                    let b = !self.eval_eqi(lhs, i);
                    self.jmp_cond(iseq, b, 9, 5);
                }
                Inst::JMP_F_GTI => {
                    let lhs = self.stack_pop()?;
                    let i = iseq.read32(self.pc + 1) as i32;
                    let b = self.eval_gti(lhs, i)?;
                    self.jmp_cond(iseq, b, 9, 5);
                }
                Inst::JMP_F_GEI => {
                    let lhs = self.stack_pop()?;
                    let i = iseq.read32(self.pc + 1) as i32;
                    let b = self.eval_gei(lhs, i)?;
                    self.jmp_cond(iseq, b, 9, 5);
                }
                Inst::JMP_F_LTI => {
                    let lhs = self.stack_pop()?;
                    let i = iseq.read32(self.pc + 1) as i32;
                    let b = self.eval_lti(lhs, i)?;
                    self.jmp_cond(iseq, b, 9, 5);
                }
                Inst::JMP_F_LEI => {
                    let lhs = self.stack_pop()?;
                    let i = iseq.read32(self.pc + 1) as i32;
                    let b = self.eval_lei(lhs, i)?;
                    self.jmp_cond(iseq, b, 9, 5);
                }

                Inst::OPT_CASE => {
                    let val = self.stack_pop()?;
                    let map = self
                        .globals
                        .case_dispatch
                        .get_entry(iseq.read32(self.pc + 1));
                    let disp = match map.get(&val) {
                        Some(disp) => *disp as i64,
                        None => iseq.read_disp(self.pc + 5),
                    };
                    self.jump_pc(9, disp);
                }
                Inst::SEND => {
                    let receiver = self.stack_pop()?;
                    try_push!(self.vm_send(iseq, receiver));
                    self.pc += 21;
                }
                Inst::SEND_SELF => {
                    try_push!(self.vm_send(iseq, self_value));
                    self.pc += 21;
                }
                Inst::OPT_SEND => {
                    let receiver = self.stack_pop()?;
                    try_push!(self.vm_fast_send(iseq, receiver));
                    self.pc += 11;
                }
                Inst::OPT_NSEND => {
                    let receiver = self.stack_pop()?;
                    try_no_push!(self.vm_fast_send(iseq, receiver));
                    self.pc += 11;
                }
                Inst::OPT_SEND_SELF => {
                    try_push!(self.vm_fast_send(iseq, self_value));
                    self.pc += 11;
                }
                Inst::OPT_NSEND_SELF => {
                    try_no_push!(self.vm_fast_send(iseq, self_value));
                    self.pc += 11;
                }
                Inst::OPT_SEND_BLK => {
                    let receiver = self.stack_pop()?;
                    try_push!(self.vm_fast_send_with_block(iseq, receiver));
                    self.pc += 19;
                }
                Inst::OPT_NSEND_BLK => {
                    let receiver = self.stack_pop()?;
                    try_no_push!(self.vm_fast_send_with_block(iseq, receiver));
                    self.pc += 19;
                }
                Inst::OPT_SEND_SELF_BLK => {
                    try_push!(self.vm_fast_send_with_block(iseq, self_value));
                    self.pc += 19;
                }
                Inst::OPT_NSEND_SELF_BLK => {
                    try_no_push!(self.vm_fast_send_with_block(iseq, self_value));
                    self.pc += 19;
                }
                Inst::FOR => {
                    let receiver = self.stack_pop()?;
                    try_push!(self.vm_for(iseq, receiver));
                    self.pc += 13;
                }
                Inst::YIELD => {
                    let args_num = iseq.read32(self.pc + 1) as usize;
                    let args = self.pop_args_to_args(args_num);
                    try_push!(self.eval_yield(&args));
                    self.pc += 5;
                }
                Inst::DEF_CLASS => {
                    let is_module = iseq.read8(self.pc + 1) == 1;
                    let id = iseq.read_id(self.pc + 2);
                    let method = iseq.read_methodref(self.pc + 6);
                    let base = self.stack_pop()?;
                    let super_val = self.stack_pop()?;
                    let val = self.define_class(base, id, is_module, super_val)?;
                    self.class_push(val);
                    let mut iseq = method.as_iseq();
                    iseq.class_defined = self.get_class_defined();
                    let res = self.eval_send(method, val, &Args::new0());
                    self.class_pop();
                    try_push!(res);
                    self.pc += 14;
                }
                Inst::DEF_SCLASS => {
                    let method = iseq.read_methodref(self.pc + 1);
                    let singleton = self.stack_pop()?.get_singleton_class()?;
                    self.class_push(singleton);
                    let mut iseq = method.as_iseq();
                    iseq.class_defined = self.get_class_defined();
                    let res = self.eval_send(method, singleton, &Args::new0());
                    self.class_pop();
                    try_push!(res);
                    self.pc += 9;
                }
                Inst::DEF_METHOD => {
                    let id = iseq.read_id(self.pc + 1);
                    let method = iseq.read_methodref(self.pc + 5);
                    let mut iseq = method.as_iseq();
                    iseq.class_defined = self.get_method_iseq().class_defined.clone();
                    self.define_method(self_value, id, method);
                    if self.define_mode().module_function {
                        self.define_singleton_method(self_value, id, method)?;
                    };
                    self.pc += 13;
                }
                Inst::DEF_SMETHOD => {
                    let id = iseq.read_id(self.pc + 1);
                    let method = iseq.read_methodref(self.pc + 5);
                    let mut iseq = method.as_iseq();
                    iseq.class_defined = self.get_method_iseq().class_defined.clone();
                    let singleton = self.stack_pop()?;
                    self.define_singleton_method(singleton, id, method)?;
                    if self.define_mode().module_function {
                        self.define_method(singleton, id, method);
                    };
                    self.pc += 13;
                }
                Inst::TO_S => {
                    let val = self.stack_pop()?;
                    let s = val.val_to_s(self)?;
                    let res = Value::string(s);
                    self.stack_push(res);
                    self.pc += 1;
                }
                Inst::POP => {
                    self.stack_pop()?;
                    self.pc += 1;
                }
                Inst::DUP => {
                    let len = iseq.read_usize(self.pc + 1);
                    let stack_len = self.stack_len();
                    for i in stack_len - len..stack_len {
                        let val = self.exec_stack[i];
                        self.stack_push(val);
                    }
                    self.pc += 5;
                }
                Inst::SINKN => {
                    let len = iseq.read_usize(self.pc + 1);
                    let val = self.stack_pop()?;
                    let stack_len = self.stack_len();
                    self.exec_stack.insert(stack_len - len, val);
                    self.pc += 5;
                }
                Inst::TOPN => {
                    let len = iseq.read_usize(self.pc + 1);
                    let val = self.exec_stack.remove(self.stack_len() - 1 - len);
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::TAKE => {
                    let len = iseq.read_usize(self.pc + 1);
                    let val = self.stack_pop()?;
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
                inst => {
                    return Err(RubyError::internal(format!(
                        "Unimplemented instruction. {}",
                        Inst::inst_name(inst)
                    )))
                }
            }
        }
    }
}

// helper functions for run_context_main.
impl VM {
    fn vm_send(&mut self, iseq: &mut ISeq, receiver: Value) -> VMResult {
        let method_id = iseq.read_id(self.pc + 1);
        let args_num = iseq.read16(self.pc + 5);
        let kw_rest_num = iseq.read8(self.pc + 7);
        let flag = iseq.read8(self.pc + 8);
        let block = iseq.read64(self.pc + 9);
        let cache = iseq.read32(self.pc + 17);

        let mut kwrest = vec![];
        for _ in 0..kw_rest_num {
            let val = self.stack_pop()?;
            kwrest.push(val);
        }

        let keyword = if flag & 0b01 == 1 {
            let mut val = self.stack_pop()?;
            let hash = val.as_mut_hash().unwrap();
            for h in kwrest {
                for (k, v) in h.expect_hash("Arg")? {
                    hash.insert(k, v);
                }
            }
            val
        } else if kwrest.len() == 0 {
            Value::nil()
        } else {
            let mut hash = FxHashMap::default();
            for h in kwrest {
                for (k, v) in h.expect_hash("Arg")? {
                    hash.insert(HashKey(k), v);
                }
            }
            Value::hash_from_map(hash)
        };

        let block = if block != 0 {
            Block::Block(block.into(), self.current_context())
        } else if flag & 0b10 == 2 {
            let val = self.stack_pop()?;
            if val.is_nil() {
                Block::None
            } else {
                if val.as_proc().is_none() {
                    return Err(RubyError::internal(format!(
                        "Must be Proc. {:?}:{}",
                        val,
                        val.get_class_name()
                    )));
                    /*return Err(RubyError::typeerr(format!(
                        "Wrong argument type {:?} (expected Proc).",
                        val,
                    )));*/
                }
                Block::Proc(val)
            }
        } else {
            Block::None
        };
        let mut args = self.pop_args_to_args(args_num as usize);
        args.block = block;
        args.kw_arg = keyword;
        self.send_icache(cache, method_id, receiver, &args)
    }

    fn vm_fast_send_with_block(&mut self, iseq: &mut ISeq, receiver: Value) -> VMResult {
        // With block and no keyword/block/splat arguments for OPT_SEND.
        let method_id = iseq.read_id(self.pc + 1);
        let args_num = iseq.read16(self.pc + 5) as usize;
        let block = iseq.read64(self.pc + 7);
        assert!(block != 0);
        let cache = iseq.read32(self.pc + 15);
        let len = self.stack_len();
        let arg_slice = &self.exec_stack[len - args_num..];

        match self
            .globals
            .find_method_from_icache(cache, receiver, method_id)
        {
            Some(method) => match &*method {
                MethodInfo::BuiltinFunc { func, name } => {
                    let mut args = Args::from_slice(arg_slice);
                    args.block = Block::Block(block.into(), self.current_context());
                    self.set_stack_len(len - args_num);
                    self.invoke_native(func, *name, receiver, &args)
                }
                MethodInfo::AttrReader { id } => {
                    if args_num != 0 {
                        return Err(RubyError::argument_wrong(args_num, 0));
                    }
                    Self::invoke_getter(*id, receiver)
                }
                MethodInfo::AttrWriter { id } => {
                    if args_num != 1 {
                        return Err(RubyError::argument_wrong(args_num, 1));
                    }
                    Self::invoke_setter(*id, receiver, self.stack_pop()?)
                }
                MethodInfo::RubyFunc { iseq } => {
                    let block = Block::Block(block.into(), self.current_context());
                    if iseq.opt_flag {
                        let mut context = Context::new(receiver, block, *iseq, None);
                        let req_len = iseq.params.req;
                        if args_num != req_len {
                            return Err(RubyError::argument_wrong(args_num, req_len));
                        };
                        context.copy_from_slice0(arg_slice);
                        self.set_stack_len(len - args_num);
                        self.run_context(&context)
                    } else {
                        let mut args = Args::from_slice(arg_slice);
                        args.block = block;
                        self.set_stack_len(len - args_num);
                        let context = Context::from_args(self, receiver, *iseq, &args, None)?;
                        self.run_context(&context)
                    }
                }
                _ => unreachable!(),
            },
            None => {
                let mut args = Args::from_slice(arg_slice);
                args.block = Block::Block(block.into(), self.current_context());
                self.set_stack_len(len - args_num);
                self.send_method_missing(method_id, receiver, &args)
            }
        }
    }

    fn vm_fast_send(&mut self, iseq: &mut ISeq, receiver: Value) -> VMResult {
        // No block nor keyword/block/splat arguments for OPT_SEND.
        let method_id = iseq.read_id(self.pc + 1);
        let args_num = iseq.read16(self.pc + 5) as usize;
        let cache = iseq.read32(self.pc + 7);
        let len = self.stack_len();
        let arg_slice = &self.exec_stack[len - args_num..];

        match self
            .globals
            .find_method_from_icache(cache, receiver, method_id)
        {
            Some(method) => match &*method {
                MethodInfo::BuiltinFunc { func, name } => {
                    let args = Args::from_slice(arg_slice);
                    self.set_stack_len(len - args_num);
                    self.invoke_native(func, *name, receiver, &args)
                }
                MethodInfo::AttrReader { id } => {
                    if args_num != 0 {
                        return Err(RubyError::argument_wrong(args_num, 0));
                    }
                    Self::invoke_getter(*id, receiver)
                }
                MethodInfo::AttrWriter { id } => {
                    if args_num != 1 {
                        return Err(RubyError::argument_wrong(args_num, 1));
                    }
                    Self::invoke_setter(*id, receiver, self.stack_pop()?)
                }
                MethodInfo::RubyFunc { iseq } => {
                    if iseq.opt_flag {
                        let mut context = Context::new(receiver, Block::None, *iseq, None);
                        let req_len = iseq.params.req;
                        if args_num != req_len {
                            return Err(RubyError::argument_wrong(args_num, req_len));
                        };
                        context.copy_from_slice0(arg_slice);
                        self.set_stack_len(len - args_num);
                        self.run_context(&context)
                    } else {
                        let args = Args::from_slice(arg_slice);
                        self.set_stack_len(len - args_num);
                        let context = Context::from_args(self, receiver, *iseq, &args, None)?;
                        self.run_context(&context)
                    }
                }
                _ => unreachable!(),
            },
            None => {
                let args = Args::from_slice(arg_slice);
                self.set_stack_len(len - args_num);
                self.send_method_missing(method_id, receiver, &args)
            }
        }
    }

    fn vm_for(&mut self, iseq: &mut ISeq, receiver: Value) -> VMResult {
        // With block and no keyword/block/splat arguments for OPT_SEND.
        let block = iseq.read64(self.pc + 1);
        assert!(block != 0);
        let block = Block::Block(block.into(), self.current_context());
        let args = Args::new0_block(block);
        let cache = iseq.read32(self.pc + 9);

        match self
            .globals
            .find_method_from_icache(cache, receiver, IdentId::EACH)
        {
            Some(method) => match &*method {
                MethodInfo::BuiltinFunc { func, name } => {
                    self.invoke_native(func, *name, receiver, &args)
                }
                MethodInfo::RubyFunc { iseq } => {
                    let context = Context::from_args(self, receiver, *iseq, &args, None)?;
                    self.run_context(&context)
                }
                _ => unreachable!(),
            },
            None => self.send_method_missing(IdentId::EACH, receiver, &args),
        }
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

    /// Get class list in the current context.
    ///
    /// At first, this method searches the class list of outer context,
    /// and adds a class given as an argument `new_class` on the top of the list.
    /// return None in top-level.
    fn get_class_defined(&self) -> Vec<Module> {
        self.class_context.iter().map(|(v, _)| *v).collect()
    }
}

// handling global/class varables.

impl VM {
    pub fn get_global_var(&self, id: IdentId) -> Option<Value> {
        self.globals.get_global_var(id)
    }

    pub fn set_global_var(&mut self, id: IdentId, val: Value) {
        self.globals.set_global_var(id, val);
    }

    // Search lexical class stack for the constant.
    fn get_env_const(&self, id: IdentId) -> Option<Value> {
        let class_defined = &self.get_method_iseq().class_defined;
        match class_defined.len() {
            0 => None,
            1 => class_defined[0].get_const(id),
            _ => class_defined[1..]
                .iter()
                .rev()
                .find_map(|c| c.get_const(id)),
        }
    }

    /// Search class inheritance chain for the constant.
    pub fn get_super_const(mut class: Module, id: IdentId) -> VMResult {
        let is_module = class.is_module();
        loop {
            match class.get_const(id) {
                Some(val) => return Ok(val),
                None => match class.upper() {
                    Some(upper) => class = upper,
                    None => {
                        if is_module {
                            if let Some(val) = BuiltinClass::object().get_const(id) {
                                return Ok(val);
                            }
                        }
                        return Err(RubyError::name(format!("Uninitialized constant {:?}.", id)));
                    }
                },
            }
        }
    }

    pub fn get_const(&self, parent: Module, id: IdentId) -> VMResult {
        match parent.get_const(id) {
            Some(val) => Ok(val),
            None => Err(RubyError::name(format!("Uninitialized constant {:?}.", id))),
        }
    }

    fn set_class_var(&self, id: IdentId, val: Value) -> Result<(), RubyError> {
        if self.exec_context.len() == 0 {
            return Err(RubyError::runtime("class varable access from toplevel."));
        }
        let self_val = self.current_context().self_value;
        let org_class = match self_val.if_mod_class() {
            Some(module) => module,
            None => self_val.get_class(),
        };
        let mut class = org_class;
        loop {
            if class.set_var_if_exists(id, val) {
                return Ok(());
            } else {
                match class.upper() {
                    Some(superclass) => class = superclass,
                    None => {
                        org_class.set_var(id, val);
                        return Ok(());
                    }
                }
            };
        }
    }

    fn get_class_var(&self, id: IdentId) -> VMResult {
        if self.exec_context.len() == 0 {
            return Err(RubyError::runtime("class varable access from toplevel."));
        }
        let self_val = self.current_context().self_value;
        let mut class = match self_val.if_mod_class() {
            Some(module) => module,
            None => self_val.get_class(),
        };
        loop {
            match class.get_var(id) {
                Some(val) => {
                    return Ok(val);
                }
                None => match class.upper() {
                    Some(superclass) => {
                        class = superclass;
                    }
                    None => {
                        return Err(RubyError::name(format!(
                            "Uninitialized class variable {:?}.",
                            id
                        )));
                    }
                },
            }
        }
    }
}

// Utilities for method call
impl VM {
    pub fn send(&mut self, method_id: IdentId, receiver: Value, args: &Args) -> VMResult {
        match self.globals.find_method_from_receiver(receiver, method_id) {
            Some(method) => return self.eval_send(method, receiver, args),
            None => {}
        };
        self.send_method_missing(method_id, receiver, args)
    }

    pub fn send0(&mut self, method_id: IdentId, receiver: Value) -> VMResult {
        let args = Args::new0();
        self.send(method_id, receiver, &args)
    }

    pub fn send1(&mut self, method_id: IdentId, receiver: Value, arg: Value) -> VMResult {
        let args = Args::new1(arg);
        self.send(method_id, receiver, &args)
    }

    fn send_icache(
        &mut self,
        cache: u32,
        method_id: IdentId,
        receiver: Value,
        args: &Args,
    ) -> VMResult {
        match self
            .globals
            .find_method_from_icache(cache, receiver, method_id)
        {
            Some(method) => return self.eval_send(method, receiver, args),
            None => {}
        }
        self.send_method_missing(method_id, receiver, args)
    }

    fn send_method_missing(
        &mut self,
        method_id: IdentId,
        receiver: Value,
        args: &Args,
    ) -> VMResult {
        match self
            .globals
            .find_method_from_receiver(receiver, IdentId::_METHOD_MISSING)
        {
            Some(method) => {
                let len = args.len();
                let mut new_args = Args::new(len + 1);
                new_args[0] = Value::symbol(method_id);
                new_args[1..len + 1].copy_from_slice(args);
                self.eval_send(method, receiver, &new_args)
            }
            None => Err(RubyError::undefined_method(method_id, receiver)),
        }
    }

    fn fallback_for_binop(&mut self, method: IdentId, lhs: Value, rhs: Value) -> VMResult {
        let class = lhs.get_class_for_method();
        match self.globals.find_method(class, method) {
            Some(mref) => {
                let arg = Args::new1(rhs);
                let val = self.eval_send(mref, lhs, &arg)?;
                Ok(val)
            }
            None => Err(RubyError::undefined_op(format!("{:?}", method), rhs, lhs)),
        }
    }
}

macro_rules! eval_op_i {
    ($vm:ident, $iseq:ident, $lhs:expr, $i:ident, $op:ident, $id:expr) => {
        if $lhs.is_packed_fixnum() {
            return Ok(Value::integer($lhs.as_packed_fixnum().$op($i as i64)));
        } else if $lhs.is_packed_num() {
            return Ok(Value::float($lhs.as_packed_flonum().$op($i as f64)));
        }
        return $vm.fallback_for_binop($id, $lhs, Value::integer($i as i64));
    };
}

macro_rules! eval_op {
    ($vm:ident, $rhs:expr, $lhs:expr, $op:ident, $id:expr) => {
        if $lhs.is_packed_fixnum() {
            let lhs = $lhs.as_packed_fixnum();
            if $rhs.is_packed_fixnum() {
                let rhs = $rhs.as_packed_fixnum();
                return Ok(Value::integer(lhs.$op(rhs)));
            } else if $rhs.is_packed_num() {
                let rhs = $rhs.as_packed_flonum();
                return Ok(Value::float((lhs as f64).$op(rhs)));
            }
        } else if $lhs.is_packed_num() {
            let lhs = $lhs.as_packed_flonum();
            if $rhs.is_packed_fixnum() {
                let rhs = $rhs.as_packed_fixnum();
                return Ok(Value::float(lhs.$op(rhs as f64)));
            } else if $rhs.is_packed_num() {
                let rhs = $rhs.as_packed_flonum();
                return Ok(Value::float(lhs.$op(rhs)));
            }
        }
        return $vm.fallback_for_binop($id, $lhs, $rhs);
    };
}

impl VM {
    fn eval_add(&mut self, rhs: Value, lhs: Value) -> VMResult {
        use std::ops::Add;
        eval_op!(self, rhs, lhs, add, IdentId::_ADD);
    }

    fn eval_sub(&mut self, rhs: Value, lhs: Value) -> VMResult {
        use std::ops::Sub;
        eval_op!(self, rhs, lhs, sub, IdentId::_SUB);
    }

    fn eval_mul(&mut self, rhs: Value, lhs: Value) -> VMResult {
        use std::ops::Mul;
        eval_op!(self, rhs, lhs, mul, IdentId::_MUL);
    }

    fn eval_addi(&mut self, lhs: Value, i: i32) -> VMResult {
        use std::ops::Add;
        eval_op_i!(self, iseq, lhs, i, add, IdentId::_ADD);
    }

    fn eval_subi(&mut self, lhs: Value, i: i32) -> VMResult {
        use std::ops::Sub;
        eval_op_i!(self, iseq, lhs, i, sub, IdentId::_SUB);
    }

    fn eval_div(&mut self, rhs: Value, lhs: Value) -> VMResult {
        use std::ops::Div;
        if rhs.is_zero() {
            return Err(RubyError::zero_div("Divided by zero."));
        }
        eval_op!(self, rhs, lhs, div, IdentId::_DIV);
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
            (RV::Integer(lhs), RV::Integer(rhs)) => Value::integer(lhs.rem_floor(rhs)),
            (RV::Integer(lhs), RV::Float(rhs)) => Value::float(rem_floorf64(lhs as f64, rhs)),
            (RV::Float(lhs), RV::Integer(rhs)) => Value::float(rem_floorf64(lhs, rhs as f64)),
            (RV::Float(lhs), RV::Float(rhs)) => Value::float(rem_floorf64(lhs, rhs)),
            (_, _) => return self.fallback_for_binop(IdentId::_REM, lhs, rhs),
        };
        Ok(val)
    }

    fn eval_exp(&mut self, rhs: Value, lhs: Value) -> VMResult {
        let val = match (lhs.unpack(), rhs.unpack()) {
            (RV::Integer(lhs), RV::Integer(rhs)) => {
                if 0 <= rhs && rhs <= std::u32::MAX as i64 {
                    Value::integer(lhs.pow(rhs as u32))
                } else {
                    Value::float((lhs as f64).powf(rhs as f64))
                }
            }
            (RV::Integer(lhs), RV::Float(rhs)) => Value::float((lhs as f64).powf(rhs)),
            (RV::Float(lhs), RV::Integer(rhs)) => Value::float(lhs.powf(rhs as f64)),
            (RV::Float(lhs), RV::Float(rhs)) => Value::float(lhs.powf(rhs)),
            _ => {
                return self.fallback_for_binop(IdentId::_POW, lhs, rhs);
            }
        };
        Ok(val)
    }

    fn eval_neg(&mut self, lhs: Value) -> VMResult {
        let val = match lhs.unpack() {
            RV::Integer(i) => Value::integer(-i),
            RV::Float(f) => Value::float(-f),
            _ => return self.send0(IdentId::get_id("-@"), lhs),
        };
        Ok(val)
    }

    fn eval_shl(&mut self, rhs: Value, lhs: Value) -> VMResult {
        if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            return Ok(Value::integer(
                lhs.as_packed_fixnum() << rhs.as_packed_fixnum(),
            ));
        }
        if let Some(mut ainfo) = lhs.as_array() {
            ainfo.push(rhs);
            return Ok(lhs);
        }
        let val = self.fallback_for_binop(IdentId::_SHL, lhs, rhs)?;
        Ok(val)
    }

    fn eval_shr(&mut self, rhs: Value, lhs: Value) -> VMResult {
        if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            return Ok(Value::integer(
                lhs.as_packed_fixnum() >> rhs.as_packed_fixnum(),
            ));
        }
        let val = self.fallback_for_binop(IdentId::_SHR, lhs, rhs)?;
        Ok(val)
    }

    fn eval_bitand(&mut self, rhs: Value, lhs: Value) -> VMResult {
        if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            return Ok(Value::integer(
                lhs.as_packed_fixnum() & rhs.as_packed_fixnum(),
            ));
        }
        match (lhs.unpack(), rhs.unpack()) {
            (RV::True, _) => Ok(Value::bool(rhs.to_bool())),
            (RV::False, _) => Ok(Value::false_val()),
            (RV::Integer(lhs), RV::Integer(rhs)) => Ok(Value::integer(lhs & rhs)),
            (RV::Nil, _) => Ok(Value::false_val()),
            (_, _) => self.fallback_for_binop(IdentId::get_id("&"), lhs, rhs),
        }
    }

    fn eval_bitandi(&mut self, lhs: Value, i: i32) -> VMResult {
        let i = i as i64;
        if lhs.is_packed_fixnum() {
            return Ok(Value::integer(lhs.as_packed_fixnum() & i));
        }
        match lhs.unpack() {
            RV::True => Ok(Value::true_val()),
            RV::False => Ok(Value::false_val()),
            RV::Integer(lhs) => Ok(Value::integer(lhs & i)),
            RV::Nil => Ok(Value::false_val()),
            _ => self.fallback_for_binop(IdentId::get_id("&"), lhs, Value::integer(i)),
        }
    }

    fn eval_bitor(&mut self, rhs: Value, lhs: Value) -> VMResult {
        if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            return Ok(Value::integer(
                lhs.as_packed_fixnum() | rhs.as_packed_fixnum(),
            ));
        }
        match (lhs.unpack(), rhs.unpack()) {
            (RV::True, _) => Ok(Value::true_val()),
            (RV::False, _) => Ok(Value::bool(rhs.to_bool())),
            (RV::Integer(lhs), RV::Integer(rhs)) => Ok(Value::integer(lhs | rhs)),
            (RV::Nil, _) => Ok(Value::bool(rhs.to_bool())),
            (_, _) => self.fallback_for_binop(IdentId::get_id("|"), lhs, rhs),
        }
    }

    fn eval_bitori(&mut self, lhs: Value, i: i32) -> VMResult {
        let i = i as i64;
        if lhs.is_packed_fixnum() {
            return Ok(Value::integer(lhs.as_packed_fixnum() | i));
        }
        match lhs.unpack() {
            RV::True => Ok(Value::true_val()),
            RV::False => Ok(Value::true_val()),
            RV::Integer(lhs) => Ok(Value::integer(lhs | i)),
            RV::Nil => Ok(Value::true_val()),
            _ => self.fallback_for_binop(IdentId::get_id("|"), lhs, Value::integer(i)),
        }
    }

    fn eval_bitxor(&mut self, rhs: Value, lhs: Value) -> VMResult {
        match (lhs.unpack(), rhs.unpack()) {
            (RV::True, _) => Ok(Value::bool(!rhs.to_bool())),
            (RV::False, _) => Ok(Value::bool(rhs.to_bool())),
            (RV::Integer(lhs), RV::Integer(rhs)) => Ok(Value::integer(lhs ^ rhs)),
            (RV::Nil, _) => Ok(Value::bool(rhs.to_bool())),
            (_, _) => return self.fallback_for_binop(IdentId::get_id("^"), lhs, rhs),
        }
    }

    fn eval_bitnot(&mut self, lhs: Value) -> VMResult {
        match lhs.unpack() {
            RV::Integer(lhs) => Ok(Value::integer(!lhs)),
            _ => Err(RubyError::undefined_method(IdentId::get_id("~"), lhs)),
        }
    }
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
                $vm.fallback_for_binop($id, $lhs, $rhs).map(|x| x.to_bool())
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
                $vm.fallback_for_binop($id, $lhs, $rhs).map(|x| x.to_bool())
            }
        } else {
            match ($lhs.unpack(), $rhs.unpack()) {
                (RV::Integer(lhs), RV::Integer(rhs)) => Ok(lhs.$op(&rhs)),
                (RV::Float(lhs), RV::Integer(rhs)) => Ok(lhs.$op(&(rhs as f64))),
                (RV::Integer(lhs), RV::Float(rhs)) => Ok((lhs as f64).$op(&rhs)),
                (RV::Float(lhs), RV::Float(rhs)) => Ok(lhs.$op(&rhs)),
                (_, _) => $vm.fallback_for_binop($id, $lhs, $rhs).map(|x| x.to_bool()),
            }
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
                    let res = $vm.fallback_for_binop($id, $lhs, Value::integer($i as i64));
                    res.map(|x| x.to_bool())
                }
            }
        }
    };
}

impl VM {
    pub fn eval_eq(&mut self, rhs: Value, lhs: Value) -> Result<bool, RubyError> {
        if lhs.id() == rhs.id() {
            return Ok(true);
        };
        if lhs.is_packed_value() || rhs.is_packed_value() {
            if lhs.is_packed_num() && rhs.is_packed_num() {
                match (lhs.is_packed_fixnum(), rhs.is_packed_fixnum()) {
                    (true, false) => {
                        return Ok(lhs.as_packed_fixnum() as f64 == rhs.as_packed_flonum())
                    }
                    (false, true) => {
                        return Ok(lhs.as_packed_flonum() == rhs.as_packed_fixnum() as f64)
                    }
                    _ => return Ok(false),
                }
            }
            return Ok(false);
        };
        match (&lhs.rvalue().kind, &rhs.rvalue().kind) {
            (ObjKind::Integer(lhs), ObjKind::Integer(rhs)) => Ok(*lhs == *rhs),
            (ObjKind::Float(lhs), ObjKind::Float(rhs)) => Ok(*lhs == *rhs),
            (ObjKind::Integer(lhs), ObjKind::Float(rhs)) => Ok(*lhs as f64 == *rhs),
            (ObjKind::Float(lhs), ObjKind::Integer(rhs)) => Ok(*lhs == *rhs as f64),
            (ObjKind::Complex { r: r1, i: i1 }, ObjKind::Complex { r: r2, i: i2 }) => {
                Ok(*r1 == *r2 && *i1 == *i2)
            }
            (ObjKind::String(lhs), ObjKind::String(rhs)) => Ok(lhs.as_bytes() == rhs.as_bytes()),
            (ObjKind::Array(lhs), ObjKind::Array(rhs)) => Ok(lhs.elements == rhs.elements),
            (ObjKind::Range(lhs), ObjKind::Range(rhs)) => Ok(lhs == rhs),
            (ObjKind::Hash(lhs), ObjKind::Hash(rhs)) => Ok(**lhs == **rhs),
            (ObjKind::Regexp(lhs), ObjKind::Regexp(rhs)) => Ok(*lhs == *rhs),
            (ObjKind::Time(lhs), ObjKind::Time(rhs)) => Ok(*lhs == *rhs),
            (ObjKind::Invalid, _) => {
                panic!("Invalid rvalue. (maybe GC problem) {:?}", lhs.rvalue())
            }
            (_, ObjKind::Invalid) => {
                panic!("Invalid rvalue. (maybe GC problem) {:?}", rhs.rvalue())
            }
            (_, _) => {
                let val = match self.fallback_for_binop(IdentId::_EQ, lhs, rhs) {
                    Ok(val) => val,
                    _ => return Ok(false),
                };
                Ok(val.to_bool())
            }
        }
    }

    pub fn eval_eqi(&self, lhs: Value, i: i32) -> bool {
        lhs.equal_i(i)
    }

    pub fn eval_teq(&mut self, rhs: Value, lhs: Value) -> Result<bool, RubyError> {
        match lhs.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Module(_) => {
                    Ok(self.fallback_for_binop(IdentId::_TEQ, lhs, rhs)?.to_bool())
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
                _ => Ok(self.eval_eq(lhs, rhs)?),
            },
            None => Ok(self.eval_eq(lhs, rhs)?),
        }
    }

    fn eval_rescue(&self, val: Value, exceptions: &[Value]) -> Result<bool, RubyError> {
        let mut module = if val.is_class() {
            Module::new(val)
        } else {
            val.get_class()
        };
        loop {
            if !module.is_module() {
                if exceptions.iter().any(|x| {
                    if let Some(ary) = x.as_splat() {
                        ary.as_array()
                            .unwrap()
                            .elements
                            .iter()
                            .any(|elem| elem.id() == module.id())
                    } else {
                        x.id() == module.id()
                    }
                }) {
                    return Ok(true);
                }
            };

            match module.upper() {
                Some(upper) => module = upper,
                None => break,
            }
        }
        Ok(false)
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

    pub fn eval_compare(&mut self, rhs: Value, lhs: Value) -> VMResult {
        if rhs.id() == lhs.id() {
            return Ok(Value::integer(0));
        };
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
                return self.fallback_for_binop(IdentId::_CMP, lhs, rhs);
            }
        };
        match res {
            Some(ord) => Ok(Value::integer(ord as i64)),
            None => Ok(Value::nil()),
        }
    }

    fn set_index(&mut self) -> Result<(), RubyError> {
        let val = self.stack_pop()?;
        let idx = self.stack_pop()?;
        let mut receiver = self.stack_pop()?;

        match receiver.as_mut_rvalue() {
            Some(oref) => {
                match oref.kind {
                    ObjKind::Array(ref mut aref) => {
                        aref.set_elem1(idx, val)?;
                    }
                    ObjKind::Hash(ref mut href) => href.insert(idx, val),
                    _ => {
                        self.send(IdentId::_INDEX_ASSIGN, receiver, &Args::new2(idx, val))?;
                    }
                };
            }
            None => {
                self.send(IdentId::_INDEX_ASSIGN, receiver, &Args::new2(idx, val))?;
            }
        }
        Ok(())
    }

    fn set_index_imm(&mut self, idx: u32) -> Result<(), RubyError> {
        let mut receiver = self.stack_pop()?;
        let val = self.stack_pop()?;
        match receiver.as_mut_rvalue() {
            Some(oref) => {
                match oref.kind {
                    ObjKind::Array(ref mut aref) => {
                        aref.set_elem_imm(idx, val);
                    }
                    ObjKind::Hash(ref mut href) => href.insert(Value::integer(idx as i64), val),
                    _ => {
                        self.send(
                            IdentId::_INDEX_ASSIGN,
                            receiver,
                            &Args::new2(Value::integer(idx as i64), val),
                        )?;
                    }
                };
            }
            None => {
                self.send(
                    IdentId::_INDEX_ASSIGN,
                    receiver,
                    &Args::new2(Value::integer(idx as i64), val),
                )?;
            }
        }
        Ok(())
    }

    fn get_index(&mut self) -> VMResult {
        let idx = self.stack_pop()?;
        let receiver = self.stack_top();
        let val = match receiver.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Array(aref) => aref.get_elem1(idx)?,
                ObjKind::Hash(href) => match href.get(&idx) {
                    Some(val) => *val,
                    None => Value::nil(),
                },
                _ => self.send(IdentId::_INDEX, receiver, &Args::new1(idx))?,
            },
            _ => self.fallback_for_binop(IdentId::_INDEX, receiver, idx)?,
        };
        self.stack_pop()?;
        Ok(val)
    }

    fn get_index_imm(&mut self, idx: u32) -> VMResult {
        let receiver = self.stack_top();
        let val = match receiver.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Array(aref) => aref.get_elem_imm(idx),
                ObjKind::Hash(href) => match href.get(&Value::integer(idx as i64)) {
                    Some(val) => *val,
                    None => Value::nil(),
                },
                ObjKind::Method(mref) => {
                    let args = Args::new1(Value::integer(idx as i64));
                    self.eval_send(mref.method, mref.receiver, &args)?
                }
                _ => {
                    let args = Args::new1(Value::integer(idx as i64));
                    self.send(IdentId::_INDEX, receiver, &args)?
                }
            },
            None if receiver.is_packed_fixnum() => {
                let i = receiver.as_packed_fixnum();
                let val = if 63 < idx { 0 } else { (i >> idx) & 1 };
                Value::integer(val)
            }
            _ => self.fallback_for_binop(IdentId::_INDEX, receiver, Value::integer(idx as i64))?,
        };
        self.stack_pop()?;
        Ok(val)
    }

    /// Generate new class object with `super_val` as a superclass.
    fn define_class(
        &mut self,
        base: Value,
        id: IdentId,
        is_module: bool,
        super_val: Value,
    ) -> Result<Module, RubyError> {
        let current_class = if base.is_nil() {
            self.class()
        } else {
            Module::new(base)
        };
        match current_class.get_const(id) {
            Some(val) => {
                let val = Module::new(val);
                if val.is_module() != is_module {
                    return Err(RubyError::typeerr(format!(
                        "{:?} is not {}.",
                        id,
                        if is_module { "module" } else { "class" },
                    )));
                };
                let val_super = match val.superclass() {
                    Some(v) => v.into(),
                    None => Value::nil(),
                };
                if !super_val.is_nil() && val_super.id() != super_val.id() {
                    return Err(RubyError::typeerr(format!(
                        "superclass mismatch for class {:?}.",
                        id,
                    )));
                };
                Ok(val)
            }
            None => {
                let val = if is_module {
                    if !super_val.is_nil() {
                        panic!("Module can not have superclass.");
                    };
                    Module::module()
                } else {
                    let super_val = if super_val.is_nil() {
                        self.globals.builtins.object
                    } else {
                        super_val.expect_class("Superclass")?
                    };
                    Module::class_under(super_val)
                };
                self.globals.set_const(current_class, id, val);
                Ok(val)
            }
        }
    }

    pub fn sort_array(&mut self, vec: &mut Vec<Value>) -> Result<(), RubyError> {
        if vec.len() > 0 {
            let val = vec[0];
            for i in 1..vec.len() {
                match self.eval_compare(vec[i], val)? {
                    v if v.is_nil() => {
                        let lhs = val.get_class_name();
                        let rhs = vec[i].get_class_name();
                        return Err(RubyError::argument(format!(
                            "Comparison of {} with {} failed.",
                            lhs, rhs
                        )));
                    }
                    _ => {}
                }
            }
            vec.sort_by(|a, b| self.eval_compare(*b, *a).unwrap().to_ordering());
        }
        Ok(())
    }
}

impl VM {
    fn create_regexp(&mut self, arg: Value) -> VMResult {
        let mut arg = match arg.as_string() {
            Some(arg) => arg.to_string(),
            None => return Err(RubyError::argument("Illegal argument for CREATE_REGEXP")),
        };
        match arg.pop().unwrap() {
            'i' => arg.insert_str(0, "(?mi)"),
            'm' => arg.insert_str(0, "(?ms)"),
            'x' => arg.insert_str(0, "(?mx)"),
            'o' => arg.insert_str(0, "(?mo)"),
            '-' => arg.insert_str(0, "(?m)"),
            _ => return Err(RubyError::internal("Illegal internal regexp expression.")),
        };
        Ok(Value::regexp_from(self, &arg)?)
    }
}

// API's for handling values.

impl VM {
    pub fn val_inspect(&mut self, val: Value) -> Result<String, RubyError> {
        let s = match val.unpack() {
            RV::Uninitialized => "[Uninitialized]".to_string(),
            RV::Nil => "nil".to_string(),
            RV::True => "true".to_string(),
            RV::False => "false".to_string(),
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
                ObjKind::Range(rinfo) => rinfo.inspect(self)?,
                ObjKind::Module(cref) => cref.inspect(),
                ObjKind::Array(aref) => aref.to_s(self)?,
                ObjKind::Regexp(rref) => format!("/{}/", rref.as_str().to_string()),
                ObjKind::Ordinary => oref.inspect()?,
                ObjKind::Hash(href) => href.to_s(self)?,
                ObjKind::Complex { .. } => format!("{:?}", oref.kind),
                _ => {
                    let id = IdentId::get_id("inspect");
                    self.send0(id, val)?.as_string().unwrap().to_string()
                }
            },
        };
        Ok(s)
    }
}

impl VM {
    /// Evaluate method with given `self_val`, `args` and no outer context.
    #[inline]
    pub fn eval_send(
        &mut self,
        methodref: MethodRef,
        self_val: impl Into<Value>,
        args: &Args,
    ) -> VMResult {
        let self_val = self_val.into();
        self.invoke_method(methodref, self_val, None, args)
    }

    /// Evaluate method with self_val of current context, current context as outer context, and given `args`.
    pub fn eval_block(&mut self, block: &Block, args: &Args) -> VMResult {
        match block {
            Block::Block(method, outer) => {
                self.invoke_method(*method, outer.self_value, Some(*outer), args)
            }
            Block::Proc(proc) => self.invoke_proc(*proc, args),
            _ => unreachable!(),
        }
    }

    /// Evaluate method with self_val of current context, current context as outer context, and given `args`.
    pub fn eval_block_self(
        &mut self,
        block: &Block,
        self_val: impl Into<Value>,
        args: &Args,
    ) -> VMResult {
        let self_val = self_val.into();
        match block {
            Block::Block(method, outer) => {
                self.invoke_method(*method, self_val, Some(*outer), args)
            }
            Block::Proc(proc) => {
                let pref = match proc.as_proc() {
                    Some(proc) => proc,
                    None => return Err(RubyError::internal("Illegal proc.")),
                };
                let context = Context::from_args(
                    self,
                    self_val,
                    pref.context.iseq_ref.unwrap(),
                    args,
                    pref.context.outer,
                )?;
                self.run_context(&context)
            }
            _ => unreachable!(),
        }
    }

    /// Evaluate given block with given `args`.
    pub fn eval_yield(&mut self, args: &Args) -> VMResult {
        let context = self.get_method_context();
        match &context.block {
            Block::Block(method, ctx) => {
                self.invoke_method(*method, ctx.self_value, Some(*ctx), args)
            }
            Block::Proc(proc) => self.invoke_proc(*proc, args),
            Block::None => return Err(RubyError::local_jump("No block given.")),
        }
    }

    /// Evaluate Proc object.
    pub fn invoke_proc(&mut self, proc: Value, args: &Args) -> VMResult {
        let pref = proc.as_proc().unwrap();
        let context = Context::from_args(
            self,
            pref.context.self_value,
            pref.context.iseq_ref.unwrap(),
            args,
            pref.context.outer,
        )?;
        self.run_context(&context)
    }

    /// Evaluate method with given `self_val`, `outer` context, and `args`.
    pub fn invoke_method(
        &mut self,
        methodref: MethodRef,
        self_val: impl Into<Value>,
        outer: Option<ContextRef>,
        args: &Args,
    ) -> VMResult {
        let self_val = self_val.into();
        use MethodInfo::*;
        let outer = outer.map(|ctx| ctx.get_current());
        match &*methodref {
            BuiltinFunc { func, name } => self.invoke_native(func, *name, self_val, args),
            AttrReader { id } => {
                args.check_args_num(0)?;
                Self::invoke_getter(*id, self_val)
            }
            AttrWriter { id } => {
                args.check_args_num(1)?;
                Self::invoke_setter(*id, self_val, args[0])
            }
            RubyFunc { iseq } => {
                let context = Context::from_args(self, self_val, *iseq, args, outer)?;
                self.run_context(&context)
            }
            _ => unreachable!(),
        }
    }

    // helper methods
    fn invoke_native(
        &mut self,
        func: &BuiltinFunc,
        _name: IdentId,
        self_val: Value,
        args: &Args,
    ) -> VMResult {
        #[cfg(feature = "perf")]
        self.perf.get_perf(Perf::EXTERN);

        #[cfg(any(feature = "trace", feature = "trace-func"))]
        println!("---> BuiltinFunc {:?}", _name);

        let len = self.temp_stack.len();
        self.temp_push(self_val);
        self.temp_push_args(args);
        let res = func(self, self_val, args);
        self.temp_stack.truncate(len);

        #[cfg(any(feature = "trace", feature = "trace-func"))]
        println!("<--- {:?}", res);

        res
    }

    fn invoke_getter(id: IdentId, self_val: Value) -> VMResult {
        match self_val.as_rvalue() {
            Some(oref) => match oref.get_var(id) {
                Some(v) => Ok(v),
                None => Ok(Value::nil()),
            },
            None => unreachable!("AttrReader must be used only for class instance."),
        }
    }

    fn invoke_setter(id: IdentId, mut self_val: Value, val: Value) -> VMResult {
        match self_val.as_mut_rvalue() {
            Some(oref) => {
                oref.set_var(id, val);
                Ok(val)
            }
            None => unreachable!("AttrReader must be used only for class instance."),
        }
    }
}

// API's for handling instance/singleton methods.

impl VM {
    /// Define a method on `target_obj`.
    /// If `target_obj` is not Class, use Class of it.
    pub fn define_method(&mut self, target_obj: Value, id: IdentId, method: MethodRef) {
        match target_obj.if_mod_class() {
            Some(mut module) => module.add_method(&mut self.globals, id, method),
            None => target_obj
                .get_class()
                .add_method(&mut self.globals, id, method),
        };
    }

    /// Define a method on a singleton class of `target_obj`.
    pub fn define_singleton_method(
        &mut self,
        target_obj: Value,
        id: IdentId,
        method: MethodRef,
    ) -> Result<(), RubyError> {
        target_obj
            .get_singleton_class()?
            .add_method(&mut self.globals, id, method);
        Ok(())
    }

    /// Get method(MethodRef) for class.
    ///
    /// If the method was not found, return NoMethodError.
    pub fn get_method(
        &mut self,
        rec_class: Module,
        method_id: IdentId,
    ) -> Result<MethodRef, RubyError> {
        match self.globals.find_method(rec_class, method_id) {
            Some(m) => Ok(m),
            None => Err(RubyError::undefined_method_for_class(method_id, rec_class)),
        }
    }

    /// Get method(MethodRef) for receiver.
    pub fn get_method_from_receiver(
        &mut self,
        receiver: Value,
        method_id: IdentId,
    ) -> Result<MethodRef, RubyError> {
        let rec_class = receiver.get_class_for_method();
        self.get_method(rec_class, method_id)
    }
}

impl VM {
    /// Yield args to parent fiber. (execute Fiber.yield)
    pub fn fiber_yield(&mut self, args: &Args) -> VMResult {
        let val = match args.len() {
            0 => Value::nil(),
            1 => args[0],
            _ => Value::array_from(args.to_vec()),
        };
        match &self.parent_fiber {
            None => return Err(RubyError::fiber("Can not yield from main fiber.")),
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
                #[cfg(feature = "trace-func")]
                println!("<=== yield Ok({:?})", val);

                tx.send(Ok(val)).unwrap();
                // Wait for fiber's response
                match rx.recv() {
                    Ok(FiberMsg::Resume) => {}
                    _ => return Err(RubyError::fiber("terminated")),
                }
                #[cfg(feature = "perf")]
                self.perf.get_perf_no_count(_inst);
                // TODO: this return value is not correct. The arg of Fiber#resume should be returned.
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

    fn pop_key_value_pair(&mut self, arg_num: usize) -> FxHashMap<HashKey, Value> {
        let mut hash = FxHashMap::default();
        for _ in 0..arg_num {
            let value = self.stack_pop().unwrap();
            let key = self.stack_pop().unwrap();
            hash.insert(HashKey(key), value);
        }
        hash
    }

    /// Pop values and store them in new `Args`. `args_num` specifies the number of values to be popped.
    /// If there is some Array or Range with splat operator, break up the value and store each of them.
    fn pop_args_to_args(&mut self, arg_num: usize) -> Args {
        let mut args = Args::new(0);
        let len = self.stack_len();

        for val in self.exec_stack[len - arg_num..].iter() {
            match val.as_splat() {
                Some(inner) => match inner.as_rvalue() {
                    None => args.push(inner),
                    Some(obj) => match &obj.kind {
                        ObjKind::Array(a) => args.append(&a.elements),
                        ObjKind::Range(r) => {
                            let start = r.start.expect_integer("Expect Integer.").unwrap();
                            let end = r.end.expect_integer("Expect Integer.").unwrap()
                                + if r.exclude { 0 } else { 1 };
                            (start..end).for_each(|i| args.push(Value::integer(i)));
                        }
                        _ => args.push(inner),
                    },
                },
                None => args.push(*val),
            };
        }
        self.set_stack_len(len - arg_num);
        args
    }

    pub fn create_range(&mut self, start: Value, end: Value, exclude_end: bool) -> VMResult {
        if self.eval_compare(start, end)?.is_nil() {
            return Err(RubyError::argument("Bad value for range."));
        }
        Ok(Value::range(start, end, exclude_end))
    }

    /// Create new Proc object from `method`,
    /// moving outer `Context`s on stack to heap.
    pub fn create_proc(&mut self, block: &Block) -> VMResult {
        match block {
            Block::Block(method, outer) => {
                let context = self.create_block_context(*method, *outer)?;
                Ok(Value::procobj(context))
            }
            Block::Proc(proc) => Ok(proc.dup()),
            _ => unreachable!(),
        }
    }

    /// Create new Lambda object from `method`,
    /// moving outer `Context`s on stack to heap.
    pub fn create_lambda(&mut self, block: &Block) -> VMResult {
        match block {
            Block::Block(method, outer) => {
                let mut context = self.create_block_context(*method, *outer)?;
                context.kind = ISeqKind::Method(IdentId::get_id(""));
                Ok(Value::procobj(context))
            }
            Block::Proc(proc) => Ok(proc.dup()),
            _ => unreachable!(),
        }
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
        mut args: Args,
    ) -> VMResult {
        args.block = Block::Block(*METHODREF_ENUM, self.current_context());
        let fiber = self.create_enum_info(method_id, receiver, args);
        Ok(Value::enumerator(fiber))
    }

    /// Move outer execution contexts on the stack to the heap.
    fn move_outer_to_heap(&mut self, outer: ContextRef) -> ContextRef {
        let mut stack_context = outer;
        let mut prev_ctx: Option<ContextRef> = None;
        let mut iter = self.exec_context.iter_mut().rev();
        loop {
            if !stack_context.on_stack {
                break;
            };
            let heap_context = stack_context.move_to_heap();
            loop {
                match iter.next() {
                    None => unreachable!("not found."),
                    Some(ctx) if *ctx == stack_context => {
                        assert!(ctx.on_stack);
                        *ctx = heap_context;
                        break;
                    }
                    _ => {}
                }
            }
            if let Some(mut ctx) = prev_ctx {
                (*ctx).outer = Some(heap_context)
            };
            prev_ctx = Some(heap_context);

            stack_context = match heap_context.outer {
                Some(context) => context,
                None => break,
            };
        }
        //eprintln!("****moved.");

        outer.moved_to_heap.unwrap()
    }

    /// Create a new execution context for a block.
    pub fn create_block_context(
        &mut self,
        method: MethodRef,
        outer: ContextRef,
    ) -> Result<ContextRef, RubyError> {
        let outer = self.move_outer_to_heap(outer);
        let iseq = method.as_iseq();
        Ok(ContextRef::new_heap(
            outer.self_value,
            Block::None,
            iseq,
            Some(outer),
        ))
    }

    /// Create fancy_regex::Regex from `string`.
    /// Escapes all regular expression meta characters in `string`.
    /// Returns RubyError if `string` was invalid regular expression.
    pub fn regexp_from_escaped_string(&mut self, string: &str) -> Result<RegexpInfo, RubyError> {
        RegexpInfo::from_escaped(&mut self.globals, string).map_err(|err| RubyError::regexp(err))
    }

    /// Create fancy_regex::Regex from `string` without escaping meta characters.
    /// Returns RubyError if `string` was invalid regular expression.
    pub fn regexp_from_string(&mut self, string: &str) -> Result<RegexpInfo, RubyError> {
        RegexpInfo::from_string(&mut self.globals, string).map_err(|err| RubyError::regexp(err))
    }
}

impl VM {
    pub fn canonicalize_path(&mut self, path: &PathBuf) -> Result<PathBuf, RubyError> {
        match path.canonicalize() {
            Ok(path) => Ok(path),
            Err(ioerr) => {
                let msg = format!("File not found. {:?}\n{}", path, ioerr);
                Err(RubyError::runtime(msg))
            }
        }
    }

    pub fn load_file(&mut self, path: &PathBuf) -> Result<String, RubyError> {
        use crate::loader::*;
        match loader::load_file(path) {
            Ok(program) => {
                self.globals.add_source_file(path);
                Ok(program)
            }
            Err(err) => {
                let err_str = match err {
                    LoadError::NotFound(msg) => {
                        format!("No such file or directory -- {:?}\n{}", path, msg)
                    }
                    LoadError::CouldntOpen(msg) => {
                        format!("Cannot open file. '{:?}'\n{}", path, msg)
                    }
                };
                Err(RubyError::load(err_str))
            }
        }
    }

    pub fn exec_file(&mut self, file_name: &str) {
        use crate::loader::*;
        let path = match Path::new(file_name).canonicalize() {
            Ok(path) => path,
            Err(ioerr) => {
                eprintln!("LoadError: {}\n{}", file_name, ioerr);
                return;
            }
        };
        let (absolute_path, program) = match loader::load_file(&path) {
            Ok(program) => (path, program),
            Err(err) => {
                match err {
                    LoadError::NotFound(msg) => eprintln!("LoadError: {}\n{}", file_name, msg),
                    LoadError::CouldntOpen(msg) => eprintln!("LoadError: {}\n{}", file_name, msg),
                };
                return;
            }
        };
        self.globals.add_source_file(&absolute_path);
        let file = absolute_path
            .file_name()
            .map(|x| x.to_string_lossy())
            .unwrap_or(std::borrow::Cow::Borrowed(""));
        self.set_global_var(IdentId::get_id("$0"), Value::string(file));
        #[cfg(feature = "verbose")]
        eprintln!("load file: {:?}", &absolute_path);
        self.exec_program(absolute_path, &program);
        #[cfg(feature = "emit-iseq")]
        self.globals.const_values.dump();
    }

    pub fn exec_program(&mut self, absolute_path: PathBuf, program: &str) {
        match self.run(absolute_path, program) {
            Ok(_) => {
                #[cfg(feature = "perf")]
                {
                    self.perf.print_perf();
                    self.globals.print_method_cache_stats();
                    self.globals.print_constant_cache_stats();
                }
                #[cfg(feature = "gc-debug")]
                self.globals.print_mark();
            }
            Err(err) => {
                err.show_err();
                err.show_all_loc();
            }
        };
    }
}
