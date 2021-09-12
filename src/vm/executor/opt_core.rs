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
    pub fn run_context_main(&mut self) -> Result<(), RubyError> {
        loop {
            self.gc();
            let iseq = &self.context().iseq_ref.iseq;

            #[cfg(not(tarpaulin_include))]
            macro_rules! dispatch {
                ($eval:expr) => {
                    match $eval {
                        Ok(VMResKind::Invoke) => break,
                        Err(err) => match err.kind {
                            RubyErrorKind::BlockReturn => {}
                            RubyErrorKind::MethodReturn if self.is_method() => {
                                let val = self.globals.error_register;
                                if self.called() {
                                    self.stack_push(val);
                                    return Ok(());
                                } else {
                                    self.unwind_context();
                                    #[cfg(any(feature = "trace", feature = "trace-func"))]
                                    if self.globals.startup_flag {
                                        eprintln!("<--- Ok({:?})", self.globals.error_register);
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
                    self.pc += 1;
                    let val = Value::bool(self.$eval()?);
                    self.stack_push(val);
                }};
            }

            #[cfg(not(tarpaulin_include))]
            macro_rules! cmp_i {
                ($eval:ident) => {{
                    let idx = self.stack_len() - 1;
                    let lhs = self.exec_stack[idx];
                    let i = iseq.read32(self.pc + 1) as i32;
                    self.pc += 5;
                    self.exec_stack[idx] = Value::bool(self.$eval(lhs, i)?);
                }};
            }

            #[cfg(not(tarpaulin_include))]
            macro_rules! jmp_cmp {
                ($eval:ident) => {{
                    let b = self.$eval()?;
                    self.jmp_cond(iseq, b, 5, 1);
                }};
            }

            #[cfg(not(tarpaulin_include))]
            macro_rules! jmp_cmp_i {
                ($eval:ident) => {{
                    let lhs = self.stack_pop();
                    let i = iseq.read32(self.pc + 1) as i32;
                    let b = self.$eval(lhs, i)?;
                    self.jmp_cond(iseq, b, 9, 5);
                }};
            }

            loop {
                self.context().cur_pc = self.pc;
                #[cfg(feature = "perf")]
                self.globals.perf.get_perf(iseq[self.pc]);
                #[cfg(feature = "trace")]
                if self.globals.startup_flag {
                    eprintln!(
                        "{:>4x}: {:<40} tmp: {:<4} stack: {:<3} top: {}",
                        self.pc.into_usize(),
                        Inst::inst_info(&self.globals, self.context().iseq_ref, self.pc),
                        self.temp_stack.len(),
                        self.stack_len(),
                        match self.exec_stack.last() {
                            Some(x) => format!("{:?}", x),
                            None => "".to_string(),
                        }
                    );
                }
                match iseq[self.pc] {
                    Inst::RETURN => {
                        // - reached the end of the method or block.
                        // - `return` in method.
                        // - `next` in block AND outer of loops.
                        if self.called() {
                            return Ok(());
                        } else {
                            let use_value = self.context().use_value;
                            self.unwind_continue(use_value);
                            break;
                        }
                    }
                    Inst::BREAK => {
                        // - `break`  in block or eval AND outer of loops.
                        #[cfg(debug_assertions)]
                        assert!(self.kind() == ISeqKind::Block || self.kind() == ISeqKind::Other);
                        let val = self.stack_pop();
                        self.unwind_context();
                        self.globals.error_register = val;
                        if self.called() {
                            let err = RubyError::block_return();
                            return Err(err);
                        } else {
                            #[cfg(any(feature = "trace", feature = "trace-func"))]
                            if self.globals.startup_flag {
                                eprintln!("<--- BlockReturn({:?})", self.globals.error_register);
                            }
                            break;
                        }
                    }
                    Inst::MRETURN => {
                        // - `return` in block
                        #[cfg(debug_assertions)]
                        assert!(self.kind() == ISeqKind::Block);
                        self.globals.error_register = self.stack_pop();
                        let err = RubyError::method_return();
                        return Err(err);
                    }
                    Inst::THROW => {
                        // - raise error
                        self.globals.error_register = self.stack_pop();
                        return Err(RubyError::value());
                    }
                    Inst::PUSH_NIL => {
                        self.stack_push(Value::nil());
                        self.pc += 1;
                    }
                    Inst::PUSH_SELF => {
                        self.stack_push(self.context().self_value);
                        self.pc += 1;
                    }
                    Inst::PUSH_VAL => {
                        let val = iseq.read64(self.pc + 1);
                        self.stack_push(Value::from(val));
                        self.pc += 9;
                    }
                    Inst::ADD => {
                        self.pc += 1;
                        self.exec_add()?;
                    }
                    Inst::ADDI => {
                        let i = iseq.read32(self.pc + 1) as i32;
                        self.pc += 5;
                        self.exec_addi(i)?;
                    }
                    Inst::SUB => {
                        self.pc += 1;
                        self.exec_sub()?;
                    }
                    Inst::SUBI => {
                        let i = iseq.read32(self.pc + 1) as i32;
                        self.pc += 5;
                        self.exec_subi(i)?;
                    }
                    Inst::MUL => {
                        self.pc += 1;
                        self.exec_mul()?;
                    }
                    Inst::POW => {
                        let (lhs, rhs) = self.stack_pop2();
                        self.pc += 1;
                        self.exec_exp(rhs, lhs)?;
                    }
                    Inst::DIV => {
                        self.pc += 1;
                        self.exec_div()?;
                    }
                    Inst::REM => {
                        let (lhs, rhs) = self.stack_pop2();
                        self.pc += 1;
                        self.exec_rem(rhs, lhs)?;
                    }
                    Inst::SHR => {
                        let (lhs, rhs) = self.stack_pop2();
                        self.pc += 1;
                        self.exec_shr(rhs, lhs)?;
                    }
                    Inst::SHL => {
                        let (lhs, rhs) = self.stack_pop2();
                        self.pc += 1;
                        self.exec_shl(rhs, lhs)?;
                    }
                    Inst::NEG => {
                        let lhs = self.stack_pop();
                        self.pc += 1;
                        self.exec_neg(lhs)?;
                    }
                    Inst::BAND => {
                        let (lhs, rhs) = self.stack_pop2();
                        self.pc += 1;
                        self.exec_bitand(rhs, lhs)?;
                    }
                    Inst::BOR => {
                        let (lhs, rhs) = self.stack_pop2();
                        self.pc += 1;
                        self.exec_bitor(rhs, lhs)?;
                    }
                    Inst::BXOR => {
                        let (lhs, rhs) = self.stack_pop2();
                        self.pc += 1;
                        let val = self.eval_bitxor(rhs, lhs)?;
                        self.stack_push(val);
                    }
                    Inst::BNOT => {
                        let lhs = self.stack_pop();
                        self.pc += 1;
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
                        let (lhs, rhs) = self.stack_pop2();
                        self.pc += 1;
                        self.exec_teq(rhs, lhs)?;
                    }
                    Inst::EQI => cmp_i!(eval_eqi),
                    Inst::NEI => cmp_i!(eval_nei),
                    Inst::GTI => cmp_i!(eval_gti),
                    Inst::GEI => cmp_i!(eval_gei),
                    Inst::LTI => cmp_i!(eval_lti),
                    Inst::LEI => cmp_i!(eval_lei),
                    Inst::CMP => {
                        let (lhs, rhs) = self.stack_pop2();
                        self.pc += 1;
                        let val = self.eval_compare(rhs, lhs)?;
                        self.stack_push(val);
                    }
                    Inst::NOT => {
                        let lhs = self.stack_pop();
                        self.pc += 1;
                        let val = Value::bool(!lhs.to_bool());
                        self.stack_push(val);
                    }
                    Inst::RESCUE => {
                        let len = iseq.read32(self.pc + 1) as usize;
                        self.pc += 5;
                        let stack_len = self.exec_stack.len();
                        let val = self.exec_stack[stack_len - len - 1];
                        let ex = &self.exec_stack[stack_len - len..];
                        let b = self.eval_rescue(val, ex);
                        self.set_stack_len(stack_len - len - 1);
                        self.stack_push(Value::bool(b));
                    }
                    Inst::CONCAT_STRING => {
                        let num = iseq.read32(self.pc + 1) as usize;
                        self.pc += 5;
                        let stack_len = self.stack_len();
                        let res = self
                            .exec_stack
                            .drain(stack_len - num..)
                            .fold(String::new(), |acc, x| acc + x.as_string().unwrap());

                        let val = Value::string(res);
                        self.stack_push(val);
                    }
                    Inst::SET_LOCAL => {
                        let id = iseq.read_lvar_id(self.pc + 1);
                        self.pc += 5;
                        let val = self.stack_pop();
                        self.context()[id] = val;
                    }
                    Inst::GET_LOCAL => {
                        let id = iseq.read_lvar_id(self.pc + 1);
                        self.pc += 5;
                        let val = self.context()[id];
                        self.stack_push(val);
                    }
                    Inst::SET_DYNLOCAL => {
                        let id = iseq.read_lvar_id(self.pc + 1);
                        let outer = iseq.read32(self.pc + 5);
                        self.pc += 9;
                        let val = self.stack_pop();
                        let mut cref = self.get_outer_context(outer);
                        cref[id] = val;
                    }
                    Inst::GET_DYNLOCAL => {
                        let id = iseq.read_lvar_id(self.pc + 1);
                        let outer = iseq.read32(self.pc + 5);
                        self.pc += 9;
                        let cref = self.get_outer_context(outer);
                        let val = cref[id];
                        self.stack_push(val);
                    }
                    Inst::CHECK_LOCAL => {
                        let id = iseq.read_lvar_id(self.pc + 1);
                        let outer = iseq.read32(self.pc + 5);
                        self.pc += 9;
                        let cref = self.get_outer_context(outer);
                        let val = cref[id].is_uninitialized();
                        self.stack_push(Value::bool(val));
                    }
                    Inst::SET_CONST => {
                        let id = iseq.read_id(self.pc + 1);
                        self.pc += 5;
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
                        let id = iseq.read_id(self.pc + 1);
                        self.pc += 5;
                        let is_undef = self.find_const(id).is_err();
                        self.stack_push(Value::bool(is_undef));
                    }
                    Inst::GET_CONST => {
                        let id = iseq.read_id(self.pc + 1);
                        let slot = iseq.read32(self.pc + 5);
                        self.pc += 9;
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
                        let id = iseq.read_id(self.pc + 1);
                        self.pc += 5;
                        let parent = BuiltinClass::object();
                        let val = self.get_scope(parent, id)?;
                        self.stack_push(val);
                    }
                    Inst::CHECK_SCOPE => {
                        let parent = self.stack_pop();
                        let id = iseq.read_id(self.pc + 1);
                        self.pc += 5;
                        let is_undef = match parent.expect_mod_class() {
                            Ok(parent) => self.get_scope(parent, id).is_err(),
                            Err(_) => true,
                        };
                        self.stack_push(Value::bool(is_undef));
                    }
                    Inst::GET_SCOPE => {
                        let parent = self.stack_pop().expect_mod_class()?;
                        let id = iseq.read_id(self.pc + 1);
                        self.pc += 5;
                        let val = self.get_scope(parent, id)?;
                        self.stack_push(val);
                    }
                    Inst::SET_IVAR => {
                        let var_id = iseq.read_id(self.pc + 1);
                        self.pc += 5;
                        let new_val = self.stack_pop();
                        let self_value = self.context().self_value;
                        self_value.set_var(var_id, new_val);
                    }
                    Inst::GET_IVAR => {
                        let var_id = iseq.read_id(self.pc + 1);
                        self.pc += 5;
                        let self_value = self.context().self_value;
                        let val = self_value.get_var(var_id).unwrap_or_default();
                        self.stack_push(val);
                    }
                    Inst::CHECK_IVAR => {
                        let var_id = iseq.read_id(self.pc + 1);
                        self.pc += 5;
                        let self_value = self.context().self_value;
                        let val = Value::bool(self_value.get_var(var_id).is_none());
                        self.stack_push(val);
                    }
                    Inst::SET_GVAR => {
                        let var_id = iseq.read_id(self.pc + 1);
                        self.pc += 5;
                        let new_val = self.stack_pop();
                        self.set_global_var(var_id, new_val);
                    }
                    Inst::GET_GVAR => {
                        let var_id = iseq.read_id(self.pc + 1);
                        self.pc += 5;
                        let val = self.get_global_var(var_id).unwrap_or_default();
                        self.stack_push(val);
                    }
                    Inst::CHECK_GVAR => {
                        let var_id = iseq.read_id(self.pc + 1);
                        self.pc += 5;
                        let val = Value::bool(self.get_global_var(var_id).is_none());
                        self.stack_push(val);
                    }
                    Inst::GET_SVAR => {
                        let var_id = iseq.read32(self.pc + 1);
                        self.pc += 5;
                        let val = self.get_special_var(var_id);
                        self.stack_push(val);
                    }
                    Inst::SET_SVAR => {
                        let var_id = iseq.read32(self.pc + 1);
                        self.pc += 5;
                        let new_val = self.stack_pop();
                        self.set_special_var(var_id, new_val)?;
                    }
                    Inst::SET_CVAR => {
                        let var_id = iseq.read_id(self.pc + 1);
                        self.pc += 5;
                        let new_val = self.stack_pop();
                        self.set_class_var(var_id, new_val)?;
                    }
                    Inst::GET_CVAR => {
                        let var_id = iseq.read_id(self.pc + 1);
                        self.pc += 5;
                        let val = self.get_class_var(var_id)?;
                        self.stack_push(val);
                    }
                    Inst::SET_INDEX => {
                        self.pc += 1;
                        dispatch!(self.invoke_set_index());
                    }
                    Inst::GET_INDEX => {
                        self.pc += 1;
                        let idx = self.stack_pop();
                        let receiver = self.stack_pop();
                        dispatch!(self.invoke_get_index(receiver, idx));
                    }
                    Inst::SET_IDX_I => {
                        let idx = iseq.read32(self.pc + 1);
                        self.pc += 5;
                        dispatch!(self.invoke_set_index_imm(idx));
                    }
                    Inst::GET_IDX_I => {
                        let idx = iseq.read32(self.pc + 1);
                        self.pc += 5;
                        let receiver = self.stack_pop();
                        dispatch!(self.invoke_get_index_imm(receiver, idx));
                    }
                    Inst::SPLAT => {
                        let val = self.stack_pop();
                        let res = Value::splat(val);
                        self.stack_push(res);
                        self.pc += 1;
                    }
                    Inst::CONST_VAL => {
                        let id = iseq.read_usize(self.pc + 1);
                        let val = self.globals.const_values.get(id);
                        self.stack_push(val);
                        self.pc += 5;
                    }
                    Inst::CREATE_RANGE => {
                        let start = self.stack_pop();
                        let end = self.stack_pop();
                        let exclude_end = self.stack_pop().to_bool();
                        let range = self.create_range(start, end, exclude_end)?;
                        self.stack_push(range);
                        self.pc += 1;
                    }
                    Inst::CREATE_ARRAY => {
                        let arg_num = iseq.read_usize(self.pc + 1);
                        let elems = self.pop_args_to_vec(arg_num);
                        let array = Value::array_from(elems);
                        self.stack_push(array);
                        self.pc += 5;
                    }
                    Inst::CREATE_PROC => {
                        let method = iseq.read_method(self.pc + 1).unwrap();
                        let block = self.new_block(method);
                        let proc_obj = self.create_proc(&block);
                        self.stack_push(proc_obj);
                        self.pc += 5;
                    }
                    Inst::CREATE_HASH => {
                        let arg_num = iseq.read_usize(self.pc + 1);
                        let key_value = self.pop_key_value_pair(arg_num);
                        let hash = Value::hash_from_map(key_value);
                        self.stack_push(hash);
                        self.pc += 5;
                    }
                    Inst::CREATE_REGEXP => {
                        let arg = self.stack_pop();
                        let regexp = self.create_regexp(arg)?;
                        self.stack_push(regexp);
                        self.pc += 1;
                    }
                    Inst::JMP => {
                        let disp = iseq.read_disp(self.pc + 1);
                        self.jump_pc(5, disp);
                    }
                    Inst::JMP_BACK => {
                        let disp = iseq.read_disp(self.pc + 1);
                        self.gc();
                        self.jump_pc(5, disp);
                    }
                    Inst::JMP_F => {
                        let val = self.stack_pop();
                        let b = val.to_bool();
                        self.jmp_cond(iseq, b, 5, 1);
                    }
                    Inst::JMP_T => {
                        let val = self.stack_pop();
                        let b = !val.to_bool();
                        self.jmp_cond(iseq, b, 5, 1);
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
                        let map = self
                            .globals
                            .case_dispatch
                            .get_entry(iseq.read32(self.pc + 1));
                        let disp = match map.get(&HashKey(val)) {
                            Some(disp) => *disp,
                            None => iseq.read_disp(self.pc + 5),
                        };
                        self.jump_pc(9, disp);
                    }
                    Inst::OPT_CASE2 => {
                        let val = self.stack_pop();
                        let disp = if let Some(i) = val.as_fixnum() {
                            let map = self
                                .globals
                                .case_dispatch2
                                .get_entry(iseq.read32(self.pc + 1));
                            if map.0 <= i && i <= map.1 {
                                map.2[(i - map.0) as usize]
                            } else {
                                iseq.read_disp(self.pc + 5)
                            }
                        } else {
                            iseq.read_disp(self.pc + 5)
                        };
                        self.jump_pc(9, disp);
                    }
                    Inst::CHECK_METHOD => {
                        let receiver = self.stack_pop();
                        let method = iseq.read_id(self.pc + 1);
                        let rec_class = receiver.get_class_for_method();
                        let is_undef = rec_class.search_method(method).is_none();
                        self.stack_push(Value::bool(is_undef));
                        self.pc += 5;
                    }
                    Inst::SEND => {
                        let receiver = self.stack_pop();
                        dispatch!(self.vm_send(iseq, receiver));
                    }
                    Inst::SEND_SELF => {
                        dispatch!(self.vm_send(iseq, None));
                    }
                    Inst::OPT_SEND => {
                        let receiver = self.stack_pop();
                        dispatch!(self.vm_fast_send(iseq, receiver, true));
                    }
                    Inst::OPT_SEND_SELF => {
                        dispatch!(self.vm_fast_send(iseq, None, true));
                    }
                    Inst::OPT_SEND_N => {
                        let receiver = self.stack_pop();
                        dispatch!(self.vm_fast_send(iseq, receiver, false));
                    }
                    Inst::OPT_SEND_SELF_N => {
                        dispatch!(self.vm_fast_send(iseq, None, false));
                    }
                    Inst::YIELD => {
                        let args_num = iseq.read32(self.pc + 1) as usize;
                        self.pc += 5;
                        let args = self.pop_args_to_args(args_num);
                        dispatch!(self.vm_yield(&args));
                    }
                    Inst::SUPER => {
                        let args_num = iseq.read32(self.pc + 1) as usize;
                        let _block = iseq.read_method(self.pc + 3);
                        let flag = iseq.read8(self.pc + 7) == 1;
                        self.pc += 8;
                        let self_value = self.context().self_value;
                        dispatch!(self.vm_super(self_value, args_num, flag));
                    }
                    Inst::DEF_CLASS => {
                        let is_module = iseq.read8(self.pc + 1) == 1;
                        let id = iseq.read_id(self.pc + 2);
                        let method = iseq.read_method(self.pc + 6).unwrap();
                        self.pc += 10;
                        let base = self.stack_pop();
                        let super_val = self.stack_pop();
                        let val = self.define_class(base, id, is_module, super_val)?;
                        let mut iseq = method.as_iseq();
                        iseq.class_defined = self.get_class_defined(val);
                        assert!(iseq.is_classdef());
                        dispatch!(self.invoke_method(method, val, &Args2::new(0)));
                    }
                    Inst::DEF_SCLASS => {
                        let method = iseq.read_method(self.pc + 1).unwrap();
                        self.pc += 5;
                        let singleton = self.stack_pop().get_singleton_class()?;
                        let mut iseq = method.as_iseq();
                        iseq.class_defined = self.get_class_defined(singleton);
                        assert!(iseq.is_classdef());
                        dispatch!(self.invoke_method(method, singleton, &Args2::new(0)));
                    }
                    Inst::DEF_METHOD => {
                        let id = iseq.read_id(self.pc + 1);
                        let method = iseq.read_method(self.pc + 5).unwrap();
                        self.pc += 9;
                        let mut iseq = method.as_iseq();
                        iseq.class_defined = self.get_method_iseq().class_defined.clone();
                        let self_value = self.context().self_value;
                        self.define_method(self_value, id, method);
                        if self.is_module_function() {
                            self.define_singleton_method(self_value, id, method)?;
                        };
                    }
                    Inst::DEF_SMETHOD => {
                        let id = iseq.read_id(self.pc + 1);
                        let method = iseq.read_method(self.pc + 5).unwrap();
                        self.pc += 9;
                        let mut iseq = method.as_iseq();
                        iseq.class_defined = self.get_method_iseq().class_defined.clone();
                        let singleton = self.stack_pop();
                        self.define_singleton_method(singleton, id, method)?;
                        if self.is_module_function() {
                            self.define_method(singleton, id, method);
                        };
                    }
                    Inst::TO_S => {
                        let val = self.stack_pop();
                        let s = val.val_to_s(self)?;
                        let res = Value::string(s);
                        self.stack_push(res);
                        self.pc += 1;
                    }
                    Inst::POP => {
                        self.stack_pop();
                        self.pc += 1;
                    }
                    Inst::DUP => {
                        let len = iseq.read_usize(self.pc + 1);
                        let stack_len = self.stack_len();
                        self.exec_stack.extend_from_within(stack_len - len..);
                        self.pc += 5;
                    }
                    Inst::SINKN => {
                        let len = iseq.read_usize(self.pc + 1);
                        let val = self.stack_pop();
                        let stack_len = self.stack_len();
                        self.exec_stack.insert(stack_len - len, val);
                        self.pc += 5;
                    }
                    Inst::TOPN => {
                        let len = iseq.read_usize(self.pc + 1);
                        let val = self.exec_stack.remove(self.stack_len() - 1 - len);
                        self.stack_push(val);
                        self.pc += 5;
                    }
                    Inst::TAKE => {
                        let len = iseq.read_usize(self.pc + 1);
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
                                    /*for _ in ary_len..len {
                                        self.stack_push(Value::nil());
                                    }*/
                                }
                            }
                            None => {
                                self.stack_push(val);
                                self.exec_stack
                                    .resize(self.stack_len() + len - 1, Value::nil());
                                /*for _ in 0..len - 1 {
                                    self.stack_push(Value::nil());
                                }*/
                            }
                        }

                        self.pc += 5;
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
        self.unwind_context();
        //let prev_len = self.context().prev_stack_len;
        //self.set_stack_len(prev_len);
        //self.pc = self.context().prev_pc;
        //self.context_pop();
        #[cfg(any(feature = "trace", feature = "trace-func"))]
        if self.globals.startup_flag {
            eprintln!("<--- Ok({:?})", val);
        }
        if use_value {
            self.stack_push(val);
        }
    }

    fn handle_hash_args(&mut self, kw_rest_num: u8, flag: ArgFlag) -> VMResult {
        let mut stack_len = self.stack_len() - kw_rest_num as usize;
        let kwrest = &self.exec_stack[stack_len..];
        let kw = if flag.has_hash_arg() {
            let mut val = self.exec_stack[stack_len - 1];
            stack_len -= 1;
            let hash = val.as_mut_hash().unwrap();
            for h in kwrest {
                for (k, v) in h.expect_hash("Arg")? {
                    hash.insert(k, v);
                }
            }
            val
        } else if kwrest.len() == 0 {
            Value::nil()
        } else {
            let mut hash = FxIndexMap::default();
            for h in kwrest {
                for (k, v) in h.expect_hash("Arg")? {
                    hash.insert(HashKey(k), v);
                }
            }
            Value::hash_from_map(hash)
        };
        self.set_stack_len(stack_len);
        Ok(kw)
    }

    fn handle_block_arg(&mut self, block: u32, flag: ArgFlag) -> Result<Option<Block>, RubyError> {
        let block = if block != 0 {
            Some(self.new_block(block))
        } else if flag.has_block_arg() {
            let val = self.stack_pop();
            if val.is_nil() {
                None
            } else {
                // TODO: Support to_proc().
                if val.as_proc().is_none() {
                    return Err(RubyError::internal(format!(
                        "Must be Proc. {:?}:{}",
                        val,
                        val.get_class_name()
                    )));
                }
                Some(Block::Proc(val))
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
    fn vm_fast_send(
        &mut self,
        iseq: &ISeq,
        receiver: impl Into<Option<Value>>,
        use_value: bool,
    ) -> Result<VMResKind, RubyError> {
        // With block and no keyword/block/splat/delegate arguments for OPT_SEND.
        let method_name = iseq.read_id(self.pc + 1);
        let args_num = iseq.read16(self.pc + 5) as usize;
        let block = iseq.read32(self.pc + 7);
        let cache_id = iseq.read32(self.pc + 11);
        let receiver = receiver.into().unwrap_or_else(|| self.context().self_value);
        self.pc += 15;
        self.invoke_fast_send(method_name, receiver, cache_id, args_num, block, use_value)
    }

    fn vm_send(
        &mut self,
        iseq: &ISeq,
        receiver: impl Into<Option<Value>>,
    ) -> Result<VMResKind, RubyError> {
        let method_id = iseq.read_id(self.pc + 1);
        let args_num = iseq.read16(self.pc + 5);
        let hash_num = iseq.read8(self.pc + 7);
        let flag = iseq.read_argflag(self.pc + 8);
        let block = iseq.read32(self.pc + 9);
        let cache = iseq.read32(self.pc + 13);
        self.pc += 17;
        let block = self.handle_block_arg(block, flag)?;
        let keyword = self.handle_hash_args(hash_num, flag)?;
        let mut args = self.pop_args_to_args(args_num as usize);
        if flag.has_delegate() {
            let method_context = self.get_method_context();
            match method_context.delegate_args {
                Some(v) => {
                    let ary = &v.as_array().unwrap().elements;
                    args.append(ary);
                    self.stack_append(ary);
                }
                None => {}
            }
        }
        args.block = block;
        args.kw_arg = keyword;

        let receiver = receiver.into().unwrap_or_else(|| self.context().self_value);
        let rec_class = receiver.get_class_for_method();
        match MethodRepo::find_method_inline_cache(cache, rec_class, method_id) {
            Some(method) => self.invoke_method(method, receiver, &args),
            None => self.invoke_method_missing(method_id, receiver, &args, true),
        }
    }

    fn vm_super(
        &mut self,
        self_value: Value,
        args_num: usize,
        flag: bool,
    ) -> Result<VMResKind, RubyError> {
        // TODO: support keyword parameter, etc..
        let iseq = self.get_method_context().iseq_ref;
        if let ISeqKind::Method(Some(m_id)) = iseq.kind {
            let class = self_value.get_class_for_method();
            let method = class
                .superclass()
                .map(|class| MethodRepo::find_method(class, m_id))
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
                    self.stack_push(self.context()[i]);
                }
                Args2::new(args_num + param_num)
            } else {
                self.pop_args_to_args(args_num)
            };
            self.invoke_method(method, self_value, &args)
        } else {
            return Err(RubyError::nomethod("super called outside of method"));
        }
    }

    /// Invoke the block given to the method with `args`.
    fn vm_yield(&mut self, args: &Args2) -> Result<VMResKind, RubyError> {
        match &self.get_method_context().block {
            Some(Block::Block(method, ctx)) => {
                let ctx = ctx.get_current();
                self.invoke_func(*method, ctx.self_value, Some(ctx), args, true)
            }
            Some(Block::Proc(proc)) => self.invoke_proc(*proc, args),
            None => return Err(RubyError::local_jump("No block given.")),
        }
    }
}

impl VM {
    fn invoke_fast_send(
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
                    let mut args = Args2::new(args_num);
                    args.block = Block::from_u32(block, self);
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
                        let mut context =
                            self.new_stack_context_with(receiver, block, iseq, None, args_num);
                        context.copy_from_slice0(&self.exec_stack[len - args_num..]);
                        context
                    } else {
                        let mut args = Args2::new(args_num);
                        args.block = block;
                        let mut ctx = ContextRef::from_noopt(self, receiver, iseq, &args, None)?;
                        ctx.prev_stack_len = len - args_num;
                        ctx
                    };
                    context.use_value = use_value;
                    self.invoke_new_context(context);
                    return Ok(VMResKind::Invoke);
                }
                _ => unreachable!(),
            },
            None => {
                let mut args = Args2::new(args_num);
                args.block = Block::from_u32(block, self);
                //self.set_stack_len(len - args_num);
                return self.invoke_method_missing(method_name, receiver, &args, use_value);
            }
        };
        if use_value {
            self.stack_push(val);
        }
        Ok(VMResKind::Return)
    }
}
