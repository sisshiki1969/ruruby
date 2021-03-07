use super::*;

impl VM {
    pub fn run_context_main(&mut self) -> VMResult {
        let ctx = self.context();
        let iseq = &mut ctx.iseq_ref.unwrap().iseq;
        let self_value = ctx.self_value;
        self.gc();
        for (i, (outer, lvar)) in ctx.iseq_ref.unwrap().forvars.iter().enumerate() {
            self.get_outer_context(*outer)[*lvar as usize] = ctx[i];
        }
        /// Evaluate expr, and push return value to stack.
        macro_rules! try_push {
            ($eval:expr) => {
                match $eval {
                    Ok(val) => self.stack_push(val),
                    Err(err) => match err.kind {
                        RubyErrorKind::BlockReturn(val) => self.stack_push(val),
                        RubyErrorKind::MethodReturn(val) if self.is_method() => return Ok(val),
                        _ => return Err(err),
                    },
                };
            };
        }

        /// Evaluate expr, and discard return value.
        macro_rules! try_no_push {
            ($eval:expr) => {
                match $eval {
                    Ok(_) => {}
                    Err(err) => match err.kind {
                        RubyErrorKind::BlockReturn(_) => {}
                        RubyErrorKind::MethodReturn(val) if self.is_method() => return Ok(val),
                        _ => return Err(err),
                    },
                };
            };
        }

        loop {
            #[cfg(feature = "perf")]
            self.globals.perf.get_perf(iseq[self.pc]);
            #[cfg(feature = "trace")]
            {
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
                    let val = self.stack_pop();
                    return Ok(val);
                }
                Inst::BREAK => {
                    // - `break`  in block or eval AND outer of loops.
                    #[cfg(debug_assertions)]
                    assert!(
                        self.context().kind == ISeqKind::Block
                            || self.context().kind == ISeqKind::Other
                    );
                    let val = self.stack_pop();
                    let err = RubyError::block_return(val);
                    return Err(err);
                }
                Inst::MRETURN => {
                    // - `return` in block
                    #[cfg(debug_assertions)]
                    assert_eq!(self.context().kind, ISeqKind::Block);
                    let val = self.stack_pop();
                    let err = RubyError::method_return(val);
                    return Err(err);
                }
                Inst::THROW => {
                    // - raise error
                    let val = self.stack_pop();
                    return Err(RubyError::value(val));
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
                    let val = self.eval_add(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::ADDI => {
                    let lhs = self.stack_pop();
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = self.eval_addi(lhs, i)?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::SUB => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let val = self.eval_sub(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::SUBI => {
                    let lhs = self.stack_pop();
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = self.eval_subi(lhs, i)?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::MUL => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let val = self.eval_mul(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::POW => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let val = self.eval_exp(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::DIV => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let val = self.eval_div(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::REM => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let val = self.eval_rem(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::SHR => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let val = self.eval_shr(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::SHL => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let val = self.eval_shl(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::NEG => {
                    let lhs = self.stack_pop();
                    let val = self.eval_neg(lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::BAND => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let val = self.eval_bitand(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::B_ANDI => {
                    let lhs = self.stack_pop();
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = self.eval_bitandi(lhs, i)?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::BOR => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let val = self.eval_bitor(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::B_ORI => {
                    let lhs = self.stack_pop();
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = self.eval_bitori(lhs, i)?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::BXOR => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let val = self.eval_bitxor(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::BNOT => {
                    let lhs = self.stack_pop();
                    let val = self.eval_bitnot(lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }

                Inst::EQ => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = Value::bool(self.eval_eq(rhs, lhs)?);
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::EQI => {
                    let lhs = self.stack_pop();
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = Value::bool(self.eval_eqi(lhs, i));
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::NE => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let val = Value::bool(!self.eval_eq(rhs, lhs)?);
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::NEI => {
                    let lhs = self.stack_pop();
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = Value::bool(!self.eval_eqi(lhs, i));
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::TEQ => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let res = self.eval_teq(rhs, lhs)?;
                    let val = Value::bool(res);
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::GT => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let val = self.eval_gt(rhs, lhs).map(|x| Value::bool(x))?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::GTI => {
                    let lhs = self.stack_pop();
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = self.eval_gti(lhs, i).map(|x| Value::bool(x))?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::GE => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let val = self.eval_ge(rhs, lhs).map(|x| Value::bool(x))?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::GEI => {
                    let lhs = self.stack_pop();
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = self.eval_gei(lhs, i).map(|x| Value::bool(x))?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::LT => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let val = self.eval_lt(rhs, lhs).map(|x| Value::bool(x))?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::LTI => {
                    let lhs = self.stack_pop();
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = self.eval_lti(lhs, i).map(|x| Value::bool(x))?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::LE => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let val = self.eval_le(rhs, lhs).map(|x| Value::bool(x))?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::LEI => {
                    let lhs = self.stack_pop();
                    let i = iseq.read32(self.pc + 1) as i32;
                    let val = self.eval_lei(lhs, i).map(|x| Value::bool(x))?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::CMP => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let val = self.eval_compare(rhs, lhs)?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::NOT => {
                    let lhs = self.stack_pop();
                    let val = Value::bool(!lhs.to_bool());
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::RESCUE => {
                    let len = iseq.read32(self.pc + 1) as usize;
                    let stack_len = self.exec_stack.len();
                    let val = self.exec_stack[stack_len - len - 1];
                    let ex = &self.exec_stack[stack_len - len..];
                    let b = self.eval_rescue(val, ex)?;
                    self.set_stack_len(stack_len - len - 1);
                    self.stack_push(Value::bool(b));
                    self.pc += 5;
                }
                Inst::CONCAT_STRING => {
                    let num = iseq.read32(self.pc + 1) as usize;
                    let stack_len = self.stack_len();
                    let res = self
                        .exec_stack
                        .drain(stack_len - num..)
                        .fold(String::new(), |acc, x| acc + x.as_string().unwrap());

                    let val = Value::string(res);
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::SET_LOCAL => {
                    let id = iseq.read_lvar_id(self.pc + 1);
                    let val = self.stack_pop();
                    self.context()[id] = val;
                    self.pc += 5;
                }
                Inst::GET_LOCAL => {
                    let id = iseq.read_lvar_id(self.pc + 1);
                    let val = self.context()[id];
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::LVAR_ADDI => {
                    let id = iseq.read_lvar_id(self.pc + 1);
                    let i = iseq.read32(self.pc + 5) as i32;
                    let mut ctx = self.context();
                    let val = ctx[id];
                    ctx[id] = self.eval_addi(val, i)?;
                    self.pc += 9;
                }
                Inst::SET_DYNLOCAL => {
                    let id = iseq.read_lvar_id(self.pc + 1);
                    let outer = iseq.read32(self.pc + 5);
                    let val = self.stack_pop();
                    let mut cref = self.get_outer_context(outer);
                    cref[id] = val;
                    self.pc += 9;
                }
                Inst::GET_DYNLOCAL => {
                    let id = iseq.read_lvar_id(self.pc + 1);
                    let outer = iseq.read32(self.pc + 5);
                    let cref = self.get_outer_context(outer);
                    let val = cref[id];
                    self.stack_push(val);
                    self.pc += 9;
                }
                Inst::CHECK_LOCAL => {
                    let id = iseq.read_lvar_id(self.pc + 1);
                    let outer = iseq.read32(self.pc + 5);
                    let cref = self.get_outer_context(outer);
                    let val = cref[id].is_uninitialized();
                    self.stack_push(Value::bool(val));
                    self.pc += 9;
                }
                Inst::SET_CONST => {
                    let id = iseq.read_id(self.pc + 1);
                    let parent = match self.stack_pop() {
                        v if v.is_nil() => match self.get_method_iseq().class_defined.last() {
                            Some(class) => *class,
                            None => BuiltinClass::object(),
                        }, //self.class(),
                        v => v.expect_mod_class()?,
                    };
                    let val = self.stack_pop();
                    self.globals.set_const(parent, id, val);
                    self.pc += 5;
                }
                Inst::GET_CONST => {
                    let id = iseq.read_id(self.pc + 1);
                    let slot = iseq.read32(self.pc + 5);
                    let val = match self.globals.find_const_cache(slot) {
                        Some(val) => val,
                        None => {
                            let val = match self.get_env_const(id) {
                                Some(val) => val,
                                None => VM::get_super_const(self.class(), id)?,
                            };
                            self.globals.set_const_cache(slot, val);
                            val
                        }
                    };

                    self.stack_push(val);
                    self.pc += 9;
                }
                Inst::GET_CONST_TOP => {
                    let id = iseq.read_id(self.pc + 1);
                    let parent = BuiltinClass::object();
                    let val = self.get_const(parent, id)?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::GET_SCOPE => {
                    let parent = self.stack_pop().expect_mod_class()?;
                    let id = iseq.read_id(self.pc + 1);
                    let val = self.get_const(parent, id)?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::SET_IVAR => {
                    let var_id = iseq.read_id(self.pc + 1);
                    let new_val = self.stack_pop();
                    self_value.set_var(var_id, new_val);
                    self.pc += 5;
                }
                Inst::GET_IVAR => {
                    let var_id = iseq.read_id(self.pc + 1);
                    let val = match self_value.get_var(var_id) {
                        Some(val) => val,
                        None => Value::nil(),
                    };
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::CHECK_IVAR => {
                    let var_id = iseq.read_id(self.pc + 1);
                    let val = match self_value.get_var(var_id) {
                        Some(_) => Value::false_val(),
                        None => Value::true_val(),
                    };
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::IVAR_ADDI => {
                    let var_id = iseq.read_id(self.pc + 1);
                    let i = iseq.read32(self.pc + 5) as i32;
                    let v = self_value
                        .rvalue_mut()
                        .var_table_mut()
                        .entry(var_id)
                        .or_insert(Value::nil());
                    *v = self.eval_addi(*v, i)?;

                    self.pc += 9;
                }
                Inst::SET_GVAR => {
                    let var_id = iseq.read_id(self.pc + 1);
                    let new_val = self.stack_pop();
                    self.set_global_var(var_id, new_val);
                    self.pc += 5;
                }
                Inst::GET_GVAR => {
                    let var_id = iseq.read_id(self.pc + 1);
                    let val = self.get_global_var(var_id).unwrap_or(Value::nil());
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::CHECK_GVAR => {
                    let var_id = iseq.read_id(self.pc + 1);
                    let val = match self.get_global_var(var_id) {
                        Some(_) => Value::false_val(),
                        None => Value::true_val(),
                    };
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::SET_CVAR => {
                    let var_id = iseq.read_id(self.pc + 1);
                    let new_val = self.stack_pop();
                    self.set_class_var(var_id, new_val)?;
                    self.pc += 5;
                }
                Inst::GET_CVAR => {
                    let var_id = iseq.read_id(self.pc + 1);
                    let val = self.get_class_var(var_id)?;
                    self.stack_push(val);
                    self.pc += 5;
                }
                Inst::SET_INDEX => {
                    self.set_index()?;
                    self.pc += 1;
                }
                Inst::GET_INDEX => {
                    let val = self.get_index()?;
                    self.stack_push(val);
                    self.pc += 1;
                }
                Inst::SET_IDX_I => {
                    let idx = iseq.read32(self.pc + 1);
                    self.set_index_imm(idx)?;
                    self.pc += 5;
                }
                Inst::GET_IDX_I => {
                    let idx = iseq.read32(self.pc + 1);
                    let val = self.get_index_imm(idx)?;
                    self.stack_push(val);
                    self.pc += 5;
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
                    let ctx = self.context();
                    let proc_obj = self.create_proc(&Block::Block(method, ctx))?;
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

                Inst::JMP_F_EQ => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let b = self.eval_eq(rhs, lhs)?;
                    self.jmp_cond(iseq, b, 5, 1);
                }
                Inst::JMP_F_NE => {
                    let lhs = self.stack_pop();
                    let rhs = self.stack_pop();
                    let b = !self.eval_eq(rhs, lhs)?;
                    self.jmp_cond(iseq, b, 5, 1);
                }
                Inst::JMP_F_GT => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let b = self.eval_gt(rhs, lhs)?;
                    self.jmp_cond(iseq, b, 5, 1);
                }
                Inst::JMP_F_GE => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let b = self.eval_ge(rhs, lhs)?;
                    self.jmp_cond(iseq, b, 5, 1);
                }
                Inst::JMP_F_LT => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let b = self.eval_lt(rhs, lhs)?;
                    self.jmp_cond(iseq, b, 5, 1);
                }
                Inst::JMP_F_LE => {
                    let rhs = self.stack_pop();
                    let lhs = self.stack_pop();
                    let b = self.eval_le(rhs, lhs)?;
                    self.jmp_cond(iseq, b, 5, 1);
                }

                Inst::JMP_F_EQI => {
                    let lhs = self.stack_pop();
                    let i = iseq.read32(self.pc + 1) as i32;
                    let b = self.eval_eqi(lhs, i);
                    self.jmp_cond(iseq, b, 9, 5);
                }
                Inst::JMP_F_NEI => {
                    let lhs = self.stack_pop();
                    let i = iseq.read32(self.pc + 1) as i32;
                    let b = !self.eval_eqi(lhs, i);
                    self.jmp_cond(iseq, b, 9, 5);
                }
                Inst::JMP_F_GTI => {
                    let lhs = self.stack_pop();
                    let i = iseq.read32(self.pc + 1) as i32;
                    let b = self.eval_gti(lhs, i)?;
                    self.jmp_cond(iseq, b, 9, 5);
                }
                Inst::JMP_F_GEI => {
                    let lhs = self.stack_pop();
                    let i = iseq.read32(self.pc + 1) as i32;
                    let b = self.eval_gei(lhs, i)?;
                    self.jmp_cond(iseq, b, 9, 5);
                }
                Inst::JMP_F_LTI => {
                    let lhs = self.stack_pop();
                    let i = iseq.read32(self.pc + 1) as i32;
                    let b = self.eval_lti(lhs, i)?;
                    self.jmp_cond(iseq, b, 9, 5);
                }
                Inst::JMP_F_LEI => {
                    let lhs = self.stack_pop();
                    let i = iseq.read32(self.pc + 1) as i32;
                    let b = self.eval_lei(lhs, i)?;
                    self.jmp_cond(iseq, b, 9, 5);
                }

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
                Inst::SEND => {
                    let receiver = self.stack_pop();
                    try_push!(self.vm_send(iseq, receiver));
                    self.pc += 17;
                }
                Inst::SEND_SELF => {
                    try_push!(self.vm_send(iseq, self_value));
                    self.pc += 17;
                }
                Inst::OPT_SEND => {
                    let receiver = self.stack_pop();
                    try_push!(self.vm_fast_send(iseq, receiver));
                    self.pc += 15;
                }
                Inst::OPT_NSEND => {
                    let receiver = self.stack_pop();
                    try_no_push!(self.vm_fast_send(iseq, receiver));
                    self.pc += 15;
                }
                Inst::OPT_SEND_SELF => {
                    try_push!(self.vm_fast_send(iseq, self_value));
                    self.pc += 15;
                }
                Inst::OPT_NSEND_SELF => {
                    try_no_push!(self.vm_fast_send(iseq, self_value));
                    self.pc += 15;
                }
                Inst::FOR => {
                    let receiver = self.stack_pop();
                    try_push!(self.vm_for(iseq, receiver));
                    self.pc += 9;
                }
                Inst::YIELD => {
                    let args_num = iseq.read32(self.pc + 1) as usize;
                    let args = self.pop_args_to_args(args_num);
                    try_push!(self.eval_yield(&args));
                    self.pc += 5;
                }
                Inst::DEF_CLASS => {
                    let is_module = iseq.read8(self.pc + 1) == 1;
                    let id = iseq.read_id(self.pc + 2);
                    let method = iseq.read_method(self.pc + 6);
                    let base = self.stack_pop();
                    let super_val = self.stack_pop();
                    let val = self.define_class(base, id, is_module, super_val)?;
                    self.class_push(val);
                    let mut iseq = method.as_iseq();
                    iseq.class_defined = self.get_class_defined();
                    let res = self.eval_method(method, val, &Args::new0());
                    self.class_pop();
                    try_push!(res);
                    self.pc += 10;
                }
                Inst::DEF_SCLASS => {
                    let method = iseq.read_method(self.pc + 1);
                    let singleton = self.stack_pop().get_singleton_class()?;
                    self.class_push(singleton);
                    let mut iseq = method.as_iseq();
                    iseq.class_defined = self.get_class_defined();
                    let res = self.eval_method(method, singleton, &Args::new0());
                    self.class_pop();
                    try_push!(res);
                    self.pc += 5;
                }
                Inst::DEF_METHOD => {
                    let id = iseq.read_id(self.pc + 1);
                    let method = iseq.read_method(self.pc + 5);
                    let mut iseq = method.as_iseq();
                    iseq.class_defined = self.get_method_iseq().class_defined.clone();
                    self.define_method(self_value, id, method);
                    if self.define_mode().module_function {
                        self.define_singleton_method(self_value, id, method)?;
                    };
                    self.pc += 9;
                }
                Inst::DEF_SMETHOD => {
                    let id = iseq.read_id(self.pc + 1);
                    let method = iseq.read_method(self.pc + 5);
                    let mut iseq = method.as_iseq();
                    iseq.class_defined = self.get_method_iseq().class_defined.clone();
                    let singleton = self.stack_pop();
                    self.define_singleton_method(singleton, id, method)?;
                    if self.define_mode().module_function {
                        self.define_method(singleton, id, method);
                    };
                    self.pc += 9;
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
                Inst::REP_UNINIT => {
                    let mut val = self.stack_pop();
                    if val.is_uninitialized() {
                        val = Value::nil()
                    }
                    self.stack_push(val);
                    self.pc += 1;
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

// helper functions for run_context_main.
impl VM {
    fn vm_send(&mut self, iseq: &mut ISeq, receiver: Value) -> VMResult {
        let method_id = iseq.read_id(self.pc + 1);
        let args_num = iseq.read16(self.pc + 5);
        let kw_rest_num = iseq.read8(self.pc + 7);
        let flag = iseq.read8(self.pc + 8);
        let block = iseq.read32(self.pc + 9);
        let cache = iseq.read32(self.pc + 13);

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
            let mut hash = FxHashMap::default();
            for h in kwrest {
                for (k, v) in h.expect_hash("Arg")? {
                    hash.insert(HashKey(k), v);
                }
            }
            Value::hash_from_map(hash)
        };

        let block = if block != 0 {
            Block::Block(block.into(), self.context())
        } else if flag & 0b10 == 2 {
            let val = self.stack_pop();
            if val.is_nil() {
                Block::None
            } else {
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
        self.send_icache(cache, method_id, receiver, &args)
    }

    fn vm_fast_send(&mut self, iseq: &mut ISeq, receiver: Value) -> VMResult {
        // With block and no keyword/block/splat arguments for OPT_SEND.
        let method_id = iseq.read_id(self.pc + 1);
        let args_num = iseq.read16(self.pc + 5) as usize;
        let block = iseq.read32(self.pc + 7);
        let cache_id = iseq.read32(self.pc + 11);
        let len = self.stack_len();
        let arg_slice = &self.exec_stack[len - args_num..];
        let rec_class = receiver.get_class_for_method();
        match MethodRepo::find_method_inline_cache(cache_id, rec_class, method_id) {
            Some(method) => match MethodRepo::get(method) {
                MethodInfo::BuiltinFunc { func, name } => {
                    let mut args = Args::from_slice(arg_slice);
                    args.block = match block {
                        0 => Block::None,
                        i => Block::Block(MethodId::from(i), self.context()),
                    };
                    self.set_stack_len(len - args_num);
                    self.invoke_native(&func, method, name, receiver, &args)
                }
                MethodInfo::AttrReader { id } => {
                    if args_num != 0 {
                        return Err(RubyError::argument_wrong(args_num, 0));
                    }
                    Self::invoke_getter(id, receiver)
                }
                MethodInfo::AttrWriter { id } => {
                    if args_num != 1 {
                        return Err(RubyError::argument_wrong(args_num, 1));
                    }
                    Self::invoke_setter(id, receiver, self.stack_pop())
                }
                MethodInfo::RubyFunc { iseq } => {
                    let block = match block {
                        0 => Block::None,
                        i => Block::Block(MethodId::from(i), self.context()),
                    };
                    if iseq.opt_flag {
                        let mut context = Context::new(receiver, block, iseq, None);
                        let req_len = iseq.params.req;
                        if args_num != req_len {
                            return Err(RubyError::argument_wrong(args_num, req_len));
                        };
                        context.copy_from_slice0(arg_slice);
                        self.set_stack_len(len - args_num);
                        self.run_context(&context)
                    } else {
                        let mut args = Args::from_slice(arg_slice);
                        args.block = block;
                        self.set_stack_len(len - args_num);
                        let context = Context::from_args(self, receiver, iseq, &args, None)?;
                        self.run_context(&context)
                    }
                }
                _ => unreachable!(),
            },
            None => {
                let mut args = Args::from_slice(arg_slice);
                args.block = match block {
                    0 => Block::None,
                    i => Block::Block(MethodId::from(i), self.context()),
                };
                self.set_stack_len(len - args_num);
                self.send_method_missing(method_id, receiver, &args)
            }
        }
    }

    fn vm_for(&mut self, iseq: &mut ISeq, receiver: Value) -> VMResult {
        // With block and no keyword/block/splat arguments for OPT_SEND.
        let block = iseq.read_method(self.pc + 1);
        //assert!(block != 0);
        let block = Block::Block(block, self.context());
        let args = Args::new0_block(block);
        let cache = iseq.read32(self.pc + 5);
        let rec_class = receiver.get_class_for_method();
        match MethodRepo::find_method_inline_cache(cache, rec_class, IdentId::EACH) {
            Some(method) => match MethodRepo::get(method) {
                MethodInfo::BuiltinFunc { func, name } => {
                    self.invoke_native(&func, method, name, receiver, &args)
                }
                MethodInfo::RubyFunc { iseq } => {
                    let context = Context::from_args(self, receiver, iseq, &args, None)?;
                    self.run_context(&context)
                }
                _ => unreachable!(),
            },
            None => self.send_method_missing(IdentId::EACH, receiver, &args),
        }
    }
}
