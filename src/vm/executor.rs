use crate::coroutine::*;
use crate::parse::codegen::ContextKind;
use crate::*;

#[cfg(feature = "perf")]
use super::perf::*;
use std::path::{Path, PathBuf};
use vm_inst::*;
mod opt_core;

pub type ValueTable = FxHashMap<IdentId, Value>;
pub type VMResult = Result<Value, RubyError>;

#[derive(Debug)]
pub struct VM {
    // Global info
    pub globals: GlobalsRef,
    // VM state
    exec_context: Vec<ContextRef>,
    cur_context: Option<ContextRef>,
    class_context: Vec<(Module, DefineMode)>,
    exec_stack: Vec<Value>,
    temp_stack: Vec<Value>,
    //exception: bool,
    pc: ISeqPos,
    pub handle: Option<FiberHandle>,
}

pub type VMRef = Ref<VM>;

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
        self.cur_context.iter().for_each(|c| c.mark(alloc));
        self.exec_context.iter().for_each(|c| c.mark(alloc));
        self.class_context.iter().for_each(|(v, _)| v.mark(alloc));
        self.exec_stack.iter().for_each(|v| v.mark(alloc));
        self.temp_stack.iter().for_each(|v| v.mark(alloc));
    }
}

impl VM {
    pub fn new(mut globals: GlobalsRef) -> Self {
        let mut vm = VM {
            globals,
            class_context: vec![(BuiltinClass::object(), DefineMode::default())],
            cur_context: None,
            exec_context: vec![],
            exec_stack: vec![],
            temp_stack: vec![],
            pc: ISeqPos::from(0),
            handle: None,
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
            vm.globals.perf = Perf::new();
        }

        #[cfg(feature = "perf-method")]
        {
            MethodPerf::clear_stats();
            vm.globals.clear_const_cache();
        }

        vm
    }

    pub fn create_fiber(&mut self) -> Self {
        VM {
            globals: self.globals,
            cur_context: None,
            exec_context: vec![],
            temp_stack: vec![],
            class_context: self.class_context.clone(),
            exec_stack: vec![],
            pc: ISeqPos::from(0),
            handle: None,
        }
    }

    pub fn context(&self) -> ContextRef {
        let ctx = self.cur_context.unwrap();
        debug_assert!(!ctx.on_stack || ctx.moved_to_heap.is_none());
        ctx
    }

    fn get_method_context(&self) -> ContextRef {
        let mut context = self.context();
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
        self.cur_context
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

    pub fn is_method(&self) -> bool {
        self.context().iseq_ref.unwrap().is_method()
    }

    fn stack_push(&mut self, val: Value) {
        self.exec_stack.push(val)
    }

    pub fn stack_pop(&mut self) -> Value {
        self.exec_stack
            .pop()
            .unwrap_or_else(|| panic!("exec stack is empty."))
    }

    pub fn stack_top(&mut self) -> Value {
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

    pub fn temp_pop_vec(&mut self, len: usize) -> Vec<Value> {
        self.temp_stack.split_off(len)
    }

    /// Push objects to the temporary area.
    pub fn temp_push_vec(&mut self, slice: &[Value]) {
        self.temp_stack.extend_from_slice(slice);
    }

    pub fn context_push(&mut self, ctx: ContextRef) {
        match self.cur_context {
            Some(c) => {
                self.exec_context.push(c);
                self.cur_context = Some(ctx);
            }
            None => self.cur_context = Some(ctx),
        }
    }

    pub fn context_pop(&mut self) -> Option<ContextRef> {
        match self.cur_context {
            Some(c) => {
                self.cur_context = self.exec_context.pop();
                Some(c)
            }
            None => None,
        }
    }

    #[cfg(not(tarpaulin_include))]
    pub fn clear(&mut self) {
        self.exec_stack.clear();
        self.class_context = vec![(BuiltinClass::object(), DefineMode::default())];
        self.exec_context.clear();
        self.cur_context = None;
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

    pub fn jump_pc(&mut self, inst_offset: usize, disp: ISeqDisp) {
        self.pc = (self.pc + inst_offset + disp).into();
    }

    pub fn parse_program(&mut self, path: PathBuf, program: &str) -> Result<MethodId, RubyError> {
        let parser = Parser::new();
        let result = parser.parse_program(path, program)?;

        #[cfg(feature = "perf")]
        self.globals.perf.set_prev_inst(Perf::INVALID);

        let methodref = Codegen::new(result.source_info).gen_iseq(
            &mut self.globals,
            vec![],
            result.node,
            result.lvar_collector,
            true,
            ContextKind::Method(None),
            None,
        )?;
        Ok(methodref)
    }

    pub fn parse_program_eval(
        &mut self,
        path: PathBuf,
        program: &str,
    ) -> Result<MethodId, RubyError> {
        let parser = Parser::new();
        let extern_context = self.context();
        let result = parser.parse_program_eval(path, program, Some(extern_context))?;

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
            None,
        )?;
        Ok(method)
    }

    pub fn run(&mut self, path: PathBuf, program: &str) -> VMResult {
        let method = self.parse_program(path, program)?;
        let mut iseq = method.as_iseq();
        iseq.class_defined = self.get_class_defined();
        let self_value = self.globals.main_object;
        let val = self.eval_method(method, self_value, &Args::new0())?;
        #[cfg(feature = "perf")]
        self.globals.perf.get_perf(Perf::INVALID);
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
        self.globals.perf.set_prev_inst(Perf::CODEGEN);

        let method = Codegen::new(result.source_info).gen_iseq(
            &mut self.globals,
            vec![],
            result.node,
            result.lvar_collector,
            true,
            ContextKind::Method(None),
            None,
        )?;
        let iseq = method.as_iseq();
        context.iseq_ref = Some(iseq);
        context.adjust_lvar_size();
        //context.pc = 0;

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

    pub fn run_context(&mut self, context: impl Into<ContextRef>) -> Result<(), RubyError> {
        let context = context.into();
        #[cfg(feature = "perf-method")]
        MethodRepo::inc_counter(context.iseq_ref.unwrap().method);
        let stack_len = self.stack_len();
        let pc = self.pc;
        self.context_push(context);
        self.pc = ISeqPos::from(0);
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
            match self.run_context_main() {
                Ok(()) => {
                    self.context_pop().unwrap();
                    debug_assert_eq!(stack_len + 1, self.stack_len());
                    self.pc = pc;
                    #[cfg(any(feature = "trace", feature = "trace-func"))]
                    println!("<--- Ok({:?})", self.stack_top());
                    return Ok(());
                }
                Err(mut err) => {
                    match err.kind {
                        RubyErrorKind::BlockReturn => {
                            self.context_pop().unwrap();
                            let val = self.stack_pop();
                            self.set_stack_len(stack_len);
                            self.stack_push(val);
                            self.pc = pc;
                            #[cfg(any(feature = "trace", feature = "trace-func"))]
                            {
                                println!("<--- BlockReturn({:?})", self.stack_top());
                            }
                            return Err(err);
                        }
                        _ => {}
                    }
                    err.info.push((self.source_info(), self.get_loc()));
                    //eprintln!("{:?}", iseq.exception_table);
                    if let RubyErrorKind::Internal(msg) = &err.kind {
                        eprintln!();
                        err.show_err();
                        err.show_all_loc();
                        unreachable!("{}", msg);
                    };
                    let iseq = self.context().iseq_ref.unwrap();
                    let catch = iseq
                        .exception_table
                        .iter()
                        .find(|x| x.include(self.pc.into_usize()));
                    if let Some(entry) = catch {
                        // Exception raised inside of begin-end with rescue clauses.
                        self.pc = entry.dest.into();
                        self.set_stack_len(stack_len);
                        let val = err.to_exception_val();
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
}

impl VM {
    fn get_loc(&self) -> Loc {
        match self.context().iseq_ref {
            None => Loc(1, 1),
            Some(iseq) => {
                iseq.iseq_sourcemap
                    .iter()
                    .find(|x| x.0 == self.pc)
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
        let self_val = self.context().self_value;
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
        let self_val = self.context().self_value;
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
    ) -> Result<(), RubyError> {
        let rec_class = receiver.get_class_for_method();
        match MethodRepo::find_method_inline_cache(cache, rec_class, method_id) {
            Some(method) => return self.invoke_method(method, receiver, args),
            None => {}
        }
        self.send_method_missing(method_id, receiver, args)
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
                self.invoke_method(method, receiver, &new_args)
            }
            None => Err(RubyError::undefined_method(method_id, receiver)),
        }
    }

    fn fallback_for_binop(&mut self, method: IdentId, lhs: Value, rhs: Value) -> VMResult {
        let class = lhs.get_class_for_method();
        match MethodRepo::find_method(class, method) {
            Some(mref) => {
                let arg = Args::new1(rhs);
                let val = self.eval_method(mref, lhs, &arg)?;
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

    fn get_index(&mut self) -> VMResult {
        let idx = self.stack_pop();
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
        self.stack_pop();
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
                    self.eval_method(mref.method, mref.receiver, &args)?
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
        self.stack_pop();
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
    /// Evaluate method with given `self_val`, `args` and no outer context.
    pub fn eval_method(
        &mut self,
        methodref: MethodId,
        self_val: impl Into<Value>,
        args: &Args,
    ) -> VMResult {
        let self_val = self_val.into();
        self.invoke_func(methodref, self_val, None, args)?;
        Ok(self.stack_pop())
    }

    pub fn invoke_method(
        &mut self,
        methodref: MethodId,
        self_val: impl Into<Value>,
        args: &Args,
    ) -> Result<(), RubyError> {
        let self_val = self_val.into();
        self.invoke_func(methodref, self_val, None, args)
    }

    /// Evaluate method with self_val of current context, current context as outer context, and given `args`.
    pub fn eval_block(&mut self, block: &Block, args: &Args) -> VMResult {
        match block {
            Block::Block(method, outer) => {
                self.invoke_func(*method, outer.self_value, Some(*outer), args)?;
                Ok(self.stack_pop())
            }
            Block::Proc(proc) => self.invoke_proc(*proc, args),
            _ => unreachable!(),
        }
    }

    /// Evaluate method with self_val of current context, current context as outer context, and given `args`.
    pub fn eval_block_iter1(
        &mut self,
        block: &Block,
        args0: impl Iterator<Item = Value>,
        return_val: bool,
    ) -> VMResult {
        let res = match block {
            Block::Block(method, outer) => self.invoke_method_iter1(
                *method,
                outer.self_value,
                Some(*outer),
                args0,
                return_val,
            )?,
            Block::Proc(proc) => {
                let mut args = Args::new1(Value::nil());
                let len = self.temp_stack.len();
                for v in args0 {
                    args[0] = v;
                    let val = self.invoke_proc(*proc, &args)?;
                    if return_val {
                        self.temp_push(val);
                    }
                }
                if return_val {
                    Value::array_from(self.temp_pop_vec(len))
                } else {
                    Value::nil()
                }
            }
            _ => unreachable!(),
        };
        Ok(res)
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
                self.invoke_func(*method, self_val, Some(*outer), args)?
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
                self.run_context(&context)?
            }
            _ => unreachable!(),
        }
        Ok(self.stack_pop())
    }

    /// Evaluate given block with given `args`.
    pub fn eval_yield(&mut self, args: &Args) -> Result<(), RubyError> {
        let context = self.get_method_context();
        match &context.block {
            Block::Block(method, ctx) => {
                self.invoke_func(*method, ctx.self_value, Some(*ctx), args)
            }
            Block::Proc(proc) => {
                let val = self.invoke_proc(*proc, args)?;
                self.stack_push(val);
                Ok(())
            }
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
        self.run_context(&context)?;
        Ok(self.stack_pop())
    }

    /// Evaluate method with given `self_val`, `outer` context, and `args`.
    pub fn invoke_func(
        &mut self,
        method: MethodId,
        self_val: impl Into<Value>,
        outer: Option<ContextRef>,
        args: &Args,
    ) -> Result<(), RubyError> {
        let self_val = self_val.into();
        use MethodInfo::*;
        let outer = outer.map(|ctx| ctx.get_current());
        match MethodRepo::get(method) {
            BuiltinFunc { func, name } => {
                let val = self.invoke_native(&func, method, name, self_val, args)?;
                self.stack_push(val);
                Ok(())
            }
            AttrReader { id } => {
                args.check_args_num(0)?;
                let val = Self::invoke_getter(id, self_val)?;
                self.stack_push(val);
                Ok(())
            }
            AttrWriter { id } => {
                args.check_args_num(1)?;
                let val = Self::invoke_setter(id, self_val, args[0])?;
                self.stack_push(val);
                Ok(())
            }
            RubyFunc { iseq } => {
                let context = Context::from_args(self, self_val, iseq, args, outer)?;
                self.run_context(&context)
            }
            _ => unreachable!(),
        }
    }

    /// Evaluate method with given `self_val`, `outer` context, and `args`.
    pub fn invoke_method_iter1(
        &mut self,
        method: MethodId,
        self_val: Value,
        outer: Option<ContextRef>,
        args0: impl Iterator<Item = Value>,
        return_val: bool,
    ) -> VMResult {
        use MethodInfo::*;
        let outer = outer.map(|ctx| ctx.get_current());
        let mut args = Args::new1(Value::nil());
        let len = self.temp_stack.len();
        match MethodRepo::get(method) {
            BuiltinFunc { func, name } => {
                for v in args0 {
                    args[0] = v;
                    let res = self.invoke_native(&func, method, name, self_val, &args)?;
                    if return_val {
                        self.temp_push(res);
                    };
                }
            }
            RubyFunc { iseq } => {
                let context = Context::from_args(self, self_val, iseq, &args, outer)?;
                let mut context = ContextRef::from_ref(&context);

                if iseq.params.req + iseq.params.post > 1 {
                    if iseq.opt_flag {
                        for v in args0 {
                            context = context.get_current();
                            if let Some(ary) = v.as_array() {
                                context.fill_arguments_opt(&ary.elements, iseq.params.req);
                            } else {
                                context[0] = v;
                            }
                            self.run_context(context)?;
                            let res = self.stack_pop();
                            if return_val {
                                self.temp_push(res);
                            };
                        }
                    } else {
                        for v in args0 {
                            context = context.get_current();
                            if let Some(ary) = v.as_array() {
                                context.fill_arguments(&ary.elements, &iseq.params, Value::nil());
                            } else {
                                context[0] = v;
                            }
                            self.run_context(context)?;
                            let res = self.stack_pop();
                            if return_val {
                                self.temp_push(res);
                            };
                        }
                    }
                } else {
                    for v in args0 {
                        context = context.get_current();
                        context[0] = v;
                        self.run_context(context)?;
                        let res = self.stack_pop();
                        if return_val {
                            self.temp_push(res);
                        };
                    }
                }
            }
            _ => unreachable!(),
        };
        if return_val {
            Ok(Value::array_from(self.temp_pop_vec(len)))
        } else {
            Ok(Value::nil())
        }
    }

    // helper methods
    fn invoke_native(
        &mut self,
        func: &BuiltinFunc,
        _method_id: MethodId,
        _name: IdentId,
        self_val: Value,
        args: &Args,
    ) -> VMResult {
        #[cfg(feature = "perf")]
        self.globals.perf.get_perf(Perf::EXTERN);

        #[cfg(any(feature = "trace", feature = "trace-func"))]
        println!("---> BuiltinFunc {:?}", _name);

        #[cfg(feature = "perf-method")]
        MethodRepo::inc_counter(_method_id);

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
            None => Ok(Value::nil()),
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
    pub fn define_method(&mut self, target_obj: Value, id: IdentId, method: MethodId) {
        match target_obj.if_mod_class() {
            Some(mut module) => module.add_method(id, method),
            None => target_obj.get_class().add_method(id, method),
        };
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

    /// Get method(MethodId) for class.
    ///
    /// If the method was not found, return NoMethodError.
    pub fn get_method(
        &mut self,
        rec_class: Module,
        method_id: IdentId,
    ) -> Result<MethodId, RubyError> {
        match MethodRepo::find_method(rec_class, method_id) {
            Some(m) => Ok(m),
            None => Err(RubyError::undefined_method_for_class(method_id, rec_class)),
        }
    }

    /// Get method(MethodId) for receiver.
    pub fn get_method_from_receiver(
        &mut self,
        receiver: Value,
        method_id: IdentId,
    ) -> Result<MethodId, RubyError> {
        let rec_class = receiver.get_class_for_method();
        self.get_method(rec_class, method_id)
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

    fn pop_key_value_pair(&mut self, arg_num: usize) -> FxHashMap<HashKey, Value> {
        let mut hash = FxHashMap::default();
        for _ in 0..arg_num {
            let value = self.stack_pop();
            let key = self.stack_pop();
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
                context.kind = ISeqKind::Method(None);
                Ok(Value::procobj(context))
            }
            Block::Proc(proc) => Ok(proc.dup()),
            _ => unreachable!(),
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
        args.block = Block::Block(METHOD_ENUM, self.context());
        let fiber = self.create_enum_info(EnumInfo {
            method,
            receiver,
            args,
        });
        Ok(Value::enumerator(fiber))
    }

    /// Move outer execution contexts on the stack to the heap.
    fn move_outer_to_heap(&mut self, outer: ContextRef) -> ContextRef {
        let mut stack_context = outer;
        let mut prev_ctx: Option<ContextRef> = None;
        let mut iter = self
            .exec_context
            .iter_mut()
            .chain(self.cur_context.iter_mut())
            .rev();
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
        method: MethodId,
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

    #[cfg(not(tarpaulin_include))]
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

    #[cfg(not(tarpaulin_include))]
    pub fn exec_program(&mut self, absolute_path: PathBuf, program: &str) {
        match self.run(absolute_path, program) {
            Ok(_) => {
                #[cfg(feature = "perf")]
                self.globals.perf.print_perf();
                #[cfg(feature = "perf-method")]
                {
                    MethodRepo::print_stats();
                    self.globals.print_constant_cache_stats();
                    MethodPerf::print_stats();
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
