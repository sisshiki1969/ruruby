use crate::*;

use super::ruby_stack::StackPtr;

// Utilities for method call
// public API
impl VM {
    pub(crate) fn eval_send(
        &mut self,
        method_name: IdentId,
        receiver: Value,
        args: &Args,
    ) -> VMResult {
        self.stack_push(receiver);
        let args = self.stack_push_args(args);
        self.invoke_send(method_name, receiver, &args, true)?
            .handle(self)
    }

    pub(crate) fn eval_send0(&mut self, method_name: IdentId, receiver: Value) -> VMResult {
        self.eval_send(method_name, receiver, &Args::new0())
    }

    pub(crate) fn eval_send1(
        &mut self,
        method_name: IdentId,
        receiver: Value,
        arg: Value,
    ) -> VMResult {
        self.eval_send(method_name, receiver, &Args::new1(arg))
    }

    /// Evaluate the block with self_val of outer context, and given `args`.
    pub(crate) fn eval_block(&mut self, block: &Block, slice: &[Value]) -> VMResult {
        match block {
            Block::Block(method, outer) => {
                let outer = self.cfp_from_frame(*outer).ep();
                self.stack_push(outer.self_value());
                self.stack.extend_from_slice(slice);
                self.invoke_block(*method, outer, &Args2::new(slice.len()))?
                    .handle(self)
            }
            Block::Proc(proc) => {
                self.stack_push(Value::nil());
                self.stack.extend_from_slice(slice);
                self.invoke_proc(*proc, None, &Args2::new(slice.len()))?
                    .handle(self)
            }
            Block::Sym(sym) => {
                self.stack.extend_from_slice(slice);
                self.invoke_sym_proc(*sym, &Args2::new(slice.len()))?
                    .handle(self)
            }
        }
    }

    /// Evaluate the block with self_val of outer context with no args.
    pub(crate) fn eval_block0(&mut self, block: &Block) -> VMResult {
        self.eval_block(block, &[])
    }

    /// Evaluate the block with self_val of outer context, and given `arg0`.
    pub(crate) fn eval_block1(&mut self, block: &Block, arg0: Value) -> VMResult {
        self.eval_block(block, &[arg0])
    }

    /// Evaluate the block with self_val of outer context, and given `arg0`, `arg1`.
    pub(crate) fn eval_block2(&mut self, block: &Block, arg0: Value, arg1: Value) -> VMResult {
        self.eval_block(block, &[arg0, arg1])
    }

    /// Evaluate the block with given `self_val`, `args` and no outer context.
    pub(crate) fn eval_block_with_methodid(
        &mut self,
        method: FnId,
        self_val: impl Into<Value>,
        outer: EnvFrame,
        args: &Args,
    ) -> VMResult {
        let self_val = self_val.into();
        self.stack_push(self_val);
        let args = self.stack_push_args(args);
        self.invoke_block(method, outer, &args)?.handle(self)
    }

    pub(crate) fn eval_block_each1_iter(
        &mut self,
        block: &Block,
        iter: impl Iterator<Item = Value>,
        default: Value,
    ) -> VMResult {
        let args = Args2::new(1);
        let (method, outer, self_value) = match block {
            Block::Block(method, outer) => {
                let outer = self.cfp_from_frame(*outer).ep();
                (*method, outer, outer.self_value())
            }
            Block::Proc(proc) => {
                let pinfo = proc.as_proc().unwrap();
                (pinfo.method, pinfo.outer, pinfo.self_val)
            }
            _ => unimplemented!(),
        };

        use MethodInfo::*;
        match &self.globals.methods[method] {
            BuiltinFunc { func, name, .. } => {
                let name = *name;
                let func = *func;
                for v in iter {
                    self.stack_push(self_value);
                    self.stack_push(v);
                    self.exec_native(&func, method, name, &args)?;
                }
            }
            RubyFunc { iseq } => {
                let iseq = *iseq;
                if iseq.opt_flag {
                    for v in iter {
                        self.stack_push(self_value);
                        self.stack_push(v);
                        self.push_block_frame_fast(iseq, &args, outer, false);
                        self.run_loop()?;
                    }
                } else {
                    for v in iter {
                        self.stack_push(self_value);
                        self.stack_push(v);
                        self.push_block_frame_slow(iseq, &args, outer, false)?;
                        self.run_loop()?;
                    }
                }
            }
            _ => unreachable!(),
        };

        Ok(default)
    }

    pub(crate) fn eval_block_map1_iter(
        &mut self,
        block: &Block,
        iter: impl Iterator<Item = Value>,
    ) -> VMResult {
        let args = Args2::new(1);
        let (method, outer, self_value) = match block {
            Block::Block(method, outer) => {
                let outer = self.cfp_from_frame(*outer).ep();
                (*method, outer, outer.self_value())
            }
            Block::Proc(proc) => {
                let pinfo = proc.as_proc().unwrap();
                (pinfo.method, pinfo.outer, pinfo.self_val)
            }
            _ => unimplemented!(),
        };

        use MethodInfo::*;
        let len = self.temp_len();
        match &self.globals.methods[method] {
            BuiltinFunc { func, name, .. } => {
                let name = *name;
                let func = *func;
                for v in iter {
                    self.stack_push(self_value);
                    self.stack_push(v);
                    let res = self.exec_native(&func, method, name, &args)?;
                    self.temp_push(res);
                }
            }
            RubyFunc { iseq } => {
                let iseq = *iseq;
                if iseq.opt_flag {
                    for v in iter {
                        self.stack_push(self_value);
                        self.stack_push(v);
                        self.push_block_frame_fast(iseq, &args, outer, false);
                        let res = self.run_loop()?;
                        self.temp_push(res);
                    }
                } else {
                    for v in iter {
                        self.stack_push(self_value);
                        self.stack_push(v);
                        self.push_block_frame_slow(iseq, &args, outer, false)?;
                        let res = self.run_loop()?;
                        self.temp_push(res);
                    }
                }
            }
            _ => unreachable!(),
        };

        Ok(Value::array_from(self.temp_pop_vec(len)))
    }

    pub(crate) fn eval_block_map1(
        &mut self,
        block: &Block,
    ) -> Box<dyn Fn(&mut VM, Value) -> VMResult> {
        let args = Args2::new(1);
        let (method, outer, self_value) = match block {
            Block::Block(method, outer) => {
                let outer = self.cfp_from_frame(*outer).ep();
                (*method, outer, outer.self_value())
            }
            Block::Proc(proc) => {
                let pinfo = proc.as_proc().unwrap();
                (pinfo.method, pinfo.outer, pinfo.self_val)
            }
            Block::Sym(sym) => {
                let sym = sym.clone();
                return Box::new(move |vm: &mut VM, v: Value| -> VMResult {
                    vm.eval_send0(sym, v)
                });
            }
        };

        use MethodInfo::*;
        match &self.globals.methods[method] {
            BuiltinFunc { func, name, .. } => {
                let name = *name;
                let func = *func;
                Box::new(move |vm: &mut VM, v: Value| -> VMResult {
                    vm.stack_push(self_value);
                    vm.stack_push(v);
                    vm.exec_native(&func, method, name, &args)
                })
            }
            RubyFunc { iseq } => {
                let iseq = *iseq;
                if iseq.opt_flag {
                    Box::new(move |vm: &mut VM, v: Value| -> VMResult {
                        vm.stack_push(self_value);
                        vm.stack_push(v);
                        vm.push_block_frame_fast(iseq, &args, outer, false);
                        vm.run_loop()
                    })
                } else {
                    Box::new(move |vm: &mut VM, v: Value| -> VMResult {
                        vm.stack_push(self_value);
                        vm.stack_push(v);
                        vm.push_block_frame_slow(iseq, &args, outer, false)?;
                        vm.run_loop()
                    })
                }
            }
            _ => unreachable!(),
        }
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
                let outer = self.cfp_from_frame(*outer).ep();
                self.stack_push(self_value);
                let args = self.stack_push_args(args);
                self.invoke_block(*method, outer, &args)?.handle(self)
            }
            Block::Proc(proc) => self.eval_proc(*proc, self_value, &args),
            _ => unimplemented!(),
        }
    }

    /// Evaluate the method with given `self_val`, `args` and no outer context.
    pub(crate) fn eval_method(
        &mut self,
        method: FnId,
        self_val: impl Into<Value>,
        slice: &[Value],
        args: &Args2,
    ) -> VMResult {
        let self_val = self_val.into();
        self.stack_push(self_val);
        self.stack.extend_from_slice(slice);
        self.invoke_method(method, args, true)?.handle(self)
    }

    pub(crate) fn eval_method_range(
        &mut self,
        method: FnId,
        self_val: impl Into<Value>,
        src: StackPtr,
        len: usize,
        args: &Args2,
    ) -> VMResult {
        let self_val = self_val.into();
        self.stack_push(self_val);
        self.stack.extend_from_within_ptr(src, len);
        self.invoke_method(method, args, true)?.handle(self)
    }

    /// Evaluate the method with given `self_val`, `args` and no outer context.
    pub(crate) fn eval_method0(&mut self, method: FnId, self_val: impl Into<Value>) -> VMResult {
        let self_val = self_val.into();
        let args = Args2::new(0);
        self.stack_push(self_val);
        self.invoke_method(method, &args, true)?.handle(self)
    }

    ///
    /// Evaluate `initialize` method if the method exists, with given `self_val` and `args`.
    /// If not, do nothing.
    ///
    pub(crate) fn eval_initialize(
        &mut self,
        rec_class: Module,
        self_val: Value,
        args: &Args2,
    ) -> Result<(), RubyError> {
        if let Some(method) = self
            .globals
            .methods
            .find_method(rec_class, IdentId::INITIALIZE)
        {
            let (src, len) = self.args_range();
            self.eval_method_range(method, self_val, src, len, args)?;
        }
        Ok(())
    }

    /// Execute the Proc object with given `args`, and push the returned value on the stack.
    pub(crate) fn eval_proc(
        &mut self,
        proc: Value,
        self_value: impl Into<Option<Value>>,
        args: &Args,
    ) -> VMResult {
        self.stack_push(Value::nil());
        let args = self.stack_push_args(&args);
        self.invoke_proc(proc, self_value, &args)?.handle(self)
    }

    pub fn eval_binding(&mut self, path: String, code: String, mut ctx: HeapCtxRef) -> VMResult {
        let id = self.parse_program_binding(path, code, ctx.as_ep())?;
        let iseq = self.globals.methods[id].as_iseq();
        ctx.set_iseq(iseq);
        self.push_block_frame_from_heap(ctx);
        let val = self.run_loop()?;
        Ok(val)
    }
}

impl VM {
    pub(super) fn invoke_method_missing(
        &mut self,
        method_name: IdentId,
        args: &Args2,
        use_value: bool,
    ) -> InvokeResult {
        let receiver = (self.sp() - args.len() - 1)[0];
        match self
            .globals
            .methods
            .find_method_from_receiver(receiver, IdentId::_METHOD_MISSING)
        {
            Some(method) => {
                let len = args.len();
                let new_args = Args2::new(len + 1);
                self.stack
                    .insert(self.sp() - len, Value::symbol(method_name));
                self.invoke_method(method, &new_args, use_value)
            }
            None => {
                if receiver.id() == self.self_value().id() {
                    Err(RubyError::name(format!(
                        "Undefined local variable or method `{:?}' for {:?}",
                        method_name, receiver
                    )))
                } else {
                    Err(VMError::undefined_method(method_name, receiver))
                }
            }
        }
    }

    pub(super) fn invoke_send0(&mut self, method_id: IdentId, receiver: Value) -> InvokeResult {
        self.stack_push(receiver);
        self.invoke_send(method_id, receiver, &Args2::new(0), true)
    }

    pub(super) fn invoke_send1(
        &mut self,
        method_id: IdentId,
        receiver: Value,
        arg0: Value,
    ) -> InvokeResult {
        self.stack_push(receiver);
        self.stack_push(arg0);
        self.invoke_send(method_id, receiver, &Args2::new(1), true)
    }

    pub(super) fn invoke_send2(
        &mut self,
        method_id: IdentId,
        receiver: Value,
        arg0: Value,
        arg1: Value,
        use_value: bool,
    ) -> InvokeResult {
        self.stack_push(receiver);
        self.stack_push(arg0);
        self.stack_push(arg1);
        self.invoke_send(method_id, receiver, &Args2::new(2), use_value)
    }

    pub(super) fn invoke_send(
        &mut self,
        method_id: IdentId,
        receiver: Value,
        args: &Args2,
        use_value: bool,
    ) -> InvokeResult {
        match self
            .globals
            .methods
            .find_method_from_receiver(receiver, method_id)
        {
            Some(method) => self.invoke_method(method, &args, use_value),
            None => self.invoke_method_missing(method_id, &args, use_value),
        }
    }

    /// Invoke the Proc object with given `args`.
    pub(super) fn invoke_proc(
        &mut self,
        proc: Value,
        self_value: impl Into<Option<Value>>,
        args: &Args2,
    ) -> InvokeResult {
        let pinfo = proc.as_proc().unwrap();
        let self_val = match self_value.into() {
            Some(v) => v,
            None => pinfo.self_val,
        };
        (self.sp() - args.len() - 1)[0] = self_val;
        self.invoke_block(pinfo.method, pinfo.outer, args) //TODO:lambda or proc
    }

    pub(super) fn invoke_sym_proc(&mut self, sym: IdentId, args: &Args2) -> InvokeResult {
        let len = args.len();
        if len == 0 {
            return Err(RubyError::argument("No receiver given."));
        }
        let receiver = (self.stack.sp - len)[0];
        let mut args = args.clone();
        args.set_len(len - 1);
        self.invoke_send(sym, receiver, &args, true)
    }

    // core methods

    /// Invoke the method with given `method_name`, `outer` context, and `args`, and push the returned value on the stack.
    pub(super) fn invoke_block(
        &mut self,
        fid: FnId,
        outer: EnvFrame,
        args: &Args2,
    ) -> InvokeResult {
        use MethodInfo::*;
        let val = match &self.globals.methods[fid] {
            BuiltinFunc { func, name, .. } => {
                let name = *name;
                let func = *func;
                self.exec_native(&func, fid, name, args)?
            }
            RubyFunc { iseq } => {
                let iseq = iseq.clone();
                if iseq.opt_flag {
                    self.push_block_frame_fast(iseq, args, outer, true);
                } else {
                    self.push_block_frame_slow(iseq, args, outer, true)?;
                }
                return Ok(VMResKind::Invoke);
            }
            _ => unreachable!(),
        };
        Ok(VMResKind::Return(val))
    }

    pub(super) fn invoke_method(
        &mut self,
        fid: FnId,
        args: &Args2,
        use_value: bool,
    ) -> InvokeResult {
        use MethodInfo::*;
        let val = match &self.globals.methods[fid] {
            BuiltinFunc { func, name, .. } => {
                let name = *name;
                let func = *func;
                self.exec_native(&func, fid, name, args)?
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
                let iseq = iseq.clone();
                if iseq.opt_flag {
                    return self.push_method_frame_fast(iseq, args, use_value);
                } else {
                    return self.push_method_frame_slow(iseq, args, use_value);
                }
            }
            _ => unreachable!(),
        };
        Ok(VMResKind::Return(val))
    }

    /// Invoke the method defined by Rust fn and push the returned value on the stack.
    pub(super) fn exec_native(
        &mut self,
        func: &BuiltinFunc,
        _method_id: FnId,
        _name: IdentId,
        args: &Args2,
    ) -> Result<Value, RubyError> {
        #[cfg(feature = "perf")]
        self.globals.perf.get_perf(Perf::EXTERN);

        #[cfg(feature = "trace")]
        {
            println!(
                "+++> BuiltinFunc self:{:?} name:{:?}",
                (self.sp() - args.len() - 1)[0],
                _name
            );
        }

        #[cfg(feature = "perf-method")]
        self.globals.methods.inc_counter(_method_id);

        let iseq = self.iseq;
        let pc = self.pc;
        let cfp = self.cfp;
        self.save_next_pc();
        self.push_native_frame(args.len());

        let temp_len = self.temp_len();
        let res = func(self, self.self_value(), args);
        self.temp_stack.truncate(temp_len);

        self.unwind_native_frame(cfp);
        self.pc = pc;
        self.iseq = iseq;

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
        let val = self.stack_pop();
        let mut self_val = self.stack_pop();
        match self_val.as_mut_rvalue() {
            Some(oref) => {
                oref.set_var(id, val);
                Ok(val)
            }
            None => unreachable!("AttrReader must be used only for class instance."),
        }
    }
}
