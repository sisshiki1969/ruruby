use num::{bigint::ToBigInt, BigInt, ToPrimitive};

use crate::*;

macro_rules! invoke_op_i {
    ($vm:ident, $iseq:ident, $i:ident, $op:ident, $id:expr) => {
        let lhs = $vm.stack_pop();
        let val = if let Some(i) = lhs.as_fixnum() {
            Value::integer(i.$op($i as i64))
        } else if let Some(f) = lhs.as_flonum() {
            Value::float(f.$op($i as f64))
        } else {
            return $vm.fallback_for_binop($id, lhs, Value::integer($i as i64));
        };
        $vm.stack_push(val);
        return Ok(());
    };
}

macro_rules! invoke_op {
    ($vm:ident, $op1:ident, $op2:ident, $id:expr) => {
        let len = $vm.stack_len();
        let lhs = $vm.exec_stack[len - 2];
        let rhs = $vm.exec_stack[len - 1];
        $vm.set_stack_len(len - 2);
        let val = if let Some(lhsi) = lhs.as_fixnum() {
            if let Some(rhsi) = rhs.as_fixnum() {
                match lhsi.$op2(rhsi) {
                    Some(res) => Value::integer(res),
                    None => {
                        Value::bignum((lhsi.to_bigint().unwrap()).$op1(rhsi.to_bigint().unwrap()))
                    }
                }
            } else if let Some(rhsf) = rhs.as_flonum() {
                Value::float((lhsi as f64).$op1(rhsf))
            } else {
                return $vm.fallback_for_binop($id, lhs, rhs);
            }
        } else if let Some(lhsf) = lhs.as_flonum() {
            if let Some(rhsi) = rhs.as_fixnum() {
                Value::float(lhsf.$op1(rhsi as f64))
            } else if let Some(rhsf) = rhs.as_flonum() {
                Value::float(lhsf.$op1(rhsf))
            } else {
                return $vm.fallback_for_binop($id, lhs, rhs);
            }
        } else {
            return $vm.fallback_for_binop($id, lhs, rhs);
        };
        $vm.stack_push(val);
        return Ok(())
    };
}

impl VM {
    pub(super) fn invoke_add(&mut self) -> Result<(), RubyError> {
        use std::ops::Add;
        invoke_op!(self, add, checked_add, IdentId::_ADD);
    }

    pub(super) fn invoke_sub(&mut self) -> Result<(), RubyError> {
        use std::ops::Sub;
        invoke_op!(self, sub, checked_sub, IdentId::_SUB);
    }

    pub(super) fn invoke_mul(&mut self) -> Result<(), RubyError> {
        use std::ops::Mul;
        invoke_op!(self, mul, checked_mul, IdentId::_MUL);
    }

    pub(super) fn invoke_div(&mut self) -> Result<(), RubyError> {
        use std::ops::Div;
        let rhs = self.exec_stack[self.stack_len() - 1];
        if rhs.as_float() == Some(0.0) {
            self.set_stack_len(self.stack_len() - 2);
            self.stack_push(Value::float(f64::NAN));
            return Ok(());
        }
        if rhs.as_fixnum() == Some(0) {
            return Err(RubyError::zero_div("Divided by zero."));
        }
        invoke_op!(self, div, checked_div, IdentId::_DIV);
    }

    pub(super) fn invoke_addi(&mut self, i: i32) -> Result<(), RubyError> {
        use std::ops::Add;
        invoke_op_i!(self, iseq, i, add, IdentId::_ADD);
    }

    pub(super) fn invoke_subi(&mut self, i: i32) -> Result<(), RubyError> {
        use std::ops::Sub;
        invoke_op_i!(self, iseq, i, sub, IdentId::_SUB);
    }

    pub(super) fn invoke_rem(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        fn rem_floorf64(self_: f64, other: f64) -> Result<f64, RubyError> {
            if other == 0.0 {
                return Err(RubyError::zero_div("Divided by zero."));
            }
            let res = if self_ > 0.0 && other < 0.0 {
                ((self_ - 1.0) % other) + other + 1.0
            } else if self_ < 0.0 && other > 0.0 {
                ((self_ + 1.0) % other) + other - 1.0
            } else {
                self_ % other
            };
            Ok(res)
        }
        use divrem::*;
        let val = match (lhs.unpack(), rhs.unpack()) {
            (RV::Integer(lhs), RV::Integer(rhs)) => {
                if rhs == 0 {
                    return Err(RubyError::zero_div("Divided by zero."));
                }
                Value::integer(lhs.rem_floor(rhs))
            }
            (RV::Integer(lhs), RV::Float(rhs)) => Value::float(rem_floorf64(lhs as f64, rhs)?),
            (RV::Float(lhs), RV::Integer(rhs)) => Value::float(rem_floorf64(lhs, rhs as f64)?),
            (RV::Float(lhs), RV::Float(rhs)) => Value::float(rem_floorf64(lhs, rhs)?),
            (_, _) => {
                return self.fallback_for_binop(IdentId::_REM, lhs, rhs);
            }
        };
        self.stack_push(val);
        Ok(())
    }

    pub(super) fn invoke_exp(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        let val = match (lhs.unpack(), rhs.unpack()) {
            (RV::Integer(lhs), RV::Integer(rhs)) => {
                if 0 <= rhs && rhs <= std::u32::MAX as i64 {
                    if let Some(i) = lhs.checked_pow(rhs as u32) {
                        Value::integer(i)
                    } else {
                        Value::bignum(BigInt::from(lhs).pow(rhs as u32))
                    }
                } else {
                    Value::float((lhs as f64).powf(rhs as f64))
                }
            }
            (RV::Integer(lhs), RV::Float(rhs)) => Value::float((lhs as f64).powf(rhs)),
            (RV::Float(lhs), RV::Integer(rhs)) => Value::float(lhs.powf(rhs as f64)),
            (RV::Float(lhs), RV::Float(rhs)) => Value::float(lhs.powf(rhs)),
            _ => {
                return self.fallback_for_binop(IdentId::_POW, lhs, rhs);
            }
        };
        self.stack_push(val);
        Ok(())
    }

    pub(super) fn invoke_neg(&mut self, lhs: Value) -> Result<(), RubyError> {
        let val = match lhs.unpack() {
            RV::Integer(i) => match i.checked_neg() {
                Some(i) => Value::integer(i),
                None => return Err(RubyError::runtime("Negate overflow.")),
            },
            RV::Float(f) => Value::float(-f),
            _ => return self.invoke_send0(IdentId::get_id("-@"), lhs),
        };
        self.stack_push(val);
        Ok(())
    }

    pub(super) fn invoke_shl(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        if let Some(lhsi) = lhs.as_fixnum() {
            if let Some(rhsi) = rhs.as_fixnum() {
                let val = if 0 < rhsi {
                    Self::fixnum_shl(lhsi, rhsi)
                } else {
                    Self::fixnum_shr(lhsi, -rhsi)
                };
                self.stack_push(val);
                return Ok(());
            }
        }
        if let Some(mut ainfo) = lhs.as_array() {
            ainfo.push(rhs);
            self.stack_push(lhs);
            Ok(())
        } else {
            self.fallback_for_binop(IdentId::_SHL, lhs, rhs)
        }
    }

    pub(super) fn invoke_shr(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        if let Some(lhsi) = lhs.as_fixnum() {
            if let Some(rhsi) = rhs.as_fixnum() {
                let val = if 0 < rhsi {
                    Self::fixnum_shr(lhsi, rhsi)
                } else {
                    Self::fixnum_shl(lhsi, -rhsi)
                };
                self.stack_push(val);
                return Ok(());
            }
        }
        self.fallback_for_binop(IdentId::_SHR, lhs, rhs)
    }

    /// rhs must be a non-negative value.
    fn fixnum_shr(lhs: i64, rhs: i64) -> Value {
        if rhs < u32::MAX as i64 {
            match lhs.checked_shr(rhs as u32) {
                Some(i) => Value::integer(i),
                None => Value::integer(0),
            }
        } else {
            Value::bignum(lhs.to_bigint().unwrap() >> rhs)
        }
    }

    /// rhs must be a non-negative value.
    fn fixnum_shl(lhs: i64, rhs: i64) -> Value {
        if rhs < u32::MAX as i64 {
            match lhs.checked_shl(rhs as u32) {
                Some(i) => Value::integer(i),
                None => {
                    let n = lhs.to_bigint().unwrap() << rhs;
                    Value::bignum(n)
                }
            }
        } else {
            Value::bignum(lhs.to_bigint().unwrap() << rhs)
        }
    }

    pub(super) fn invoke_bitand(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        if let Some(lhsi) = lhs.as_fixnum() {
            if let Some(rhsi) = rhs.as_fixnum() {
                let val = Value::integer(lhsi & rhsi);
                self.stack_push(val);
                return Ok(());
            }
        }
        let val = match (lhs.unpack(), rhs.unpack()) {
            (RV::True, _) => Value::bool(rhs.to_bool()),
            (RV::False, _) => Value::false_val(),
            (RV::Integer(lhs), RV::Integer(rhs)) => Value::integer(lhs & rhs),
            (RV::Nil, _) => Value::false_val(),
            (_, _) => {
                return self.fallback_for_binop(IdentId::get_id("&"), lhs, rhs);
            }
        };
        self.stack_push(val);
        Ok(())
    }

    pub(super) fn invoke_bitor(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        if let Some(lhsi) = lhs.as_fixnum() {
            if let Some(rhsi) = rhs.as_fixnum() {
                let val = Value::integer(lhsi | rhsi);
                self.stack_push(val);
                return Ok(());
            }
        }
        let val = match (lhs.unpack(), rhs.unpack()) {
            (RV::True, _) => Value::true_val(),
            (RV::False, _) | (RV::Nil, _) => Value::bool(rhs.to_bool()),
            (RV::Integer(lhs), RV::Integer(rhs)) => Value::integer(lhs | rhs),
            (_, _) => {
                return self.fallback_for_binop(IdentId::get_id("|"), lhs, rhs);
            }
        };
        self.stack_push(val);
        Ok(())
    }

    pub(super) fn eval_bitxor(&mut self, rhs: Value, lhs: Value) -> VMResult {
        match (lhs.unpack(), rhs.unpack()) {
            (RV::True, _) => Ok(Value::bool(!rhs.to_bool())),
            (RV::False, _) | (RV::Nil, _) => Ok(Value::bool(rhs.to_bool())),
            (RV::Integer(lhs), RV::Integer(rhs)) => Ok(Value::integer(lhs ^ rhs)),
            (_, _) => {
                self.fallback_for_binop(IdentId::get_id("^"), lhs, rhs)?;
                Ok(self.stack_pop())
            }
        }
    }

    pub(super) fn eval_bitnot(&mut self, lhs: Value) -> VMResult {
        match lhs.unpack() {
            RV::Integer(lhs) => Ok(Value::integer(!lhs)),
            _ => Err(RubyError::undefined_method(IdentId::get_id("~"), lhs)),
        }
    }
}

macro_rules! eval_cmp {
    ($vm:ident, $func_name:ident, $op:ident, $id:expr) => {
        pub(super) fn $func_name(&mut $vm) -> Result<bool, RubyError> {
            let len = $vm.stack_len();
            let lhs = unsafe { *$vm.exec_stack.get_unchecked(len - 2) };
            let rhs = unsafe { *$vm.exec_stack.get_unchecked(len - 1) };
            $vm.set_stack_len(len - 2);
            let res = eval_cmp2!($vm, rhs, lhs, $op, $id);
            res
        }
    };
}

macro_rules! eval_cmp2 {
    ($vm:ident, $rhs:expr, $lhs:expr, $op:ident, $id:expr) => {{
        if let Some(lhsi) = $lhs.as_fixnum() {
            if let Some(rhsi) = $rhs.as_fixnum() {
                Ok(lhsi.$op(&rhsi))
            } else if let Some(rhsf) = $rhs.as_flonum() {
                Ok((lhsi as f64).$op(&rhsf))
            } else {
                $vm.fallback_for_binop($id, $lhs, $rhs)?;
                Ok($vm.stack_pop().to_bool())
            }
        } else if let Some(lhsf) = $lhs.as_flonum() {
            if let Some(rhsi) = $rhs.as_fixnum() {
                Ok(lhsf.$op(&(rhsi as f64)))
            } else if let Some(rhsf) = $rhs.as_flonum() {
                Ok(lhsf.$op(&rhsf))
            } else {
                $vm.fallback_for_binop($id, $lhs, $rhs)?;
                Ok($vm.stack_pop().to_bool())
            }
        } else {
            $vm.fallback_for_binop($id, $lhs, $rhs)?;
            Ok($vm.stack_pop().to_bool())
        }
    }};
}

macro_rules! eval_cmp_i {
    ($vm:ident,$func_name:ident, $op:ident, $id:expr) => {
        pub(super) fn $func_name(&mut $vm, lhs: Value, i: i32) -> Result<bool, RubyError> {
            if let Some(lhsi) = lhs.as_fixnum() {
                let i = i as i64;
                Ok(lhsi.$op(&i))
            } else if let Some(lhsf) = lhs.as_flonum() {
                let i = i as f64;
                Ok(lhsf.$op(&i))
            } else {
                $vm.fallback_for_binop($id, lhs, Value::integer(i as i64))?;
                Ok($vm.stack_pop().to_bool())
            }
        }
    };
}

impl VM {
    pub(super) fn invoke_eq(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        let b = self.eval_eq2(rhs, lhs)?;
        self.stack_push(Value::bool(b));
        Ok(())
    }

    pub(super) fn invoke_teq(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        let b = match lhs.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Module(_) => {
                    return self.fallback_for_binop(IdentId::_TEQ, lhs, rhs);
                }
                ObjKind::Regexp(re) => {
                    let given = match rhs.unpack() {
                        RV::Symbol(sym) => IdentId::get_name(sym),
                        RV::Object(_) => match rhs.as_string() {
                            Some(s) => s.to_owned(),
                            None => {
                                self.stack_push(Value::false_val());
                                return Ok(());
                            }
                        },
                        _ => {
                            self.stack_push(Value::false_val());
                            return Ok(());
                        }
                    };
                    RegexpInfo::find_one(self, &*re, &given)?.is_some()
                }
                _ => return self.invoke_eq(lhs, rhs),
            },
            None => return self.invoke_eq(lhs, rhs),
        };
        self.stack_push(Value::bool(b));
        Ok(())
    }

    pub fn eval_teq(&mut self, rhs: Value, lhs: Value) -> Result<bool, RubyError> {
        match lhs.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Module(_) => {
                    self.fallback_for_binop(IdentId::_TEQ, lhs, rhs)?;
                    Ok(self.stack_pop().to_bool())
                }
                ObjKind::Regexp(re) => {
                    let given = match rhs.unpack() {
                        RV::Symbol(sym) => IdentId::get_name(sym),
                        RV::Object(_) => match rhs.as_string() {
                            Some(s) => s.to_owned(),
                            None => return Ok(false),
                        },
                        _ => return Ok(false),
                    };
                    let res = RegexpInfo::find_one(self, &*re, &given)?.is_some();
                    Ok(res)
                }
                _ => Ok(self.eval_eq2(lhs, rhs)?),
            },
            None => Ok(self.eval_eq2(lhs, rhs)?),
        }
    }

    /// Equality of Value.
    ///
    /// This kind of equality is used for `==` method (or operator) of Ruby.
    /// Generally, objects that are considered to have a same value are `==`.
    /// Numeric which have a same mathematical value are `==`.
    /// String or Symbol which indicate an identical string or symbol are `==`.
    /// Some classes have original difinitions of `==`.
    ///
    /// ex. 3.0 == 3.
    pub fn eval_eq2(&mut self, rhs: Value, lhs: Value) -> Result<bool, RubyError> {
        if rhs.is_packed_value() || lhs.is_packed_value() {
            if let Some(lhsi) = lhs.as_fixnum() {
                if let Some(rhsf) = rhs.as_flonum() {
                    return Ok(lhsi as f64 == rhsf);
                }
            } else if let Some(lhsf) = lhs.as_flonum() {
                if let Some(rhsi) = rhs.as_fixnum() {
                    return Ok(rhsi as f64 == lhsf);
                } else if let Some(rhsf) = rhs.as_flonum() {
                    if lhsf.is_nan() && rhsf.is_nan() {
                        return Ok(false);
                    }
                }
            }
            return Ok(lhs.id() == rhs.id());
        }
        if lhs.id() == rhs.id() {
            return Ok(true);
        };
        match (&lhs.rvalue().kind, &rhs.rvalue().kind) {
            (ObjKind::BigNum(lhs), ObjKind::BigNum(rhs)) => Ok(*lhs == *rhs),
            (ObjKind::Float(lhs), ObjKind::Float(rhs)) => Ok(*lhs == *rhs),
            (ObjKind::BigNum(lhs), ObjKind::Float(rhs)) => Ok(lhs.to_f64().unwrap() == *rhs),
            (ObjKind::Float(lhs), ObjKind::BigNum(rhs)) => Ok(*lhs == rhs.to_f64().unwrap()),
            (ObjKind::Complex { r: r1, i: i1 }, ObjKind::Complex { r: r2, i: i2 }) => {
                Ok(r1.to_real() == r2.to_real() && i1.to_real() == i2.to_real())
            }
            (ObjKind::String(lhs), ObjKind::String(rhs)) => Ok(lhs.as_bytes() == rhs.as_bytes()),
            (ObjKind::Array(lhs), ObjKind::Array(rhs)) => {
                if lhs.len() != rhs.len() {
                    return Ok(false);
                }
                for (l, r) in lhs.elements.iter().zip(rhs.elements.iter()) {
                    if !self.eval_eq2(*r, *l)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            (ObjKind::Range(lhs), ObjKind::Range(rhs)) => Ok(rhs.exclude == lhs.exclude
                && self.eval_eq2(rhs.start, lhs.start)?
                && self.eval_eq2(rhs.end, lhs.end)?),
            (ObjKind::Hash(lhs), ObjKind::Hash(rhs)) => Ok(**lhs == **rhs),
            (ObjKind::Regexp(lhs), ObjKind::Regexp(rhs)) => Ok(*lhs == *rhs),
            (ObjKind::Time(lhs), ObjKind::Time(rhs)) => Ok(*lhs == *rhs),
            (ObjKind::Invalid, _) | (_, ObjKind::Invalid) => {
                panic!("Invalid rvalue. (maybe GC problem) {:?}", lhs.rvalue())
            }
            (_, _) => {
                let val = match self.fallback_for_binop(IdentId::_EQ, lhs, rhs) {
                    Ok(()) => self.stack_pop(),
                    _ => return Ok(false),
                };
                Ok(val.to_bool())
            }
        }
    }

    pub(super) fn eval_eq(&mut self) -> Result<bool, RubyError> {
        let len = self.stack_len();
        let rhs = self.exec_stack[len - 1];
        let lhs = self.exec_stack[len - 2];
        self.set_stack_len(len - 2);
        self.eval_eq2(rhs, lhs)
    }

    pub(super) fn eval_ne(&mut self) -> Result<bool, RubyError> {
        Ok(!self.eval_eq()?)
    }

    eval_cmp!(self, eval_ge, ge, IdentId::_GE);
    eval_cmp!(self, eval_gt, gt, IdentId::_GT);
    eval_cmp!(self, eval_le, le, IdentId::_LE);
    eval_cmp!(self, eval_lt, lt, IdentId::_LT);

    pub fn eval_gt2(&mut self, rhs: Value, lhs: Value) -> Result<bool, RubyError> {
        eval_cmp2!(self, rhs, lhs, gt, IdentId::_GT)
    }

    pub(super) fn eval_eqi(&mut self, lhs: Value, i: i32) -> Result<bool, RubyError> {
        let res = if let Some(lhsi) = lhs.as_fixnum() {
            lhsi == i as i64
        } else if let Some(lhsf) = lhs.as_flonum() {
            lhsf == i as f64
        } else {
            match lhs.unpack() {
                RV::Integer(lhs) => lhs == i as i64,
                RV::Float(lhs) => lhs == i as f64,
                _ => {
                    self.fallback_for_binop(IdentId::_EQ, lhs, Value::integer(i as i64))?;
                    return Ok(self.stack_pop().to_bool());
                }
            }
        };

        Ok(res)
    }
    pub(super) fn eval_nei(&mut self, lhs: Value, i: i32) -> Result<bool, RubyError> {
        Ok(!self.eval_eqi(lhs, i)?)
    }

    eval_cmp_i!(self, eval_gei, ge, IdentId::_GE);
    eval_cmp_i!(self, eval_gti, gt, IdentId::_GT);
    eval_cmp_i!(self, eval_lei, le, IdentId::_LE);
    eval_cmp_i!(self, eval_lti, lt, IdentId::_LT);

    pub fn eval_compare(&mut self, rhs: Value, lhs: Value) -> VMResult {
        if rhs.id() == lhs.id() {
            return Ok(Value::integer(0));
        };
        let res = match lhs.unpack() {
            RV::Integer(lhs) => match rhs.unpack() {
                RV::Integer(rhs) => lhs.partial_cmp(&rhs),
                RV::Float(rhs) => (lhs as f64).partial_cmp(&rhs),
                _ => return Ok(Value::nil()),
            },
            RV::Float(lhs) => match rhs.unpack() {
                RV::Integer(rhs) => lhs.partial_cmp(&(rhs as f64)),
                RV::Float(rhs) => lhs.partial_cmp(&rhs),
                _ => return Ok(Value::nil()),
            },
            _ => {
                self.fallback_for_binop(IdentId::_CMP, lhs, rhs)?;
                return Ok(self.stack_pop());
            }
        };
        match res {
            Some(ord) => Ok(Value::integer(ord as i64)),
            None => Ok(Value::nil()),
        }
    }

    pub(super) fn invoke_set_index(&mut self) -> Result<(), RubyError> {
        let val = self.stack_pop();
        let idx = self.stack_pop();
        let mut receiver = self.stack_pop();

        match receiver.as_mut_rvalue() {
            Some(oref) => {
                match oref.kind {
                    ObjKind::Array(ref mut aref) => {
                        aref.set_elem1(idx, val)?;
                    }
                    ObjKind::Hash(ref mut href) => href.insert(idx, val),
                    _ => {
                        self.eval_send2(IdentId::_INDEX_ASSIGN, receiver, idx, val)?;
                    }
                };
            }
            None => {
                self.eval_send2(IdentId::_INDEX_ASSIGN, receiver, idx, val)?;
            }
        }
        Ok(())
    }

    pub(super) fn invoke_set_index_imm(&mut self, idx: u32) -> Result<(), RubyError> {
        let mut receiver = self.stack_pop();
        let val = self.stack_pop();
        match receiver.as_mut_rvalue() {
            Some(oref) => {
                match oref.kind {
                    ObjKind::Array(ref mut aref) => {
                        aref.set_elem_imm(idx as usize, val);
                    }
                    ObjKind::Hash(ref mut href) => href.insert(Value::integer(idx as i64), val),
                    _ => {
                        self.eval_send2(
                            IdentId::_INDEX_ASSIGN,
                            receiver,
                            Value::integer(idx as i64),
                            val,
                        )?;
                    }
                };
            }
            None => {
                self.eval_send2(
                    IdentId::_INDEX_ASSIGN,
                    receiver,
                    Value::integer(idx as i64),
                    val,
                )?;
            }
        }
        Ok(())
    }

    pub(super) fn invoke_get_index(
        &mut self,
        receiver: Value,
        idx: Value,
    ) -> Result<(), RubyError> {
        let val = match receiver.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Array(aref) => aref.get_elem1(idx)?,
                ObjKind::Hash(href) => match href.get(&idx) {
                    Some(val) => *val,
                    None => Value::nil(),
                },
                _ => return self.invoke_send1(IdentId::_INDEX, receiver, idx),
            },
            _ => return self.invoke_send1(IdentId::_INDEX, receiver, idx),
        };
        self.stack_push(val);
        Ok(())
    }

    pub(super) fn invoke_get_index_imm(
        &mut self,
        receiver: Value,
        idx: u32,
    ) -> Result<(), RubyError> {
        let val = match receiver.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Array(aref) => aref.get_elem_imm(idx as usize),
                ObjKind::Hash(href) => match href.get(&Value::integer(idx as i64)) {
                    Some(val) => *val,
                    None => Value::nil(),
                },
                ObjKind::Method(mref) if mref.receiver.is_some() => {
                    let args = Args::new1(Value::integer(idx as i64));
                    return self.exec_method(mref.method, mref.receiver.unwrap(), &args);
                }
                _ => {
                    return self.invoke_send1(
                        IdentId::_INDEX,
                        receiver,
                        Value::integer(idx as i64),
                    );
                }
            },
            None => {
                if let Some(i) = receiver.as_fixnum() {
                    let val = if 63 < idx { 0 } else { (i >> idx) & 1 };
                    Value::integer(val)
                } else {
                    return self.invoke_send1(
                        IdentId::_INDEX,
                        receiver,
                        Value::integer(idx as i64),
                    );
                }
            }
        };
        self.stack_push(val);
        Ok(())
    }
}
