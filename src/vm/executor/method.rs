use crate::*;

// Utilities for method call
// public API
impl VM {
    pub fn eval_send(&mut self, method_name: IdentId, receiver: Value, args: &Args) -> VMResult {
        self.send(method_name, receiver, args)
    }

    pub fn eval_send0(&mut self, method_name: IdentId, receiver: Value) -> VMResult {
        let args = Args::new0();
        self.send(method_name, receiver, &args)
    }

    pub fn eval_send2(
        &mut self,
        method_name: IdentId,
        receiver: Value,
        arg0: Value,
        arg1: Value,
    ) -> VMResult {
        let args = Args::new2(arg0, arg1);
        self.send(method_name, receiver, &args)
    }
}

impl VM {
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

    /// Evaluate the method with given `self_val`, `args` and no outer context.
    pub fn eval_method(
        &mut self,
        method: MethodId,
        self_val: impl Into<Value>,
        args: &Args,
    ) -> VMResult {
        self.eval_method_with_outer(method, self_val, None, args)
    }

    /// Evaluate the method with given `self_val`, `args` and no outer context.
    pub fn eval_method_with_outer(
        &mut self,
        method: MethodId,
        self_val: impl Into<Value>,
        outer: impl Into<Option<ContextRef>>,
        args: &Args,
    ) -> VMResult {
        let self_val = self_val.into();
        let outer = outer.into();
        self.exec_func(method, self_val, outer, args)?;
        Ok(self.stack_pop())
    }

    pub fn eval_binding(&mut self, path: String, code: String, mut ctx: ContextRef) -> VMResult {
        let method = self.parse_program_binding(path, code, ctx)?;
        ctx.iseq_ref = Some(method.as_iseq());
        self.run_context(ctx)?;
        Ok(self.stack_pop())
    }
}

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

    pub(super) fn invoke_send0(
        &mut self,
        method_id: IdentId,
        receiver: Value,
    ) -> Result<(), RubyError> {
        self.invoke_send(method_id, receiver, &Args::new0())
    }

    pub(super) fn invoke_send1(
        &mut self,
        method_id: IdentId,
        receiver: Value,
        arg0: Value,
    ) -> Result<(), RubyError> {
        self.invoke_send(method_id, receiver, &Args::new1(arg0))
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

    pub(super) fn invoke_method_missing(
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

    pub(super) fn fallback_for_binop(
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

impl VM {
    pub(super) fn eval_fast_send(
        &mut self,
        method_name: IdentId,
        receiver: Value,
        cache_id: u32,
        args_num: usize,
        block: u32,
        use_value: bool,
    ) -> Result<VMResKind, RubyError> {
        let len = self.stack_len();
        let rec_class = receiver.get_class_for_method();
        let val = match MethodRepo::find_method_inline_cache(cache_id, rec_class, method_name) {
            Some(method) => match MethodRepo::get(method) {
                MethodInfo::BuiltinFunc { func, name, .. } => {
                    let mut args = Args::from_slice(&self.exec_stack[len - args_num..]);
                    args.block = Block::from_u32(block, self);
                    self.set_stack_len(len - args_num);
                    self.exec_native(&func, method, name, receiver, &args)?
                }
                MethodInfo::AttrReader { id } => {
                    if args_num != 0 {
                        return Err(RubyError::argument_wrong(args_num, 0));
                    }
                    self.exec_getter(id, receiver)?
                }
                MethodInfo::AttrWriter { id } => {
                    if args_num != 1 {
                        return Err(RubyError::argument_wrong(args_num, 1));
                    }
                    let val = self.stack_pop();
                    self.exec_setter(id, receiver, val)?
                }
                MethodInfo::RubyFunc { iseq } => {
                    let block = Block::from_u32(block, self);
                    let mut context = if iseq.opt_flag {
                        let req_len = iseq.params.req;
                        if args_num != req_len {
                            return Err(RubyError::argument_wrong(args_num, req_len));
                        };
                        let mut context = self.new_stack_context_with(receiver, block, iseq, None);
                        context.copy_from_slice0(&self.exec_stack[len - args_num..]);
                        context
                    } else {
                        let mut args = Args::from_slice(&self.exec_stack[len - args_num..]);
                        args.block = block;
                        ContextRef::from_noopt(self, receiver, iseq, &args, None)?
                    };
                    context.use_value = use_value;
                    self.set_stack_len(len - args_num);
                    self.invoke_new_context(context);
                    return Ok(VMResKind::Invoke);
                }
                _ => unreachable!(),
            },
            None => {
                let mut args = Args::from_slice(&self.exec_stack[len - args_num..]);
                args.block = Block::from_u32(block, self);
                self.set_stack_len(len - args_num);
                return self.invoke_method_missing(method_name, receiver, &args, use_value);
            }
        };
        if use_value {
            self.stack_push(val);
        }
        Ok(VMResKind::Return)
    }
}

impl VM {
    /// Invoke the method with given `self_val` and `args`.
    pub(super) fn exec_method(
        &mut self,
        method: MethodId,
        self_val: impl Into<Value>,
        args: &Args,
    ) -> Result<(), RubyError> {
        self.exec_func(method, self_val, None, args)
    }

    /// Execute the Proc object with given `args`, and push the returned value on the stack.
    pub fn exec_proc(&mut self, proc: Value, args: &Args) -> Result<(), RubyError> {
        let pinfo = proc.as_proc().unwrap();
        let context = ContextRef::from(self, pinfo.self_val, pinfo.iseq, args, pinfo.outer)?;
        self.run_context(context)
    }

    /// Invoke the Proc object with given `args`.
    pub(super) fn invoke_proc(&mut self, proc: Value, args: &Args) -> Result<VMResKind, RubyError> {
        let pinfo = proc.as_proc().unwrap();
        let context = ContextRef::from(self, pinfo.self_val, pinfo.iseq, args, pinfo.outer)?;
        self.invoke_new_context(context);
        Ok(VMResKind::Invoke)
    }

    /// Invoke the method with given `self_val`, `outer` context, and `args`, and push the returned value on the stack.
    pub(super) fn invoke_func(
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

    fn send(&mut self, method_id: IdentId, receiver: Value, args: &Args) -> VMResult {
        match MethodRepo::find_method_from_receiver(receiver, method_id) {
            Some(method) => return self.eval_method(method, receiver, args),
            None => {}
        };
        self.send_method_missing(method_id, receiver, args)?;
        Ok(self.stack_pop())
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

    /// Invoke the method defined by Rust fn and push the returned value on the stack.
    pub(super) fn exec_native(
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
        let res = func(self, self_value, args);
        self.temp_stack.truncate(len);

        #[cfg(any(feature = "trace", feature = "trace-func"))]
        if self.globals.startup_flag {
            println!("<+++ {:?}", res);
        }
        match res {
            Ok(val) => Ok(val),
            Err(err) => {
                if err.is_block_return() {
                    let val = self.globals.error_register;
                    Ok(val)
                } else {
                    Err(err)
                }
            }
        }
    }

    /// Invoke attr_getter and return the value.
    pub(super) fn exec_getter(&mut self, id: IdentId, self_val: Value) -> Result<Value, RubyError> {
        let val = match self_val.as_rvalue() {
            Some(oref) => oref.get_var(id).unwrap_or_default(),
            None => Value::nil(),
        };
        Ok(val)
    }

    /// Invoke attr_setter and return the value.
    pub(super) fn exec_setter(
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
