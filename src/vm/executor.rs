use crate::coroutine::FiberHandle;
use crate::parse::codegen::{ContextKind, ExceptionType};
use crate::*;
use fancy_regex::Captures;
use std::ops::{Index, IndexMut};

#[cfg(feature = "perf")]
use super::perf::*;
use std::path::PathBuf;
use vm_inst::*;
mod constants;
mod fiber;
mod loader;
mod method;
mod ops;
mod opt_core;

pub type ValueTable = FxHashMap<IdentId, Value>;
pub type VMResult = Result<Value, RubyError>;

const VM_STACK_INITIAL_SIZE: usize = 4096;

//
//  Stack handling
//
//  before frame preparation
//
//   lfp                     cfp                                                   sp
//    v                       v                           <--- new local frame -->  v
// +------+------+--+------+------+------+------+--------+------+------+--+------+------------------------
// |  a0  |  a1  |..|  an  | lfp2 | cfp2 |  pc2 |  ....  |  b0  |  b1  |..|  bn  |
// +------+------+--+------+------+------+------+--------+------+------+--+------+------------------------
//  <---- local frame ----> <-- control frame ->
//
//
//  after frame preparation
//
//   lfp1                    cfp1                          lfp                     cfp                 sp
//    v                       v                             v                       v                   v
// +------+------+--+------+------+------+------+--------+------+------+--+------+------+------+------+---
// |  a0  |  a1  |..|  an  | lfp2 | cfp2 |  pc2 |  ....  |  b0  |  b1  |..|  bn  | lfp1 | cfp1 | pc1  |
// +------+------+--+------+------+------+------+--------+------+------+--+------+------+------+------+---
//                                                        <---- local frame ----> <-- control frame ->
//
//  after execution
//
//   lfp                     cfp                           sp
//    v                       v                             v
// +------+------+--+------+------+------+------+--------+------------------------------------------------
// |  a0  |  a1  |..|  an  | lfp2 | cfp2 |  pc2 |  ....  |
// +------+------+--+------+------+------+------+--------+------------------------------------------------
//

#[derive(Debug)]
pub struct VM {
    // Global info
    pub globals: GlobalsRef,
    // VM state
    cur_context: Option<ContextRef>,
    ctx_stack: ContextStore,
    exec_stack: Vec<Value>,
    temp_stack: Vec<Value>,
    /// program counter
    pc: ISeqPos,
    /// local frame pointer
    lfp: usize,
    /// control frame pointer
    cfp: usize,
    pub handle: Option<FiberHandle>,
    sp_last_match: Option<String>,   // $&        : Regexp.last_match(0)
    sp_post_match: Option<String>,   // $'        : Regexp.post_match
    sp_matches: Vec<Option<String>>, // $1 ... $n : Regexp.last_match(n)
}

pub type VMRef = Ref<VM>;

pub enum VMResKind {
    Return,
    Invoke,
}

impl VMResKind {
    fn handle(self, vm: &mut VM) -> Result<(), RubyError> {
        match self {
            VMResKind::Return => Ok(()),
            VMResKind::Invoke => {
                vm.context().called = true;
                vm.run_loop()
            }
        }
    }
}

impl Index<usize> for VM {
    type Output = Value;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.cfp - self.lfp);
        &self.exec_stack[self.lfp + index]
    }
}

impl IndexMut<usize> for VM {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.cfp - self.lfp);
        &mut self.exec_stack[self.lfp + index]
    }
}

impl VM {
    pub fn args(&self) -> &[Value] {
        &self.exec_stack[self.lfp..self.cfp]
    }
}

// API's
impl GC for VM {
    fn mark(&self, alloc: &mut Allocator) {
        let mut ctx = self.cur_context;
        while let Some(c) = ctx {
            c.mark(alloc);
            ctx = c.caller;
        }
        self.exec_stack.iter().for_each(|v| v.mark(alloc));
        self.temp_stack.iter().for_each(|v| v.mark(alloc));
    }
}

// handling cxt_stack
impl VM {
    /*pub fn new_stack_context(&mut self, context: Context) -> ContextRef {
        self.ctx_stack.push(context)
    }*/

    pub fn new_stack_context_with(
        &mut self,
        self_value: Value,
        block: Option<Block>,
        iseq: ISeqRef,
        outer: Option<ContextRef>,
        args_len: usize,
    ) -> ContextRef {
        let ctx = self.ctx_stack.push_with(self_value, block, iseq, outer);
        self.prepare_stack(args_len);
        ctx
    }
}

impl VM {
    pub fn new(mut globals: GlobalsRef) -> Self {
        let mut vm = VM {
            globals,
            cur_context: None,
            ctx_stack: ContextStore::new(),
            exec_stack: Vec::with_capacity(VM_STACK_INITIAL_SIZE),
            temp_stack: vec![],
            pc: ISeqPos::from(0),
            lfp: 0,
            cfp: 0,
            handle: None,
            sp_last_match: None,
            sp_post_match: None,
            sp_matches: vec![],
        };

        let method = vm.parse_program("", "".to_string()).unwrap();
        let dummy_info = MethodRepo::get(method);
        MethodRepo::update(MethodId::default(), dummy_info);

        let load_path = include_str!(concat!(env!("OUT_DIR"), "/libpath.rb"));
        match vm.run("(startup)", load_path.to_string()) {
            Ok(val) => globals.set_global_var_by_str("$:", val),
            Err(_) => {}
        };

        match vm.run(
            "ruruby/startup/startup.rb",
            include_str!("../startup/startup.rb").to_string(),
        ) {
            Ok(_) => {}
            Err(err) => {
                vm.show_err(&err);
                err.show_loc(0);
                panic!("Error occured in executing startup.rb.");
            }
        };

        vm.globals.startup_flag = true;

        #[cfg(feature = "perf")]
        {
            vm.globals.perf = Perf::new();
        }

        #[cfg(feature = "perf-method")]
        {
            MethodRepo::clear_stats();
            vm.globals.clear_const_cache();
        }

        vm
    }

    pub fn create_fiber(&mut self) -> Self {
        VM {
            globals: self.globals,
            cur_context: None,
            ctx_stack: ContextStore::new(),
            temp_stack: vec![],
            exec_stack: Vec::with_capacity(VM_STACK_INITIAL_SIZE),
            pc: ISeqPos::from(0),
            lfp: 0,
            cfp: 0,
            handle: None,
            sp_last_match: None,
            sp_post_match: None,
            sp_matches: vec![],
        }
    }

    pub fn context(&self) -> ContextRef {
        self.cur_context.unwrap()
    }

    fn get_method_context(&self) -> ContextRef {
        let mut context = self.context();
        while let Some(c) = context.outer {
            context = c;
        }
        context
    }

    pub fn get_method_iseq(&self) -> ISeqRef {
        self.get_method_context().iseq_ref
    }

    pub fn source_info(&self) -> SourceInfoRef {
        self.context().iseq_ref.source_info
    }

    pub fn get_source_path(&self) -> PathBuf {
        self.context().iseq_ref.source_info.path.clone()
    }

    fn is_method(&self) -> bool {
        self.context().is_method()
    }

    fn called(&self) -> bool {
        self.context().called
    }

    #[cfg(debug_assertions)]
    fn kind(&self) -> ISeqKind {
        self.context().iseq_ref.kind
    }

    pub fn stack_push(&mut self, val: Value) {
        self.exec_stack.push(val)
    }

    pub fn stack_push_reg(&mut self, lfp: usize, cfp: usize, pc: ISeqPos) {
        self.stack_push(Value::integer(lfp as i64));
        self.stack_push(Value::integer(cfp as i64));
        self.stack_push(Value::integer(pc.into_usize() as i64));
    }

    pub fn stack_fetch_reg(&mut self) -> (usize, usize, ISeqPos) {
        let cfp = self.cfp;
        (
            self.exec_stack[cfp].as_fixnum().unwrap() as usize,
            self.exec_stack[cfp + 1].as_fixnum().unwrap() as usize,
            ISeqPos::from(self.exec_stack[cfp + 2].as_fixnum().unwrap() as usize),
        )
    }

    pub fn stack_pop(&mut self) -> Value {
        self.exec_stack
            .pop()
            .unwrap_or_else(|| panic!("exec stack is empty."))
    }

    pub fn stack_pop2(&mut self) -> (Value, Value) {
        let len = self.stack_len();
        let lhs = self.exec_stack[len - 2];
        let rhs = self.exec_stack[len - 1];
        self.set_stack_len(len - 2);
        (lhs, rhs)
    }

    pub fn stack_top(&self) -> Value {
        *self
            .exec_stack
            .last()
            .unwrap_or_else(|| panic!("exec stack is empty."))
    }

    pub fn stack_len(&self) -> usize {
        self.exec_stack.len()
    }

    fn set_stack_len(&mut self, len: usize) {
        self.exec_stack.truncate(len);
    }

    pub fn stack_append(&mut self, slice: &[Value]) {
        self.exec_stack.extend_from_slice(slice)
    }

    pub fn stack_push_args(&mut self, args: &Args) {
        self.exec_stack.extend_from_slice(args)
    }

    pub fn get_args(&self) -> &[Value] {
        &self.exec_stack[self.lfp..self.cfp]
    }

    pub fn args_len(&self) -> usize {
        self.cfp - self.lfp
    }

    pub fn check_args_num(&self, num: usize) -> Result<(), RubyError> {
        let len = self.args_len();
        if len == num {
            Ok(())
        } else {
            Err(RubyError::argument_wrong(len, num))
        }
    }

    pub fn check_args_range(&self, min: usize, max: usize) -> Result<(), RubyError> {
        let len = self.args_len();
        if min <= len && len <= max {
            Ok(())
        } else {
            Err(RubyError::argument_wrong_range(len, min, max))
        }
    }

    pub fn check_args_min(&self, min: usize) -> Result<(), RubyError> {
        let len = self.args_len();
        if min <= len {
            Ok(())
        } else {
            Err(RubyError::argument(format!(
                "Wrong number of arguments. (given {}, expected {}+)",
                len, min
            )))
        }
    }

    /// Push an object to the temporary area.
    pub fn temp_push(&mut self, v: Value) {
        self.temp_stack.push(v);
    }

    pub fn temp_push_args(&mut self, args: &Args) {
        self.temp_stack.extend_from_slice(args);
        self.temp_stack.push(args.kw_arg);
        if let Some(Block::Proc(val)) = args.block {
            self.temp_stack.push(val)
        }
    }

    pub fn temp_pop_vec(&mut self, len: usize) -> Vec<Value> {
        self.temp_stack.split_off(len)
    }

    pub fn temp_len(&self) -> usize {
        self.temp_stack.len()
    }

    /// Push objects to the temporary area.
    pub fn temp_push_vec(&mut self, slice: &[Value]) {
        self.temp_stack.extend_from_slice(slice);
    }

    pub fn context_push(&mut self, mut ctx: ContextRef) {
        ctx.caller = self.cur_context;
        self.cur_context = Some(ctx);
    }

    pub fn prepare_stack(&mut self, args_len: usize) {
        let prev_lfp = self.lfp;
        let prev_cfp = self.cfp;
        self.lfp = self.stack_len() - args_len;
        self.cfp = self.stack_len();
        self.stack_push_reg(prev_lfp, prev_cfp, self.pc);
    }

    fn unwind_stack(&mut self) {
        let (lfp, cfp, pc) = self.stack_fetch_reg();
        self.set_stack_len(self.lfp);
        self.lfp = lfp;
        self.cfp = cfp;
        self.pc = pc;
    }

    fn clear_stack(&mut self) {
        self.set_stack_len(self.cfp + 3);
    }

    /// Pop one context, and restore the pc and exec_stack length.
    fn unwind_context(&mut self) {
        self.unwind_stack();
        //self.pc = self.context().prev_pc;
        match self.cur_context {
            Some(c) => {
                self.cur_context = c.caller;
                if !c.from_heap() {
                    self.ctx_stack.pop(c)
                };
            }
            None => {}
        }
    }

    #[cfg(not(tarpaulin_include))]
    pub fn clear(&mut self) {
        self.exec_stack.clear();
        self.cur_context = None;
    }

    /// Get Class of current class context.
    pub fn current_class(&self) -> Module {
        self.context().self_value.get_class_if_object()
    }

    pub fn is_module_function(&self) -> bool {
        self.context().module_function
    }

    pub fn set_module_function(&mut self, flag: bool) {
        self.context().module_function = flag;
    }

    pub fn jump_pc(&mut self, inst_offset: usize, disp: ISeqDisp) {
        self.pc = (self.pc + inst_offset + disp).into();
    }

    pub fn parse_program(
        &mut self,
        path: impl Into<PathBuf>,
        program: String,
    ) -> Result<MethodId, RubyError> {
        let path = path.into();
        let result = Parser::parse_program(program, path.clone())?;
        //let source_info = SourceInfoRef::new(SourceInfo::new(path, program));
        #[cfg(feature = "perf")]
        self.globals.perf.set_prev_inst(Perf::INVALID);

        let methodref = Codegen::new(result.source_info).gen_iseq(
            &mut self.globals,
            vec![],
            result.node,
            result.lvar_collector,
            true,
            ContextKind::Method(None),
            vec![],
        )?;
        Ok(methodref)
    }

    pub fn parse_program_eval(
        &mut self,
        path: impl Into<PathBuf>,
        program: String,
        extern_context: ContextRef,
    ) -> Result<MethodId, RubyError> {
        let path = path.into();
        let result = Parser::parse_program_eval(program, path, Some(extern_context))?;

        #[cfg(feature = "perf")]
        self.globals.perf.set_prev_inst(Perf::INVALID);

        let mut codegen = Codegen::new(result.source_info);
        codegen.set_external_context(extern_context);
        let method = codegen.gen_iseq(
            &mut self.globals,
            vec![],
            result.node,
            result.lvar_collector,
            true,
            ContextKind::Eval,
            vec![],
        )?;
        Ok(method)
    }

    pub fn parse_program_binding(
        &mut self,
        path: impl Into<PathBuf>,
        program: String,
        context: ContextRef,
    ) -> Result<MethodId, RubyError> {
        let path = path.into();
        let result = Parser::parse_program_binding(program, path, context)?;

        #[cfg(feature = "perf")]
        self.globals.perf.set_prev_inst(Perf::INVALID);

        let mut codegen = Codegen::new(result.source_info);
        if let Some(outer) = context.outer {
            codegen.set_external_context(outer)
        };
        let method = codegen.gen_iseq(
            &mut self.globals,
            vec![],
            result.node,
            result.lvar_collector,
            true,
            ContextKind::Eval,
            vec![],
        )?;
        Ok(method)
    }

    pub fn run(&mut self, path: impl Into<PathBuf>, program: String) -> VMResult {
        let prev_len = self.stack_len();
        let method = self.parse_program(path, program)?;
        let self_value = self.globals.main_object;
        let val = self.eval_method(method, self_value, &Args::new0())?;
        #[cfg(feature = "perf")]
        self.globals.perf.get_perf(Perf::INVALID);
        assert!(
            self.stack_len() == prev_len,
            "exec_stack length must be {}. actual:{}",
            prev_len,
            self.stack_len()
        );
        Ok(val)
    }

    #[cfg(not(tarpaulin_include))]
    pub fn run_repl(&mut self, result: ParseResult, mut context: ContextRef) -> VMResult {
        #[cfg(feature = "perf")]
        self.globals.perf.set_prev_inst(Perf::CODEGEN);

        let method = Codegen::new(result.source_info).gen_iseq(
            &mut self.globals,
            vec![],
            result.node,
            result.lvar_collector,
            true,
            ContextKind::Method(None),
            vec![],
        )?;
        let iseq = method.as_iseq();
        context.iseq_ref = iseq;
        self.lfp = self.stack_len();
        self.cfp = self.stack_len();
        self.stack_push_reg(0, 0, self.pc);
        self.run_context(context)?;
        #[cfg(feature = "perf")]
        self.globals.perf.get_perf(Perf::INVALID);

        let val = self.stack_pop();
        let stack_len = self.stack_len();
        if stack_len != 0 {
            eprintln!("Error: stack length is illegal. {}", stack_len);
        };

        Ok(val)
    }
}

impl VM {
    fn gc(&mut self) {
        let malloced = MALLOC_AMOUNT.load(std::sync::atomic::Ordering::Relaxed);
        let (object_trigger, malloc_trigger) = ALLOC.with(|m| {
            let m = m.borrow();
            (m.is_allocated(), m.malloc_threshold < malloced)
        });
        if !object_trigger && !malloc_trigger {
            return;
        }
        #[cfg(feature = "perf")]
        self.globals.perf.get_perf(Perf::GC);
        self.globals.gc();
        if malloc_trigger {
            let malloced = MALLOC_AMOUNT.load(std::sync::atomic::Ordering::Relaxed);
            ALLOC.with(|m| m.borrow_mut().malloc_threshold = malloced * 2);
        }
    }

    fn jmp_cond(&mut self, iseq: &ISeq, cond: bool, inst_offset: usize, dest_offset: usize) {
        if cond {
            self.pc += inst_offset;
        } else {
            let disp = iseq.read_disp(self.pc + dest_offset);
            self.jump_pc(inst_offset, disp);
        }
    }

    /// Save the pc and exec_stack length of current context in the `context`, and push it to the context stack.
    /// Set the pc to 0.
    fn invoke_new_context(&mut self, context: ContextRef) {
        #[cfg(feature = "perf-method")]
        {
            MethodRepo::inc_counter(context.iseq_ref.method);
        }
        //context.prev_pc = self.pc;
        self.context_push(context);
        self.pc = ISeqPos::from(0);
        #[cfg(any(feature = "trace", feature = "trace-func"))]
        if self.globals.startup_flag {
            let ch = if context.called { "+++" } else { "---" };
            let iseq = context.iseq_ref;
            eprintln!(
                "{}> {:?} {:?} {:?}",
                ch, iseq.method, iseq.kind, iseq.source_info.path
            );
        }
        #[cfg(feature = "trace")]
        if self.globals.startup_flag {
            eprintln!("--------invoke new context------------------------------------------");
            context.dump();
            eprintln!("--------------------------------------------------------------------");
        }
    }

    pub fn run_context(&mut self, mut context: ContextRef) -> Result<(), RubyError> {
        context.called = true;
        self.invoke_new_context(context);
        self.run_loop()
    }

    fn run_loop(&mut self) -> Result<(), RubyError> {
        loop {
            match self.run_context_main() {
                Ok(_) => {
                    let use_value = self.context().use_value;
                    assert!(self.context().called);
                    // normal return from method.
                    if use_value {
                        let val = self.stack_pop();
                        self.unwind_context();
                        self.stack_push(val);
                        #[cfg(any(feature = "trace", feature = "trace-func"))]
                        if self.globals.startup_flag {
                            eprintln!("<+++ Ok({:?})", val);
                        }
                    } else {
                        self.unwind_context();
                        #[cfg(any(feature = "trace", feature = "trace-func"))]
                        if self.globals.startup_flag {
                            eprintln!("<+++ Ok");
                        }
                    }
                    return Ok(());
                }
                Err(mut err) => {
                    match err.kind {
                        RubyErrorKind::BlockReturn => {
                            #[cfg(any(feature = "trace", feature = "trace-func"))]
                            if self.globals.startup_flag {
                                eprintln!("<+++ BlockReturn({:?})", self.globals.error_register,);
                            }
                            return Err(err);
                        }
                        RubyErrorKind::MethodReturn => {
                            // TODO: Is it necessary to check use_value?
                            loop {
                                if self.context().called {
                                    #[cfg(any(feature = "trace", feature = "trace-func"))]
                                    if self.globals.startup_flag {
                                        eprintln!(
                                            "<+++ MethodReturn({:?})",
                                            self.globals.error_register
                                        );
                                    }
                                    self.unwind_context();
                                    return Err(err);
                                };
                                self.unwind_context();
                                if self.context().is_method() {
                                    break;
                                }
                            }
                            let val = self.globals.error_register;
                            #[cfg(any(feature = "trace", feature = "trace-func"))]
                            if self.globals.startup_flag {
                                eprintln!("<--- MethodReturn({:?})", val);
                            }
                            self.stack_push(val);
                            continue;
                        }
                        _ => {}
                    }
                    loop {
                        let context = self.context();
                        if err.info.len() == 0 || context.iseq_ref.kind != ISeqKind::Block {
                            err.info.push((self.source_info(), self.get_loc()));
                        }
                        if let RubyErrorKind::Internal(msg) = &err.kind {
                            err.clone().show_err();
                            err.show_all_loc();
                            unreachable!("{}", msg);
                        };
                        let iseq = context.iseq_ref;
                        let catch = iseq
                            .exception_table
                            .iter()
                            .find(|x| x.include(context.cur_pc.into_usize()));
                        if let Some(entry) = catch {
                            // Exception raised inside of begin-end with rescue clauses.
                            self.pc = entry.dest.into();
                            match entry.ty {
                                ExceptionType::Rescue => self.clear_stack(),
                                ExceptionType::Continue => {}
                            };
                            let val = err
                                .to_exception_val()
                                .unwrap_or(self.globals.error_register);
                            #[cfg(any(feature = "trace", feature = "trace-func"))]
                            if self.globals.startup_flag {
                                eprintln!(":::: Exception({:?})", val);
                            }
                            self.stack_push(val);
                            break;
                        } else {
                            // Exception raised outside of begin-end.
                            //self.check_stack_integrity();
                            //self.ctx_stack.dump();
                            self.unwind_context();
                            if context.called {
                                #[cfg(any(feature = "trace", feature = "trace-func"))]
                                if self.globals.startup_flag {
                                    eprintln!("<+++ {:?}", err.kind);
                                }
                                return Err(err);
                            }
                            #[cfg(any(feature = "trace", feature = "trace-func"))]
                            if self.globals.startup_flag {
                                eprintln!("<--- {:?}", err.kind);
                            }
                        }
                    }
                }
            }
        }
    }
}

impl VM {
    pub fn show_err(&self, err: &RubyError) {
        if err.is_exception() {
            let val = self.globals.error_register;
            match val.if_exception() {
                Some(err) => err.clone().show_err(),
                None => eprintln!("{:?}", val),
            }
        } else {
            err.clone().show_err();
        }
    }

    fn get_loc(&self) -> Loc {
        let pc = self.context().cur_pc;
        let iseq = self.context().iseq_ref;
        match iseq.iseq_sourcemap.iter().find(|x| x.0 == pc) {
            Some((_, loc)) => *loc,
            None => {
                eprintln!("Bad sourcemap. pc={:?} {:?}", self.pc, iseq.iseq_sourcemap);
                Loc(0, 0)
            }
        }
    }

    /// Get class list in the current context.
    ///
    /// At first, this method searches the class list of outer context,
    /// and adds a class given as an argument `new_class` on the top of the list.
    /// return None in top-level.
    fn get_class_defined(&self, new_module: impl Into<Module>) -> Vec<Module> {
        let mut ctx = self.cur_context;
        let mut v = vec![new_module.into()];
        while let Some(c) = ctx {
            if c.iseq_ref.is_classdef() {
                v.push(Module::new(c.self_value));
            }
            ctx = c.caller;
        }
        v.reverse();
        v
    }
}

// Handling global varables.
impl VM {
    pub fn get_global_var(&self, id: IdentId) -> Option<Value> {
        self.globals.get_global_var(id)
    }

    pub fn set_global_var(&mut self, id: IdentId, val: Value) {
        self.globals.set_global_var(id, val);
    }
}

// Handling special variables.
impl VM {
    pub fn get_special_var(&self, id: u32) -> Value {
        if id == 0 {
            self.sp_last_match
                .to_owned()
                .map(|s| Value::string(s))
                .unwrap_or_default()
        } else if id == 1 {
            self.sp_post_match
                .to_owned()
                .map(|s| Value::string(s))
                .unwrap_or_default()
        } else if id >= 100 {
            self.get_special_matches(id as usize - 100)
        } else {
            unreachable!()
        }
    }

    pub fn set_special_var(&self, _id: u32, _val: Value) -> Result<(), RubyError> {
        unreachable!()
    }

    /// Save captured strings to special variables.
    /// $n (n:0,1,2,3...) <- The string which matched with nth parenthesis in the last successful match.
    /// $& <- The string which matched successfully at last.
    /// $' <- The string after $&.
    pub fn get_captures(&mut self, captures: &Captures, given: &str) {
        //let id1 = IdentId::get_id("$&");
        //let id2 = IdentId::get_id("$'");
        match captures.get(0) {
            Some(m) => {
                self.sp_last_match = Some(given[m.start()..m.end()].to_string());
                self.sp_post_match = Some(given[m.end()..].to_string());
            }
            None => {
                self.sp_last_match = None;
                self.sp_post_match = None;
            }
        };

        self.sp_matches.clear();
        for i in 1..captures.len() {
            self.sp_matches.push(
                captures
                    .get(i)
                    .map(|m| given[m.start()..m.end()].to_string()),
            );
        }
    }

    pub fn get_special_matches(&self, nth: usize) -> Value {
        match self.sp_matches.get(nth - 1) {
            None => Value::nil(),
            Some(s) => s.to_owned().map(|s| Value::string(s)).unwrap_or_default(),
        }
    }
}

// Handling class variables.
impl VM {
    fn set_class_var(&self, id: IdentId, val: Value) -> Result<(), RubyError> {
        if self.cur_context.is_none() {
            return Err(RubyError::runtime("class varable access from toplevel."));
        }
        let self_val = self.context().self_value;
        let org_class = self_val.get_class_if_object();
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
        if self.cur_context.is_none() {
            return Err(RubyError::runtime("class varable access from toplevel."));
        }
        let self_val = self.context().self_value;
        let mut class = self_val.get_class_if_object();
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

impl VM {
    fn eval_rescue(&self, val: Value, exceptions: &[Value]) -> bool {
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
                    return true;
                }
            };

            match module.upper() {
                Some(upper) => module = upper,
                None => break,
            }
        }
        false
    }

    /// Generate new class object with `super_val` as a superclass.
    fn define_class(
        &mut self,
        base: Value,
        id: IdentId,
        is_module: bool,
        super_val: Value,
    ) -> Result<Module, RubyError> {
        let mut current_class = if base.is_nil() {
            self.current_class()
        } else {
            Module::new(base)
        };
        match current_class.get_const_noautoload(id) {
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
            _ => {
                let val = if is_module {
                    if !super_val.is_nil() {
                        panic!("Module can not have superclass.");
                    };
                    Module::module()
                } else {
                    let super_val = if super_val.is_nil() {
                        BuiltinClass::object()
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
                    self.eval_send0(id, val)?.as_string().unwrap().to_string()
                }
            },
        };
        Ok(s)
    }
}

// API's for handling instance/singleton methods.

impl VM {
    /// Define a method on `target_obj`.
    /// If `target_obj` is not Class, use Class of it.
    pub fn define_method(&mut self, target_obj: Value, id: IdentId, method: MethodId) {
        target_obj.get_class_if_object().add_method(id, method);
    }

    /// Define a method on a singleton class of `target_obj`.
    pub fn define_singleton_method(
        &mut self,
        target_obj: Value,
        id: IdentId,
        method: MethodId,
    ) -> Result<(), RubyError> {
        target_obj.get_singleton_class()?.add_method(id, method);
        Ok(())
    }
}

impl VM {
    /// Get local variable table.
    fn get_outer_context(&mut self, outer: u32) -> ContextRef {
        let mut context = self.context();
        for _ in 0..outer {
            context = context.outer.unwrap();
        }
        context
    }

    fn pop_key_value_pair(&mut self, arg_num: usize) -> FxIndexMap<HashKey, Value> {
        let mut hash = FxIndexMap::default();
        let len = self.exec_stack.len() - arg_num * 2;
        for i in 0..arg_num {
            let key = self.exec_stack[len + i * 2];
            let value = self.exec_stack[len + i * 2 + 1];
            hash.insert(HashKey(key), value);
        }
        self.set_stack_len(len);
        hash
    }

    /// Pop values and store them in new `Args`. `args_num` specifies the number of values to be popped.
    /// If there is some Array or Range with splat operator, break up the value and store each of them.
    fn pop_args_to_args(&mut self, arg_num: usize) -> Args2 {
        let range = self.prepare_args(arg_num);
        let args = Args2::new(range.end - range.start);
        args
    }

    fn pop_args_to_vec(&mut self, arg_num: usize) -> Vec<Value> {
        let range = self.prepare_args(arg_num);
        self.exec_stack.split_off(range.start)
    }

    fn prepare_args(&mut self, arg_num: usize) -> std::ops::Range<usize> {
        let arg_start = self.stack_len() - arg_num;
        let mut i = arg_start;
        while i < self.stack_len() {
            let len = self.stack_len();
            let val = self.exec_stack[i];
            match val.as_splat() {
                Some(inner) => match inner.as_rvalue() {
                    None => {
                        self.exec_stack[i] = inner;
                        i += 1;
                    }
                    Some(obj) => match &obj.kind {
                        ObjKind::Array(a) => {
                            let ary_len = a.len();
                            if ary_len == 0 {
                                self.exec_stack.remove(i);
                            } else {
                                self.exec_stack.resize(len + ary_len - 1, Value::nil());
                                self.exec_stack.copy_within(i + 1..len, i + ary_len);
                                self.exec_stack[i..i + ary_len].copy_from_slice(&a[..]);
                                i += ary_len;
                            }
                        }
                        // TODO: should use `to_a` method.
                        ObjKind::Range(r) => {
                            let start = r.start.coerce_to_fixnum("Expect Integer.").unwrap();
                            let end = r.end.coerce_to_fixnum("Expect Integer.").unwrap()
                                + if r.exclude { 0 } else { 1 };
                            if end >= start {
                                let ary_len = (end - start) as usize;
                                self.exec_stack.resize(len + ary_len - 1, Value::nil());
                                self.exec_stack.copy_within(i + 1..len, i + ary_len);
                                for (idx, val) in (start..end).enumerate() {
                                    self.exec_stack[i + idx] = Value::integer(val);
                                }
                                i += ary_len;
                            } else {
                                self.exec_stack.remove(i);
                            };
                        }
                        _ => {
                            self.exec_stack[i] = inner;
                            i += 1;
                        }
                    },
                },
                None => i += 1,
            };
        }
        arg_start..self.stack_len()
    }

    pub fn new_block(&mut self, id: impl Into<MethodId>) -> Block {
        let ctx = self.context();
        Block::Block(id.into(), ctx)
    }

    pub fn new_block_with_outer(&mut self, id: impl Into<MethodId>, outer: ContextRef) -> Block {
        Block::Block(id.into(), outer)
    }

    pub fn create_range(&mut self, start: Value, end: Value, exclude_end: bool) -> VMResult {
        if self.eval_compare(start, end)?.is_nil() {
            return Err(RubyError::argument("Bad value for range."));
        }
        Ok(Value::range(start, end, exclude_end))
    }

    /// Create new Proc object from `block`,
    /// moving outer `Context`s on stack to heap.
    pub fn create_proc(&mut self, block: &Block) -> Value {
        match block {
            Block::Block(method, outer) => self.create_proc_from_block(*method, *outer),
            Block::Proc(proc) => *proc,
        }
    }

    pub fn create_proc_from_block(&mut self, method: MethodId, outer: ContextRef) -> Value {
        let iseq = method.as_iseq();
        Value::procobj(self, outer.self_value, iseq, outer)
    }

    /// Create new Lambda object from `block`,
    /// moving outer `Context`s on stack to heap.
    pub fn create_lambda(&mut self, block: &Block) -> VMResult {
        match block {
            Block::Block(method, outer) => {
                let mut iseq = method.as_iseq();
                iseq.kind = ISeqKind::Method(None);
                Ok(Value::procobj(self, outer.self_value, iseq, *outer))
            }
            Block::Proc(proc) => Ok(*proc),
        }
    }

    #[cfg(not(tarpaulin_include))]
    #[allow(dead_code)]
    fn check_stack_integrity(&self) {
        eprintln!("Checking context integlity..");
        let mut cur = self.cur_context;
        let mut n = 0;
        while let Some(c) = cur {
            eprint!("[{}]:", n);
            c.pp();
            assert!(c.alive());
            let mut o = c.outer;
            let mut on = 1;
            while let Some(ctx) = o {
                eprint!("    [outer:{}]:", on);
                ctx.pp();
                assert!(ctx.alive());
                o = ctx.outer;
                on += 1;
            }
            cur = c.caller;
            n += 1;
        }
        eprintln!("--------------------------------");
    }

    /// Move outer execution contexts on the stack to the heap.
    pub fn move_outer_to_heap(&mut self, outer: ContextRef) -> ContextRef {
        if outer.on_heap() {
            return outer;
        }
        let outer_heap = outer.move_to_heap();
        if let Some(mut c) = self.cur_context {
            if let CtxKind::Dead(c_heap) = c.on_stack {
                c = c_heap;
                self.cur_context = Some(c);
            }
            while let Some(mut caller) = c.caller {
                if let CtxKind::Dead(caller_heap) = caller.on_stack {
                    caller = caller_heap;
                    c.caller = Some(caller);
                }
                match c.outer {
                    Some(o) => {
                        if let CtxKind::Dead(o_heap) = o.on_stack {
                            c.outer = Some(o_heap);
                        }
                    }
                    None => {}
                }
                c = caller;
            }
            match c.outer {
                Some(o) => {
                    if let CtxKind::Dead(o_heap) = o.on_stack {
                        c.outer = Some(o_heap);
                    }
                }
                None => {}
            }
        };
        outer_heap
    }

    /// Create a new execution context for a block.
    ///
    /// A new context is generated on heap, and all of the outer context chains are moved to heap.
    pub fn create_block_context(&mut self, method: MethodId, outer: ContextRef) -> ContextRef {
        assert!(outer.alive());
        let outer = self.move_outer_to_heap(outer);
        let iseq = method.as_iseq();
        ContextRef::new_heap(outer.self_value, None, iseq, Some(outer))
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
