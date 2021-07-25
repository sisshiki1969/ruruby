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
    ///
    /// true: normal mode
    /// false: invoke mode
    pub fn run_context_main(&mut self) -> Result<(), RubyError> {
        loop {
            let iseqref = self.context().iseq_ref.unwrap();
            let iseq = &iseqref.iseq;
            let self_value = self.context().self_value;
            self.gc();

            macro_rules! try_err {
                ($eval:expr) => {
                    match $eval {
                        Ok(()) => {}
                        Err(err) => match err.kind {
                            RubyErrorKind::BlockReturn => {}
                            RubyErrorKind::MethodReturn if self.is_method() => {
                                if self.context().called {
                                    return Ok(());
                                } else {
                                    self.unwind_continue();
                                    break;
                                }
                            }
                            _ => return Err(err),
                        },
                    };
                };
            }

            macro_rules! try_send {
                ($eval:expr) => {
                    match $eval {
                        Ok(VMResKind::Invoke) => break,
                        Err(err) => match err.kind {
                            RubyErrorKind::BlockReturn => {}
                            RubyErrorKind::MethodReturn if self.is_method() => {
                                if self.context().called {
                                    return Ok(());
                                } else {
                                    self.unwind_continue();
                                    break;
                                }
                            }
                            _ => return Err(err),
                        },
                        _ => {}
                    };
                };
            }

            macro_rules! jmp_cmp {
                ($eval:ident) => {{
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let b = self.$eval(rhs, lhs)?;
                    self.jmp_cond(iseq, b, 5, 1);
                }};
            }

            macro_rules! jmp_cmp_i {
                ($eval:ident) => {{
                    let i = iseq.read32(self.pc + 1) as i32;
                    let lhs = self.stack_pop();
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
                    println!(
                        "{:>4x}: {:<40} tmp: {:<4} stack: {:<3} top: {}",
                        self.pc.into_usize(),
                        Inst::inst_info(&self.globals, self.context().iseq_ref.unwrap(), self.pc),
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
                        let use_value = self.context().use_value;
                        if self.context().called {
                            return Ok(());
                        } else {
                            self.unwind_continue();
                            if !use_value {
                                self.stack_pop();
                            }
                            break;
                        }
                    }
                    Inst::BREAK => {
                        // - `break`  in block or eval AND outer of loops.
                        #[cfg(debug_assertions)]
                        assert!(iseqref.kind == ISeqKind::Block || iseqref.kind == ISeqKind::Other);
                        let called = self.context().called;
                        let val = self.stack_pop();
                        self.unwind_context();
                        self.globals.acc = val;
                        if called {
                            let err = RubyError::block_return();
                            #[cfg(any(feature = "trace", feature = "trace-func"))]
                            if self.globals.startup_flag {
                                eprintln!(
                                    "<+++ BlockReturn({:?}) stack:{}",
                                    self.globals.acc,
                                    self.stack_len()
                                );
                            }
                            return Err(err);
                        } else {
                            #[cfg(any(feature = "trace", feature = "trace-func"))]
                            if self.globals.startup_flag {
                                eprintln!("<--- BlockReturn({:?})", self.globals.acc);
                            }
                            break;
                        }
                    }
                    Inst::MRETURN => {
                        // - `return` in block
                        #[cfg(debug_assertions)]
                        assert!(iseqref.kind == ISeqKind::Block);
                        let err = RubyError::method_return();
                        return Err(err);
                    }
                    Inst::THROW => {
                        // - raise error
                        self.globals.acc = self.stack_pop();
                        return Err(RubyError::value());
                    }
                    Inst::PUSH_NIL => {
                        self.stack_push(Value::nil());
                        self.pc += 1;
                    }
                    Inst::PUSH_TRUE => {
                        self.stack_push(Value::true_val());
                        self.pc += 1;
                    }
                    Inst::PUSH_FALSE => {
                        self.stack_push(Value::false_val());
                        self.pc += 1;
                    }
                    Inst::PUSH_SELF => {
                        self.stack_push(self_value);
                        self.pc += 1;
                    }
                    Inst::PUSH_FIXNUM => {
                        let num = iseq.read64(self.pc + 1);
                        self.stack_push(Value::integer(num as i64));
                        self.pc += 9;
                    }
                    Inst::PUSH_FLONUM => {
                        let num = f64::from_bits(iseq.read64(self.pc + 1));
                        self.stack_push(Value::float(num));
                        self.pc += 9;
                    }
                    Inst::PUSH_SYMBOL => {
                        let id = iseq.read_id(self.pc + 1);
                        self.stack_push(Value::symbol(id));
                        self.pc += 5;
                    }
                    Inst::ADD => {
                        let rhs = self.stack_pop();
                        let lhs = self.stack_pop();
                        self.pc += 1;
                        self.invoke_add(rhs, lhs)?;
                    }
                    Inst::ADDI => {
                        let lhs = self.stack_pop();
                        let i = iseq.read32(self.pc + 1) as i32;
                        self.pc += 5;
                        self.invoke_addi(lhs, i)?;
                    }
                    Inst::SUB => {
                        let rhs = self.stack_pop();
                        let lhs = self.stack_pop();
                        self.pc += 1;
                        self.invoke_sub(rhs, lhs)?;
                    }
                    Inst::SUBI => {
                        let lhs = self.stack_pop();
                        let i = iseq.read32(self.pc + 1) as i32;
                        self.pc += 5;
                        self.invoke_subi(lhs, i)?;
                    }
                    Inst::MUL => {
                        let rhs = self.stack_pop();
                        let lhs = self.stack_pop();
                        self.pc += 1;
                        self.invoke_mul(rhs, lhs)?;
                    }
                    Inst::POW => {
                        let rhs = self.stack_pop();
                        let lhs = self.stack_pop();
                        self.pc += 1;
                        self.invoke_exp(rhs, lhs)?;
                    }
                    Inst::DIV => {
                        let rhs = self.stack_pop();
                        let lhs = self.stack_pop();
                        self.pc += 1;
                        self.invoke_div(rhs, lhs)?;
                    }
                    Inst::REM => {
                        let rhs = self.stack_pop();
                        let lhs = self.stack_pop();
                        self.pc += 1;
                        self.invoke_rem(rhs, lhs)?;
                    }
                    Inst::SHR => {
                        let rhs = self.stack_pop();
                        let lhs = self.stack_pop();
                        self.pc += 1;
                        self.invoke_shr(rhs, lhs)?;
                    }
                    Inst::SHL => {
                        let rhs = self.stack_pop();
                        let lhs = self.stack_pop();
                        self.pc += 1;
                        self.invoke_shl(rhs, lhs)?;
                    }
                    Inst::NEG => {
                        let lhs = self.stack_pop();
                        self.pc += 1;
                        self.invoke_neg(lhs)?;
                    }
                    Inst::BAND => {
                        let rhs = self.stack_pop();
                        let lhs = self.stack_pop();
                        self.pc += 1;
                        self.invoke_bitand(rhs, lhs)?;
                    }
                    Inst::BOR => {
                        let rhs = self.stack_pop();
                        let lhs = self.stack_pop();
                        self.pc += 1;
                        self.invoke_bitor(rhs, lhs)?;
                    }
                    Inst::BXOR => {
                        let rhs = self.stack_pop();
                        let lhs = self.stack_pop();
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

                    Inst::EQ => {
                        let lhs = self.stack_pop();
                        let rhs = self.stack_pop();
                        self.pc += 1;
                        self.invoke_eq(rhs, lhs)?;
                    }
                    Inst::EQI => {
                        let lhs = self.stack_pop();
                        let i = iseq.read32(self.pc + 1) as i32;
                        self.pc += 5;
                        let val = Value::bool(self.eval_eqi(lhs, i)?);
                        self.stack_push(val);
                    }
                    Inst::NE => {
                        let lhs = self.stack_pop();
                        let rhs = self.stack_pop();
                        self.pc += 1;
                        self.invoke_neq(rhs, lhs)?;
                    }
                    Inst::NEI => {
                        let lhs = self.stack_pop();
                        let i = iseq.read32(self.pc + 1) as i32;
                        self.pc += 5;
                        let val = Value::bool(self.eval_nei(lhs, i)?);
                        self.stack_push(val);
                    }
                    Inst::TEQ => {
                        let rhs = self.stack_pop();
                        let lhs = self.stack_pop();
                        self.pc += 1;
                        self.invoke_teq(rhs, lhs)?;
                    }
                    Inst::GT => {
                        let rhs = self.stack_pop();
                        let lhs = self.stack_pop();
                        self.pc += 1;
                        let val = self.eval_gt(rhs, lhs).map(|x| Value::bool(x))?;
                        self.stack_push(val);
                    }
                    Inst::GTI => {
                        let lhs = self.stack_pop();
                        let i = iseq.read32(self.pc + 1) as i32;
                        self.pc += 5;
                        let val = self.eval_gti(lhs, i).map(|x| Value::bool(x))?;
                        self.stack_push(val);
                    }
                    Inst::GE => {
                        let rhs = self.stack_pop();
                        let lhs = self.stack_pop();
                        self.pc += 1;
                        let val = self.eval_ge(rhs, lhs).map(|x| Value::bool(x))?;
                        self.stack_push(val);
                    }
                    Inst::GEI => {
                        let lhs = self.stack_pop();
                        let i = iseq.read32(self.pc + 1) as i32;
                        self.pc += 5;
                        let val = self.eval_gei(lhs, i).map(|x| Value::bool(x))?;
                        self.stack_push(val);
                    }
                    Inst::LT => {
                        let rhs = self.stack_pop();
                        let lhs = self.stack_pop();
                        self.pc += 1;
                        let val = self.eval_lt(rhs, lhs).map(|x| Value::bool(x))?;
                        self.stack_push(val);
                    }
                    Inst::LTI => {
                        let lhs = self.stack_pop();
                        let i = iseq.read32(self.pc + 1) as i32;
                        self.pc += 5;
                        let val = self.eval_lti(lhs, i).map(|x| Value::bool(x))?;
                        self.stack_push(val);
                    }
                    Inst::LE => {
                        let rhs = self.stack_pop();
                        let lhs = self.stack_pop();
                        self.pc += 1;
                        let val = self.eval_le(rhs, lhs).map(|x| Value::bool(x))?;
                        self.stack_push(val);
                    }
                    Inst::LEI => {
                        let lhs = self.stack_pop();
                        let i = iseq.read32(self.pc + 1) as i32;
                        self.pc += 5;
                        let val = self.eval_lei(lhs, i).map(|x| Value::bool(x))?;
                        self.stack_push(val);
                    }
                    Inst::CMP => {
                        let rhs = self.stack_pop();
                        let lhs = self.stack_pop();
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
                            v if v.is_nil() => match self.get_method_iseq().class_defined.last() {
                                Some(class) => *class,
                                None => BuiltinClass::object(),
                            },
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
                        self_value.set_var(var_id, new_val);
                    }
                    Inst::GET_IVAR => {
                        let var_id = iseq.read_id(self.pc + 1);
                        self.pc += 5;
                        let val = match self_value.get_var(var_id) {
                            Some(val) => val,
                            None => Value::nil(),
                        };
                        self.stack_push(val);
                    }
                    Inst::CHECK_IVAR => {
                        let var_id = iseq.read_id(self.pc + 1);
                        self.pc += 5;
                        let val = match self_value.get_var(var_id) {
                            Some(_) => Value::false_val(),
                            None => Value::true_val(),
                        };
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
                        let val = self.get_global_var(var_id).unwrap_or(Value::nil());
                        self.stack_push(val);
                    }
                    Inst::CHECK_GVAR => {
                        let var_id = iseq.read_id(self.pc + 1);
                        self.pc += 5;
                        let val = match self.get_global_var(var_id) {
                            Some(_) => Value::false_val(),
                            None => Value::true_val(),
                        };
                        self.stack_push(val);
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
                        self.set_index()?;
                    }
                    Inst::GET_INDEX => {
                        self.pc += 1;
                        let idx = self.stack_pop();
                        let receiver = self.stack_pop();
                        self.invoke_get_index(receiver, idx)?;
                    }
                    Inst::SET_IDX_I => {
                        let idx = iseq.read32(self.pc + 1);
                        self.pc += 5;
                        self.set_index_imm(idx)?;
                    }
                    Inst::GET_IDX_I => {
                        let idx = iseq.read32(self.pc + 1);
                        self.pc += 5;
                        let receiver = self.stack_pop();
                        self.invoke_get_index_imm(receiver, idx)?;
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
                        let elems = self.pop_args_to_args(arg_num).into_vec();
                        let array = Value::array_from(elems);
                        self.stack_push(array);
                        self.pc += 5;
                    }
                    Inst::CREATE_PROC => {
                        let method = iseq.read_method(self.pc + 1);
                        let block = self.new_block(method);
                        let proc_obj = self.create_proc(&block)?;
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
                        let disp = match map.get(&val) {
                            Some(disp) => *disp,
                            None => iseq.read_disp(self.pc + 5),
                        };
                        self.jump_pc(9, disp);
                    }
                    Inst::OPT_CASE2 => {
                        let val = self.stack_pop();
                        let disp = if let Some(i) = val.as_integer() {
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
                        try_send!(self.vm_send(iseq, receiver));
                    }
                    Inst::SEND_SELF => {
                        try_send!(self.vm_send(iseq, self_value));
                    }
                    Inst::OPT_SEND => {
                        let receiver = self.stack_pop();
                        try_send!(self.vm_fast_send(iseq, receiver, true));
                    }
                    Inst::OPT_SEND_SELF => {
                        try_send!(self.vm_fast_send(iseq, self_value, true));
                    }
                    Inst::OPT_SEND_N => {
                        let receiver = self.stack_pop();
                        try_send!(self.vm_fast_send(iseq, receiver, false));
                    }
                    Inst::OPT_SEND_SELF_N => {
                        try_send!(self.vm_fast_send(iseq, self_value, false));
                    }
                    Inst::YIELD => {
                        let args_num = iseq.read32(self.pc + 1) as usize;
                        self.pc += 5;
                        let args = self.pop_args_to_args(args_num);
                        try_send!(self.vm_yield(&args));
                    }
                    Inst::DEF_CLASS => {
                        let is_module = iseq.read8(self.pc + 1) == 1;
                        let id = iseq.read_id(self.pc + 2);
                        let method = iseq.read_method(self.pc + 6);
                        self.pc += 10;
                        let base = self.stack_pop();
                        let super_val = self.stack_pop();
                        let val = self.define_class(base, id, is_module, super_val)?;
                        self.class_push(val);
                        let mut iseq = method.as_iseq();
                        iseq.class_defined = self.get_class_defined();
                        assert!(iseq.is_classdef());
                        let res = self.exec_method(method, val, &Args::new0());
                        self.class_pop();
                        try_err!(res);
                    }
                    Inst::DEF_SCLASS => {
                        let method = iseq.read_method(self.pc + 1);
                        self.pc += 5;
                        let singleton = self.stack_pop().get_singleton_class()?;
                        self.class_push(singleton);
                        let mut iseq = method.as_iseq();
                        iseq.class_defined = self.get_class_defined();
                        assert!(iseq.is_classdef());
                        let res = self.exec_method(method, singleton, &Args::new0());
                        self.class_pop();
                        try_err!(res);
                    }
                    Inst::DEF_METHOD => {
                        let id = iseq.read_id(self.pc + 1);
                        let method = iseq.read_method(self.pc + 5);
                        self.pc += 9;
                        let mut iseq = method.as_iseq();
                        iseq.class_defined = self.get_method_iseq().class_defined.clone();
                        self.define_method(self_value, id, method);
                        if self.define_mode().module_function {
                            self.define_singleton_method(self_value, id, method)?;
                        };
                    }
                    Inst::DEF_SMETHOD => {
                        let id = iseq.read_id(self.pc + 1);
                        let method = iseq.read_method(self.pc + 5);
                        self.pc += 9;
                        let mut iseq = method.as_iseq();
                        iseq.class_defined = self.get_method_iseq().class_defined.clone();
                        let singleton = self.stack_pop();
                        self.define_singleton_method(singleton, id, method)?;
                        if self.define_mode().module_function {
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
                        for i in stack_len - len..stack_len {
                            let val = self.exec_stack[i];
                            self.stack_push(val);
                        }
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
                                    for i in 0..len {
                                        self.stack_push(elem[i]);
                                    }
                                } else {
                                    for i in 0..ary_len {
                                        self.stack_push(elem[i]);
                                    }
                                    for _ in ary_len..len {
                                        self.stack_push(Value::nil());
                                    }
                                }
                            }
                            None => {
                                self.stack_push(val);
                                for _ in 0..len - 1 {
                                    self.stack_push(Value::nil());
                                }
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
    fn unwind_continue(&mut self) {
        assert_eq!(self.context().prev_stack_len + 1, self.stack_len());
        self.pc = self.context().prev_pc;
        self.context_pop();
        #[cfg(any(feature = "trace", feature = "trace-func"))]
        if self.globals.startup_flag {
            eprintln!("<--- Ok({:?})", self.stack_top());
        }
    }

    /// continue current context -> true
    ///
    /// invoke new context -> false
    fn vm_send(&mut self, iseq: &ISeq, receiver: Value) -> Result<VMResKind, RubyError> {
        let method_id = iseq.read_id(self.pc + 1);
        let args_num = iseq.read16(self.pc + 5);
        let kw_rest_num = iseq.read8(self.pc + 7);
        let flag = iseq.read8(self.pc + 8);
        let block = iseq.read32(self.pc + 9);
        let cache = iseq.read32(self.pc + 13);
        self.pc += 17;
        let mut kwrest = vec![];
        for _ in 0..kw_rest_num {
            let val = self.stack_pop();
            kwrest.push(val);
        }

        let keyword = if flag & 0b01 == 1 {
            let mut val = self.stack_pop();
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

        let block = if block != 0 {
            self.new_block(block)
        } else if flag & 0b10 == 2 {
            let val = self.stack_pop();
            if val.is_nil() {
                Block::None
            } else {
                // TODO: Support to_proc().
                if val.as_proc().is_none() {
                    return Err(RubyError::internal(format!(
                        "Must be Proc. {:?}:{}",
                        val,
                        val.get_class_name()
                    )));
                }
                Block::Proc(val)
            }
        } else {
            Block::None
        };
        let mut args = self.pop_args_to_args(args_num as usize);
        args.block = block;
        args.kw_arg = keyword;

        let rec_class = receiver.get_class_for_method();
        match MethodRepo::find_method_inline_cache(cache, rec_class, method_id) {
            Some(method) => return self.invoke_func(method, receiver, None, &args, true),
            None => {}
        }
        self.invoke_method_missing(method_id, receiver, &args, true)
    }

    /// continue current context -> true
    ///
    /// invoke new context -> false
    fn vm_fast_send(
        &mut self,
        iseq: &ISeq,
        receiver: Value,
        use_value: bool,
    ) -> Result<VMResKind, RubyError> {
        // With block and no keyword/block/splat arguments for OPT_SEND.
        let method_id = iseq.read_id(self.pc + 1);
        let args_num = iseq.read16(self.pc + 5) as usize;
        let block = iseq.read32(self.pc + 7);
        let cache_id = iseq.read32(self.pc + 11);
        self.pc += 15;
        let len = self.stack_len();
        let rec_class = receiver.get_class_for_method();
        let val = match MethodRepo::find_method_inline_cache(cache_id, rec_class, method_id) {
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
                return self.invoke_method_missing(method_id, receiver, &args, use_value);
            }
        };
        if use_value {
            self.stack_push(val);
        }
        Ok(VMResKind::Return)
    }

    /// Invoke the block given to the method with `args`.
    fn vm_yield(&mut self, args: &Args) -> Result<VMResKind, RubyError> {
        match &self.get_method_context().block {
            Block::Block(method, ctx) => {
                let ctx = ctx.get_current();
                self.invoke_func(*method, ctx.self_value, Some(ctx), args, true)
            }
            Block::Proc(proc) => self.invoke_proc(*proc, args),
            Block::None => return Err(RubyError::local_jump("No block given.")),
        }
    }
}
