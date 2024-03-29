use super::*;
impl VM {
    /// VM main loop.
    ///
    /// return Ok(val) when
    /// - reached the end of the method or block.
    /// - `return` in method.
    /// - `next` in block AND outer of loops.
    ///
    /// return Err(err) when
    /// - `break`  in block or eval AND outer of loops.
    /// - `return` in block
    /// - exception was raised
    #[inline(always)]
    pub(crate) fn run_context_main(&mut self, invoke_count: &mut usize) -> VMResult {
        // Reach this point when a Ruby method/block was 'call'ed.
        loop {
            // Reach this point when a Ruby method/block was 'invoke'ed/'call'ed,
            // or returned from a Ruby method/block.
            self.checked_gc();
            let self_val = self.self_value();

            #[cfg(not(tarpaulin_include))]
            macro_rules! dispatch {
                ($eval:expr, $use_value:expr) => {
                    match $eval {
                        Ok(VMResKind::Invoke) => {
                            *invoke_count += 1;
                            break;
                        }
                        Ok(VMResKind::Return(v)) => {
                            if $use_value {
                                self.stack_push(v);
                            }
                        }
                        Err(err) => match err.kind {
                            RubyErrorKind::BlockReturn => {}
                            RubyErrorKind::MethodReturn if self.iseq.is_method() => {
                                let val = self.globals.val;
                                if *invoke_count == 0 {
                                    return Ok(val);
                                } else {
                                    self.unwind_frame();
                                    *invoke_count -= 1;
                                    #[cfg(feature = "trace")]
                                    eprintln!("<--- Ok({:?})", self.globals.val);
                                    self.stack_push(val);
                                    break;
                                }
                            }
                            RubyErrorKind::MethodReturn => {
                                return Err(err);
                            }
                            _ => return Err(err),
                        },
                    }
                };
            }

            #[cfg(not(tarpaulin_include))]
            macro_rules! cmp {
                ($eval:ident) => {{
                    let val = Value::bool(self.$eval()?);
                    self.stack_push(val);
                }};
            }

            #[cfg(not(tarpaulin_include))]
            macro_rules! cmp_i {
                ($eval:ident) => {{
                    let lhs = self.stack_pop();
                    let i = self.pc.read32() as i32;
                    let v = Value::bool(self.$eval(lhs, i)?);
                    self.stack_push(v);
                }};
            }

            #[cfg(not(tarpaulin_include))]
            macro_rules! jmp_cmp {
                ($eval:ident) => {{
                    let b = self.$eval()?;
                    self.jmp_cond(b);
                }};
            }

            #[cfg(not(tarpaulin_include))]
            macro_rules! jmp_cmp_i {
                ($eval:ident) => {{
                    let lhs = self.stack_pop();
                    let i = self.pc.read32() as i32;
                    let b = self.$eval(lhs, i)?;
                    self.jmp_cond(b);
                }};
            }

            loop {
                #[cfg(feature = "perf")]
                {
                    self.globals.perf.get_perf(self.pc.fetch8());
                }
                #[cfg(feature = "trace")]
                {
                    if self.globals.startup_flag {
                        let pc = self.pc_offset();
                        eprintln!(
                            "{:0>5}: {:<40} tmp:{:<3} stack:{:<5} top:{:?}",
                            pc.into_usize(),
                            self.globals.inst_info(self.iseq, pc),
                            self.temp_stack.len(),
                            self.stack_len(),
                            self.stack.last(),
                        );
                    }
                }
                match self.pc.read8() {
                    Inst::RETURN => {
                        // - reached the end of the method or block.
                        // - `return` in method.
                        // - `next` in block AND outer of loops.
                        if *invoke_count == 0 {
                            return Ok(self.stack_pop());
                        } else {
                            let use_value = !self.discard_val();
                            let val = self.stack_pop();
                            self.unwind_frame();
                            *invoke_count -= 1;
                            #[cfg(feature = "trace")]
                            eprintln!("<--- Ok({:?})", val);
                            if use_value {
                                self.stack_push(val);
                            }
                            break;
                        }
                    }
                    Inst::BREAK => {
                        // - `break`  in block or eval AND outer of loops.
                        debug_assert!(
                            self.kind() == ISeqKind::Block || self.kind() == ISeqKind::Other
                        );
                        self.globals.val = self.stack_pop();
                        self.unwind_frame();
                        if *invoke_count == 0 {
                            let err = RubyError::block_return();
                            return Err(err);
                        } else {
                            *invoke_count -= 1;
                            #[cfg(feature = "trace")]
                            eprintln!("<--- BlockReturn({:?})", self.globals.val);
                            break;
                        }
                    }
                    Inst::MRETURN => {
                        // - `return` in block
                        debug_assert!(self.kind() == ISeqKind::Block);
                        self.globals.val = self.stack_pop();
                        let err = RubyError::method_return();
                        return Err(err);
                    }
                    Inst::THROW => {
                        // - raise error
                        self.globals.val = self.stack_pop();
                        self.pc -= 1;
                        return Err(RubyError::value());
                    }
                    Inst::PUSH_NIL => self.stack_push(Value::nil()),
                    Inst::PUSH_SELF => self.stack_push(self_val),
                    Inst::PUSH_VAL => {
                        let val = self.pc.read64();
                        self.stack_push(Value::from(val));
                    }
                    Inst::ADD => dispatch!(self.invoke_add(), true),
                    Inst::ADDI => {
                        let i = self.pc.read32() as i32;
                        dispatch!(self.invoke_addi(i), true);
                    }
                    Inst::SUB => dispatch!(self.invoke_sub(), true),
                    Inst::SUBI => {
                        let i = self.pc.read32() as i32;
                        dispatch!(self.invoke_subi(i), true);
                    }
                    Inst::MUL => dispatch!(self.invoke_mul(), true),
                    Inst::POW => dispatch!(self.invoke_exp(), true),
                    Inst::DIV => dispatch!(self.invoke_div(), true),
                    Inst::REM => dispatch!(self.invoke_rem(), true),
                    Inst::SHR => dispatch!(self.invoke_shr(), true),
                    Inst::SHL => dispatch!(self.invoke_shl(), true),

                    Inst::NEG => dispatch!(self.invoke_neg(), true),
                    Inst::BAND => dispatch!(self.invoke_bitand(), true),

                    Inst::BOR => dispatch!(self.invoke_bitor(), true),
                    Inst::BXOR => dispatch!(self.invoke_bitxor(), true),
                    Inst::BNOT => dispatch!(self.invoke_bitnot(), true),

                    Inst::EQ => cmp!(eval_eq),
                    Inst::NE => cmp!(eval_ne),
                    Inst::GT => cmp!(eval_gt),
                    Inst::GE => cmp!(eval_ge),
                    Inst::LT => cmp!(eval_lt),
                    Inst::LE => cmp!(eval_le),
                    Inst::TEQ => dispatch!(self.invoke_teq(), true),
                    Inst::EQI => cmp_i!(eval_eqi),
                    Inst::NEI => cmp_i!(eval_nei),
                    Inst::GTI => cmp_i!(eval_gti),
                    Inst::GEI => cmp_i!(eval_gei),
                    Inst::LTI => cmp_i!(eval_lti),
                    Inst::LEI => cmp_i!(eval_lei),
                    Inst::CMP => {
                        let (lhs, rhs) = self.stack_pop2();
                        dispatch!(self.invoke_compare(rhs, lhs), true)
                    }
                    Inst::NOT => {
                        let lhs = self.stack_pop();
                        let val = Value::bool(!lhs.to_bool());
                        self.stack_push(val);
                    }
                    Inst::RESCUE => {
                        let len = self.pc.read32() as usize;
                        let val = (self.stack.sp - len - 1)[0];
                        let ex = &(self.stack.sp - len)[0..len];
                        let b = self.eval_rescue(val, ex);
                        self.stack.sp -= len + 1;
                        self.stack_push(Value::bool(b));
                    }
                    Inst::CONCAT_STRING => {
                        let num = self.pc.read32() as usize;
                        let res = (self.stack.sp - num)[0..num]
                            .iter()
                            .fold(String::new(), |acc, x| acc + x.as_string().unwrap());
                        self.stack.sp -= num;

                        let val = Value::string(res);
                        self.stack_push(val);
                    }
                    Inst::SET_LOCAL => {
                        let id = self.pc.read_lvar_id();
                        let val = self.stack_pop();
                        self.lfp[id] = val;
                    }
                    Inst::GET_LOCAL => {
                        let id = self.pc.read_lvar_id();
                        let val = self.lfp[id];
                        self.stack_push(val);
                    }
                    Inst::SET_DYNLOCAL => {
                        let id = self.pc.read_lvar_id();
                        let outer = self.pc.read32();
                        let val = self.stack_pop();
                        self.get_dyn_local(outer)[id] = val;
                    }
                    Inst::GET_DYNLOCAL => {
                        let id = self.pc.read_lvar_id();
                        let outer = self.pc.read32();
                        let val = self.get_dyn_local(outer)[id];
                        self.stack_push(val);
                    }
                    Inst::CHECK_LOCAL => {
                        let id = self.pc.read_lvar_id();
                        let outer = self.pc.read32();
                        let val = self.get_dyn_local(outer)[id].is_uninitialized();
                        self.stack_push(Value::bool(val));
                    }
                    Inst::SET_CONST => {
                        let id = self.pc.read_id();
                        let object = self.globals.classes.object;
                        let parent = match self.stack_pop() {
                            v if v.is_nil() => self
                                .get_method_iseq()
                                .class_defined
                                .last()
                                .cloned()
                                .unwrap_or_else(|| object),
                            v => v.expect_mod_class()?,
                        };
                        let val = self.stack_pop();
                        self.globals.set_const(parent, id, val);
                    }
                    Inst::CHECK_CONST => {
                        let id = self.pc.read_id();
                        let is_undef = self.find_const(id).is_err();
                        self.stack_push(Value::bool(is_undef));
                    }
                    Inst::GET_CONST => {
                        let id = self.pc.read_id();
                        let slot = self.pc.read32();
                        let val = match self.globals.find_const_cache(slot) {
                            Some(val) => val,
                            None => {
                                let val = self.find_const(id)?;
                                self.globals.set_const_cache(slot, val);
                                val
                            }
                        };
                        self.stack_push(val);
                    }
                    Inst::GET_CONST_TOP => {
                        let id = self.pc.read_id();
                        let parent = self.globals.classes.object;
                        let val = self.get_scope(parent, id)?;
                        self.stack_push(val);
                    }
                    Inst::CHECK_SCOPE => {
                        let parent = self.stack_pop();
                        let id = self.pc.read_id();
                        let is_undef = match parent.expect_mod_class() {
                            Ok(parent) => self.get_scope(parent, id).is_err(),
                            Err(_) => true,
                        };
                        self.stack_push(Value::bool(is_undef));
                    }
                    Inst::GET_SCOPE => {
                        let parent = self.stack_pop().expect_mod_class()?;
                        let id = self.pc.read_id();
                        let val = self.get_scope(parent, id)?;
                        self.stack_push(val);
                    }
                    Inst::SET_IVAR => {
                        let var_id = self.pc.read_id();
                        let new_val = self.stack_pop();
                        self_val.set_var(var_id, new_val);
                    }
                    Inst::GET_IVAR => {
                        let var_id = self.pc.read_id();
                        let val = self_val.get_var(var_id).unwrap_or_default();
                        self.stack_push(val);
                    }
                    Inst::CHECK_IVAR => {
                        let var_id = self.pc.read_id();
                        let val = Value::bool(self_val.get_var(var_id).is_none());
                        self.stack_push(val);
                    }
                    Inst::SET_GVAR => {
                        let var_id = self.pc.read_id();
                        let new_val = self.stack_pop();
                        self.set_global_var(var_id, new_val);
                    }
                    Inst::GET_GVAR => {
                        let var_id = self.pc.read_id();
                        let val = self.get_global_var(var_id).unwrap_or_default();
                        self.stack_push(val);
                    }
                    Inst::CHECK_GVAR => {
                        let var_id = self.pc.read_id();
                        let val = Value::bool(self.get_global_var(var_id).is_none());
                        self.stack_push(val);
                    }
                    Inst::GET_SVAR => {
                        let var_id = self.pc.read32();
                        let val = self.get_special_var(var_id);
                        self.stack_push(val);
                    }
                    Inst::SET_SVAR => {
                        let var_id = self.pc.read32();
                        let new_val = self.stack_pop();
                        self.set_special_var(var_id, new_val)?;
                    }
                    Inst::SET_CVAR => {
                        let var_id = self.pc.read_id();
                        let new_val = self.stack_pop();
                        self.set_class_var(var_id, new_val)?;
                    }
                    Inst::GET_CVAR => {
                        let var_id = self.pc.read_id();
                        let val = self.get_class_var(var_id)?;
                        self.stack_push(val);
                    }
                    Inst::SET_INDEX => {
                        dispatch!(self.invoke_set_index(), false);
                    }
                    Inst::GET_INDEX => {
                        let idx = self.stack_pop();
                        let receiver = self.stack_pop();
                        dispatch!(self.invoke_get_index(receiver, idx), true);
                    }
                    Inst::SET_IDX_I => {
                        let idx = self.pc.read32();
                        dispatch!(self.invoke_set_index_imm(idx), false);
                    }
                    Inst::GET_IDX_I => {
                        let idx = self.pc.read32();
                        let receiver = self.stack_pop();
                        dispatch!(self.invoke_get_index_imm(receiver, idx), true);
                    }
                    Inst::SPLAT => {
                        let val = self.stack_pop();
                        let res = Value::splat(val);
                        self.stack_push(res);
                    }
                    Inst::CONST_VAL => {
                        let id = self.pc.read_usize();
                        let val = self.globals.const_values.get(id);
                        self.stack_push(val);
                    }
                    Inst::CREATE_RANGE => {
                        let start = self.stack_pop();
                        let end = self.stack_pop();
                        let exclude_end = self.stack_pop().to_bool();
                        let range = self.create_range(start, end, exclude_end)?;
                        self.stack_push(range);
                    }
                    Inst::CREATE_ARRAY => {
                        let arg_num = self.pc.read_usize();
                        let array = self.pop_args_to_array(arg_num);
                        self.stack_push(array);
                    }
                    Inst::CREATE_PROC => {
                        let method = self.pc.read_method().unwrap();
                        let proc_obj = Value::procobj(self, self_val, method, self.cfp);
                        self.stack_push(proc_obj);
                    }
                    Inst::CREATE_HASH => {
                        let arg_num = self.pc.read_usize();
                        let key_value = self.pop_key_value_pair(arg_num);
                        let hash = Value::hash_from_map(key_value);
                        self.stack_push(hash);
                    }
                    Inst::CREATE_REGEXP => {
                        let arg = self.stack_pop();
                        let regexp = self.create_regexp(arg)?;
                        self.stack_push(regexp);
                    }
                    Inst::JMP => {
                        let disp = self.pc.read_disp();
                        self.pc += disp;
                    }
                    Inst::JMP_BACK => {
                        let disp = self.pc.read_disp();
                        self.checked_gc();
                        self.pc += disp;
                    }
                    Inst::JMP_F => {
                        let val = self.stack_pop();
                        let b = val.to_bool();
                        self.jmp_cond(b);
                    }
                    Inst::JMP_T => {
                        let val = self.stack_pop();
                        let b = !val.to_bool();
                        self.jmp_cond(b);
                    }

                    Inst::JMP_F_EQ => jmp_cmp!(eval_eq),
                    Inst::JMP_F_NE => jmp_cmp!(eval_ne),
                    Inst::JMP_F_GT => jmp_cmp!(eval_gt),
                    Inst::JMP_F_GE => jmp_cmp!(eval_ge),
                    Inst::JMP_F_LT => jmp_cmp!(eval_lt),
                    Inst::JMP_F_LE => jmp_cmp!(eval_le),

                    Inst::JMP_F_EQI => jmp_cmp_i!(eval_eqi),
                    Inst::JMP_F_NEI => jmp_cmp_i!(eval_nei),
                    Inst::JMP_F_GTI => jmp_cmp_i!(eval_gti),
                    Inst::JMP_F_GEI => jmp_cmp_i!(eval_gei),
                    Inst::JMP_F_LTI => jmp_cmp_i!(eval_lti),
                    Inst::JMP_F_LEI => jmp_cmp_i!(eval_lei),

                    Inst::OPT_CASE => {
                        let val = self.stack_pop();
                        let entry = self.pc.read32();
                        let map = self.globals.case_dispatch.get_entry(entry);
                        let default = self.pc.read_disp();
                        let disp = match map.get(&HashKey(val)) {
                            Some(disp) => *disp,
                            None => default,
                        };
                        self.pc += disp;
                    }
                    Inst::OPT_CASE2 => {
                        let val = self.stack_pop();
                        let entry = self.pc.read32();
                        let default = self.pc.read_disp();
                        let disp = if let Some(i) = val.as_fixnum() {
                            let map = self.globals.case_dispatch2.get_entry(entry);
                            if map.0 <= i && i <= map.1 {
                                map.2[(i - map.0) as usize]
                            } else {
                                default
                            }
                        } else {
                            default
                        };
                        self.pc += disp;
                    }
                    Inst::CHECK_METHOD => {
                        let receiver = self.stack_pop();
                        let method = self.pc.read_id();
                        let rec_class = self.globals.get_class_for_method(receiver);
                        let is_undef = rec_class.search_method(method).is_none();
                        self.stack_push(Value::bool(is_undef));
                    }
                    Inst::SEND => dispatch!(self.vm_send(), true),
                    Inst::OPT_SEND => dispatch!(self.vm_fast_send(true), true),
                    Inst::OPT_SEND_N => dispatch!(self.vm_fast_send(false), false),
                    Inst::YIELD => {
                        let args_num = self.pc.read32() as usize;
                        let args = self.pop_args_to_args(args_num);
                        dispatch!(self.vm_yield(&args), true);
                    }
                    Inst::SUPER => {
                        let args_num = self.pc.read16() as usize;
                        let _block = self.pc.read_method();
                        let flag = self.pc.read8() == 1;
                        //let self_value = self.self_value();
                        dispatch!(self.vm_super(self_val, args_num, flag), true);
                    }
                    Inst::DEF_CLASS => {
                        let is_module = self.pc.read8() == 1;
                        let id = self.pc.read_id();
                        let method = self.pc.read_method().unwrap();
                        let base = self.stack_pop();
                        let super_val = self.stack_pop();
                        let val = self.define_class(base, id, is_module, super_val)?;
                        let mut iseq = self.globals.methods[method].as_iseq();
                        iseq.class_defined = self.get_class_defined(val);
                        debug_assert!(iseq.is_classdef());
                        self.stack_push(val.into());
                        dispatch!(self.invoke_method(method, &Args2::new(0), true), true);
                    }
                    Inst::DEF_SCLASS => {
                        let method = self.pc.read_method().unwrap();
                        let singleton = self.stack_pop().get_singleton_class()?;
                        let mut iseq = self.globals.methods[method].as_iseq();
                        iseq.class_defined = self.get_class_defined(singleton);
                        debug_assert!(iseq.is_classdef());
                        self.stack_push(singleton.into());
                        dispatch!(self.invoke_method(method, &Args2::new(0), true), true);
                    }
                    Inst::DEF_METHOD => {
                        let id = self.pc.read_id();
                        let method = self.pc.read_method().unwrap();
                        let mut iseq = self.globals.methods[method].as_iseq();
                        iseq.class_defined = self.get_method_iseq().class_defined.clone();
                        //let self_value = self.self_value();
                        self.define_method(self_val, id, method);
                        if self.is_module_function() {
                            self.define_singleton_method(self_val, id, method)?;
                        }
                    }
                    Inst::DEF_SMETHOD => {
                        let id = self.pc.read_id();
                        let method = self.pc.read_method().unwrap();
                        let mut iseq = self.globals.methods[method].as_iseq();
                        iseq.class_defined = self.get_method_iseq().class_defined.clone();
                        let singleton = self.stack_pop();
                        self.define_singleton_method(singleton, id, method)?;
                        if self.is_module_function() {
                            self.define_method(singleton, id, method);
                        }
                    }
                    Inst::TO_S => {
                        let val = self.stack_pop();

                        let s = val.val_to_s(self)?;
                        let res = Value::string(s);
                        self.stack_push(res);
                    }
                    Inst::POP => {
                        self.stack_pop();
                    }
                    Inst::DUP => {
                        let len = self.pc.read_usize();
                        self.stack.extend_from_within_ptr(self.sp() - len, len);
                    }
                    Inst::SINKN => {
                        let len = self.pc.read_usize();
                        let val = self.stack_pop();
                        self.stack.insert(self.sp() - len, val);
                    }
                    Inst::TOPN => {
                        let len = self.pc.read_usize();
                        let val = self.stack.remove(self.sp() - 1 - len);
                        self.stack_push(val);
                    }
                    Inst::TAKE => {
                        let len = self.pc.read_usize();
                        let val = self.stack_pop();
                        match val.as_array() {
                            Some(info) => {
                                //let elem = &info.elements;
                                let ary_len = info.len();
                                if len <= ary_len {
                                    self.stack.extend_from_slice(&info[0..len]);
                                } else {
                                    self.stack.extend_from_slice(&info[0..ary_len]);
                                    self.stack.grow(len - ary_len);
                                }
                            }
                            None => {
                                self.stack_push(val);
                                self.stack.grow(len - 1);
                            }
                        }
                    }
                    inst => {
                        return Err(RubyError::internal(format!(
                            "Unimplemented instruction. {}",
                            Inst::inst_name(inst)
                        )))
                    }
                }
            }
        }
    }
}

// helper functions for run_context_main.
impl VM {
    /// Merge keyword args and hash splat args.
    fn handle_hash_args(&mut self, flag: ArgFlag) -> VMResult {
        if !flag.has_hash_arg() && !flag.has_hash_splat() {
            Ok(Value::nil())
        } else {
            let kwsplat = if flag.has_hash_splat() {
                Some(self.stack_pop())
            } else {
                None
            };
            let mut kw = if flag.has_hash_arg() {
                self.stack_pop()
            } else {
                Value::hash_from_map(FxIndexMap::default())
            };
            let hash = kw.as_mut_hash().unwrap();
            if let Some(kwsplat) = kwsplat {
                for h in kwsplat.as_array().unwrap().iter() {
                    for (k, v) in h.expect_hash("Arg")? {
                        hash.insert(k, v);
                    }
                }
            }
            Ok(kw)
        }
    }

    fn handle_block_arg(&mut self, block: u32, flag: ArgFlag) -> Result<Option<Block>, RubyError> {
        let block = if block != 0 {
            Some(Block::Block(block.into(), self.cur_frame()))
        } else if flag.has_block_arg() {
            let val = self.stack_pop();
            if val.is_nil() {
                None
            } else if val.as_proc().is_some() {
                Some(val.into())
            } else if let Some(id) = val.as_symbol() {
                Some(id.into())
            } else {
                let res = self.eval_send0(IdentId::get_id("to_proc"), val)?;
                self.temp_push(res);
                if res.as_proc().is_none() {
                    return Err(RubyError::internal(format!(
                        "Must be Proc. {:?}:{}",
                        val,
                        val.get_class_name()
                    )));
                } else {
                    Some(res.into())
                }
            }
        } else {
            None
        };
        Ok(block)
    }

    /// ### return value
    /// - VMResKind::Return
    /// continue current context
    /// - VMResKind::Invoke
    /// new context
    fn vm_fast_send(&mut self, use_value: bool) -> InvokeResult {
        // In the case of Without keyword/block/splat/delegate arguments.
        let method_name = self.pc.read_id();
        let args_num = self.pc.read16() as usize;
        let block = self.pc.read32();
        let cache_id = self.pc.read32();
        let args = if block != 0 {
            Args2::new_with_block(args_num, Block::Block(block.into(), self.cur_frame()))
        } else {
            Args2::new(args_num as usize)
        };

        self.send(
            method_name,
            (self.sp() - args_num - 1)[0],
            &args,
            use_value,
            cache_id,
        )
    }

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
    fn vm_send(&mut self) -> InvokeResult {
        let method_name = self.pc.read_id();
        let args_num = self.pc.read16() as usize;
        let flag = self.pc.read_argflag();
        let block = self.pc.read32();
        let cache_id = self.pc.read32();
        let use_value = true;
        let block = self.handle_block_arg(block, flag)?;
        let keyword = self.handle_hash_args(flag)?;
        let receiver = (self.sp() - args_num - 1)[0];
        let mut args = if flag.has_splat() {
            self.pop_args_to_args(args_num)
        } else {
            Args2::new(args_num)
        };
        if flag.has_delegate() {
            if let Some(v) = self.cur_delegate() {
                let ary = &**v.as_array().expect("Delegate arg must be Array or nil.");
                args.append(ary);
                self.stack.extend_from_slice(ary);
            }
        }
        args.block = block;
        args.kw_arg = keyword;
        self.send(method_name, receiver, &args, use_value, cache_id)
    }

    fn send(
        &mut self,
        method_name: IdentId,
        receiver: Value,
        args: &Args2,
        use_value: bool,
        cache_id: u32,
    ) -> InvokeResult {
        let rec_class = self.globals.get_class_for_method(receiver);
        match self
            .globals
            .methods
            .find_method_inline_cache(cache_id, rec_class, method_name)
        {
            Some(method) => self.invoke_method(method, &args, use_value),
            None => self.invoke_method_missing(method_name, &args, use_value),
        }
    }

    fn vm_super(
        &mut self,
        self_value: Value,
        args_num: usize,
        delegate_flag: bool,
    ) -> InvokeResult {
        // TODO: support keyword parameter, etc..
        let iseq = self.get_method_iseq();
        if let ISeqKind::Method(Some(m_id)) = iseq.kind {
            let class = self.globals.get_class_for_method(self_value);
            let method = class
                .superclass()
                .map(|class| self.globals.methods.find_method(class, m_id))
                .flatten()
                .ok_or_else(|| {
                    RubyError::nomethod(format!(
                        "no superclass method `{:?}' for {:?}.",
                        m_id, self_value
                    ))
                })?;
            let args = if delegate_flag {
                // When `super` has no arguments, use arguments which were passed to the current method.
                let param_num = iseq.params.param_ident.len();
                for i in 0..param_num as isize {
                    self.stack_push(self.lfp[i]);
                }
                Args2::new(args_num + param_num)
            } else {
                self.pop_args_to_args(args_num)
            };
            self.invoke_method(method, &args, true)
        } else {
            Err(RubyError::nomethod("super called outside of method"))
        }
    }

    /// Invoke the block given to the method with `args`.
    fn vm_yield(&mut self, args: &Args2) -> InvokeResult {
        match &self.get_method_block() {
            Some(Block::Block(method, outer)) => {
                let outer = self.cfp_from_frame(*outer).ep();
                self.stack
                    .insert(self.sp() - args.len(), outer.self_value());
                self.invoke_block(*method, outer, args)
            }
            Some(Block::Proc(proc)) => {
                self.stack.insert(self.sp() - args.len(), Value::nil());
                self.invoke_proc(*proc, None, args)
            }
            Some(Block::Sym(sym)) => self.invoke_sym_proc(*sym, args),
            None => Err(RubyError::local_jump("No block given.")),
        }
    }
}

impl VM {
    #[inline]
    pub(crate) fn sort_by<T, F>(
        &mut self,
        vec: &mut Vec<T>,
        mut compare: F,
    ) -> Result<(), RubyError>
    where
        F: FnMut(&mut VM, &T, &T) -> Result<std::cmp::Ordering, RubyError>,
    {
        self.merge_sort(vec, |vm, a, b| {
            Ok(compare(vm, a, b)? == std::cmp::Ordering::Less)
        })
    }

    fn merge_sort<T, F>(&mut self, v: &mut [T], mut is_less: F) -> Result<(), RubyError>
    where
        F: FnMut(&mut VM, &T, &T) -> Result<bool, RubyError>,
    {
        // Slices of up to this length get sorted using insertion sort.
        const MAX_INSERTION: usize = 20;
        // Very short runs are extended using insertion sort to span at least this many elements.
        const MIN_RUN: usize = 10;

        let len = v.len();

        // Short arrays get sorted in-place via insertion sort to avoid allocations.
        if len <= MAX_INSERTION {
            if len >= 2 {
                for i in (0..len - 1).rev() {
                    self.insert_head(&mut v[i..], &mut is_less)?;
                }
            }
            return Ok(());
        }

        let mut buf = Vec::with_capacity(len / 2);
        let mut runs = vec![];
        let mut end = len;
        while end > 0 {
            // Find the next natural run, and reverse it if it's strictly descending.
            let mut start = end - 1;
            if start > 0 {
                start -= 1;
                unsafe {
                    if is_less(self, v.get_unchecked(start + 1), v.get_unchecked(start))? {
                        while start > 0
                            && is_less(self, v.get_unchecked(start), v.get_unchecked(start - 1))?
                        {
                            start -= 1;
                        }
                        v[start..end].reverse();
                    } else {
                        while start > 0
                            && !is_less(self, v.get_unchecked(start), v.get_unchecked(start - 1))?
                        {
                            start -= 1;
                        }
                    }
                }
            }

            // Insert some more elements into the run if it's too short. Insertion sort is faster than
            // merge sort on short sequences, so this significantly improves performance.
            while start > 0 && end - start < MIN_RUN {
                start -= 1;
                self.insert_head(&mut v[start..end], &mut is_less)?;
            }

            // Push this run onto the stack.
            runs.push(Run {
                start,
                len: end - start,
            });
            end = start;

            // Merge some pairs of adjacent runs to satisfy the invariants.
            while let Some(r) = collapse(&runs) {
                let left = runs[r + 1];
                let right = runs[r];
                self.merge(
                    &mut v[left.start..right.start + right.len],
                    left.len,
                    buf.as_mut_ptr(),
                    &mut is_less,
                )?;
                runs[r] = Run {
                    start: left.start,
                    len: left.len + right.len,
                };
                runs.remove(r + 1);
            }
        }

        debug_assert!(runs.len() == 1 && runs[0].start == 0 && runs[0].len == len);

        return Ok(());

        #[inline]
        fn collapse(runs: &[Run]) -> Option<usize> {
            let n = runs.len();
            if n >= 2
                && (runs[n - 1].start == 0
                    || runs[n - 2].len <= runs[n - 1].len
                    || (n >= 3 && runs[n - 3].len <= runs[n - 2].len + runs[n - 1].len)
                    || (n >= 4 && runs[n - 4].len <= runs[n - 3].len + runs[n - 2].len))
            {
                if n >= 3 && runs[n - 3].len < runs[n - 1].len {
                    Some(n - 3)
                } else {
                    Some(n - 2)
                }
            } else {
                None
            }
        }

        #[derive(Clone, Copy)]
        struct Run {
            start: usize,
            len: usize,
        }
    }

    fn insert_head<T, F>(&mut self, v: &mut [T], is_less: &mut F) -> Result<(), RubyError>
    where
        F: FnMut(&mut VM, &T, &T) -> Result<bool, RubyError>,
    {
        if v.len() >= 2 && is_less(self, &v[1], &v[0])? {
            unsafe {
                let mut tmp = std::mem::ManuallyDrop::new(std::ptr::read(&v[0]));

                // initially held exactly once.
                let mut hole = InsertionHole {
                    src: &mut *tmp,
                    dest: &mut v[1],
                };
                std::ptr::copy_nonoverlapping(&v[1], &mut v[0], 1);

                for i in 2..v.len() {
                    if !is_less(self, &v[i], &*tmp)? {
                        break;
                    }
                    std::ptr::copy_nonoverlapping(&v[i], &mut v[i - 1], 1);
                    hole.dest = &mut v[i];
                }
                // `hole` gets dropped and thus copies `tmp` into the remaining hole in `v`.
            }
        }
        return Ok(());

        // When dropped, copies from `src` into `dest`.
        struct InsertionHole<T> {
            src: *mut T,
            dest: *mut T,
        }

        impl<T> Drop for InsertionHole<T> {
            fn drop(&mut self) {
                unsafe {
                    std::ptr::copy_nonoverlapping(self.src, self.dest, 1);
                }
            }
        }
    }

    fn merge<T, F>(
        &mut self,
        v: &mut [T],
        mid: usize,
        buf: *mut T,
        is_less: &mut F,
    ) -> Result<(), RubyError>
    where
        F: FnMut(&mut VM, &T, &T) -> Result<bool, RubyError>,
    {
        let len = v.len();
        let v = v.as_mut_ptr();
        let (v_mid, v_end) = unsafe { (v.add(mid), v.add(len)) };

        let mut hole;

        if mid <= len - mid {
            // The left run is shorter.
            unsafe {
                std::ptr::copy_nonoverlapping(v, buf, mid);
                hole = MergeHole {
                    start: buf,
                    end: buf.add(mid),
                    dest: v,
                };
            }

            // Initially, these pointers point to the beginnings of their arrays.
            let left = &mut hole.start;
            let mut right = v_mid;
            let out = &mut hole.dest;

            while *left < hole.end && right < v_end {
                // Consume the lesser side.
                // If equal, prefer the left run to maintain stability.
                unsafe {
                    let to_copy = if is_less(self, &*right, &**left)? {
                        get_and_increment(&mut right)
                    } else {
                        get_and_increment(left)
                    };
                    std::ptr::copy_nonoverlapping(to_copy, get_and_increment(out), 1);
                }
            }
        } else {
            // The right run is shorter.
            unsafe {
                std::ptr::copy_nonoverlapping(v_mid, buf, len - mid);
                hole = MergeHole {
                    start: buf,
                    end: buf.add(len - mid),
                    dest: v_mid,
                };
            }

            // Initially, these pointers point past the ends of their arrays.
            let left = &mut hole.dest;
            let right = &mut hole.end;
            let mut out = v_end;

            while v < *left && buf < *right {
                // Consume the greater side.
                // If equal, prefer the right run to maintain stability.
                unsafe {
                    let to_copy = if is_less(self, &*right.offset(-1), &*left.offset(-1))? {
                        decrement_and_get(left)
                    } else {
                        decrement_and_get(right)
                    };
                    std::ptr::copy_nonoverlapping(to_copy, decrement_and_get(&mut out), 1);
                }
            }
        };
        return Ok(());
        // Finally, `hole` gets dropped. If the shorter run was not fully consumed, whatever remains of
        // it will now be copied into the hole in `v`.

        unsafe fn get_and_increment<T>(ptr: &mut *mut T) -> *mut T {
            let old = *ptr;
            *ptr = ptr.offset(1);
            old
        }

        unsafe fn decrement_and_get<T>(ptr: &mut *mut T) -> *mut T {
            *ptr = ptr.offset(-1);
            *ptr
        }

        // When dropped, copies the range `start..end` into `dest..`.
        struct MergeHole<T> {
            start: *mut T,
            end: *mut T,
            dest: *mut T,
        }

        impl<T> Drop for MergeHole<T> {
            fn drop(&mut self) {
                // `T` is not a zero-sized type, so it's okay to divide by its size.
                let len = (self.end as usize - self.start as usize) / std::mem::size_of::<T>();
                unsafe {
                    std::ptr::copy_nonoverlapping(self.start, self.dest, len);
                }
            }
        }
    }
}
