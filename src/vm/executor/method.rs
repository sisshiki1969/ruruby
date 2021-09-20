use crate::*;

// Utilities for method call
// public API
impl VM {
    pub fn eval_send(&mut self, method_name: IdentId, receiver: Value, args: &Args) -> VMResult {
        self.exec_send(method_name, receiver, args)?;
        Ok(self.stack_pop())
    }

    pub fn eval_send0(&mut self, method_name: IdentId, receiver: Value) -> VMResult {
        self.exec_send0(method_name, receiver)?;
        Ok(self.stack_pop())
    }

    pub fn eval_send2(
        &mut self,
        method_name: IdentId,
        receiver: Value,
        arg0: Value,
        arg1: Value,
    ) -> VMResult {
        let args = Args::new2(arg0, arg1);
        self.exec_send(method_name, receiver, &args)?;
        Ok(self.stack_pop())
    }

    /// Evaluate the block with self_val of outer context, and given `args`.
    pub fn eval_block(&mut self, block: &Block, args: &Args) -> VMResult {
        match block {
            Block::Block(method, outer) => {
                //let outer = self.caller_frame_context();
                self.exec_func(*method, outer.self_value, Some(*outer), args)?;
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
        let args = Args2::new(1);
        match block {
            Block::Block(method, outer) => {
                let self_value = outer.self_value;
                use MethodInfo::*;
                match MethodRepo::get(*method) {
                    BuiltinFunc { func, name, .. } => {
                        for v in iter {
                            self.stack_push(v);
                            self.stack_push(self_value);
                            self.exec_native(&func, *method, name, &args)?;
                        }
                    }
                    RubyFunc { iseq } => {
                        for v in iter {
                            self.stack_push(v);
                            self.stack_push(self_value);
                            let context = self.push_frame_from_block(
                                iseq,
                                &args,
                                outer.get_current(),
                                false,
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
                    self.stack_push(v);
                    self.stack_push(self_value);
                    let context = self.push_frame(iseq, &args, outer, false)?;
                    match self.run_context(context) {
                        Err(err) => match err.kind {
                            RubyErrorKind::BlockReturn => return Ok(self.globals.error_register),
                            _ => return Err(err),
                        },
                        Ok(()) => {}
                    };
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
                let args = self.stack_push_args(args);
                self.stack_push(self_value);
                let context = self.push_frame(pref.iseq, &args, pref.outer, true)?;
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
        let iseq = self.parse_program_binding(path, code, ctx)?.as_iseq();
        ctx.set_iseq(iseq);
        self.stack_push(ctx.self_value);
        self.prepare_frame(0, true, ctx, iseq);
        self.run_context(ctx)?;
        Ok(self.stack_pop())
    }

    pub fn eval_proc(&mut self, proc: Value, args: &Args) -> VMResult {
        self.exec_proc(proc, args)?;
        Ok(self.stack_pop())
    }
}

impl VM {
    fn exec_send(
        &mut self,
        method_id: IdentId,
        receiver: Value,
        args: &Args,
    ) -> Result<(), RubyError> {
        let args = self.stack_push_args(args);
        self.stack_push(receiver);
        match MethodRepo::find_method_from_receiver(receiver, method_id) {
            Some(method) => self.invoke_method(method, &args),
            None => self.invoke_method_missing(method_id, &args, true),
        }?
        .handle(self)
    }

    pub(super) fn exec_send0(
        &mut self,
        method_id: IdentId,
        receiver: Value,
    ) -> Result<(), RubyError> {
        self.exec_send(method_id, receiver, &Args::new0())
    }

    pub(super) fn exec_send1(
        &mut self,
        method_id: IdentId,
        receiver: Value,
        arg0: Value,
    ) -> Result<(), RubyError> {
        self.exec_send(method_id, receiver, &Args::new1(arg0))
    }
}

impl VM {
    /// Execute the Proc object with given `args`, and push the returned value on the stack.
    fn exec_proc(&mut self, proc: Value, args: &Args) -> Result<(), RubyError> {
        let args = self.stack_push_args(args);
        self.invoke_proc(proc, &args)?.handle(self)
    }

    /// Invoke the method with given `self_val`, `outer` context, and `args`, and push the returned value on the stack.
    fn exec_func(
        &mut self,
        method_id: MethodId,
        self_val: impl Into<Value>,
        outer: Option<ContextRef>,
        args: &Args,
    ) -> Result<(), RubyError> {
        let args = self.stack_push_args(args);
        self.stack_push(self_val.into());
        self.invoke_func(method_id, outer, &args, true)?
            .handle(self)
    }

    pub(super) fn invoke_method(
        &mut self,
        method: MethodId,
        args: &Args2,
    ) -> Result<VMResKind, RubyError> {
        self.invoke_func(method, None, args, true)
    }

    pub(super) fn invoke_method_missing(
        &mut self,
        method_id: IdentId,
        args: &Args2,
        use_value: bool,
    ) -> Result<VMResKind, RubyError> {
        let receiver = self.stack_top();
        match MethodRepo::find_method_from_receiver(receiver, IdentId::_METHOD_MISSING) {
            Some(method) => {
                let len = args.len();
                let new_args = Args2::new(len + 1);
                self.exec_stack
                    .insert(self.stack_len() - len - 1, Value::symbol(method_id));
                self.invoke_func(method, None, &new_args, use_value)
            }
            None => {
                if receiver.id() == self.self_value().id() {
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

    pub(super) fn invoke_send1(
        &mut self,
        method_id: IdentId,
        receiver: Value,
        arg0: Value,
        use_value: bool,
    ) -> Result<VMResKind, RubyError> {
        self.invoke_send(method_id, receiver, &Args::new1(arg0), use_value)
    }

    pub(super) fn invoke_send2(
        &mut self,
        method_id: IdentId,
        receiver: Value,
        arg0: Value,
        arg1: Value,
        use_value: bool,
    ) -> Result<VMResKind, RubyError> {
        self.invoke_send(method_id, receiver, &Args::new2(arg0, arg1), use_value)
    }

    pub(super) fn invoke_send(
        &mut self,
        method_id: IdentId,
        receiver: Value,
        args: &Args,
        use_value: bool,
    ) -> Result<VMResKind, RubyError> {
        let args = self.stack_push_args(args);
        self.stack_push(receiver);
        match MethodRepo::find_method_from_receiver(receiver, method_id) {
            Some(method) => self.invoke_func(method, None, &args, use_value),
            None => self.invoke_method_missing(method_id, &args, use_value),
        }
    }

    // core methods

    /// Invoke the method with given `self_val`, `outer` context, and `args`, and push the returned value on the stack.
    pub(super) fn invoke_func(
        &mut self,
        method: MethodId,
        outer: Option<ContextRef>,
        args: &Args2,
        use_value: bool,
    ) -> Result<VMResKind, RubyError> {
        use MethodInfo::*;
        let val = match MethodRepo::get(method) {
            BuiltinFunc { func, name, .. } => self.exec_native(&func, method, name, args)?,
            AttrReader { id } => {
                args.check_args_num(0)?;
                self.exec_getter(id)?
            }
            AttrWriter { id } => {
                args.check_args_num(1)?;
                self.exec_setter(id)?
            }
            RubyFunc { iseq } => {
                let context = self.push_frame(iseq, args, outer, use_value)?;
                self.push_new_context(context);
                return Ok(VMResKind::Invoke);
            }
            _ => unreachable!(),
        };
        if use_value {
            self.stack_push(val);
        }
        Ok(VMResKind::Return)
    }

    /// Invoke the Proc object with given `args`.
    pub(super) fn invoke_proc(
        &mut self,
        proc: Value,
        args: &Args2,
    ) -> Result<VMResKind, RubyError> {
        let pinfo = proc.as_proc().unwrap();
        self.stack_push(pinfo.self_val);
        let context = self.push_frame(pinfo.iseq, args, pinfo.outer, true)?;
        self.push_new_context(context);
        Ok(VMResKind::Invoke)
    }

    /// Invoke the method defined by Rust fn and push the returned value on the stack.
    pub(super) fn exec_native(
        &mut self,
        func: &BuiltinFunc,
        _method_id: MethodId,
        _name: IdentId,
        args: &Args2,
    ) -> Result<Value, RubyError> {
        #[cfg(feature = "perf")]
        self.globals.perf.get_perf(Perf::EXTERN);

        #[cfg(any(feature = "trace", feature = "trace-func"))]
        if self.globals.startup_flag {
            println!("+++> BuiltinFunc {:?}", _name);
        }

        #[cfg(feature = "perf-method")]
        MethodRepo::inc_counter(_method_id);

        self.prepare_frame(args.len(), true, None, None);
        let temp_len = self.temp_len();
        let res = func(self, self.self_value(), &args);
        self.temp_stack.truncate(temp_len);
        self.unwind_frame();

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
    pub(super) fn exec_getter(&mut self, id: IdentId) -> Result<Value, RubyError> {
        let val = match self.stack_pop().as_rvalue() {
            Some(oref) => oref.get_var(id).unwrap_or_default(),
            None => Value::nil(),
        };
        Ok(val)
    }

    /// Invoke attr_setter and return the value.
    pub(super) fn exec_setter(&mut self, id: IdentId) -> Result<Value, RubyError> {
        let mut self_val = self.stack_pop();
        let val = self.stack_pop();
        match self_val.as_mut_rvalue() {
            Some(oref) => {
                oref.set_var(id, val);
                Ok(val)
            }
            None => unreachable!("AttrReader must be used only for class instance."),
        }
    }
}
