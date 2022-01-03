use crate::coroutine::FiberHandle;
use crate::*;
use fancy_regex::Captures;
pub use frame::arg_handler::*;
pub use frame::*;
use ruby_stack::*;
use std::ops::Index;

#[cfg(feature = "perf")]
use super::perf::*;
use std::path::PathBuf;
mod constants;
mod fiber;
pub mod frame;
mod loader;
mod method;
mod ops;
mod opt_core;
pub mod repl;
mod ruby_stack;

pub type ValueTable = FxHashMap<IdentId, Value>;
pub type VMResult = Result<Value, RubyError>;
pub type InvokeResult = Result<VMResKind, RubyError>;

#[derive(Debug)]
pub struct VM {
    // Global info
    pub globals: GlobalsRef,
    // VM state
    stack: RubyStack,
    temp_stack: Vec<Value>,
    /// program counter
    pc: ISeqPtr,
    /// local frame pointer
    pub lfp: LocalFrame,
    /// control frame pointer
    pub cfp: ControlFrame,
    /// current iseq
    pub iseq: ISeqRef,
    pub handle: Option<FiberHandle>,
    sp_last_match: Option<String>,   // $&        : Regexp.last_match(0)
    sp_post_match: Option<String>,   // $'        : Regexp.post_match
    sp_matches: Vec<Option<String>>, // $1 ... $n : Regexp.last_match(n)
    pub gc_count: usize,
}

pub type VMRef = Ref<VM>;

pub enum VMResKind {
    Return(Value),
    Invoke,
}

impl VMResKind {
    #[inline]
    pub fn handle(self, vm: &mut VM) -> VMResult {
        match self {
            VMResKind::Return(v) => Ok(v),
            VMResKind::Invoke => vm.run_loop(),
        }
    }
}

impl Index<usize> for VM {
    type Output = Value;
    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        &self.lfp[index]
    }
}

// API's
impl GC<RValue> for VM {
    fn mark(&self, alloc: &mut Allocator<RValue>) {
        self.stack.iter().for_each(|v| v.mark(alloc));
        self.temp_stack.iter().for_each(|v| v.mark(alloc));
        let mut cfp = Some(self.cfp);
        while let Some(f) = cfp {
            if f.is_ruby_func() {
                let ep = f.ep();
                if self.check_boundary(ep.as_ptr()).is_none() {
                    f.locals().iter().for_each(|v| {
                        v.mark(alloc);
                    });
                }
                if let Some(d) = ep.outer() {
                    d.mark(alloc)
                }
            };
            cfp = f.prev();
        }
    }
}

impl VM {
    pub fn new() -> VMRef {
        let mut globals = GlobalsRef::new(Globals::new());
        let vm = VM {
            globals,
            stack: RubyStack::new(),
            temp_stack: vec![],
            pc: ISeqPtr::default(),
            lfp: LocalFrame::default(),
            cfp: ControlFrame::default(),
            iseq: ISeqRef::default(),
            handle: None,
            sp_last_match: None,
            sp_post_match: None,
            sp_matches: vec![],
            gc_count: 0,
        };
        let mut vm = VMRef::new(vm);
        globals.main_fiber = Some(vm);
        vm.init_frame();

        if !vm.globals.startup_flag {
            let method = vm.parse_program("", "".to_string()).unwrap();
            let dummy_info = vm.globals.methods[method].to_owned();
            vm.globals.methods.update(FnId::default(), dummy_info);

            let load_path = include_str!(concat!(env!("OUT_DIR"), "/libpath.rb"));
            if let Ok(val) = vm.run("(startup)", load_path.to_string()) {
                vm.globals.set_global_var_by_str("$:", val)
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
        }

        #[cfg(feature = "perf")]
        {
            vm.globals.perf = Perf::new();
        }

        #[cfg(feature = "perf-method")]
        {
            vm.globals.methods.clear_stats();
            vm.globals.clear_const_cache();
        }

        vm
    }

    pub(crate) fn create_fiber(&mut self) -> Self {
        let mut vm = VM {
            globals: self.globals,
            temp_stack: vec![],
            stack: RubyStack::new(),
            pc: ISeqPtr::default(),
            lfp: LocalFrame::default(),
            cfp: ControlFrame::default(),
            iseq: ISeqRef::default(),
            handle: None,
            sp_last_match: None,
            sp_post_match: None,
            sp_matches: vec![],
            gc_count: 0,
        };
        vm.init_frame();
        vm
    }

    fn kind(&self) -> ISeqKind {
        self.iseq.kind
    }

    #[inline]
    fn pc_offset(&self) -> ISeqPos {
        let offset = self.pc - self.iseq.iseq.as_ptr();
        assert!(offset >= 0);
        ISeqPos::from(offset as usize)
    }

    #[inline(always)]
    fn set_pc(&mut self, pos: ISeqPos) {
        self.pc = self.iseq.iseq.as_ptr() + pos.into_usize();
    }

    #[inline(always)]
    pub(crate) fn stack_push(&mut self, val: Value) {
        self.stack.push(val)
    }

    #[inline(always)]
    pub(crate) fn stack_pop(&mut self) -> Value {
        self.stack.pop()
    }

    #[inline(always)]
    pub(crate) fn stack_pop2(&mut self) -> (Value, Value) {
        self.stack.pop2()
    }

    #[inline(always)]
    pub(crate) fn stack_len(&self) -> usize {
        self.stack.len()
    }

    #[inline(always)]
    pub(crate) fn sp(&self) -> StackPtr {
        self.stack.sp
    }

    pub(crate) fn check_boundary(&self, p: *mut Value) -> Option<usize> {
        self.stack.check_boundary(p)
    }

    pub(crate) fn stack_push_args(&mut self, args: &Args) -> Args2 {
        self.stack.extend_from_slice(args);
        Args2::from(args)
    }

    // handling arguments
    pub(crate) fn args(&self) -> &[Value] {
        debug_assert!(!self.cfp.is_ruby_func());
        let len = self.cfp.flag_len();
        unsafe { std::slice::from_raw_parts((self.cfp.get_prev_sp() + 1).as_ptr(), len) }
    }

    pub(crate) fn args_range(&self) -> (StackPtr, usize) {
        let local_len = self.cfp.flag_len();
        let cfp = self.cfp.as_sp();
        (cfp - local_len - 1, local_len)
    }

    #[inline(always)]
    pub(crate) fn self_value(&self) -> Value {
        self.cfp.self_value()
    }

    /// Push an object to the temporary area.
    pub(crate) fn temp_push(&mut self, v: Value) {
        self.temp_stack.push(v);
    }

    pub(crate) fn temp_pop_vec(&mut self, len: usize) -> Vec<Value> {
        self.temp_stack.split_off(len)
    }

    pub(crate) fn temp_len(&self) -> usize {
        self.temp_stack.len()
    }

    /// Push objects to the temporary area.
    pub(crate) fn temp_extend_from_slice(&mut self, slice: &[Value]) {
        self.temp_stack.extend_from_slice(slice);
    }

    #[inline]
    pub(super) fn get_dyn_local(&self, outer: u32) -> LocalFrame {
        self.get_outer_frame(outer).get_lfp()
    }

    #[cfg(not(tarpaulin_include))]
    pub fn clear(&mut self) {
        self.stack.sp = self.stack.bottom() + frame::CONT_FRAME_LEN + frame::RUBY_FRAME_LEN;
    }

    /// Get Class of current class context.
    pub(crate) fn current_class(&self) -> Module {
        self.self_value().get_class_if_object()
    }

    pub(crate) fn parse_program(
        &mut self,
        path: impl Into<PathBuf>,
        code: String,
    ) -> Result<FnId, RubyError> {
        #[cfg(feature = "perf")]
        self.globals.perf.set_prev_inst(Perf::INVALID);

        Codegen::gen_toplevel(
            &mut self.globals,
            ContextKind::Method(None),
            path,
            code,
            None,
        )
    }

    pub(crate) fn parse_program_eval(
        &mut self,
        path: impl Into<PathBuf>,
        code: String,
    ) -> Result<FnId, RubyError> {
        let extern_context = self.move_cfp_to_heap(self.caller_cfp());
        #[cfg(feature = "perf")]
        self.globals.perf.set_prev_inst(Perf::INVALID);

        Codegen::gen_toplevel_binding(
            &mut self.globals,
            ContextKind::Eval,
            path,
            code,
            None,
            Some(extern_context),
        )
    }

    pub(crate) fn parse_program_binding(
        &mut self,
        path: impl Into<PathBuf>,
        code: String,
        frame: EnvFrame,
    ) -> Result<FnId, RubyError> {
        #[cfg(feature = "perf")]
        self.globals.perf.set_prev_inst(Perf::INVALID);

        let path = path.into();
        Codegen::gen_toplevel_binding(
            &mut self.globals,
            ContextKind::Eval,
            path,
            code,
            Some(frame),
            frame.outer(),
        )
    }

    pub fn run(&mut self, path: impl Into<PathBuf>, program: String) -> VMResult {
        let prev_len = self.stack_len();
        let method = self.parse_program(path, program)?;
        let self_value = self.globals.main_object;
        let val = self.eval_method0(method, self_value)?;
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
}

impl VM {
    #[inline(always)]
    pub fn checked_gc(&mut self) {
        #[cfg(feature = "perf")]
        self.globals.perf.get_perf(Perf::GC);
        #[cfg(not(feature = "gc-stress"))]
        {
            self.gc_count += 1;
            if self.gc_count & 0b111 != 0 {
                return;
            }
        }
        ALLOC.with(|m| m.borrow_mut().check_gc(&*self.globals));
    }

    #[inline(always)]
    pub fn gc(&mut self) {
        ALLOC.with(|m| m.borrow_mut().gc(&*self.globals));
    }

    #[inline]
    fn jmp_cond(&mut self, cond: bool) {
        let disp = self.pc.read_disp();
        if !cond {
            self.pc += disp;
        }
    }

    /// VM main loop.
    ///
    /// This fn is called when a Ruby method/block is 'call'ed.
    /// That means VM main loop is called recursively.
    ///
    /// Be aware that this fn does not restore vm.iseq and vm.pc.
    pub(crate) fn run_loop(&mut self) -> VMResult {
        let mut invoke_count = 0usize;
        debug_assert!(self.is_ruby_func());
        loop {
            match self.run_context_main(&mut invoke_count) {
                Ok(val) => {
                    // 'Returned from 'call'ed method/block.
                    self.unwind_frame();
                    #[cfg(feature = "trace")]
                    if !self.discard_val() {
                        eprintln!("<+++ Ok({:?})", val);
                    } else {
                        eprintln!("<+++ Ok");
                    }
                    return Ok(val);
                }
                Err(mut err) => {
                    match err.kind {
                        RubyErrorKind::BlockReturn => {
                            #[cfg(feature = "trace")]
                            eprintln!("<+++ BlockReturn({:?})", self.globals.val);
                            return Err(err);
                        }
                        RubyErrorKind::MethodReturn => {
                            // In the case of MethodReturn, returned value is to be saved in vm.globals.val.
                            loop {
                                if invoke_count == 0 {
                                    #[cfg(feature = "trace")]
                                    eprintln!("<+++ MethodReturn({:?})", self.globals.val);
                                    self.unwind_frame();
                                    return Err(err);
                                };
                                self.unwind_frame();
                                #[cfg(feature = "trace")]
                                eprintln!("<--- MethodReturn({:?})", self.globals.val);
                                invoke_count -= 1;
                                if self.iseq.is_method() {
                                    break;
                                }
                            }
                            let val = self.globals.val;
                            self.stack_push(val);
                            continue;
                        }
                        _ => {}
                    }
                    // Handle Exception.
                    loop {
                        let cur_pc = self.pc_offset();
                        let iseq = self.iseq;
                        if err.info.is_empty() || iseq.kind != ISeqKind::Block {
                            err.info
                                .push((iseq.source_info.clone(), iseq.get_loc(cur_pc)));
                        }
                        if let RubyErrorKind::Internal(msg) = &err.kind {
                            self.globals.show_err(&err);
                            err.show_all_loc();
                            unreachable!("{}", msg);
                        };
                        let catch = iseq.exception_table.iter().find(|x| x.include(cur_pc));
                        if let Some(entry) = catch {
                            // Exception raised inside of begin-end with rescue clauses.
                            self.set_pc(entry.dest);
                            match entry.ty {
                                ExceptionType::Rescue => self.clear_stack(),
                                _ => {}
                            };
                            let val = self
                                .globals
                                .from_exception(&err)
                                .unwrap_or(self.globals.val);
                            #[cfg(feature = "trace")]
                            eprintln!(":::: Exception({:?})", val);
                            self.stack_push(val);
                            break;
                        } else {
                            // Exception raised outside of begin-end.
                            if invoke_count == 0 {
                                self.unwind_frame();
                                #[cfg(feature = "trace")]
                                eprintln!("<+++ {:?}", err.kind);
                                return Err(err);
                            }
                            self.unwind_frame();
                            invoke_count -= 1;
                            #[cfg(feature = "trace")]
                            eprintln!("<--- {:?}", err.kind);
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
            let val = self.globals.val;
            match val.if_exception() {
                Some(err) => self.globals.show_err(err),
                None => eprintln!("{:?}", val),
            }
        } else {
            self.globals.show_err(err);
        }
    }

    /// Get class list in the current context.
    ///
    /// At first, this method searches the class list of outer context,
    /// and adds a class given as an argument `new_class` on the top of the list.
    /// return None in top-level.
    fn get_class_defined(&self, new_module: impl Into<Module>) -> Vec<Module> {
        let mut cfp = Some(self.cfp);
        let mut v = vec![new_module.into()];
        while let Some(f) = cfp {
            if f.is_ruby_func() {
                let iseq = f.ep().iseq();
                if iseq.is_classdef() {
                    v.push(Module::new(f.self_value()));
                }
            }
            cfp = f.prev();
        }
        v.reverse();
        v
    }
}

// Handling global varables.
impl VM {
    pub(crate) fn get_global_var(&self, id: IdentId) -> Option<Value> {
        self.globals.get_global_var(id)
    }

    pub fn set_global_var(&mut self, id: IdentId, val: Value) {
        self.globals.set_global_var(id, val);
    }
}

// Handling special variables.
impl VM {
    pub(crate) fn get_special_var(&self, id: u32) -> Value {
        if id == 0 {
            self.sp_last_match
                .as_ref()
                .map(Value::string)
                .unwrap_or_default()
        } else if id == 1 {
            self.sp_post_match
                .as_ref()
                .map(Value::string)
                .unwrap_or_default()
        } else if id >= 100 {
            self.get_special_matches(id as usize - 100)
        } else {
            unreachable!()
        }
    }

    pub(crate) fn set_special_var(&self, _id: u32, _val: Value) -> Result<(), RubyError> {
        unreachable!()
    }

    /// Save captured strings to special variables.
    /// $n (n:0,1,2,3...) <- The string which matched with nth parenthesis in the last successful match.
    /// $& <- The string which matched successfully at last.
    /// $' <- The string after $&.
    pub(crate) fn get_captures(&mut self, captures: &Captures, given: &str) {
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

    pub(crate) fn get_special_matches(&self, nth: usize) -> Value {
        match self.sp_matches.get(nth - 1) {
            None => Value::nil(),
            Some(s) => s.as_ref().map(Value::string).unwrap_or_default(),
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
            if !module.is_module()
                && exceptions.iter().any(|x| {
                    if let Some(ary) = x.as_splat() {
                        ary.as_array()
                            .unwrap()
                            .iter()
                            .any(|elem| elem.id() == module.id())
                    } else {
                        x.id() == module.id()
                    }
                })
            {
                return true;
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
                    assert!(super_val.is_nil(), "Module can not have superclass.");
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

    pub(crate) fn sort_array(&mut self, vec: &mut Vec<Value>) -> Result<(), RubyError> {
        if !vec.is_empty() {
            let val = vec[0];
            for v in &vec[1..] {
                match self.eval_compare(*v, val)? {
                    val if val.is_nil() => {
                        let lhs = val.get_class_name();
                        let rhs = v.get_class_name();
                        return Err(RubyError::argument(format!(
                            "Comparison of {} with {} failed.",
                            lhs, rhs
                        )));
                    }
                    _ => {}
                }
            }
            self.sort_by(vec, |vm, a, b| vm.eval_compare(*b, *a)?.to_ordering())?;
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
        Value::regexp_from(self, &arg)
    }
}

// API's for handling values.

impl VM {
    pub(crate) fn val_inspect(&mut self, val: Value) -> Result<String, RubyError> {
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
            RV::Object(oref) => match oref.kind() {
                ObjKind::INVALID => "[Invalid]".to_string(),
                ObjKind::STRING => oref.string().inspect(),
                ObjKind::RANGE => oref.range().inspect(self)?,
                ObjKind::MODULE | ObjKind::CLASS => oref.module().inspect(),
                ObjKind::REGEXP => format!("/{}/", oref.regexp().as_str().to_string()),
                ObjKind::ORDINARY => oref.inspect()?,
                ObjKind::HASH => oref.rhash().to_s(self)?,
                ObjKind::COMPLEX => format!("{:?}", oref.complex()),
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
    pub(crate) fn define_method(&mut self, target_obj: Value, id: IdentId, method: FnId) {
        target_obj
            .get_class_if_object()
            .add_method(&mut self.globals, id, method);
    }

    /// Define a method on a singleton class of `target_obj`.
    pub(crate) fn define_singleton_method(
        &mut self,
        target_obj: Value,
        id: IdentId,
        method: FnId,
    ) -> Result<(), RubyError> {
        target_obj
            .get_singleton_class()?
            .add_method(&mut self.globals, id, method);
        Ok(())
    }
}

impl VM {
    /// Get local variable table.
    fn get_outer_frame(&self, outer: u32) -> EnvFrame {
        let mut f = self.cfp.ep();
        for _ in 0..outer {
            f = f.outer().unwrap();
        }
        f
    }

    fn pop_key_value_pair(&mut self, arg_num: usize) -> FxIndexMap<HashKey, Value> {
        let mut hash = FxIndexMap::default();
        let p = self.sp() - arg_num * 2;
        for i in 0..arg_num {
            let key = p[(i * 2) as isize];
            let value = p[(i * 2 + 1) as isize];
            hash.insert(HashKey(key), value);
        }
        self.stack.sp = p;
        hash
    }

    /// Pop values and store them in new `Args`. `args_num` specifies the number of values to be popped.
    /// If there is some Array or Range with splat operator, break up the value and store each of them.
    fn pop_args_to_args(&mut self, arg_num: usize) -> Args2 {
        let arg_start = self.stack.sp - arg_num;
        let mut p = arg_start;
        while p < self.stack.sp {
            let prev_sp = self.stack.sp;
            let val = p[0];
            match val.as_splat() {
                Some(inner) => match inner.as_rvalue() {
                    None => {
                        p[0] = inner;
                        p += 1;
                    }
                    Some(obj) => match obj.kind() {
                        ObjKind::ARRAY => {
                            let a = &**obj.array();
                            let ary_len = a.len();
                            if ary_len == 0 {
                                self.stack.remove(p);
                            } else {
                                self.stack.grow(ary_len - 1);
                                RubyStack::stack_copy_within(p, 1..(prev_sp - p) as usize, ary_len);
                                p[0..ary_len].copy_from_slice(&a[..]);
                                p += ary_len;
                            }
                        }
                        // TODO: should use `to_a` method.
                        ObjKind::RANGE => {
                            let r = &*obj.range();
                            let start = r.start.coerce_to_fixnum("Expect Integer.").unwrap();
                            let end = r.end.coerce_to_fixnum("Expect Integer.").unwrap()
                                + if r.exclude { 0 } else { 1 };
                            if end >= start {
                                let ary_len = (end - start) as usize;
                                self.stack.grow(ary_len - 1);
                                RubyStack::stack_copy_within(p, 1..(prev_sp - p) as usize, ary_len);
                                for (idx, val) in (start..end).enumerate() {
                                    p[idx as isize] = Value::integer(val);
                                }
                                p += ary_len;
                            } else {
                                self.stack.remove(p);
                            };
                        }
                        _ => {
                            p[0] = inner;
                            p += 1;
                        }
                    },
                },
                None => p += 1,
            };
        }
        let len = (self.stack.sp - arg_start) as usize;
        Args2::new(len)
    }

    fn pop_args_to_array(&mut self, arg_num: usize) -> Value {
        let v = Value::array_empty();
        let mut ary = v.as_array().unwrap();
        let end = self.stack.sp;
        let mut p = end - arg_num;
        self.stack.sp = p;
        while p < end {
            let val = p[0];
            match val.as_splat() {
                Some(inner) => match inner.as_rvalue() {
                    None => ary.push(inner),
                    Some(obj) => match obj.kind() {
                        ObjKind::ARRAY => ary.extend_from_slice(&**obj.array()),
                        // TODO: should use `to_a` method.
                        ObjKind::RANGE => {
                            let r = &*obj.range();
                            let start = r.start.coerce_to_fixnum("Expect Integer.").unwrap();
                            let end = r.end.coerce_to_fixnum("Expect Integer.").unwrap()
                                + if r.exclude { 0 } else { 1 };
                            if end >= start {
                                for val in start..end {
                                    ary.push(Value::integer(val));
                                }
                            }
                        }
                        _ => ary.push(inner),
                    },
                },
                None => {
                    ary.push(val);
                }
            };
            p += 1;
        }
        v
    }

    pub(crate) fn create_range(&mut self, start: Value, end: Value, exclude_end: bool) -> VMResult {
        if self.eval_compare(start, end)?.is_nil() {
            return Err(RubyError::argument("Bad value for range."));
        }
        Ok(Value::range(start, end, exclude_end))
    }

    /// Create new Proc object from `block`,
    /// moving outer `Context`s on stack to heap.
    pub(crate) fn create_proc(&mut self, block: &Block) -> Value {
        match block {
            Block::Block(method, outer) => {
                let outer = self.cfp_from_frame(*outer);
                let self_val = outer.self_value();
                Value::procobj(self, self_val, *method, outer)
            }
            Block::Proc(proc) => *proc,
            Block::Sym(sym) => {
                let fid = Codegen::gen_sym_to_proc_iseq(&mut self.globals, *sym);
                Value::procobj(self, Value::nil(), fid, self.caller_cfp())
            }
        }
    }

    /// Create new Lambda object from `block`,
    /// moving outer `Context`s on stack to heap.
    pub(crate) fn create_lambda(&mut self, block: &Block) -> VMResult {
        match block {
            Block::Block(method, outer) => {
                let outer = self.cfp_from_frame(*outer);
                let self_val = outer.self_value();
                let mut iseq = self.globals.methods[*method].as_iseq();
                iseq.kind = ISeqKind::Method(None);
                Ok(Value::procobj(self, self_val, *method, outer))
            }
            Block::Proc(proc) => Ok(*proc),
            _ => unimplemented!(),
        }
    }

    /// Create a new execution context for a block.
    ///
    /// A new context is generated on heap, and all of the outer context chains are moved to heap.
    pub(crate) fn create_binding_context(
        &mut self,
        method: FnId,
        outer: ControlFrame,
    ) -> HeapCtxRef {
        let outer = self.move_cfp_to_heap(outer);
        let iseq = self.globals.methods[method].as_iseq();
        HeapCtxRef::new_heap(outer.self_value(), iseq, Some(outer))
    }

    pub(crate) fn create_heap(&mut self, block: &Block) -> HeapCtxRef {
        let (self_value, fid, outer) = match block {
            Block::Block(fid, outer) => {
                let outer = self.move_cfp_to_heap(self.cfp_from_frame(*outer));
                (outer.self_value(), *fid, Some(outer))
            }
            Block::Proc(proc) => {
                let pinfo = proc.as_proc().unwrap();
                (pinfo.self_val, pinfo.method, Some(pinfo.outer))
            }
            Block::Sym(sym) => {
                let fid = Codegen::gen_sym_to_proc_iseq(&mut self.globals, *sym);
                (Value::nil(), fid, None)
            }
        };
        HeapCtxRef::new_heap(self_value, self.globals.methods[fid].as_iseq(), outer)
    }

    /// Create fancy_regex::Regex from `string`.
    /// Escapes all regular expression meta characters in `string`.
    /// Returns RubyError if `string` was invalid regular expression.
    pub(crate) fn regexp_from_escaped_string(
        &mut self,
        string: &str,
    ) -> Result<RegexpInfo, RubyError> {
        RegexpInfo::from_escaped(&mut self.globals, string).map_err(VMError::regexp)
    }

    /// Create fancy_regex::Regex from `string` without escaping meta characters.
    /// Returns RubyError if `string` was invalid regular expression.
    pub(crate) fn regexp_from_string(&mut self, string: &str) -> Result<RegexpInfo, RubyError> {
        RegexpInfo::from_string(&mut self.globals, string).map_err(VMError::regexp)
    }
}
