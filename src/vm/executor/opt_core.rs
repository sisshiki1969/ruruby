use super::*;

impl VM {
    /// Run VM.
    ///
    /// return Ok(()) when
    /// - reached the end of the method or block.
    /// - `return` in method.
    /// - `next` in block AND outer of loops.
    ///
    /// return Err(err) when
    /// - `break`  in block or eval AND outer of loops.
    /// - `return` in block
    /// - raise error
    pub(crate) fn run_context_main(&mut self) -> Result<(), RubyError> {
        loop {
            self.gc();

            #[cfg(not(tarpaulin_include))]
            macro_rules! dispatch {
                ($eval:expr) => {
                    match $eval {
                        Ok(VMResKind::Invoke) => break,
                        Err(err) => match err.kind {
                            RubyErrorKind::BlockReturn => {}
                            RubyErrorKind::MethodReturn if self.cur_iseq().is_method() => {
                                let val = self.globals.val;
                                if self.is_called() {
                                    self.stack_push(val);
                                    return Ok(());
                                } else {
                                    self.unwind_frame();
                                    #[cfg(any(feature = "trace", feature = "trace-func"))]
                                    if self.globals.startup_flag {
                                        eprintln!("<--- Ok({:?})", self.globals.val);
                                    }
                                    self.stack_push(val);
                                    break;
                                }
                            }
                            RubyErrorKind::MethodReturn => {
                                return Err(err);
                            }
                            _ => return Err(err),
                        },
                        _ => {}
                    };
                };
            }

            #[cfg(not(tarpaulin_include))]
            macro_rules! cmp {
                ($eval:ident) => {{
                    self.inc_pc(1);
                    let val = Value::bool(self.$eval()?);
                    self.stack_push(val);
                }};
            }

            #[cfg(not(tarpaulin_include))]
            macro_rules! cmp_i {
                ($eval:ident) => {{
                    let idx = self.stack_len() - 1;
                    let lhs = self.exec_stack[idx];
                    let i = (self.pc + 1).read32() as i32;
                    self.inc_pc(5);
                    self.exec_stack[idx] = Value::bool(self.$eval(lhs, i)?);
                }};
            }

            #[cfg(not(tarpaulin_include))]
            macro_rules! jmp_cmp {
                ($eval:ident) => {{
                    let b = self.$eval()?;
                    self.jmp_cond(b, 5, 1);
                }};
            }

            #[cfg(not(tarpaulin_include))]
            macro_rules! jmp_cmp_i {
                ($eval:ident) => {{
                    let lhs = self.stack_pop();
                    let i = (self.pc + 1).read32() as i32;
                    let b = self.$eval(lhs, i)?;
                    self.jmp_cond(b, 9, 5);
                }};
            }

            loop {
                //self.cur_frame_pc_set(self.pc);
                #[cfg(feature = "perf")]
                self.globals.perf.get_perf(self.pc.read8());
                #[cfg(feature = "trace")]
                if self.globals.startup_flag {
                    let pc = self.pc_offset();
                    eprintln!(
                        "{:>4x}: {:<40} tmp: {:<4} stack: {:<3} top: {}",
                        pc,
                        Inst::inst_info(&self.globals, self.cur_iseq(), ISeqPos::from(pc)),
                        self.temp_stack.len(),
                        self.stack_len(),
                        match self.exec_stack.last() {
                            Some(x) => format!("{:?}", x),
                            None => "".to_string(),
                        }
                    );
                }
                match self.pc.read8() {
                    Inst::RETURN => {
                        // - reached the end of the method or block.
                        // - `return` in method.
                        // - `next` in block AND outer of loops.
                        if self.is_called() {
                            return Ok(());
                        } else {
                            let use_value = !self.discard_val();
                            self.unwind_continue(use_value);
                            break;
                        }
                    }
                    Inst::BREAK => {
                        // - `break`  in block or eval AND outer of loops.
                        #[cfg(debug_assertions)]
                        assert!(self.kind() == ISeqKind::Block || self.kind() == ISeqKind::Other);
                        self.globals.val = self.stack_pop();
                        let called = self.is_called();
                        self.unwind_frame();
                        if called {
                            let err = RubyError::block_return();
                            return Err(err);
                        } else {
                            #[cfg(any(feature = "trace", feature = "trace-func"))]
                            if self.globals.startup_flag {
                                eprintln!("<--- BlockReturn({:?})", self.globals.val);
                            }
                            break;
                        }
                    }
                    Inst::MRETURN => {
                        // - `return` in block
                        #[cfg(debug_assertions)]
                        assert!(self.kind() == ISeqKind::Block);
                        self.globals.val = self.stack_pop();
                        let err = RubyError::method_return();
                        return Err(err);
                    }
                    Inst::THROW => {
                        // - raise error
                        self.globals.val = self.stack_pop();
                        return Err(RubyError::value());
                    }
                    Inst::PUSH_NIL => {
                        self.inc_pc(1);
                        self.stack_push(Value::nil());
                    }
                    Inst::PUSH_SELF => {
                        self.inc_pc(1);
                        self.stack_push(self.self_value());
                    }
                    Inst::PUSH_VAL => {
                        let val = (self.pc + 1).read64();
                        self.inc_pc(9);
                        self.stack_push(Value::from(val));
                    }
                    Inst::ADD => {
                        self.inc_pc(1);
                        self.exec_add()?;
                    }
                    Inst::ADDI => {
                        let i = (self.pc + 1).read32() as i32;
                        self.inc_pc(5);
                        self.exec_addi(i)?;
                    }
                    Inst::SUB => {
                        self.inc_pc(1);
                        self.exec_sub()?;
                    }
                    Inst::SUBI => {
                        let i = (self.pc + 1).read32() as i32;
                        self.inc_pc(5);
                        self.exec_subi(i)?;
                    }
                    Inst::MUL => {
                        self.inc_pc(1);
                        self.exec_mul()?;
                    }
                    Inst::POW => {
                        self.inc_pc(1);
                        let (lhs, rhs) = self.stack_pop2();
                        self.exec_exp(rhs, lhs)?;
                    }
                    Inst::DIV => {
                        self.inc_pc(1);
                        self.exec_div()?;
                    }
                    Inst::REM => {
                        self.inc_pc(1);
                        let (lhs, rhs) = self.stack_pop2();
                        self.exec_rem(rhs, lhs)?;
                    }
                    Inst::SHR => {
                        self.inc_pc(1);
                        let (lhs, rhs) = self.stack_pop2();
                        self.exec_shr(rhs, lhs)?;
                    }
                    Inst::SHL => {
                        self.inc_pc(1);
                        let (lhs, rhs) = self.stack_pop2();
                        self.exec_shl(rhs, lhs)?;
                    }
                    Inst::NEG => {
                        self.inc_pc(1);
                        let lhs = self.stack_pop();
                        self.exec_neg(lhs)?;
                    }
                    Inst::BAND => {
                        self.inc_pc(1);
                        let (lhs, rhs) = self.stack_pop2();
                        self.exec_bitand(rhs, lhs)?;
                    }
                    Inst::BOR => {
                        self.inc_pc(1);
                        let (lhs, rhs) = self.stack_pop2();
                        self.exec_bitor(rhs, lhs)?;
                    }
                    Inst::BXOR => {
                        self.inc_pc(1);
                        let (lhs, rhs) = self.stack_pop2();
                        let val = self.eval_bitxor(rhs, lhs)?;
                        self.stack_push(val);
                    }
                    Inst::BNOT => {
                        self.inc_pc(1);
                        let lhs = self.stack_pop();
                        let val = self.eval_bitnot(lhs)?;
                        self.stack_push(val);
                    }

                    Inst::EQ => cmp!(eval_eq),
                    Inst::NE => cmp!(eval_ne),
                    Inst::GT => cmp!(eval_gt),
                    Inst::GE => cmp!(eval_ge),
                    Inst::LT => cmp!(eval_lt),
                    Inst::LE => cmp!(eval_le),
                    Inst::TEQ => {
                        self.inc_pc(1);
                        let (lhs, rhs) = self.stack_pop2();
                        self.exec_teq(rhs, lhs)?;
                    }
                    Inst::EQI => cmp_i!(eval_eqi),
                    Inst::NEI => cmp_i!(eval_nei),
                    Inst::GTI => cmp_i!(eval_gti),
                    Inst::GEI => cmp_i!(eval_gei),
                    Inst::LTI => cmp_i!(eval_lti),
                    Inst::LEI => cmp_i!(eval_lei),
                    Inst::CMP => {
                        self.inc_pc(1);
                        let (lhs, rhs) = self.stack_pop2();
                        let val = self.eval_compare(rhs, lhs)?;
                        self.stack_push(val);
                    }
                    Inst::NOT => {
                        self.inc_pc(1);
                        let lhs = self.stack_pop();
                        let val = Value::bool(!lhs.to_bool());
                        self.stack_push(val);
                    }
                    Inst::RESCUE => {
                        let len = (self.pc + 1).read32() as usize;
                        self.inc_pc(5);
                        let stack_len = self.exec_stack.len();
                        let val = self.exec_stack[stack_len - len - 1];
                        let ex = &self.exec_stack[stack_len - len..stack_len];
                        let b = self.eval_rescue(val, ex);
                        self.set_stack_len(stack_len - len - 1);
                        self.stack_push(Value::bool(b));
                    }
                    Inst::CONCAT_STRING => {
                        let num = (self.pc + 1).read32() as usize;
                        self.inc_pc(5);
                        let stack_len = self.stack_len();
                        let res = self
                            .exec_stack
                            .drain(stack_len - num..stack_len)
                            .fold(String::new(), |acc, x| acc + x.as_string().unwrap());

                        let val = Value::string(res);
                        self.stack_push(val);
                    }
                    Inst::SET_LOCAL => {
                        let id = (self.pc + 1).read_lvar_id();
                        self.inc_pc(5);
                        let val = self.stack_pop();
                        self.set_local(id, val);
                    }
                    Inst::GET_LOCAL => {
                        let id = (self.pc + 1).read_lvar_id();
                        self.inc_pc(5);
                        let val = self.get_local(id);
                        self.stack_push(val);
                    }
                    Inst::SET_DYNLOCAL => {
                        let id = (self.pc + 1).read_lvar_id();
                        let outer = (self.pc + 5).read32();
                        self.inc_pc(9);
                        let val = self.stack_pop();
                        self.set_dyn_local(id, outer, val);
                    }
                    Inst::GET_DYNLOCAL => {
                        let id = (self.pc + 1).read_lvar_id();
                        let outer = (self.pc + 5).read32();
                        self.inc_pc(9);
                        let val = self.get_dyn_local(id, outer);
                        self.stack_push(val);
                    }
                    Inst::CHECK_LOCAL => {
                        let id = (self.pc + 1).read_lvar_id();
                        let outer = (self.pc + 5).read32();
                        self.inc_pc(9);
                        let val = self.get_dyn_local(id, outer).is_uninitialized();
                        self.stack_push(Value::bool(val));
                    }
                    Inst::SET_CONST => {
                        let id = (self.pc + 1).read_id();
                        self.inc_pc(5);
                        let parent = match self.stack_pop() {
                            v if v.is_nil() => self
                                .get_method_iseq()
                                .class_defined
                                .last()
                                .cloned()
                                .unwrap_or_else(|| BuiltinClass::object()),
                            v => v.expect_mod_class()?,
                        };
                        let val = self.stack_pop();
                        self.globals.set_const(parent, id, val);
                    }
                    Inst::CHECK_CONST => {
                        let id = (self.pc + 1).read_id();
                        self.inc_pc(5);
                        let is_undef = self.find_const(id).is_err();
                        self.stack_push(Value::bool(is_undef));
                    }
                    Inst::GET_CONST => {
                        let id = (self.pc + 1).read_id();
                        let slot = (self.pc + 5).read32();
                        self.inc_pc(9);
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
                        let id = (self.pc + 1).read_id();
                        self.inc_pc(5);
                        let parent = BuiltinClass::object();
                        let val = self.get_scope(parent, id)?;
                        self.stack_push(val);
                    }
                    Inst::CHECK_SCOPE => {
                        let parent = self.stack_pop();
                        let id = (self.pc + 1).read_id();
                        self.inc_pc(5);
                        let is_undef = match parent.expect_mod_class() {
                            Ok(parent) => self.get_scope(parent, id).is_err(),
                            Err(_) => true,
                        };
                        self.stack_push(Value::bool(is_undef));
                    }
                    Inst::GET_SCOPE => {
                        let parent = self.stack_pop().expect_mod_class()?;
                        let id = (self.pc + 1).read_id();
                        self.inc_pc(5);
                        let val = self.get_scope(parent, id)?;
                        self.stack_push(val);
                    }
                    Inst::SET_IVAR => {
                        let var_id = (self.pc + 1).read_id();
                        self.inc_pc(5);
                        let new_val = self.stack_pop();
                        let self_value = self.self_value();
                        self_value.set_var(var_id, new_val);
                    }
                    Inst::GET_IVAR => {
                        let var_id = (self.pc + 1).read_id();
                        self.inc_pc(5);
                        let self_value = self.self_value();
                        let val = self_value.get_var(var_id).unwrap_or_default();
                        self.stack_push(val);
                    }
                    Inst::CHECK_IVAR => {
                        let var_id = (self.pc + 1).read_id();
                        self.inc_pc(5);
                        let self_value = self.self_value();
                        let val = Value::bool(self_value.get_var(var_id).is_none());
                        self.stack_push(val);
                    }
                    Inst::SET_GVAR => {
                        let var_id = (self.pc + 1).read_id();
                        self.inc_pc(5);
                        let new_val = self.stack_pop();
                        self.set_global_var(var_id, new_val);
                    }
                    Inst::GET_GVAR => {
                        let var_id = (self.pc + 1).read_id();
                        self.inc_pc(5);
                        let val = self.get_global_var(var_id).unwrap_or_default();
                        self.stack_push(val);
                    }
                    Inst::CHECK_GVAR => {
                        let var_id = (self.pc + 1).read_id();
                        self.inc_pc(5);
                        let val = Value::bool(self.get_global_var(var_id).is_none());
                        self.stack_push(val);
                    }
                    Inst::GET_SVAR => {
                        let var_id = (self.pc + 1).read32();
                        self.inc_pc(5);
                        let val = self.get_special_var(var_id);
                        self.stack_push(val);
                    }
                    Inst::SET_SVAR => {
                        let var_id = (self.pc + 1).read32();
                        self.inc_pc(5);
                        let new_val = self.stack_pop();
                        self.set_special_var(var_id, new_val)?;
                    }
                    Inst::SET_CVAR => {
                        let var_id = (self.pc + 1).read_id();
                        self.inc_pc(5);
                        let new_val = self.stack_pop();
                        self.set_class_var(var_id, new_val)?;
                    }
                    Inst::GET_CVAR => {
                        let var_id = (self.pc + 1).read_id();
                        self.inc_pc(5);
                        let val = self.get_class_var(var_id)?;
                        self.stack_push(val);
                    }
                    Inst::SET_INDEX => {
                        self.inc_pc(1);
                        dispatch!(self.invoke_set_index());
                    }
                    Inst::GET_INDEX => {
                        self.inc_pc(1);
                        let idx = self.stack_pop();
                        let receiver = self.stack_pop();
                        dispatch!(self.invoke_get_index(receiver, idx));
                    }
                    Inst::SET_IDX_I => {
                        let idx = (self.pc + 1).read32();
                        self.inc_pc(5);
                        dispatch!(self.invoke_set_index_imm(idx));
                    }
                    Inst::GET_IDX_I => {
                        let idx = (self.pc + 1).read32();
                        self.inc_pc(5);
                        let receiver = self.stack_pop();
                        dispatch!(self.invoke_get_index_imm(receiver, idx));
                    }
                    Inst::SPLAT => {
                        self.inc_pc(1);
                        let val = self.stack_pop();
                        let res = Value::splat(val);
                        self.stack_push(res);
                    }
                    Inst::CONST_VAL => {
                        let id = (self.pc + 1).read_usize();
                        self.inc_pc(5);
                        let val = self.globals.const_values.get(id);
                        self.stack_push(val);
                    }
                    Inst::CREATE_RANGE => {
                        self.inc_pc(1);
                        let start = self.stack_pop();
                        let end = self.stack_pop();
                        let exclude_end = self.stack_pop().to_bool();
                        let range = self.create_range(start, end, exclude_end)?;
                        self.stack_push(range);
                    }
                    Inst::CREATE_ARRAY => {
                        let arg_num = (self.pc + 1).read_usize();
                        self.inc_pc(5);
                        let elems = self.pop_args_to_vec(arg_num);
                        let array = Value::array_from(elems);
                        self.stack_push(array);
                    }
                    Inst::CREATE_PROC => {
                        let method = (self.pc + 1).read_method().unwrap();
                        self.inc_pc(5);
                        let proc_obj = self.create_proc_from_block(method, self.cur_frame());
                        self.stack_push(proc_obj);
                    }
                    Inst::CREATE_HASH => {
                        let arg_num = (self.pc + 1).read_usize();
                        self.inc_pc(5);
                        let key_value = self.pop_key_value_pair(arg_num);
                        let hash = Value::hash_from_map(key_value);
                        self.stack_push(hash);
                    }
                    Inst::CREATE_REGEXP => {
                        self.inc_pc(1);
                        let arg = self.stack_pop();
                        let regexp = self.create_regexp(arg)?;
                        self.stack_push(regexp);
                    }
                    Inst::JMP => {
                        let disp = (self.pc + 1).read_disp();
                        self.jump_pc(5, disp);
                    }
                    Inst::JMP_BACK => {
                        let disp = (self.pc + 1).read_disp();
                        self.gc();
                        self.jump_pc(5, disp);
                    }
                    Inst::JMP_F => {
                        let val = self.stack_pop();
                        let b = val.to_bool();
                        self.jmp_cond(b, 5, 1);
                    }
                    Inst::JMP_T => {
                        let val = self.stack_pop();
                        let b = !val.to_bool();
                        self.jmp_cond(b, 5, 1);
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
                        let map = self.globals.case_dispatch.get_entry((self.pc + 1).read32());
                        let disp = match map.get(&HashKey(val)) {
                            Some(disp) => *disp,
                            None => (self.pc + 5).read_disp(),
                        };
                        self.jump_pc(9, disp);
                    }
                    Inst::OPT_CASE2 => {
                        let val = self.stack_pop();
                        let disp = if let Some(i) = val.as_fixnum() {
                            let map = self
                                .globals
                                .case_dispatch2
                                .get_entry((self.pc + 1).read32());
                            if map.0 <= i && i <= map.1 {
                                map.2[(i - map.0) as usize]
                            } else {
                                (self.pc + 5).read_disp()
                            }
                        } else {
                            (self.pc + 5).read_disp()
                        };
                        self.jump_pc(9, disp);
                    }
                    Inst::CHECK_METHOD => {
                        let receiver = self.stack_pop();
                        let method = (self.pc + 1).read_id();
                        self.inc_pc(5);
                        let rec_class = receiver.get_class_for_method();
                        let is_undef = rec_class.search_method(method).is_none();
                        self.stack_push(Value::bool(is_undef));
                    }
                    Inst::SEND => {
                        let receiver = self.stack_pop();
                        dispatch!(self.vm_send(receiver));
                    }
                    Inst::SEND_SELF => {
                        dispatch!(self.vm_send(None));
                    }
                    Inst::OPT_SEND => {
                        dispatch!(self.vm_fast_send(true));
                    }
                    Inst::OPT_SEND_SELF => {
                        let receiver = self.self_value();
                        self.stack_push(receiver);
                        dispatch!(self.vm_fast_send(true));
                    }
                    Inst::OPT_SEND_N => {
                        dispatch!(self.vm_fast_send(false));
                    }
                    Inst::OPT_SEND_SELF_N => {
                        let receiver = self.self_value();
                        self.stack_push(receiver);
                        dispatch!(self.vm_fast_send(false));
                    }
                    Inst::YIELD => {
                        let args_num = (self.pc + 1).read32() as usize;
                        self.inc_pc(5);
                        let args = self.pop_args_to_args(args_num);
                        dispatch!(self.vm_yield(&args));
                    }
                    Inst::SUPER => {
                        let args_num = (self.pc + 1).read32() as usize;
                        let _block = (self.pc + 3).read_method();
                        let flag = (self.pc + 7).read8() == 1;
                        self.inc_pc(8);
                        let self_value = self.self_value();
                        dispatch!(self.vm_super(self_value, args_num, flag));
                    }
                    Inst::DEF_CLASS => {
                        let is_module = (self.pc + 1).read8() == 1;
                        let id = (self.pc + 2).read_id();
                        let method = (self.pc + 6).read_method().unwrap();
                        self.inc_pc(10);
                        let base = self.stack_pop();
                        let super_val = self.stack_pop();
                        let val = self.define_class(base, id, is_module, super_val)?;
                        let mut iseq = method.as_iseq(&self.globals);
                        iseq.class_defined = self.get_class_defined(val);
                        assert!(iseq.is_classdef());
                        self.stack_push(val.into());
                        dispatch!(self.invoke_method(method, &Args2::new(0)));
                    }
                    Inst::DEF_SCLASS => {
                        let method = (self.pc + 1).read_method().unwrap();
                        self.inc_pc(5);
                        let singleton = self.stack_pop().get_singleton_class()?;
                        let mut iseq = method.as_iseq(&self.globals);
                        iseq.class_defined = self.get_class_defined(singleton);
                        assert!(iseq.is_classdef());
                        self.stack_push(singleton.into());
                        dispatch!(self.invoke_method(method, &Args2::new(0)));
                    }
                    Inst::DEF_METHOD => {
                        let id = (self.pc + 1).read_id();
                        let method = (self.pc + 5).read_method().unwrap();
                        self.inc_pc(9);
                        let mut iseq = method.as_iseq(&self.globals);
                        iseq.class_defined = self.get_method_iseq().class_defined.clone();
                        let self_value = self.self_value();
                        self.define_method(self_value, id, method);
                        if self.is_module_function() {
                            self.define_singleton_method(self_value, id, method)?;
                        }
                    }
                    Inst::DEF_SMETHOD => {
                        let id = (self.pc + 1).read_id();
                        let method = (self.pc + 5).read_method().unwrap();
                        self.inc_pc(9);
                        let mut iseq = method.as_iseq(&self.globals);
                        iseq.class_defined = self.get_method_iseq().class_defined.clone();
                        let singleton = self.stack_pop();
                        self.define_singleton_method(singleton, id, method)?;
                        if self.is_module_function() {
                            self.define_method(singleton, id, method);
                        }
                    }
                    Inst::TO_S => {
                        let val = self.stack_pop();
                        self.inc_pc(1);
                        let s = val.val_to_s(self)?;
                        let res = Value::string(s);
                        self.stack_push(res);
                    }
                    Inst::POP => {
                        self.stack_pop();
                        self.inc_pc(1);
                    }
                    Inst::DUP => {
                        let len = (self.pc + 1).read_usize();
                        self.inc_pc(5);
                        let stack_len = self.stack_len();
                        self.exec_stack
                            .extend_from_within(stack_len - len..stack_len);
                    }
                    Inst::SINKN => {
                        let len = (self.pc + 1).read_usize();
                        self.inc_pc(5);
                        let val = self.stack_pop();
                        let stack_len = self.stack_len();
                        self.exec_stack.insert(stack_len - len, val);
                    }
                    Inst::TOPN => {
                        let len = (self.pc + 1).read_usize();
                        self.inc_pc(5);
                        let val = self.exec_stack.remove(self.stack_len() - 1 - len);
                        self.stack_push(val);
                    }
                    Inst::TAKE => {
                        let len = (self.pc + 1).read_usize();
                        self.inc_pc(5);
                        let val = self.stack_pop();
                        match val.as_array() {
                            Some(info) => {
                                let elem = &info.elements;
                                let ary_len = elem.len();
                                if len <= ary_len {
                                    self.exec_stack.extend_from_slice(&elem[0..len]);
                                } else {
                                    self.exec_stack.extend_from_slice(&elem[0..ary_len]);
                                    self.exec_stack
                                        .resize(self.stack_len() + len - ary_len, Value::nil());
                                }
                            }
                            None => {
                                self.stack_push(val);
                                self.exec_stack
                                    .resize(self.stack_len() + len - 1, Value::nil());
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
    fn unwind_continue(&mut self, use_value: bool) {
        let val = self.stack_pop();
        self.unwind_frame();
        #[cfg(any(feature = "trace", feature = "trace-func"))]
        if self.globals.startup_flag {
            eprintln!("<--- Ok({:?})", val);
        }
        if use_value {
            self.stack_push(val);
        }
    }

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
                for h in kwsplat.as_array().unwrap().elements.iter() {
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
            } else {
                if val.as_proc().is_some() {
                    Some(val.into())
                } else {
                    let res = self.eval_send0(IdentId::get_id("to_proc"), val)?;
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
            }
        } else {
            None
        };
        Ok(block)
    }

    /// ### receiver
    /// if None, use self value of the current contest as receiver.
    ///
    /// ### return value
    /// - VMResKind::Return
    /// continue current context
    /// - VMResKind::Invoke
    /// new context
    fn vm_fast_send(&mut self, use_value: bool) -> Result<VMResKind, RubyError> {
        // In the case of Without keyword/block/splat/delegate arguments.
        let receiver = self.stack_top();
        let method_name = (self.pc + 1).read_id();
        let args_num = (self.pc + 5).read16();
        let block = (self.pc + 7).read32();
        let cache_id = (self.pc + 11).read32();
        self.inc_pc(15);
        let block = if block != 0 {
            Some(Block::Block(block.into(), self.cur_frame()))
        } else {
            None
        };
        let args = Args2::new_with_block(args_num as usize, block);

        let rec_class = receiver.get_class_for_method();
        match self
            .globals
            .methods
            .find_method_inline_cache(cache_id, rec_class, method_name)
        {
            Some(method) => {
                //self.invoke_func(method, None, &args, use_value)
                use MethodInfo::*;
                let val = match self.globals.methods.get(method) {
                    BuiltinFunc { func, name, .. } => {
                        let name = *name;
                        let func = *func;
                        self.exec_native(&func, method, name, &args)?
                    }
                    AttrReader { id } => {
                        args.check_args_num(0)?;
                        let id = *id;
                        self.exec_getter(id)?
                    }
                    AttrWriter { id } => {
                        args.check_args_num(1)?;
                        let id = *id;
                        self.exec_setter(id)?
                    }
                    RubyFunc { iseq } => {
                        let iseq = *iseq;
                        if iseq.opt_flag {
                            self.push_method_frame_fast(
                                iseq,
                                &args,
                                use_value,
                                args.block.as_ref(),
                            )?;
                        } else {
                            self.push_frame(iseq, &args, None, use_value)?;
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
            None => self.invoke_method_missing(method_name, &args, use_value),
        }
    }

    fn vm_send(&mut self, receiver: impl Into<Option<Value>>) -> Result<VMResKind, RubyError> {
        let method_name = (self.pc + 1).read_id();
        let args_num = (self.pc + 5).read16();
        let flag = (self.pc + 7).read_argflag();
        let block = (self.pc + 8).read32();
        let cache_id = (self.pc + 12).read32();
        let receiver = receiver.into().unwrap_or_else(|| self.self_value());
        self.inc_pc(16);
        self.do_send(receiver, method_name, flag, block, args_num, cache_id, true)
    }

    ///
    /// Stack layout of arguments
    ///
    /// +------+------+--+------+------+------+-------+
    /// | arg0 | args |..| argn |  kw  |hashsp| block |
    /// +------+------+--+------+------+------+-------+
    ///
    /// argx:   arguments
    /// kw:     [optional] keyword arguments (Hash object)
    /// hashsp: [optional] hash splat arguments (Array of Hash object)
    /// block:  [optional] block argument
    ///
    fn do_send(
        &mut self,
        receiver: Value,
        method_name: IdentId,
        flag: ArgFlag,
        block: u32,
        args_num: u16,
        cache_id: u32,
        use_value: bool,
    ) -> Result<VMResKind, RubyError> {
        let block = self.handle_block_arg(block, flag)?;
        let keyword = self.handle_hash_args(flag)?;
        let mut args = self.pop_args_to_args(args_num as usize);
        if flag.has_delegate() {
            match self.cur_delegate() {
                Some(v) => {
                    let ary = &v
                        .as_array()
                        .expect("Delegate arg must be Array or nil.")
                        .elements;
                    args.append(ary);
                    self.stack_append(ary);
                }
                None => {}
            }
        }
        args.block = block;
        args.kw_arg = keyword;

        let rec_class = receiver.get_class_for_method();
        self.stack_push(receiver);
        match self
            .globals
            .methods
            .find_method_inline_cache(cache_id, rec_class, method_name)
        {
            Some(method) => self.invoke_func(method, None, &args, use_value),
            None => self.invoke_method_missing(method_name, &args, use_value),
        }
    }

    fn vm_super(
        &mut self,
        self_value: Value,
        args_num: usize,
        flag: bool,
    ) -> Result<VMResKind, RubyError> {
        // TODO: support keyword parameter, etc..
        let iseq = self.get_method_iseq();
        if let ISeqKind::Method(Some(m_id)) = iseq.kind {
            let class = self_value.get_class_for_method();
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
            let args = if flag {
                let param_num = iseq.params.param_ident.len();
                for i in 0..param_num {
                    self.stack_push(self.get_local(LvarId::from(i)));
                }
                Args2::new(args_num + param_num)
            } else {
                self.pop_args_to_args(args_num)
            };
            self.stack_push(self_value);
            self.invoke_method(method, &args)
        } else {
            return Err(RubyError::nomethod("super called outside of method"));
        }
    }

    /// Invoke the block given to the method with `args`.
    fn vm_yield(&mut self, args: &Args2) -> Result<VMResKind, RubyError> {
        match &self.get_method_block() {
            Some(Block::Block(method, outer)) => {
                self.stack_push(self.frame_self(*outer));
                self.invoke_func(*method, Some((*outer).into()), args, true)
            }
            Some(Block::Proc(proc)) => self.invoke_proc(*proc, None, args),
            None => return Err(RubyError::local_jump("No block given.")),
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
