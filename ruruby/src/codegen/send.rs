use super::*;

impl Codegen {
    ///
    /// Stack layout of arguments
    ///
    /// +------+------+------+--+------+------+------+-------+
    /// | self | arg0 | args |..| argn |  kw  |hashsp| block |
    /// +------+------+------+--+------+------+------+-------+
    ///
    /// argx:   arguments
    /// kw:     [optional] keyword arguments (Hash object)
    /// hashsp: [optional] hash splat arguments (Array of Hash object)
    /// block:  [optional] block argument
    ///
    pub(crate) fn gen_send(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        receiver: Node,
        method: IdentId,
        arglist: ArgList,
        safe_nav: bool,
        use_value: bool,
        loc: Loc,
    ) -> Result<(), RubyError> {
        // push receiver.
        if NodeKind::SelfValue == receiver.kind {
            iseq.gen_push_self();
        } else {
            self.gen(globals, iseq, receiver.clone(), true)?;
        }
        //let loc = self.loc;
        let delegate_flag = arglist.delegate;
        let hash_len = arglist.hash_splat.len();
        // push positional args.
        let args_num = arglist.args.len();
        let splat_flag = self.gen_nodes_check_splat(globals, iseq, arglist.args)?;
        // push keyword args as a Hash.
        let kw_args_len = arglist.kw_args.len();
        let kw_flag = kw_args_len != 0;
        if kw_flag {
            for (id, default) in arglist.kw_args {
                iseq.gen_val(Value::symbol(id));
                self.gen(globals, iseq, default, true)?;
            }
            iseq.gen_create_hash(kw_args_len);
        }
        // push double splat args.
        if hash_len > 0 {
            for arg in arglist.hash_splat {
                self.gen(globals, iseq, arg, true)?;
            }
            iseq.gen_create_array(hash_len);
        }
        let (block_ref, block_flag) = self.get_block(globals, iseq, arglist.block)?;
        // If the method call without block nor keyword/block/splat/double splat arguments, gen OPT_SEND.
        if !block_flag && !kw_flag && !splat_flag && hash_len == 0 && !delegate_flag {
            if safe_nav {
                iseq.gen_dup(1);
                iseq.gen_push_nil();
                iseq.push(Inst::NE);
                let src = iseq.gen_jmp_if_f();
                self.emit_opt_send(globals, iseq, method, args_num, block_ref, use_value, loc);
                iseq.write_disp_from_cur(src);
            } else {
                self.emit_opt_send(globals, iseq, method, args_num, block_ref, use_value, loc);
            }
        } else {
            let flag = ArgFlag::new(kw_flag, block_flag, delegate_flag, hash_len > 0, splat_flag);
            self.emit_send(globals, iseq, method, args_num, flag, block_ref, loc);

            if !use_value {
                iseq.gen_pop()
            };
        }

        Ok(())
    }

    pub(crate) fn gen_send_with_splat(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        method: IdentId,
        args_num: usize,
        has_splat: bool,
        use_value: bool,
        loc: Loc,
    ) {
        if has_splat {
            self.emit_send(globals, iseq, method, args_num, ArgFlag::splat(), None, loc);
            if !use_value {
                iseq.gen_pop();
            }
        } else {
            self.emit_opt_send(globals, iseq, method, args_num, None, use_value, loc);
        }
    }

    pub(crate) fn gen_nodes_check_splat(
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
        globals: &mut Globals,
        iseq: &mut ISeq,
        method: IdentId,
        args_num: usize,
        flag: ArgFlag,
        block: Option<FnId>,
        loc: Loc,
    ) {
        iseq.push(Inst::SEND);
        iseq.push32(method.into());
        iseq.push16(args_num as u32 as u16);
        iseq.push_argflag(flag);
        iseq.push_method(block);
        iseq.push32(globals.methods.add_inline_cache_entry());
        self.save_loc(iseq, loc);
    }

    // If the method call without block nor keyword/block/splat/double splat arguments, gen OPT_SEND.
    pub(crate) fn emit_opt_send(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        method: IdentId,
        args_num: usize,
        block: Option<FnId>,
        use_value: bool,
        loc: Loc,
    ) {
        if use_value {
            iseq.push(Inst::OPT_SEND);
        } else {
            iseq.push(Inst::OPT_SEND_N);
        };
        iseq.push32(method.into());
        iseq.push16(args_num as u32 as u16);
        iseq.push_method(block);
        iseq.push32(globals.methods.add_inline_cache_entry());
        self.save_loc(iseq, loc);
    }
}
