use crate::coroutine::*;
use crate::parse::codegen::{ContextKind, ExceptionType};
use crate::*;

#[cfg(feature = "perf")]
use super::perf::*;
use std::path::PathBuf;
use vm_inst::*;
mod loader;
mod opt_core;

pub type ValueTable = FxHashMap<IdentId, Value>;
pub type VMResult = Result<Value, RubyError>;

#[derive(Debug)]
pub struct VM {
    // Global info
    pub globals: GlobalsRef,
    // VM state
    cur_context: Option<ContextRef>,
    ctx_stack: ContextStore,
    exec_stack: Vec<Value>,
    temp_stack: Vec<Value>,
    pc: ISeqPos,
    pub handle: Option<FiberHandle>,
    pub exec_native: bool,
}

pub type VMRef = Ref<VM>;

pub enum VMResKind {
    Return,
    Invoke,
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
    pub fn new_stack_context(&mut self, context: Context) -> ContextRef {
        self.ctx_stack.push(context)
    }

    pub fn new_stack_context_with(
        &mut self,
        self_value: Value,
        block: Option<Block>,
        iseq: ISeqRef,
        outer: Option<ContextRef>,
    ) -> ContextRef {
        self.ctx_stack.push_with(self_value, block, iseq, outer)
    }
}

impl VM {
    pub fn new(mut globals: GlobalsRef) -> Self {
        let mut vm = VM {
            globals,
            cur_context: None,
            ctx_stack: ContextStore::new(),
            exec_stack: vec![],
            temp_stack: vec![],
            pc: ISeqPos::from(0),
            handle: None,
            exec_native: false,
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
                err.show_err();
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
            exec_stack: vec![],
            pc: ISeqPos::from(0),
            handle: None,
            exec_native: false,
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
        self.get_method_context().iseq_ref.unwrap()
    }

    pub fn source_info(&self) -> SourceInfoRef {
        match self.context().iseq_ref {
            Some(iseq) => iseq.source_info,
            None => SourceInfoRef::default(),
        }
    }

    pub fn get_source_path(&self) -> PathBuf {
        self.context().iseq_ref.unwrap().source_info.path.clone()
    }

    fn is_method(&self) -> bool {
        self.context().is_method()
    }

    pub fn stack_push(&mut self, val: Value) {
        self.exec_stack.push(val)
    }

    pub fn stack_pop(&mut self) -> Value {
        self.exec_stack
            .pop()
            .unwrap_or_else(|| panic!("exec stack is empty."))
    }

    pub fn stack_top(&mut self) -> Value {
        *self
            .exec_stack
            .last()
            .unwrap_or_else(|| panic!("exec stack is empty."))
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

    pub fn context_pop(&mut self) {
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
        context.iseq_ref = Some(iseq);

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
        //self.gc_counter += 1;
        if !ALLOC.with(|m| m.borrow().is_allocated()) {
            return;
        };
        #[cfg(feature = "perf")]
        self.globals.perf.get_perf(Perf::GC);
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

    /// Pop one context, and restore the pc and exec_stack length.
    fn unwind_context(&mut self) {
        self.set_stack_len(self.context().prev_stack_len);
        self.pc = self.context().prev_pc;
        self.context_pop();
    }

    /// Save the pc and exec_stack length of current context in the `context`, and push it to the context stack.
    /// Set the pc to 0.
    fn invoke_new_context(&mut self, mut context: ContextRef) {
        #[cfg(feature = "perf-method")]
        {
            MethodRepo::inc_counter(context.iseq_ref.unwrap().method);
        }
        context.prev_stack_len = self.stack_len();
        context.prev_pc = self.pc;
        self.context_push(context);
        self.pc = ISeqPos::from(0);
        #[cfg(any(feature = "trace", feature = "trace-func"))]
        if self.globals.startup_flag {
            let ch = if context.called { "+++" } else { "---" };
            let iseq = context.iseq_ref.unwrap();
            eprintln!(
                "{}> {:?} {:?} {:?}",
                ch, iseq.method, iseq.kind, iseq.source_info.path
            );
        }
        #[cfg(feature = "trace")]
        if self.globals.startup_flag {
            eprintln!("  ------invoke new context------------------------------------------");
            context.dump();
            eprintln!("  ------------------------------------------------------------------");
        }
    }

    pub fn run_context(&mut self, mut context: ContextRef) -> Result<(), RubyError> {
        context.called = true;
        self.invoke_new_context(context);
        loop {
            match self.run_context_main() {
                Ok(_) => {
                    assert!(self.context().called);
                    // normal return from method.
                    assert_eq!(
                        self.stack_len(),
                        self.context().prev_stack_len
                            + if self.context().use_value { 1 } else { 0 }
                    );
                    self.pc = self.context().prev_pc;
                    self.context_pop();

                    #[cfg(any(feature = "trace", feature = "trace-func"))]
                    if self.globals.startup_flag {
                        eprintln!("<+++ Ok({:?})", self.stack_top());
                    }
                    return Ok(());
                }
                Err(mut err) => {
                    match err.kind {
                        RubyErrorKind::BlockReturn => {
                            #[cfg(any(feature = "trace", feature = "trace-func"))]
                            if self.globals.startup_flag {
                                eprintln!(
                                    "<+++ BlockReturn({:?}) stack:{}",
                                    self.globals.error_register,
                                    self.stack_len()
                                );
                            }
                            return Err(err);
                        }
                        RubyErrorKind::MethodReturn => {
                            let val = self.stack_pop();
                            loop {
                                if self.context().called {
                                    #[cfg(any(feature = "trace", feature = "trace-func"))]
                                    if self.globals.startup_flag {
                                        eprintln!("<+++ {:?}({:?})", err.kind, val);
                                    }
                                    self.unwind_context();
                                    self.stack_push(val);
                                    return Err(err);
                                };
                                self.unwind_context();
                                if self.context().is_method() {
                                    break;
                                }
                            }
                            #[cfg(any(feature = "trace", feature = "trace-func"))]
                            if self.globals.startup_flag {
                                eprintln!("<--- {:?}({:?})", err.kind, val);
                            }
                            self.stack_push(val);
                            continue;
                        }
                        _ => {}
                    }
                    loop {
                        let context = self.context();
                        if err.info.len() == 0 || context.iseq_ref.unwrap().kind != ISeqKind::Block
                        {
                            err.info.push((self.source_info(), self.get_loc()));
                        }
                        if let RubyErrorKind::Internal(msg) = &err.kind {
                            //eprintln!();
                            err.show_err();
                            err.show_all_loc();
                            unreachable!("{}", msg);
                        };
                        let iseq = context.iseq_ref.unwrap();
                        let catch = iseq
                            .exception_table
                            .iter()
                            .find(|x| x.include(context.cur_pc.into_usize()));
                        if let Some(entry) = catch {
                            // Exception raised inside of begin-end with rescue clauses.
                            self.pc = entry.dest.into();
                            match entry.ty {
                                ExceptionType::Rescue => self.set_stack_len(context.prev_stack_len),
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
    fn get_loc(&self) -> Loc {
        let pc = self.context().cur_pc;
        match self.context().iseq_ref {
            None => Loc(1, 1),
            Some(iseq) => match iseq.iseq_sourcemap.iter().find(|x| x.0 == pc) {
                Some((_, loc)) => *loc,
                None => {
                    eprintln!("Bad sourcemap. pc={:?} {:?}", self.pc, iseq.iseq_sourcemap);
                    Loc(0, 0)
                }
            },
        }
    }

    /// Get class list in the current context.
    ///
    /// At first, this method searches the class list of outer context,
    /// and adds a class given as an argument `new_class` on the top of the list.
    /// return None in top-level.
    fn get_class_defined(&self, new_module: impl Into<Module>) -> Vec<Module> {
        /*dbg!(self
        .class_context
        .iter()
        .map(|(v, _)| *v)
        .collect::<Vec<Module>>());*/
        let mut ctx = self.cur_context;
        let mut v = vec![new_module.into()];
        while let Some(c) = ctx {
            if c.iseq_ref.unwrap().is_classdef() {
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

// Handling constants.
impl VM {
    /// Search lexical class stack and then, search class inheritance chain for a constant `id`,
    /// returning the value.
    /// Returns error if the constant was not defined, or autoload failed.
    fn find_const(&mut self, id: IdentId) -> VMResult {
        match self.get_lexical_const(id)? {
            Some(v) => Ok(v),
            None => {
                let class = self.context().self_value.get_class();
                self.get_super_const(class, id)
            }
        }
    }

    pub fn enumerate_const(&self) -> Vec<IdentId> {
        let mut map = FxHashSet::default();
        self.enumerate_env_const(&mut map);
        self.enumerate_super_const(&mut map);
        map.into_iter().collect()
    }

    /// Search lexical class stack for a constant `id`.
    /// If the constant was found, returns Ok(Some(Value)), and if not, returns Ok(None).
    /// Returns error if an autoload failed.
    fn get_lexical_const(&mut self, id: IdentId) -> Result<Option<Value>, RubyError> {
        let class_defined = &self.get_method_iseq().class_defined;
        for m in class_defined.iter().rev() {
            match self.get_mut_const(*m, id)? {
                Some(v) => return Ok(Some(v)),
                None => {}
            }
        }
        Ok(None)
    }

    fn enumerate_env_const(&self, map: &mut FxHashSet<IdentId>) {
        let class_defined = &self.get_method_iseq().class_defined;
        class_defined.iter().for_each(|m| {
            m.enumerate_const().for_each(|id| {
                map.insert(*id);
            })
        });
    }

    /// Search class inheritance chain of `class` for a constant `id`, returning the value.
    /// Returns name error if the constant was not defined.
    pub fn get_super_const(&mut self, mut class: Module, id: IdentId) -> VMResult {
        let is_module = class.is_module();
        loop {
            match self.get_mut_const(class, id)? {
                Some(val) => return Ok(val),
                None => match class.upper() {
                    Some(upper) => class = upper,
                    None => {
                        if is_module {
                            if let Some(v) = self.get_mut_const(BuiltinClass::object(), id)? {
                                return Ok(v);
                            }
                        }
                        return Err(RubyError::uninitialized_constant(id));
                    }
                },
            }
        }
    }

    pub fn enumerate_super_const(&self, map: &mut FxHashSet<IdentId>) {
        let mut class = self.context().self_value.get_class();
        let is_module = class.is_module();
        loop {
            class.enumerate_const().into_iter().for_each(|id| {
                map.insert(*id);
            });
            match class.upper() {
                Some(upper) => class = upper,
                None => {
                    if is_module {
                        BuiltinClass::object()
                            .enumerate_const()
                            .into_iter()
                            .for_each(|id| {
                                map.insert(*id);
                            })
                    }
                    break;
                }
            }
        }
    }

    /// Search constant table of `parent` for a constant `id`.
    /// If the constant was found, returns the value.
    /// Returns error if the constant was not defined or an autoload failed.
    pub fn get_scope(&mut self, parent: Module, id: IdentId) -> VMResult {
        match self.get_mut_const(parent, id)? {
            Some(val) => Ok(val),
            None => Err(RubyError::uninitialized_constant(id)),
        }
    }

    /// Search constant table of `parent` for a constant `id`.
    /// If the constant was found, returns Ok(Some(Value)), and if not, returns Ok(None).
    /// Returns error if an autoload failed.
    pub fn get_mut_const(
        &mut self,
        mut parent: Module,
        id: IdentId,
    ) -> Result<Option<Value>, RubyError> {
        match parent.get_mut_const(id) {
            Some(ConstEntry::Value(v)) => Ok(Some(*v)),
            Some(ConstEntry::Autoload(file)) => {
                self.require(file)?;
                match parent.get_mut_const(id) {
                    Some(ConstEntry::Value(v)) => Ok(Some(*v)),
                    _ => Ok(None),
                }
            }
            None => Ok(None),
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

// Utilities for method call
impl VM {
    fn invoke_send(
        &mut self,
        method_id: IdentId,
        receiver: Value,
        args: &Args,
    ) -> Result<(), RubyError> {
        match MethodRepo::find_method_from_receiver(receiver, method_id) {
            Some(method) => self.exec_method(method, receiver, args),
            None => self.send_method_missing(method_id, receiver, args),
        }
    }

    fn invoke_send0(&mut self, method_id: IdentId, receiver: Value) -> Result<(), RubyError> {
        self.invoke_send(method_id, receiver, &Args::new0())
    }

    fn invoke_send1(
        &mut self,
        method_id: IdentId,
        receiver: Value,
        arg0: Value,
    ) -> Result<(), RubyError> {
        self.invoke_send(method_id, receiver, &Args::new1(arg0))
    }

    fn send(&mut self, method_id: IdentId, receiver: Value, args: &Args) -> VMResult {
        match MethodRepo::find_method_from_receiver(receiver, method_id) {
            Some(method) => return self.eval_method(method, receiver, args),
            None => {}
        };
        self.send_method_missing(method_id, receiver, args)?;
        Ok(self.stack_pop())
    }

    pub fn send0(&mut self, method_id: IdentId, receiver: Value) -> VMResult {
        let args = Args::new0();
        self.send(method_id, receiver, &args)
    }

    fn send_method_missing(
        &mut self,
        method_id: IdentId,
        receiver: Value,
        args: &Args,
    ) -> Result<(), RubyError> {
        match MethodRepo::find_method_from_receiver(receiver, IdentId::_METHOD_MISSING) {
            Some(method) => {
                let len = args.len();
                let mut new_args = Args::new(len + 1);
                new_args[0] = Value::symbol(method_id);
                new_args[1..len + 1].copy_from_slice(args);
                self.exec_method(method, receiver, &new_args)
            }
            None => Err(RubyError::undefined_method(method_id, receiver)),
        }
    }

    fn invoke_method_missing(
        &mut self,
        method_id: IdentId,
        receiver: Value,
        args: &Args,
        use_value: bool,
    ) -> Result<VMResKind, RubyError> {
        match MethodRepo::find_method_from_receiver(receiver, IdentId::_METHOD_MISSING) {
            Some(method) => {
                let len = args.len();
                let mut new_args = Args::new(len + 1);
                new_args[0] = Value::symbol(method_id);
                new_args[1..len + 1].copy_from_slice(args);
                self.invoke_func(method, receiver, None, &new_args, use_value)
            }
            None => {
                if receiver == self.context().self_value {
                    Err(RubyError::name(format!(
                        "Undefined local variable or method `{:?}' for {:?}",
                        method_id, receiver
                    )))
                } else {
                    Err(RubyError::undefined_method(method_id, receiver))
                }
            }
        }
    }

    fn fallback_for_binop(
        &mut self,
        method: IdentId,
        lhs: Value,
        rhs: Value,
    ) -> Result<(), RubyError> {
        let class = lhs.get_class_for_method();
        match MethodRepo::find_method(class, method) {
            Some(mref) => {
                let arg = Args::new1(rhs);
                self.exec_method(mref, lhs, &arg)
            }
            None => Err(RubyError::undefined_op(format!("{:?}", method), rhs, lhs)),
        }
    }
}

macro_rules! invoke_op_i {
    ($vm:ident, $iseq:ident, $i:ident, $op:ident, $id:expr) => {
        let lhs = $vm.stack_pop();
        let val = if lhs.is_packed_fixnum() {
            Value::integer(lhs.as_packed_fixnum().$op($i as i64))
        } else if lhs.is_packed_num() {
            Value::float(lhs.as_packed_flonum().$op($i as f64))
        } else {
            return $vm.fallback_for_binop($id, lhs, Value::integer($i as i64));
        };
        $vm.stack_push(val);
        return Ok(());
    };
}

macro_rules! invoke_op {
    ($vm:ident, $op:ident, $id:expr) => {
        let len = $vm.stack_len();
        let lhs = unsafe { *$vm.exec_stack.get_unchecked(len - 2) };
        let rhs = unsafe { *$vm.exec_stack.get_unchecked(len - 1) };
        $vm.set_stack_len(len - 2);
        let val = if lhs.is_packed_fixnum() {
            if rhs.is_packed_fixnum() {
                let lhs = lhs.as_packed_fixnum();
                let rhs = rhs.as_packed_fixnum();
                Value::integer(lhs.$op(rhs))
            } else if rhs.is_packed_num() {
                let lhs = lhs.as_packed_fixnum();
                let rhs = rhs.as_packed_flonum();
                Value::float((lhs as f64).$op(rhs))
            } else {
                return $vm.fallback_for_binop($id, lhs, rhs);
            }
        } else if lhs.is_packed_num() {
            if rhs.is_packed_fixnum() {
                let lhs = lhs.as_packed_flonum();
                let rhs = rhs.as_packed_fixnum();
                Value::float(lhs.$op(rhs as f64))
            } else if rhs.is_packed_num() {
                let lhs = lhs.as_packed_flonum();
                let rhs = rhs.as_packed_flonum();
                Value::float(lhs.$op(rhs))
            } else {
                return $vm.fallback_for_binop($id, lhs, rhs);
            }
        } else {
            return $vm.fallback_for_binop($id, lhs, rhs);
        };
        $vm.stack_push(val);
        return Ok(())
    };
}

impl VM {
    fn invoke_add(&mut self) -> Result<(), RubyError> {
        use std::ops::Add;
        invoke_op!(self, add, IdentId::_ADD);
    }

    fn invoke_addi(&mut self, i: i32) -> Result<(), RubyError> {
        use std::ops::Add;
        invoke_op_i!(self, iseq, i, add, IdentId::_ADD);
    }

    fn invoke_sub(&mut self) -> Result<(), RubyError> {
        use std::ops::Sub;
        invoke_op!(self, sub, IdentId::_SUB);
    }

    fn invoke_subi(&mut self, i: i32) -> Result<(), RubyError> {
        use std::ops::Sub;
        invoke_op_i!(self, iseq, i, sub, IdentId::_SUB);
    }

    fn invoke_mul(&mut self) -> Result<(), RubyError> {
        use std::ops::Mul;
        invoke_op!(self, mul, IdentId::_MUL);
    }

    fn invoke_div(&mut self) -> Result<(), RubyError> {
        use std::ops::Div;
        if self.exec_stack[self.stack_len() - 1].is_zero() {
            return Err(RubyError::zero_div("Divided by zero."));
        }
        invoke_op!(self, div, IdentId::_DIV);
    }

    fn invoke_rem(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        fn rem_floorf64(self_: f64, other: f64) -> Result<f64, RubyError> {
            if other == 0.0 {
                return Err(RubyError::zero_div("Divided by zero."));
            }
            let res = if self_ > 0.0 && other < 0.0 {
                ((self_ - 1.0) % other) + other + 1.0
            } else if self_ < 0.0 && other > 0.0 {
                ((self_ + 1.0) % other) + other - 1.0
            } else {
                self_ % other
            };
            Ok(res)
        }
        use divrem::*;
        let val = match (lhs.unpack(), rhs.unpack()) {
            (RV::Integer(lhs), RV::Integer(rhs)) => {
                if rhs == 0 {
                    return Err(RubyError::zero_div("Divided by zero."));
                }
                Value::integer(lhs.rem_floor(rhs))
            }
            (RV::Integer(lhs), RV::Float(rhs)) => Value::float(rem_floorf64(lhs as f64, rhs)?),
            (RV::Float(lhs), RV::Integer(rhs)) => Value::float(rem_floorf64(lhs, rhs as f64)?),
            (RV::Float(lhs), RV::Float(rhs)) => Value::float(rem_floorf64(lhs, rhs)?),
            (_, _) => {
                return self.fallback_for_binop(IdentId::_REM, lhs, rhs);
            }
        };
        self.stack_push(val);
        Ok(())
    }

    fn invoke_exp(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
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
        self.stack_push(val);
        Ok(())
    }

    fn invoke_neg(&mut self, lhs: Value) -> Result<(), RubyError> {
        let val = match lhs.unpack() {
            RV::Integer(i) => Value::integer(-i),
            RV::Float(f) => Value::float(-f),
            _ => return self.invoke_send0(IdentId::get_id("-@"), lhs),
        };
        self.stack_push(val);
        Ok(())
    }

    fn invoke_shl(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            let val = Value::integer(lhs.as_packed_fixnum() << rhs.as_packed_fixnum());
            self.stack_push(val);
            Ok(())
        } else if let Some(mut ainfo) = lhs.as_array() {
            ainfo.push(rhs);
            self.stack_push(lhs);
            Ok(())
        } else {
            self.fallback_for_binop(IdentId::_SHL, lhs, rhs)
        }
    }

    fn invoke_shr(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            let val = Value::integer(lhs.as_packed_fixnum() >> rhs.as_packed_fixnum());
            self.stack_push(val);
            Ok(())
        } else {
            self.fallback_for_binop(IdentId::_SHR, lhs, rhs)
        }
    }

    fn invoke_bitand(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        let val = if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            Value::integer(lhs.as_packed_fixnum() & rhs.as_packed_fixnum())
        } else {
            match (lhs.unpack(), rhs.unpack()) {
                (RV::True, _) => Value::bool(rhs.to_bool()),
                (RV::False, _) => Value::false_val(),
                (RV::Integer(lhs), RV::Integer(rhs)) => Value::integer(lhs & rhs),
                (RV::Nil, _) => Value::false_val(),
                (_, _) => {
                    return self.fallback_for_binop(IdentId::get_id("&"), lhs, rhs);
                }
            }
        };
        self.stack_push(val);
        Ok(())
    }

    fn invoke_bitor(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        let val = if lhs.is_packed_fixnum() && rhs.is_packed_fixnum() {
            Value::integer(lhs.as_packed_fixnum() | rhs.as_packed_fixnum())
        } else {
            match (lhs.unpack(), rhs.unpack()) {
                (RV::True, _) => Value::true_val(),
                (RV::False, _) | (RV::Nil, _) => Value::bool(rhs.to_bool()),
                (RV::Integer(lhs), RV::Integer(rhs)) => Value::integer(lhs | rhs),
                (_, _) => {
                    return self.fallback_for_binop(IdentId::get_id("|"), lhs, rhs);
                }
            }
        };
        self.stack_push(val);
        Ok(())
    }

    fn eval_bitxor(&mut self, rhs: Value, lhs: Value) -> VMResult {
        match (lhs.unpack(), rhs.unpack()) {
            (RV::True, _) => Ok(Value::bool(!rhs.to_bool())),
            (RV::False, _) | (RV::Nil, _) => Ok(Value::bool(rhs.to_bool())),
            (RV::Integer(lhs), RV::Integer(rhs)) => Ok(Value::integer(lhs ^ rhs)),
            (_, _) => {
                self.fallback_for_binop(IdentId::get_id("^"), lhs, rhs)?;
                Ok(self.stack_pop())
            }
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
    ($vm:ident, $op:ident, $id:expr) => {{
        let len = $vm.stack_len();
        let lhs = unsafe { *$vm.exec_stack.get_unchecked(len - 2) };
        let rhs = unsafe { *$vm.exec_stack.get_unchecked(len - 1) };
        $vm.set_stack_len(len - 2);
        eval_cmp2!($vm, rhs, lhs, $op, $id)
    }};
}

macro_rules! eval_cmp2 {
    ($vm:ident, $rhs:expr, $lhs:expr, $op:ident, $id:expr) => {{
        if $lhs.is_packed_fixnum() {
            if $rhs.is_packed_fixnum() {
                let lhs = $lhs.as_packed_fixnum();
                let rhs = $rhs.as_packed_fixnum();
                Ok(lhs.$op(&rhs))
            } else if $rhs.is_packed_num() {
                let lhs = $lhs.as_packed_fixnum();
                let rhs = $rhs.as_packed_flonum();
                Ok((lhs as f64).$op(&rhs))
            } else {
                $vm.fallback_for_binop($id, $lhs, $rhs)?;
                Ok($vm.stack_pop().to_bool())
            }
        } else if $lhs.is_packed_num() {
            if $rhs.is_packed_fixnum() {
                let lhs = $lhs.as_packed_flonum();
                let rhs = $rhs.as_packed_fixnum();
                Ok(lhs.$op(&(rhs as f64)))
            } else if $rhs.is_packed_num() {
                let lhs = $lhs.as_packed_flonum();
                let rhs = $rhs.as_packed_flonum();
                Ok(lhs.$op(&rhs))
            } else {
                $vm.fallback_for_binop($id, $lhs, $rhs)?;
                Ok($vm.stack_pop().to_bool())
            }
        } else {
            match ($lhs.unpack(), $rhs.unpack()) {
                (RV::Integer(lhs), RV::Integer(rhs)) => Ok(lhs.$op(&rhs)),
                (RV::Float(lhs), RV::Integer(rhs)) => Ok(lhs.$op(&(rhs as f64))),
                (RV::Integer(lhs), RV::Float(rhs)) => Ok((lhs as f64).$op(&rhs)),
                (RV::Float(lhs), RV::Float(rhs)) => Ok(lhs.$op(&rhs)),
                (_, _) => {
                    $vm.fallback_for_binop($id, $lhs, $rhs)?;
                    Ok($vm.stack_pop().to_bool())
                }
            }
        }
    }};
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
                    $vm.fallback_for_binop($id, $lhs, Value::integer($i as i64))?;
                    Ok($vm.stack_pop().to_bool())
                }
            }
        }
    };
}

impl VM {
    fn invoke_eq(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        let b = self.eval_eq2(rhs, lhs)?;
        self.stack_push(Value::bool(b));
        Ok(())
    }

    fn invoke_teq(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        let b = match lhs.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Module(_) => {
                    return self.fallback_for_binop(IdentId::_TEQ, lhs, rhs);
                }
                ObjKind::Regexp(re) => {
                    let given = match rhs.unpack() {
                        RV::Symbol(sym) => IdentId::get_name(sym),
                        RV::Object(_) => match rhs.as_string() {
                            Some(s) => s.to_owned(),
                            None => {
                                self.stack_push(Value::false_val());
                                return Ok(());
                            }
                        },
                        _ => {
                            self.stack_push(Value::false_val());
                            return Ok(());
                        }
                    };
                    RegexpInfo::find_one(self, &*re, &given)?.is_some()
                }
                _ => return self.invoke_eq(lhs, rhs),
            },
            None => return self.invoke_eq(lhs, rhs),
        };
        self.stack_push(Value::bool(b));
        Ok(())
    }

    pub fn eval_teq(&mut self, rhs: Value, lhs: Value) -> Result<bool, RubyError> {
        match lhs.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Module(_) => {
                    self.fallback_for_binop(IdentId::_TEQ, lhs, rhs)?;
                    Ok(self.stack_pop().to_bool())
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
                _ => Ok(self.eval_eq2(lhs, rhs)?),
            },
            None => Ok(self.eval_eq2(lhs, rhs)?),
        }
    }

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

    pub fn eval_eq2(&mut self, rhs: Value, lhs: Value) -> Result<bool, RubyError> {
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
            (ObjKind::Invalid, _) | (_, ObjKind::Invalid) => {
                panic!("Invalid rvalue. (maybe GC problem) {:?}", lhs.rvalue())
            }
            (_, _) => {
                let val = match self.fallback_for_binop(IdentId::_EQ, lhs, rhs) {
                    Ok(()) => self.stack_pop(),
                    _ => return Ok(false),
                };
                Ok(val.to_bool())
            }
        }
    }

    fn eval_eq(&mut self) -> Result<bool, RubyError> {
        let len = self.stack_len();
        let lhs = unsafe { *self.exec_stack.get_unchecked(len - 2) };
        let rhs = unsafe { *self.exec_stack.get_unchecked(len - 1) };
        self.set_stack_len(len - 2);
        self.eval_eq2(rhs, lhs)
    }

    fn eval_ne(&mut self) -> Result<bool, RubyError> {
        Ok(!self.eval_eq()?)
    }

    fn eval_ge(&mut self) -> Result<bool, RubyError> {
        eval_cmp!(self, ge, IdentId::_GE)
    }

    fn eval_gt(&mut self) -> Result<bool, RubyError> {
        eval_cmp!(self, gt, IdentId::_GT)
    }

    pub fn eval_gt2(&mut self, rhs: Value, lhs: Value) -> Result<bool, RubyError> {
        eval_cmp2!(self, rhs, lhs, gt, IdentId::_GT)
    }

    fn eval_le(&mut self) -> Result<bool, RubyError> {
        eval_cmp!(self, le, IdentId::_LE)
    }

    fn eval_lt(&mut self) -> Result<bool, RubyError> {
        eval_cmp!(self, lt, IdentId::_LT)
    }

    fn eval_eqi(&mut self, lhs: Value, i: i32) -> Result<bool, RubyError> {
        let res = if lhs.is_packed_fixnum() {
            lhs.as_packed_fixnum() == i as i64
        } else if lhs.is_packed_num() {
            lhs.as_packed_flonum() == i as f64
        } else {
            match lhs.unpack() {
                RV::Integer(lhs) => lhs == i as i64,
                RV::Float(lhs) => lhs == i as f64,
                _ => {
                    self.fallback_for_binop(IdentId::_EQ, lhs, Value::integer(i as i64))?;
                    return Ok(self.stack_pop().to_bool());
                }
            }
        };

        Ok(res)
    }
    fn eval_nei(&mut self, lhs: Value, i: i32) -> Result<bool, RubyError> {
        Ok(!self.eval_eqi(lhs, i)?)
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
                self.fallback_for_binop(IdentId::_CMP, lhs, rhs)?;
                return Ok(self.stack_pop());
            }
        };
        match res {
            Some(ord) => Ok(Value::integer(ord as i64)),
            None => Ok(Value::nil()),
        }
    }

    fn set_index(&mut self) -> Result<(), RubyError> {
        let val = self.stack_pop();
        let idx = self.stack_pop();
        let mut receiver = self.stack_pop();

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
        let mut receiver = self.stack_pop();
        let val = self.stack_pop();
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

    fn invoke_get_index(&mut self, receiver: Value, idx: Value) -> Result<(), RubyError> {
        let val = match receiver.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Array(aref) => aref.get_elem1(idx)?,
                ObjKind::Hash(href) => match href.get(&idx) {
                    Some(val) => *val,
                    None => Value::nil(),
                },
                _ => return self.invoke_send1(IdentId::_INDEX, receiver, idx),
            },
            _ => return self.fallback_for_binop(IdentId::_INDEX, receiver, idx),
        };
        self.stack_push(val);
        Ok(())
    }

    fn invoke_get_index_imm(&mut self, receiver: Value, idx: u32) -> Result<(), RubyError> {
        let val = match receiver.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Array(aref) => aref.get_elem_imm(idx),
                ObjKind::Hash(href) => match href.get(&Value::integer(idx as i64)) {
                    Some(val) => *val,
                    None => Value::nil(),
                },
                ObjKind::Method(mref) if mref.receiver.is_some() => {
                    let args = Args::new1(Value::integer(idx as i64));
                    return self.exec_method(mref.method, mref.receiver.unwrap(), &args);
                }
                _ => {
                    return self.invoke_send1(
                        IdentId::_INDEX,
                        receiver,
                        Value::integer(idx as i64),
                    );
                }
            },
            None if receiver.is_packed_fixnum() => {
                let i = receiver.as_packed_fixnum();
                let val = if 63 < idx { 0 } else { (i >> idx) & 1 };
                Value::integer(val)
            }
            _ => {
                return self.fallback_for_binop(
                    IdentId::_INDEX,
                    receiver,
                    Value::integer(idx as i64),
                );
            }
        };
        self.stack_push(val);
        Ok(())
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
                    self.send0(id, val)?.as_string().unwrap().to_string()
                }
            },
        };
        Ok(s)
    }
}

impl VM {
    pub fn eval_send(&mut self, method_id: IdentId, receiver: Value, args: &Args) -> VMResult {
        self.send(method_id, receiver, args)
    }

    /// Evaluate the method with given `self_val`, `args` and no outer context.
    pub fn eval_method(
        &mut self,
        method: MethodId,
        self_val: impl Into<Value>,
        args: &Args,
    ) -> VMResult {
        let self_val = self_val.into();
        self.exec_func(method, self_val, None, args)?;
        Ok(self.stack_pop())
    }

    /// Evaluate the method with given `self_val`, `args` and no outer context.
    pub fn eval_method_with_outer(
        &mut self,
        method: MethodId,
        self_val: impl Into<Value>,
        outer: ContextRef,
        args: &Args,
    ) -> VMResult {
        let self_val = self_val.into();
        self.exec_func(method, self_val, Some(outer), args)?;
        Ok(self.stack_pop())
    }

    pub fn eval_binding(&mut self, path: String, code: String, mut ctx: ContextRef) -> VMResult {
        let method = self.parse_program_binding(path, code, ctx)?;
        ctx.iseq_ref = Some(method.as_iseq());
        self.run_context(ctx)?;
        Ok(self.stack_pop())
    }

    /// Invoke the method with given `self_val` and `args`.
    pub fn exec_method(
        &mut self,
        method: MethodId,
        self_val: impl Into<Value>,
        args: &Args,
    ) -> Result<(), RubyError> {
        let self_val = self_val.into();
        self.exec_func(method, self_val, None, args)
    }

    /// Evaluate the block with self_val of outer context, and given `args`.
    pub fn eval_block(&mut self, block: &Block, args: &Args) -> VMResult {
        match block {
            Block::Block(method, outer) => {
                let outer = outer.get_current();
                self.exec_func(*method, outer.self_value, Some(outer), args)?;
            }
            Block::Proc(proc) => self.exec_proc(*proc, args)?,
        }
        Ok(self.stack_pop())
    }

    /// Evaluate the block with self_val of outer context, and given `args`.
    pub fn eval_block_each1(
        &mut self,
        block: &Block,
        iter: impl Iterator<Item = Value>,
        default: Value,
    ) -> VMResult {
        let mut args = Args::new1(Value::uninitialized());
        match block {
            Block::Block(method, outer) => {
                let self_value = outer.self_value;
                use MethodInfo::*;
                match MethodRepo::get(*method) {
                    BuiltinFunc { func, name, .. } => {
                        for v in iter {
                            args[0] = v;
                            self.exec_native(&func, *method, name, self_value, &args)?;
                        }
                    }
                    RubyFunc { iseq } => {
                        let len = self.stack_len();
                        if iseq.opt_flag {
                            for v in iter {
                                args[0] = v;
                                let context = ContextRef::from_opt_block(
                                    self,
                                    self_value,
                                    iseq,
                                    &args,
                                    outer.get_current(),
                                );
                                match self.run_context(context) {
                                    Err(err) => match err.kind {
                                        RubyErrorKind::BlockReturn => {
                                            return Ok(self.globals.error_register)
                                        }
                                        _ => return Err(err),
                                    },
                                    Ok(()) => {}
                                };
                                self.set_stack_len(len);
                            }
                        } else {
                            for v in iter {
                                args[0] = v;
                                let context = ContextRef::from_noopt(
                                    self,
                                    self_value,
                                    iseq,
                                    &args,
                                    outer.get_current(),
                                )?;
                                match self.run_context(context) {
                                    Err(err) => match err.kind {
                                        RubyErrorKind::BlockReturn => {
                                            return Ok(self.globals.error_register)
                                        }
                                        _ => return Err(err),
                                    },
                                    Ok(()) => {}
                                };
                                self.set_stack_len(len);
                            }
                        }
                    }
                    _ => unreachable!(),
                };
            }
            Block::Proc(proc) => {
                let pinfo = proc.as_proc().unwrap();
                let self_value = pinfo.self_val;
                let iseq = pinfo.iseq;
                let outer = pinfo.outer;
                for v in iter {
                    args[0] = v;
                    let context = ContextRef::from(self, self_value, iseq, &args, outer)?;
                    match self.run_context(context) {
                        Err(err) => match err.kind {
                            RubyErrorKind::BlockReturn => return Ok(self.globals.error_register),
                            _ => return Err(err),
                        },
                        Ok(()) => {}
                    };
                    self.stack_pop();
                }
            }
        };
        Ok(default)
    }

    /// Evaluate the block with given `self_val` and `args`.
    pub fn eval_block_self(
        &mut self,
        block: &Block,
        self_value: impl Into<Value>,
        args: &Args,
    ) -> VMResult {
        let self_value = self_value.into();
        match block {
            Block::Block(method, outer) => {
                let outer = outer.get_current();
                self.exec_func(*method, self_value, Some(outer), args)?
            }
            Block::Proc(proc) => {
                let pref = match proc.as_proc() {
                    Some(proc) => proc,
                    None => return Err(RubyError::internal("Illegal proc.")),
                };
                let context = ContextRef::from(self, self_value, pref.iseq, args, pref.outer)?;
                self.run_context(context)?
            }
        }
        Ok(self.stack_pop())
    }

    /// Execute the Proc object with given `args`, and push the returned value on the stack.
    pub fn exec_proc(&mut self, proc: Value, args: &Args) -> Result<(), RubyError> {
        let pinfo = proc.as_proc().unwrap();
        let context = ContextRef::from(self, pinfo.self_val, pinfo.iseq, args, pinfo.outer)?;
        self.run_context(context)
    }

    /// Invoke the Proc object with given `args`.
    pub fn invoke_proc(&mut self, proc: Value, args: &Args) -> Result<VMResKind, RubyError> {
        let pinfo = proc.as_proc().unwrap();
        let context = ContextRef::from(self, pinfo.self_val, pinfo.iseq, args, pinfo.outer)?;
        self.invoke_new_context(context);
        Ok(VMResKind::Invoke)
    }

    /// Invoke the method with given `self_val`, `outer` context, and `args`, and push the returned value on the stack.
    fn exec_func(
        &mut self,
        method: MethodId,
        self_value: impl Into<Value>,
        outer: Option<ContextRef>,
        args: &Args,
    ) -> Result<(), RubyError> {
        let self_val = self_value.into();
        use MethodInfo::*;
        let val = match MethodRepo::get(method) {
            BuiltinFunc { func, name, .. } => {
                self.exec_native(&func, method, name, self_val, args)?
            }
            AttrReader { id } => {
                args.check_args_num(0)?;
                self.exec_getter(id, self_val)?
            }
            AttrWriter { id } => {
                args.check_args_num(1)?;
                self.exec_setter(id, self_val, args[0])?
            }
            RubyFunc { iseq } => {
                let context = ContextRef::from(self, self_val, iseq, args, outer)?;
                return self.run_context(context);
            }
            _ => unreachable!(),
        };
        self.stack_push(val);
        Ok(())
    }

    /// Invoke the method with given `self_val`, `outer` context, and `args`, and push the returned value on the stack.
    fn invoke_func(
        &mut self,
        method: MethodId,
        self_val: impl Into<Value>,
        outer: Option<ContextRef>,
        args: &Args,
        use_value: bool,
    ) -> Result<VMResKind, RubyError> {
        let self_val = self_val.into();
        use MethodInfo::*;
        let val = match MethodRepo::get(method) {
            BuiltinFunc { func, name, .. } => {
                self.exec_native(&func, method, name, self_val, args)?
            }
            AttrReader { id } => {
                args.check_args_num(0)?;
                self.exec_getter(id, self_val)?
            }
            AttrWriter { id } => {
                args.check_args_num(1)?;
                self.exec_setter(id, self_val, args[0])?
            }
            RubyFunc { iseq } => {
                let mut context = ContextRef::from(self, self_val, iseq, args, outer)?;
                context.use_value = use_value;
                self.invoke_new_context(context);
                return Ok(VMResKind::Invoke);
            }
            _ => unreachable!(),
        };
        if use_value {
            self.stack_push(val);
        }
        Ok(VMResKind::Return)
    }

    // helper methods

    /// Invoke the method defined by Rust fn and push the returned value on the stack.
    fn exec_native(
        &mut self,
        func: &BuiltinFunc,
        _method_id: MethodId,
        _name: IdentId,
        self_value: Value,
        args: &Args,
    ) -> Result<Value, RubyError> {
        #[cfg(feature = "perf")]
        self.globals.perf.get_perf(Perf::EXTERN);

        #[cfg(any(feature = "trace", feature = "trace-func"))]
        if self.globals.startup_flag {
            println!("+++> BuiltinFunc {:?}", _name);
        }

        #[cfg(feature = "perf-method")]
        MethodRepo::inc_counter(_method_id);

        let len = self.temp_stack.len();
        self.temp_push(self_value);
        self.temp_push_args(args);
        self.exec_native = true;
        let res = func(self, self_value, args);
        self.exec_native = false;
        self.temp_stack.truncate(len);

        #[cfg(any(feature = "trace", feature = "trace-func"))]
        if self.globals.startup_flag {
            println!("<+++ {:?}", res);
        }

        res
    }

    /// Invoke attr_getter and return the value.
    fn exec_getter(&mut self, id: IdentId, self_val: Value) -> Result<Value, RubyError> {
        let val = match self_val.as_rvalue() {
            Some(oref) => oref.get_var(id).unwrap_or_default(),
            None => Value::nil(),
        };
        Ok(val)
    }

    /// Invoke attr_setter and return the value.
    fn exec_setter(
        &mut self,
        id: IdentId,
        mut self_val: Value,
        val: Value,
    ) -> Result<Value, RubyError> {
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
            Block::Proc(proc) => proc.dup(),
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
            Block::Proc(proc) => Ok(proc.dup()),
        }
    }

    pub fn create_enum_info(&mut self, info: EnumInfo) -> Box<FiberContext> {
        let fiber_vm = self.create_fiber();
        FiberContext::new_enumerator(fiber_vm, info)
    }

    pub fn dup_enum(&mut self, eref: &FiberContext) -> Box<FiberContext> {
        match &eref.kind {
            FiberKind::Enum(info) => self.create_enum_info((**info).clone()),
            _ => unreachable!(),
        }
    }

    pub fn create_enumerator(
        &mut self,
        method: IdentId,
        receiver: Value,
        mut args: Args,
    ) -> VMResult {
        args.block = Some(self.new_block(METHOD_ENUM));
        let fiber = self.create_enum_info(EnumInfo {
            method,
            receiver,
            args,
        });
        Ok(Value::enumerator(fiber))
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
