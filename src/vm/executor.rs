use crate::coroutine::FiberHandle;
use crate::parse::codegen::{ContextKind, ExceptionType};
use crate::*;
use fancy_regex::Captures;
pub use frame::Frame;
use std::ops::{Index, IndexMut};

#[cfg(feature = "perf")]
use super::perf::*;
use std::path::PathBuf;
use vm_inst::*;
mod constants;
mod fiber;
mod frame;
mod loader;
mod method;
mod ops;
mod opt_core;

pub type ValueTable = FxHashMap<IdentId, Value>;
pub type VMResult = Result<Value, RubyError>;

const VM_STACK_INITIAL_SIZE: usize = 4096;

#[derive(Debug)]
pub struct VM {
    // Global info
    pub globals: GlobalsRef,
    // VM state
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
            VMResKind::Invoke => vm.run_loop(),
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

// API's
impl GC for VM {
    fn mark(&self, alloc: &mut Allocator) {
        let mut cfp = Some(self.cur_frame());
        while let Some(f) = cfp {
            if let Some(c) = self.frame_heap(f) {
                c.mark(alloc);
            }
            cfp = self.frame_caller(f);
        }
        self.exec_stack.iter().for_each(|v| v.mark(alloc));
        self.temp_stack.iter().for_each(|v| v.mark(alloc));
    }
}

impl VM {
    pub fn new(mut globals: GlobalsRef) -> Self {
        let mut vm = VM {
            globals,
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
        vm.init_frame();
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
        let mut vm = VM {
            globals: self.globals,
            temp_stack: vec![],
            exec_stack: Vec::with_capacity(VM_STACK_INITIAL_SIZE),
            pc: ISeqPos::from(0),
            lfp: 0,
            cfp: 0,
            handle: None,
            sp_last_match: None,
            sp_post_match: None,
            sp_matches: vec![],
        };
        vm.init_frame();
        vm
    }

    #[cfg(debug_assertions)]
    fn kind(&self) -> ISeqKind {
        self.cur_iseq().kind
    }

    pub fn stack_push(&mut self, val: Value) {
        self.exec_stack.push(val)
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

    pub fn stack_push_args(&mut self, args: &Args) -> Args2 {
        self.exec_stack.extend_from_slice(args);
        Args2::from(args)
    }

    pub fn stack_fill(&mut self, base: usize, r: std::ops::Range<usize>, val: Value) {
        self.exec_stack[base + r.start..base + r.end].fill(val);
    }

    pub fn stack_slice(&mut self, base: usize, r: std::ops::Range<usize>) -> &[Value] {
        &self.exec_stack[base + r.start..base + r.end]
    }

    pub fn stack_copy_within(&mut self, base: usize, src: std::ops::Range<usize>, dest: usize) {
        self.exec_stack
            .copy_within(base + src.start..base + src.end, base + dest);
    }

    // handling arguments

    pub fn args(&self) -> &[Value] {
        &self.exec_stack[self.lfp..self.cfp - 1]
    }

    pub fn slice(&self, start: usize, end: usize) -> &[Value] {
        &self.exec_stack[start..end]
    }

    pub fn slice_mut(&mut self, start: usize, end: usize) -> &mut [Value] {
        &mut self.exec_stack[start..end]
    }

    pub fn args_len(&self) -> usize {
        self.cfp - self.lfp - 1
    }

    pub fn self_value(&self) -> Value {
        self.exec_stack[self.cfp - 1]
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

    pub fn caller_frame_context(&self) -> HeapCtxRef {
        let mut frame = self.cur_caller_frame();
        while let Some(c) = frame {
            if let Some(ctx) = self.frame_heap(c) {
                return ctx;
            }
            frame = self.frame_caller(c);
        }
        unreachable!("no caller frame.");
    }

    pub fn get_local(&self, index: LvarId) -> Value {
        let f = self.cur_frame();
        match self.frame_heap(f) {
            Some(h) => h.lvar[*index],
            None => self.frame_locals(f)[*index],
        }
    }

    pub fn get_dyn_local(&self, index: LvarId, outer: u32) -> Value {
        match self.get_outer_context(outer) {
            Context::Frame(f) => match self.frame_heap(f) {
                Some(h) => h.lvar[*index],
                None => self.frame_locals(f)[*index],
            },
            Context::Heap(h) => h[index],
        }
    }

    pub fn set_local(&mut self, index: LvarId, val: Value) {
        let f = self.cur_frame();
        match self.frame_heap(f) {
            Some(mut h) => h.lvar[*index] = val,
            None => self.frame_mut_locals(f)[*index] = val,
        };
    }

    pub fn set_dyn_local(&mut self, index: LvarId, outer: u32, val: Value) {
        match self.get_outer_context(outer) {
            Context::Frame(f) => match self.frame_heap(f) {
                Some(mut h) => h.lvar[*index] = val,
                None => self.frame_mut_locals(f)[*index] = val,
            },
            Context::Heap(mut h) => h[index] = val,
        }
    }

    /// Pop one context, and restore the pc and exec_stack length.
    fn unwind_context(&mut self) {
        self.unwind_frame();
    }

    #[cfg(not(tarpaulin_include))]
    pub fn clear(&mut self) {
        self.set_stack_len(8);
        //self.cur_context = None;
    }

    /// Get Class of current class context.
    pub fn current_class(&self) -> Module {
        self.self_value().get_class_if_object()
    }

    pub fn get_fiber_method_context(&self) -> HeapCtxRef {
        match self.handle.expect("No parent Fiber.").kind() {
            crate::coroutine::FiberKind::Fiber(ctx) => return ctx.method_context(),
            _ => {}
        };
        self.frame_heap(self.cur_frame()).unwrap().method_context()
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
        #[cfg(feature = "perf")]
        self.globals.perf.set_prev_inst(Perf::INVALID);

        let loc = result.node.loc;
        let methodref = Codegen::new(result.source_info).gen_iseq(
            &mut self.globals,
            vec![],
            result.node,
            result.lvar_collector,
            true,
            ContextKind::Method(None),
            vec![],
            loc,
        )?;
        Ok(methodref)
    }

    pub fn parse_program_eval(
        &mut self,
        path: impl Into<PathBuf>,
        program: String,
    ) -> Result<MethodId, RubyError> {
        let extern_context = self.caller_frame_context();
        let path = path.into();
        let result = Parser::parse_program_eval(program, path, Some(extern_context))?;

        #[cfg(feature = "perf")]
        self.globals.perf.set_prev_inst(Perf::INVALID);

        let mut codegen = Codegen::new(result.source_info);
        codegen.set_external_context(extern_context);
        let loc = result.node.loc;
        let method = codegen.gen_iseq(
            &mut self.globals,
            vec![],
            result.node,
            result.lvar_collector,
            true,
            ContextKind::Eval,
            vec![],
            loc,
        )?;
        Ok(method)
    }

    pub fn parse_program_binding(
        &mut self,
        path: impl Into<PathBuf>,
        program: String,
        context: HeapCtxRef,
    ) -> Result<MethodId, RubyError> {
        let path = path.into();
        let result = Parser::parse_program_binding(program, path, context)?;

        #[cfg(feature = "perf")]
        self.globals.perf.set_prev_inst(Perf::INVALID);

        let mut codegen = Codegen::new(result.source_info);
        if let Some(outer) = context.outer {
            codegen.set_external_context(outer)
        };
        let loc = result.node.loc;
        let method = codegen.gen_iseq(
            &mut self.globals,
            vec![],
            result.node,
            result.lvar_collector,
            true,
            ContextKind::Eval,
            vec![],
            loc,
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
    pub fn run_repl(&mut self, result: ParseResult, mut context: HeapCtxRef) -> VMResult {
        #[cfg(feature = "perf")]
        self.globals.perf.set_prev_inst(Perf::CODEGEN);

        let loc = result.node.loc;
        let method = Codegen::new(result.source_info).gen_iseq(
            &mut self.globals,
            vec![],
            result.node,
            result.lvar_collector,
            true,
            ContextKind::Method(None),
            vec![],
            loc,
        )?;
        let iseq = method.as_iseq();
        context.set_iseq(iseq);
        self.stack_push(context.self_value);
        self.prepare_frame(0, true, context, context.outer.map(|c| c.into()), iseq);
        self.run_loop()?;
        #[cfg(feature = "perf")]
        self.globals.perf.get_perf(Perf::INVALID);

        let val = self.stack_pop();
        Ok(val)
    }
}

impl VM {
    fn gc(&mut self) {
        let malloced = MALLOC_AMOUNT.with(|x| x.borrow().clone());
        let (object_trigger, malloc_trigger) = ALLOC.with(|m| {
            let m = m.borrow();
            (m.is_allocated(), (m.malloc_threshold as i64) < malloced)
        });
        if !object_trigger && !malloc_trigger {
            return;
        }
        #[cfg(feature = "perf")]
        self.globals.perf.get_perf(Perf::GC);
        self.globals.gc();
        if malloc_trigger {
            let malloced = MALLOC_AMOUNT.with(|x| x.borrow().clone());
            if malloced > 0 {
                ALLOC.with(|m| m.borrow_mut().malloc_threshold = (malloced * 2) as usize);
            }
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

    pub fn run_loop(&mut self) -> Result<(), RubyError> {
        self.set_called();
        assert!(self.is_ruby_func());
        loop {
            match self.run_context_main() {
                Ok(_) => {
                    let use_value = !self.discard_val();
                    assert!(self.is_called());
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
                                if self.is_called() {
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
                                if self.cur_iseq().is_method() {
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
                        let called = self.is_called();
                        let iseq = self.cur_iseq();
                        if err.info.len() == 0 || iseq.kind != ISeqKind::Block {
                            err.info.push((self.cur_source_info(), self.get_loc()));
                        }
                        if let RubyErrorKind::Internal(msg) = &err.kind {
                            err.clone().show_err();
                            err.show_all_loc();
                            unreachable!("{}", msg);
                        };
                        let catch = iseq
                            .exception_table
                            .iter()
                            .find(|x| x.include(self.cur_frame_pc().into_usize()));
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
                            if called {
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

    /// Get class list in the current context.
    ///
    /// At first, this method searches the class list of outer context,
    /// and adds a class given as an argument `new_class` on the top of the list.
    /// return None in top-level.
    fn get_class_defined(&self, new_module: impl Into<Module>) -> Vec<Module> {
        let mut cfp = Some(self.cur_frame());
        let mut v = vec![new_module.into()];
        while let Some(f) = cfp {
            if self.frame_is_ruby_func(f) {
                let iseq = self.frame_iseq(f);
                if iseq.is_classdef() {
                    v.push(Module::new(self.frame_self(f)));
                }
            }
            cfp = self.frame_caller(f);
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
        if self.self_value().id() == self.globals.main_object.id() {
            return Err(RubyError::runtime("class varable access from toplevel."));
        }
        let self_val = self.self_value();
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
        if self.self_value().id() == self.globals.main_object.id() {
            return Err(RubyError::runtime("class varable access from toplevel."));
        }
        let self_val = self.self_value();
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
                        "superclass mismatch for class {:?}. defined as subclass of {:?}, but {:?} was given.",
                        id, val_super, super_val,
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
            self.sort_by(vec, |vm, a, b| {
                Ok(vm.eval_compare(*b, *a)?.to_ordering()?)
            })?;
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
                ObjKind::Regexp(rref) => format!("/{}/", rref.as_str().to_string()),
                ObjKind::Ordinary => oref.inspect()?,
                ObjKind::Hash(href) => href.to_s(self)?,
                ObjKind::Complex { .. } => format!("{:?}", oref.kind),
                _ => {
                    let id = IdentId::get_id("inspect");
                    self.eval_send0(id, val)?
                        .expect_string("#inspect is expected to return String.")?
                        .to_string()
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
    fn get_outer_context(&self, outer: u32) -> Context {
        let mut context = self.frame_context(self.cur_frame());
        for _ in 0..outer {
            context = self.outer_context(context).unwrap();
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
        Args2::new(range.end - range.start)
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
            Block::Block(method, outer) => {
                self.create_proc_from_block(*method, outer)
                //error!
            }
            Block::Proc(proc) => *proc,
        }
    }

    pub fn create_proc_from_block(&mut self, method: MethodId, outer: &Context) -> Value {
        let iseq = method.as_iseq();
        let self_val = self.get_context_self(outer);
        Value::procobj(self, self_val, iseq, Some(outer.clone()))
    }

    /// Create new Lambda object from `block`,
    /// moving outer `Context`s on stack to heap.
    pub fn create_lambda(&mut self, block: &Block) -> VMResult {
        match block {
            Block::Block(method, outer) => {
                let mut iseq = method.as_iseq();
                iseq.kind = ISeqKind::Method(None);
                let self_val = self.get_context_self(outer);
                Ok(Value::procobj(self, self_val, iseq, Some(outer.clone())))
            }
            Block::Proc(proc) => Ok(*proc),
        }
    }

    /// Create a new execution context for a block.
    ///
    /// A new context is generated on heap, and all of the outer context chains are moved to heap.
    pub fn create_block_context(&mut self, method: MethodId, outer: Context) -> HeapCtxRef {
        //assert!(outer.alive());
        let outer = self.move_outer_to_heap(&outer);
        let iseq = method.as_iseq();
        HeapCtxRef::new_heap(outer.self_value, None, iseq, Some(outer))
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
