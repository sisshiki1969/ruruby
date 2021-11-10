use crate::*;

// Utilities for method call
// public API
impl VM {
    pub(crate) fn eval_send(
        &mut self,
        method_name: IdentId,
        receiver: Value,
        args: &Args,
    ) -> VMResult {
        self.exec_send(method_name, receiver, args)?;
        Ok(self.stack_pop())
    }

    pub(crate) fn eval_send0(&mut self, method_name: IdentId, receiver: Value) -> VMResult {
        self.exec_send0(method_name, receiver)?;
        Ok(self.stack_pop())
    }

    /// Evaluate the block with self_val of outer context, and given `args`.
    pub(crate) fn eval_block(&mut self, block: &Block, slice: &[Value], args: &Args2) -> VMResult {
        self.stack_append(slice);
        match block {
            Block::Block(method, outer) => {
                let outer = self.dfp_from_frame(*outer);
                self.stack_push(outer.self_value());
                self.eval_func(*method, Some(outer), &args, false)
            }
            Block::Proc(proc) => self.eval_proc(*proc, None, args),
        }
    }

    /// Evaluate the block with self_val of outer context with no args.
    pub(crate) fn eval_block0(&mut self, block: &Block) -> VMResult {
        let args = Args2::new(0);
        match block {
            Block::Block(method, outer) => {
                let outer = self.dfp_from_frame(*outer);
                self.stack_push(outer.self_value());
                self.eval_func(*method, Some(outer), &args, false)
            }
            Block::Proc(proc) => self.eval_proc(*proc, None, &args),
        }
    }

    /// Evaluate the block with self_val of outer context, and given `arg0`.
    pub(crate) fn eval_block1(&mut self, block: &Block, arg0: Value) -> VMResult {
        let args = Args2::new(1);
        self.stack_push(arg0);
        match block {
            Block::Block(method, outer) => {
                let outer = self.dfp_from_frame(*outer);
                self.stack_push(outer.self_value());
                self.eval_func(*method, Some(outer), &args, false)
            }
            Block::Proc(proc) => self.eval_proc(*proc, None, &args),
        }
    }

    /// Evaluate the block with self_val of outer context, and given `arg0`, `arg1`.
    pub(crate) fn eval_block2(&mut self, block: &Block, arg0: Value, arg1: Value) -> VMResult {
        let args = Args2::new(2);
        self.stack_push(arg0);
        self.stack_push(arg1);
        match block {
            Block::Block(method, outer) => {
                let outer = self.dfp_from_frame(*outer);
                self.stack_push(outer.self_value());
                self.eval_func(*method, Some(outer), &args, false)
            }
            Block::Proc(proc) => self.eval_proc(*proc, None, &args),
        }
    }

    /// Evaluate the block with given `self_val`, `args` and no outer context.
    pub(crate) fn eval_block_with_methodid(
        &mut self,
        method: MethodId,
        self_val: impl Into<Value>,
        outer: DynamicFrame,
        args: &Args,
    ) -> VMResult {
        let self_val = self_val.into();
        let args = self.stack_push_args(args);
        self.stack_push(self_val);
        self.eval_func(method, Some(outer), &args, false)
    }

    pub(crate) fn eval_block_each1(
        &mut self,
        block: &Block,
        iter: impl Iterator<Item = Value>,
        default: Value,
    ) -> VMResult {
        let args = Args2::new(1);
        let (method, outer, self_value) = match block {
            Block::Block(method, outer) => {
                let outer = self.dfp_from_frame(*outer);
                (*method, Some(outer), outer.self_value())
            }
            Block::Proc(proc) => {
                let pinfo = proc.as_proc().unwrap();
                (pinfo.method, pinfo.outer, pinfo.self_val)
            }
        };

        use MethodInfo::*;
        match self.globals.methods.get(method) {
            BuiltinFunc { func, name, .. } => {
                let name = *name;
                let func = *func;
                for v in iter {
                    self.stack_push(v);
                    self.stack_push(self_value);
                    self.exec_native(&func, method, name, &args)?;
                }
            }
            RubyFunc { iseq } => {
                let iseq = *iseq;
                if iseq.opt_flag {
                    for v in iter {
                        self.stack_push(v);
                        self.stack_push(self_value);
                        self.push_block_frame_fast(iseq, &args, outer, false)?;
                        self.run_loop()?;
                    }
                } else {
                    for v in iter {
                        self.stack_push(v);
                        self.stack_push(self_value);
                        self.push_frame(iseq, &args, outer, false, false)?;
                        self.run_loop()?;
                    }
                }
            }
            _ => unreachable!(),
        };

        Ok(default)
    }

    /// Evaluate the block with given `self_val` and `args`.
    pub(crate) fn eval_block_self(
        &mut self,
        block: &Block,
        self_value: impl Into<Value>,
        args: &Args,
    ) -> VMResult {
        let self_value = self_value.into();
        match block {
            Block::Block(method, outer) => {
                let outer = self.dfp_from_frame(*outer);
                let args = self.stack_push_args(args);
                self.stack_push(self_value);
                self.eval_func(*method, Some(outer), &args, false)
            }
            Block::Proc(proc) => {
                let args = self.stack_push_args(args);
                self.eval_proc(*proc, self_value, &args)
            }
        }
    }

    /// Evaluate the method with given `self_val`, `args` and no outer context.
    pub(crate) fn eval_method(
        &mut self,
        method: MethodId,
        self_val: impl Into<Value>,
        slice: &[Value],
        args: &Args2,
    ) -> VMResult {
        let self_val = self_val.into();
        self.stack_append(slice);
        self.stack_push(self_val);
        self.eval_func(method, None, &args, true)
    }

    pub(crate) fn eval_method_range(
        &mut self,
        method: MethodId,
        self_val: impl Into<Value>,
        range: std::ops::Range<usize>,
        args: &Args2,
    ) -> VMResult {
        let self_val = self_val.into();
        self.stack_extend_from_within(range);
        self.stack_push(self_val);
        self.eval_func(method, None, &args, true)
    }

    /// Evaluate the method with given `self_val`, `args` and no outer context.
    pub(crate) fn eval_method0(
        &mut self,
        method: MethodId,
        self_val: impl Into<Value>,
    ) -> VMResult {
        let self_val = self_val.into();
        let args = Args2::new(0);
        self.stack_push(self_val);
        self.eval_func(method, None, &args, true)
    }

    /// Execute the Proc object with given `args`, and push the returned value on the stack.
    pub(crate) fn eval_proc(
        &mut self,
        proc: Value,
        self_value: impl Into<Option<Value>>,
        args: &Args2,
    ) -> VMResult {
        self.invoke_proc(proc, self_value, &args)?.handle_ret(self)
    }

    pub(crate) fn eval_binding(
        &mut self,
        path: String,
        code: String,
        mut ctx: HeapCtxRef,
    ) -> VMResult {
        let id = self.parse_program_binding(path, code, ctx.as_dfp())?;
        let iseq = self.globals.methods[id].as_iseq();
        ctx.set_iseq(iseq);
        self.stack_push(ctx.self_val());
        self.prepare_frame_from_heap(ctx);
        let val = self.run_loop()?;
        Ok(val)
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
        match self
            .globals
            .methods
            .find_method_from_receiver(receiver, method_id)
        {
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
    /// Invoke the method with given `self_val`, `outer` context, and `args`, and push the returned value on the stack.
    fn eval_func(
        &mut self,
        method_id: MethodId,
        outer: Option<DynamicFrame>,
        args: &Args2,
        is_method: bool,
    ) -> VMResult {
        self.invoke_func(method_id, outer, args, true, is_method)?
            .handle_ret(self)
    }

    pub(super) fn invoke_method(
        &mut self,
        method: MethodId,
        args: &Args2,
    ) -> Result<VMResKind, RubyError> {
        self.invoke_func(method, None, args, true, true)
    }

    pub(super) fn invoke_method_missing(
        &mut self,
        method_id: IdentId,
        args: &Args2,
        use_value: bool,
    ) -> Result<VMResKind, RubyError> {
        let receiver = self.stack_top();
        match self
            .globals
            .methods
            .find_method_from_receiver(receiver, IdentId::_METHOD_MISSING)
        {
            Some(method) => {
                let len = args.len();
                let new_args = Args2::new(len + 1);
                self.exec_stack
                    .insert(self.stack_len() - len - 1, Value::symbol(method_id));
                self.invoke_func(method, None, &new_args, use_value, true)
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
        self.stack_push(arg0);
        self.invoke_send(method_id, receiver, &Args2::new(1), use_value)
    }

    pub(super) fn invoke_send2(
        &mut self,
        method_id: IdentId,
        receiver: Value,
        arg0: Value,
        arg1: Value,
        use_value: bool,
    ) -> Result<VMResKind, RubyError> {
        self.stack_push(arg0);
        self.stack_push(arg1);
        self.invoke_send(method_id, receiver, &Args2::new(2), use_value)
    }

    fn invoke_send(
        &mut self,
        method_id: IdentId,
        receiver: Value,
        args: &Args2,
        use_value: bool,
    ) -> Result<VMResKind, RubyError> {
        self.stack_push(receiver);
        match self
            .globals
            .methods
            .find_method_from_receiver(receiver, method_id)
        {
            Some(method) => self.invoke_func(method, None, &args, use_value, true),
            None => self.invoke_method_missing(method_id, &args, use_value),
        }
    }

    // core methods

    /// Invoke the method with given `self_val`, `outer` context, and `args`, and push the returned value on the stack.
    #[inline(always)]
    pub(super) fn invoke_func(
        &mut self,
        method: MethodId,
        outer: Option<DynamicFrame>,
        args: &Args2,
        use_value: bool,
        is_method: bool,
    ) -> Result<VMResKind, RubyError> {
        use MethodInfo::*;
        let val = match &self.globals.methods[method] {
            BuiltinFunc { func, name, .. } => {
                let name = *name;
                let func = *func;
                self.exec_native(&func, method, name, args)?
            }
            AttrReader { id } => {
                let id = *id;
                self.exec_getter(id, args)?
            }
            AttrWriter { id } => {
                let id = *id;
                self.exec_setter(id, args)?
            }
            RubyFunc { iseq } => {
                let iseq = *iseq;
                if iseq.opt_flag {
                    if is_method {
                        debug_assert!(outer.is_none());
                        self.push_method_frame_fast(iseq, args, use_value)?;
                    } else {
                        self.push_block_frame_fast(iseq, args, outer, use_value)?;
                    }
                } else {
                    self.push_frame(iseq, args, outer, use_value, is_method)?;
                }
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
        self_value: impl Into<Option<Value>>,
        args: &Args2,
    ) -> Result<VMResKind, RubyError> {
        let pinfo = proc.as_proc().unwrap();
        let self_val = match self_value.into() {
            Some(v) => v,
            None => pinfo.self_val,
        };
        self.stack_push(self_val);
        self.invoke_func(pinfo.method, pinfo.outer, args, true, false) //TODO:lambda or proc
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

        #[cfg(feature = "trace")]
        {
            println!(
                "+++> BuiltinFunc self:{:?} name:{:?}",
                self.stack_top(),
                _name
            );
        }

        #[cfg(feature = "perf-method")]
        self.globals.methods.inc_counter(_method_id);

        self.prepare_native_frame(args.len());

        let temp_len = self.temp_len();
        let res = func(self, self.self_value(), args);
        self.temp_stack.truncate(temp_len);

        self.unwind_frame();

        #[cfg(feature = "trace")]
        println!("<+++ {:?}", res);
        match res {
            Ok(val) => Ok(val),
            Err(err) => {
                if err.is_block_return() {
                    let val = self.globals.val;
                    Ok(val)
                } else {
                    Err(err)
                }
            }
        }
    }

    /// Invoke attr_getter and return the value.
    pub(super) fn exec_getter(&mut self, id: IdentId, args: &Args2) -> VMResult {
        args.check_args_num(0)?;
        let val = match self.stack_pop().as_rvalue() {
            Some(oref) => oref.get_var(id).unwrap_or_default(),
            None => Value::nil(),
        };
        Ok(val)
    }

    /// Invoke attr_setter and return the value.
    pub(super) fn exec_setter(&mut self, id: IdentId, args: &Args2) -> VMResult {
        args.check_args_num(1)?;
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