use super::*;

impl VM {
    pub(crate) fn push_block_frame_slow(
        &mut self,
        iseq: ISeqRef,
        args: &Args2,
        outer: EnvFrame,
        use_value: bool,
    ) -> Result<(), RubyError> {
        // This is necessary to follow moving outer frame to heap during iteration.
        let outer = outer.ep();
        let base = self.sp() - args.len();
        let params = &iseq.params;
        let kw_flag = !args.kw_arg.is_nil();
        let (_positional_kwarg, ordinary_kwarg) = if params.keyword.is_empty() && !params.kwrest {
            // Note that Ruby 3.0 doesn’t behave differently when calling a method which doesn’t accept keyword
            // arguments with keyword arguments.
            // For instance, the following case is not going to be deprecated and will keep working in Ruby 3.0.
            // The keyword arguments are still treated as a positional Hash argument.
            //
            // def foo(kwargs = {})
            //   kwargs
            // end
            // foo(k: 1) #=> {:k=>1}
            //
            // https://www.ruby-lang.org/en/news/2019/12/12/separation-of-positional-and-keyword-arguments-in-ruby-3-0/
            if kw_flag {
                self.stack_push(args.kw_arg);
            }
            (kw_flag, false)
        } else {
            (false, kw_flag)
        };

        self.prepare_block_args(base, iseq);
        self.fill_positional_arguments(base, iseq);
        // Handling keyword arguments and a keyword rest paramter.
        if params.kwrest || ordinary_kwarg {
            self.fill_keyword_arguments(base, iseq, args.kw_arg, ordinary_kwarg)?;
        };
        let local_len = (self.sp() - base) as usize;
        self.push_block_frame(
            base - 1,
            use_value,
            None,
            Some(outer),
            iseq,
            local_len,
            base.as_lfp(),
        );

        // Handling block paramter.
        if let Some(id) = iseq.lvar.block_param() {
            self.fill_block_argument(base, id, &args.block);
        }
        Ok(())
    }

    pub(crate) fn push_method_frame_slow(
        &mut self,
        iseq: ISeqRef,
        args: &Args2,
        use_value: bool,
    ) -> InvokeResult {
        let base_ptr = self.sp() - args.len();
        let params = &iseq.params;
        let kw_flag = !args.kw_arg.is_nil();
        let (positional_kwarg, ordinary_kwarg) = if params.keyword.is_empty() && !params.kwrest {
            // Note that Ruby 3.0 doesn’t behave differently when calling a method which doesn’t accept keyword
            // arguments with keyword arguments.
            // For instance, the following case is not going to be deprecated and will keep working in Ruby 3.0.
            // The keyword arguments are still treated as a positional Hash argument.
            //
            // def foo(kwargs = {})
            //   kwargs
            // end
            // foo(k: 1) #=> {:k=>1}
            //
            // https://www.ruby-lang.org/en/news/2019/12/12/separation-of-positional-and-keyword-arguments-in-ruby-3-0/
            if kw_flag {
                self.stack_push(args.kw_arg);
            }
            (kw_flag, false)
        } else {
            (false, kw_flag)
        };
        params.check_arity(positional_kwarg, args)?;
        self.fill_positional_arguments(base_ptr, iseq);
        // Handling keyword arguments and a keyword rest paramter.
        if params.kwrest || ordinary_kwarg {
            self.fill_keyword_arguments(base_ptr, iseq, args.kw_arg, ordinary_kwarg)?;
        };
        let local_len = (self.sp() - base_ptr) as usize;
        self.push_method_frame(use_value, iseq, local_len, &args.block);

        // Handling block paramter.
        if let Some(id) = iseq.lvar.block_param() {
            self.fill_block_argument(base_ptr, id, &args.block);
        }
        Ok(VMResKind::Invoke)
    }

    pub(crate) fn push_block_frame_fast(
        &mut self,
        iseq: ISeqRef,
        args: &Args2,
        outer: EnvFrame,
        use_value: bool,
    ) {
        // This is necessary to follow moving outer frame to heap during iteration.
        let outer = outer.ep();
        let base = self.sp() - args.len();
        let lvars = iseq.lvars;
        self.prepare_block_args(base, iseq);
        let args_len = (self.sp() - base) as usize;
        let req_len = iseq.params.req;
        if req_len < args_len {
            self.stack.sp = base + req_len;
        }

        self.stack.resize_to(base + lvars);

        let local_len = (self.sp() - base) as usize;
        self.push_block_frame(
            base - 1,
            use_value,
            None,
            Some(outer),
            iseq,
            local_len,
            base.as_lfp(),
        );
    }

    pub(crate) fn push_method_frame_fast(
        &mut self,
        iseq: ISeqRef,
        args: &Args2,
        use_value: bool,
    ) -> InvokeResult {
        let min = iseq.params.req;
        let len = args.len();
        if len != min {
            return Err(RubyError::argument_wrong(len, min));
        }
        let local_len = iseq.lvars;
        self.stack.grow(local_len - len);
        self.push_method_frame(use_value, iseq, local_len, &args.block);
        Ok(VMResKind::Invoke)
    }
}

impl VM {
    fn prepare_block_args(&mut self, base: StackPtr, iseq: ISeqRef) {
        // if a single Array argument is given for the block requiring multiple formal parameters,
        // the arguments must be expanded.
        if self.sp() - base == 1 && iseq.mularg_flag {
            if let Some(ary) = base[0].as_array() {
                self.stack.pop();
                self.stack.extend_from_slice(&**ary);
            }
        }
    }

    fn fill_positional_arguments(&mut self, mut base: StackPtr, iseq: ISeqRef) {
        let params = &iseq.params;
        let lvars = iseq.lvars;
        let args_len = (self.sp() - base) as usize;
        let req_len = params.req;
        let rest_len = if params.rest == Some(true) { 1 } else { 0 };
        let post_len = params.post;
        let no_post_len = args_len - post_len;
        let optreq_len = req_len + params.opt;

        if optreq_len < no_post_len {
            if let Some(delegate) = params.delegate {
                let v = base[optreq_len..no_post_len].to_vec();
                base[delegate.as_usize() as isize] = Value::array_from(v);
            }
            if rest_len == 1 {
                let ary = base[optreq_len..no_post_len].to_vec();
                base[optreq_len as isize] = Value::array_from(ary);
            }
            // fill post_req params.
            RubyStack::stack_copy_within(base, no_post_len..args_len, optreq_len + rest_len);
            self.stack.sp = base
                + optreq_len
                + rest_len
                + post_len
                + if params.delegate.is_some() { 1 } else { 0 };
            self.stack.resize_to(base + lvars);
        } else {
            self.stack.resize_to(base + lvars);
            // fill post_req params.
            RubyStack::stack_copy_within(base, no_post_len..args_len, optreq_len + rest_len);
            if no_post_len < req_len {
                // fill rest req params with nil.
                base[no_post_len..req_len].fill(Value::nil());
                // fill rest opt params with uninitialized.
                base[req_len..optreq_len].fill(Value::uninitialized());
            } else {
                // fill rest opt params with uninitialized.
                base[no_post_len..optreq_len].fill(Value::uninitialized());
            }
            if rest_len == 1 {
                base[(optreq_len) as isize] = Value::array_from(vec![]);
            }
        }

        iseq.lvar
            .kw
            .iter()
            .for_each(|id| base[(id.as_usize()) as isize] = Value::uninitialized());
    }

    fn fill_keyword_arguments(
        &mut self,
        mut base: StackPtr,
        iseq: ISeqRef,
        kw_arg: Value,
        ordinary_kwarg: bool,
    ) -> Result<(), RubyError> {
        let mut kwrest = FxIndexMap::default();
        if ordinary_kwarg {
            let keyword = kw_arg.as_hash().unwrap();
            for (k, v) in keyword.iter() {
                let id = k.as_symbol().unwrap();
                match iseq.params.keyword.get(&id) {
                    Some(lvar) => base[lvar.as_usize() as isize] = v,
                    None => {
                        if iseq.params.kwrest {
                            kwrest.insert(HashKey(k), v);
                        } else {
                            return Err(RubyError::argument("Undefined keyword."));
                        }
                    }
                };
            }
        };
        if let Some(id) = iseq.lvar.kwrest_param() {
            base[id.as_usize() as isize] = Value::hash_from_map(kwrest);
        }
        Ok(())
    }

    fn fill_block_argument(&mut self, mut base: StackPtr, id: LvarId, block: &Option<Block>) {
        base[id.as_usize() as isize] = block
            .as_ref()
            .map_or(Value::nil(), |block| self.create_proc(block));
    }
}
