use super::*;

impl Codegen {
    pub fn gen_send(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        receiver: Node,
        method: IdentId,
        arglist: ArgList,
        safe_nav: bool,
        use_value: bool,
    ) -> Result<(), RubyError> {
        let loc = self.loc;
        let mut no_splat_flag = true;
        let kwrest_len = arglist.kw_rest.len();
        // push positional args.
        let args_len = arglist.args.len();
        for arg in arglist.args {
            if let NodeKind::Splat(_) = arg.kind {
                no_splat_flag = false;
            };
            self.gen(globals, iseq, arg, true)?;
        }
        // push keword args as a Hash.
        let kw_args_len = arglist.kw_args.len();
        let kw_flag = kw_args_len != 0;
        if kw_flag {
            for (id, default) in arglist.kw_args {
                iseq.gen_val(Value::symbol(id));
                self.gen(globals, iseq, default, true)?;
            }
            iseq.gen_create_hash(kw_args_len);
        }
        // push keyword rest args.
        for arg in arglist.kw_rest {
            self.gen(globals, iseq, arg, true)?;
        }
        let (block_ref, block_flag) = self.get_block(globals, iseq, arglist.block)?;
        // If the method call without block nor keyword/block/splat/double splat arguments, gen OPT_SEND.
        if !block_flag && !kw_flag && no_splat_flag && kwrest_len == 0 {
            if NodeKind::SelfValue == receiver.kind {
                self.loc = loc;
                self.emit_opt_send_self(iseq, method, args_len, block_ref, use_value);
                return Ok(());
            } else {
                self.gen(globals, iseq, receiver, true)?;
                if safe_nav {
                    iseq.gen_dup(1);
                    iseq.gen_push_nil();
                    iseq.push(Inst::NE);
                    let src = iseq.gen_jmp_if_f();
                    self.loc = loc;
                    self.emit_opt_send(iseq, method, args_len, block_ref, use_value);
                    iseq.write_disp_from_cur(src);
                    return Ok(());
                } else {
                    self.loc = loc;
                    self.emit_opt_send(iseq, method, args_len, block_ref, use_value);
                    return Ok(());
                }
            }
        } else {
            if NodeKind::SelfValue == receiver.kind {
                self.loc = loc;
                self.emit_send_self(
                    iseq,
                    method,
                    args_len,
                    kwrest_len,
                    ArgFlag::new(kw_flag, block_flag),
                    block_ref,
                );
            } else {
                self.gen(globals, iseq, receiver, true)?;
                self.loc = loc;
                self.emit_send(
                    iseq,
                    method,
                    args_len,
                    kwrest_len,
                    ArgFlag::new(kw_flag, block_flag),
                    block_ref,
                );
            }
        };
        if !use_value {
            iseq.gen_pop()
        };
        Ok(())
    }

    pub fn gen_send_with_splat(
        &mut self,
        iseq: &mut ISeq,
        method: IdentId,
        args_num: usize,
        has_splat: bool,
        use_value: bool,
    ) {
        if has_splat {
            self.emit_send(iseq, method, args_num, 0, ArgFlag::default(), None);
            if !use_value {
                iseq.gen_pop();
            }
        } else {
            self.emit_opt_send(iseq, method, args_num, None, use_value);
        }
    }

    pub fn gen_nodes_check_splat(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        nodes: Vec<Node>,
    ) -> Result<bool, RubyError> {
        let mut has_splat = false;
        for i in nodes {
            if i.is_splat() {
                has_splat = true
            }
            self.gen(globals, iseq, i, true)?;
        }
        Ok(has_splat)
    }
}

impl Codegen {
    fn emit_send(
        &mut self,
        iseq: &mut ISeq,
        method: IdentId,
        args_num: usize,
        kw_rest_num: usize,
        flag: ArgFlag,
        block: Option<MethodId>,
    ) {
        self.save_cur_loc(iseq);
        iseq.push(Inst::SEND);
        iseq.push32(method.into());
        iseq.push16(args_num as u32 as u16);
        iseq.push8(kw_rest_num as u32 as u16 as u8);
        iseq.push_argflag(flag);
        iseq.push_method(block);
        iseq.push32(MethodRepo::add_inline_cache_entry());
    }

    fn emit_send_self(
        &mut self,
        iseq: &mut ISeq,
        method: IdentId,
        args_num: usize,
        kw_rest_num: usize,
        flag: ArgFlag,
        block: Option<MethodId>,
    ) {
        self.save_cur_loc(iseq);
        iseq.push(Inst::SEND_SELF);
        iseq.push32(method.into());
        iseq.push16(args_num as u32 as u16);
        iseq.push8(kw_rest_num as u32 as u16 as u8);
        iseq.push_argflag(flag);
        iseq.push_method(block);
        iseq.push32(MethodRepo::add_inline_cache_entry());
    }

    // If the method call without block nor keyword/block/splat/double splat arguments, gen OPT_SEND.
    pub fn emit_opt_send(
        &mut self,
        iseq: &mut ISeq,
        method: IdentId,
        args_num: usize,
        block: Option<MethodId>,
        use_value: bool,
    ) {
        self.save_cur_loc(iseq);
        if use_value {
            iseq.push(Inst::OPT_SEND);
        } else {
            iseq.push(Inst::OPT_SEND_N);
        };
        iseq.push32(method.into());
        iseq.push16(args_num as u32 as u16);
        iseq.push_method(block);
        iseq.push32(MethodRepo::add_inline_cache_entry());
    }

    pub fn emit_opt_send_self(
        &mut self,
        iseq: &mut ISeq,
        method: IdentId,
        args_num: usize,
        block: Option<MethodId>,
        use_value: bool,
    ) {
        self.save_cur_loc(iseq);
        if use_value {
            iseq.push(Inst::OPT_SEND_SELF);
        } else {
            iseq.push(Inst::OPT_SEND_SELF_N);
        };
        iseq.push32(method.into());
        iseq.push16(args_num as u32 as u16);
        iseq.push_method(block);
        iseq.push32(MethodRepo::add_inline_cache_entry());
        /*if !use_value {
            iseq.gen_pop();
        }*/
    }
}
