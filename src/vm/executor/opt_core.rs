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
        let iseqref = self.context().iseq_ref.unwrap();
        #[cfg(debug_assertions)]
        let kind = iseqref.kind;
        let iseq = &iseqref.iseq;
        let self_value = self.context().self_value;
        //let stack_len = self.stack_len();
        self.gc();
        for (i, (outer, lvar)) in iseqref.forvars.iter().enumerate() {
            self.get_outer_context(*outer)[*lvar as usize] = self.context()[i];
        }
        /// Evaluate expr, and push return value to stack.
        macro_rules! try_err {
            ($eval:expr) => {
                match $eval {
                    Ok(()) => {}
                    Err(err) => match err.kind {
                        RubyErrorKind::BlockReturn => {}
                        RubyErrorKind::MethodReturn if self.is_method() => {
                            return Ok(());
                        }
                        _ => return Err(err),
                    },
                };
            };
        }

        macro_rules! try_send {
            ($eval:expr) => {
                match $eval {
                    Ok(()) => {}
                    Err(err) => match err.kind {
                        RubyErrorKind::BlockReturn => {}
                        RubyErrorKind::MethodReturn if self.is_method() => {
                            return Ok(());
                        }
                        _ => return Err(err),
                    },
                };
            };
        }

        loop {
            self.context().cur_pc = self.pc;
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
                    return Ok(());
                }
                Inst::BREAK => {
                    // - `break`  in block or eval AND outer of loops.
                    #[cfg(debug_assertions)]
                    assert!(kind == ISeqKind::Block || kind == ISeqKind::Other);
                    let err = RubyError::block_return();
                    return Err(err);
                }
                Inst::MRETURN => {
                    // - `return` in block
                    #[cfg(debug_assertions)]
                    assert!(kind == ISeqKind::Block);
                    let err = RubyError::method_return();
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
                    self.pc += 1;
                    self.invoke_eq(rhs, lhs)?;
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
                    self.pc += 1;
                    self.invoke_neq(rhs, lhs)?;
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
                    self.pc += 1;
                    self.invoke_teq(rhs, lhs)?;
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
                Inst::CHECK_CONST => {
                    let id = iseq.read_id(self.pc + 1);
                    let is_undef = match self.get_env_const(id) {
                        Some(_) => false,
                        None => VM::get_super_const(self.class(), id).is_err(),
                    };
                    self.stack_push(Value::bool(is_undef));
                    self.pc += 5;
                }
                Inst::GET_CONST => {
                    let id = iseq.read_id(self.pc + 1);
                    let slot = iseq.read32(self.pc + 5);
                    let val = match self.globals.find_const_cache(slot) {
                        Some(val) => val,
                        None => {
                            let val = self.find_const(id)?;
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
                Inst::CHECK_SCOPE => {
                    let parent = self.stack_pop();
                    let id = iseq.read_id(self.pc + 1);
                    let is_undef = match parent.expect_mod_class() {
                        Ok(parent) => self.get_const(parent, id).is_err(),
                        Err(_) => true,
                    };
                    self.stack_push(Value::bool(is_undef));
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
                    let is_undef = rec_class.get_method(method).is_none();
                    self.stack_push(Value::bool(is_undef));
                    self.pc += 5;
                }
                Inst::SEND => {
                    let receiver = self.stack_pop();
                    try_err!(self.vm_send(iseq, receiver));
                }
                Inst::SEND_SELF => {
                    try_err!(self.vm_send(iseq, self_value));
                }
                Inst::OPT_SEND => {
                    let receiver = self.stack_pop();
                    try_send!(self.vm_fast_send(iseq, receiver));
                }
                Inst::OPT_SEND_SELF => {
                    try_send!(self.vm_fast_send(iseq, self_value));
                }
                Inst::FOR => {
                    let receiver = self.stack_pop();
                    let block = iseq.read_method(self.pc + 1);
                    let cache = iseq.read32(self.pc + 5);
                    try_err!(self.vm_for(receiver, block, cache));
                }
                Inst::YIELD => {
                    let args_num = iseq.read32(self.pc + 1) as usize;
                    let args = self.pop_args_to_args(args_num);
                    try_err!(self.invoke_yield(&args));
                    self.pc += 5;
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
                    let res = self.invoke_method(method, val, &Args::new0());
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
                    let res = self.invoke_method(method, singleton, &Args::new0());
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
    fn vm_send(&mut self, iseq: &ISeq, receiver: Value) -> Result<(), RubyError> {
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

    fn vm_fast_send(&mut self, iseq: &ISeq, receiver: Value) -> Result<(), RubyError> {
        // With block and no keyword/block/splat arguments for OPT_SEND.
        let method_id = iseq.read_id(self.pc + 1);
        let args_num = iseq.read16(self.pc + 5) as usize;
        let block = iseq.read32(self.pc + 7);
        let cache_id = iseq.read32(self.pc + 11);
        self.pc += 15;
        let len = self.stack_len();
        let rec_class = receiver.get_class_for_method();
        match MethodRepo::find_method_inline_cache(cache_id, rec_class, method_id) {
            Some(method) => match MethodRepo::get(method) {
                MethodInfo::BuiltinFunc { func, name } => {
                    let mut args = Args::from_slice(&self.exec_stack[len - args_num..]);
                    args.block = Block::from_u32(block, self);
                    self.set_stack_len(len - args_num);
                    self.invoke_native(&func, method, name, receiver, &args)?;
                    Ok(())
                }
                MethodInfo::AttrReader { id } => {
                    if args_num != 0 {
                        return Err(RubyError::argument_wrong(args_num, 0));
                    }
                    self.invoke_getter(id, receiver)?;
                    Ok(())
                }
                MethodInfo::AttrWriter { id } => {
                    if args_num != 1 {
                        return Err(RubyError::argument_wrong(args_num, 1));
                    }
                    let val = self.stack_pop();
                    self.invoke_setter(id, receiver, val)?;
                    Ok(())
                }
                MethodInfo::RubyFunc { iseq } => {
                    let block = Block::from_u32(block, self);
                    if iseq.opt_flag {
                        let req_len = iseq.params.req;
                        if args_num != req_len {
                            return Err(RubyError::argument_wrong(args_num, req_len));
                        };
                        let mut context = self.new_stack_context_with(receiver, block, iseq, None);
                        context.copy_from_slice0(&self.exec_stack[len - args_num..]);
                        self.set_stack_len(len - args_num);
                        self.run_context(context)
                    } else {
                        let mut args = Args::from_slice(&self.exec_stack[len - args_num..]);
                        args.block = block;
                        self.set_stack_len(len - args_num);
                        let context = ContextRef::from_args(self, receiver, iseq, &args, None)?;
                        self.run_context(context)
                    }
                }
                _ => unreachable!(),
            },
            None => {
                let mut args = Args::from_slice(&self.exec_stack[len - args_num..]);
                args.block = Block::from_u32(block, self);
                self.set_stack_len(len - args_num);
                self.send_method_missing(method_id, receiver, &args)
            }
        }
    }

    fn vm_for(
        &mut self,
        receiver: Value,
        method_id: MethodId,
        cache: u32,
    ) -> Result<(), RubyError> {
        // With block and no keyword/block/splat arguments for OPT_SEND.
        let block = self.new_block(method_id);
        let args = Args::new0_block(block);
        self.pc += 9;
        let rec_class = receiver.get_class_for_method();
        match MethodRepo::find_method_inline_cache(cache, rec_class, IdentId::EACH) {
            Some(method) => match MethodRepo::get(method) {
                MethodInfo::BuiltinFunc { func, name } => {
                    self.invoke_native(&func, method, name, receiver, &args)
                }
                MethodInfo::RubyFunc { iseq } => {
                    let context = ContextRef::from_args(self, receiver, iseq, &args, None)?;
                    self.run_context(context)
                }
                _ => unreachable!(),
            },
            None => self.send_method_missing(IdentId::EACH, receiver, &args),
        }
    }
}
