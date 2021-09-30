use crate::*;
use divrem::DivFloor;
use num::Zero;
use num::{bigint::ToBigInt, ToPrimitive};
use std::ops::Add;
use std::ops::Mul;
use std::ops::Sub;

macro_rules! exec_op_i {
    ($vm:ident, $iseq:ident, $i:ident, $op:ident, $id:expr) => {
        let lhs = $vm.stack_pop();
        let val = if let Some(i) = lhs.as_fixnum() {
            Value::integer(i.$op($i as i64))
        } else if let Some(f) = lhs.as_flonum() {
            Value::float(f.$op($i as f64))
        } else {
            return $vm.exec_send1($id, lhs, Value::integer($i as i64));
        };
        $vm.stack_push(val);
        return Ok(());
    };
}

macro_rules! exec_op {
    ($fname:ident, $op1:ident, $op2:ident, $id:ident) => {
        pub(super) fn $fname(&mut self) -> Result<(), RubyError> {
            let (lhs, rhs) = self.stack_pop2();
            let val = if let Some(lhsi) = lhs.as_fixnum() {
                if let Some(rhsi) = rhs.as_fixnum() {
                    match lhsi.$op2(rhsi) {
                        Some(res) => Value::integer(res),
                        None => Value::bignum(
                            (lhsi.to_bigint().unwrap()).$op1(rhsi.to_bigint().unwrap()),
                        ),
                    }
                } else if let Some(rhsf) = rhs.as_flonum() {
                    Value::float((lhsi as f64).$op1(rhsf))
                } else {
                    return self.exec_send1(IdentId::$id, lhs, rhs);
                }
            } else if let Some(lhsf) = lhs.as_flonum() {
                if let Some(rhsi) = rhs.as_fixnum() {
                    Value::float(lhsf.$op1(rhsi as f64))
                } else if let Some(rhsf) = rhs.as_flonum() {
                    Value::float(lhsf.$op1(rhsf))
                } else {
                    return self.exec_send1(IdentId::$id, lhs, rhs);
                }
            } else {
                return self.exec_send1(IdentId::$id, lhs, rhs);
            };
            self.stack_push(val);
            return Ok(())
        }
    };
}

impl VM {
    exec_op!(exec_add, add, checked_add, _ADD);
    exec_op!(exec_sub, sub, checked_sub, _SUB);
    exec_op!(exec_mul, mul, checked_mul, _MUL);

    pub(super) fn exec_div(&mut self) -> Result<(), RubyError> {
        let (lhs, rhs) = self.stack_pop2();
        let val = if let Some(lhsi) = lhs.as_fixnum() {
            if let Some(rhsi) = rhs.as_fixnum() {
                if rhsi.is_zero() {
                    return Err(RubyError::zero_div("Divided by zero."));
                }
                Value::integer(lhsi.div_floor(rhsi))
            } else if let Some(rhsf) = rhs.as_flonum() {
                Value::float(lhsi as f64 / rhsf)
            } else {
                return self.exec_send1(IdentId::_DIV, lhs, rhs);
            }
        } else if let Some(lhsf) = lhs.as_flonum() {
            if let Some(rhsi) = rhs.as_fixnum() {
                Value::float(lhsf / rhsi as f64)
            } else if let Some(rhsf) = rhs.as_flonum() {
                Value::float(lhsf / rhsf)
            } else {
                return self.exec_send1(IdentId::_DIV, lhs, rhs);
            }
        } else {
            return self.exec_send1(IdentId::_DIV, lhs, rhs);
        };
        self.stack_push(val);
        return Ok(());
    }

    pub(super) fn exec_rem(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        let val = if let Some(lhs) = lhs.as_fixnum() {
            arith::rem_fixnum(lhs, rhs)?
        } else if let Some(lhs) = lhs.as_float() {
            arith::rem_float(lhs, rhs)?
        } else {
            return self.exec_send1(IdentId::_REM, lhs, rhs);
        };
        self.stack_push(val);
        Ok(())
    }

    pub(super) fn exec_addi(&mut self, i: i32) -> Result<(), RubyError> {
        exec_op_i!(self, iseq, i, add, IdentId::_ADD);
    }

    pub(super) fn exec_subi(&mut self, i: i32) -> Result<(), RubyError> {
        exec_op_i!(self, iseq, i, sub, IdentId::_SUB);
    }

    pub(super) fn exec_exp(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        let val = if let Some(i) = lhs.as_fixnum() {
            arith::exp_fixnum(i, rhs)?
        } else if let Some(f) = lhs.as_float() {
            arith::exp_float(f, rhs)?
        } else {
            return self.exec_send1(IdentId::_POW, lhs, rhs);
        };
        self.stack_push(val);
        Ok(())
    }

    pub(super) fn exec_neg(&mut self, lhs: Value) -> Result<(), RubyError> {
        let val = match lhs.unpack() {
            RV::Integer(i) => match i.checked_neg() {
                Some(i) => Value::integer(i),
                None => return Err(RubyError::runtime("Negate overflow.")),
            },
            RV::Float(f) => Value::float(-f),
            _ => return self.exec_send0(IdentId::get_id("-@"), lhs),
        };
        self.stack_push(val);
        Ok(())
    }

    pub(super) fn exec_shl(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        if let Some(lhsi) = lhs.as_fixnum() {
            if let Some(rhsi) = rhs.as_fixnum() {
                let val = arith::shl_fixnum(lhsi, rhsi)?;
                self.stack_push(val);
                return Ok(());
            }
        }
        if let Some(mut ainfo) = lhs.as_array() {
            ainfo.push(rhs);
            self.stack_push(lhs);
            Ok(())
        } else {
            self.exec_send1(IdentId::_SHL, lhs, rhs)
        }
    }

    pub(super) fn exec_shr(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        if let Some(lhsi) = lhs.as_fixnum() {
            if let Some(rhsi) = rhs.as_fixnum() {
                let val = arith::shr_fixnum(lhsi, rhsi)?;
                self.stack_push(val);
                return Ok(());
            }
        }
        self.exec_send1(IdentId::_SHR, lhs, rhs)
    }

    pub(super) fn exec_bitand(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
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
                return self.exec_send1(IdentId::get_id("&"), lhs, rhs);
            }
        };
        self.stack_push(val);
        Ok(())
    }

    pub(super) fn exec_bitor(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
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
                return self.exec_send1(IdentId::get_id("|"), lhs, rhs);
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
                self.exec_send1(IdentId::get_id("^"), lhs, rhs)?;
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
    ($func_name:ident, $op:ident, $id:ident) => {
        pub(super) fn $func_name(&mut self) -> Result<bool, RubyError> {
            let len = self.stack_len();
            let lhs = self.exec_stack[len - 2];
            let rhs = self.exec_stack[len - 1];
            self.set_stack_len(len - 2);
            let res = eval_cmp2!(self, rhs, lhs, $op, IdentId::$id);
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
                $vm.exec_send1($id, $lhs, $rhs)?;
                Ok($vm.stack_pop().to_bool())
            }
        } else if let Some(lhsf) = $lhs.as_flonum() {
            if let Some(rhsi) = $rhs.as_fixnum() {
                Ok(lhsf.$op(&(rhsi as f64)))
            } else if let Some(rhsf) = $rhs.as_flonum() {
                Ok(lhsf.$op(&rhsf))
            } else {
                $vm.exec_send1($id, $lhs, $rhs)?;
                Ok($vm.stack_pop().to_bool())
            }
        } else {
            $vm.exec_send1($id, $lhs, $rhs)?;
            Ok($vm.stack_pop().to_bool())
        }
    }};
}

macro_rules! eval_cmp_i {
    ($func_name:ident, $op:ident, $id:ident) => {
        pub(super) fn $func_name(&mut self, lhs: Value, i: i32) -> Result<bool, RubyError> {
            if let Some(lhsi) = lhs.as_fixnum() {
                let i = i as i64;
                Ok(lhsi.$op(&i))
            } else if let Some(lhsf) = lhs.as_flonum() {
                let i = i as f64;
                Ok(lhsf.$op(&i))
            } else {
                self.exec_send1(IdentId::$id, lhs, Value::integer(i as i64))?;
                Ok(self.stack_pop().to_bool())
            }
        }
    };
}

impl VM {
    pub(super) fn exec_eq(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        let b = self.eval_eq2(rhs, lhs)?;
        self.stack_push(Value::bool(b));
        Ok(())
    }

    pub(super) fn exec_teq(&mut self, rhs: Value, lhs: Value) -> Result<(), RubyError> {
        let b = match lhs.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Module(_) => {
                    return self.exec_send1(IdentId::_TEQ, lhs, rhs);
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
                _ => return self.exec_eq(lhs, rhs),
            },
            None => return self.exec_eq(lhs, rhs),
        };
        self.stack_push(Value::bool(b));
        Ok(())
    }

    pub fn eval_teq(&mut self, rhs: Value, lhs: Value) -> Result<bool, RubyError> {
        match lhs.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Module(_) => {
                    self.exec_send1(IdentId::_TEQ, lhs, rhs)?;
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
                return Err(RubyError::internal(format!(
                    "Invalid rvalue. (maybe GC problem) {:?} {:?}",
                    lhs.rvalue(),
                    rhs.rvalue()
                )))
            }
            (_, _) => {
                let val = match self.exec_send1(IdentId::_EQ, lhs, rhs) {
                    Ok(()) => self.stack_pop(),
                    _ => return Ok(false),
                };
                Ok(val.to_bool())
            }
        }
    }

    pub(super) fn eval_eq(&mut self) -> Result<bool, RubyError> {
        let (lhs, rhs) = self.stack_pop2();
        self.eval_eq2(rhs, lhs)
    }

    pub(super) fn eval_ne(&mut self) -> Result<bool, RubyError> {
        Ok(!self.eval_eq()?)
    }

    eval_cmp!(eval_ge, ge, _GE);
    eval_cmp!(eval_gt, gt, _GT);
    eval_cmp!(eval_le, le, _LE);
    eval_cmp!(eval_lt, lt, _LT);

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
                    self.exec_send1(IdentId::_EQ, lhs, Value::integer(i as i64))?;
                    return Ok(self.stack_pop().to_bool());
                }
            }
        };

        Ok(res)
    }
    pub(super) fn eval_nei(&mut self, lhs: Value, i: i32) -> Result<bool, RubyError> {
        Ok(!self.eval_eqi(lhs, i)?)
    }

    eval_cmp_i!(eval_gei, ge, _GE);
    eval_cmp_i!(eval_gti, gt, _GT);
    eval_cmp_i!(eval_lei, le, _LE);
    eval_cmp_i!(eval_lti, lt, _LT);

    pub fn eval_compare(&mut self, rhs: Value, lhs: Value) -> VMResult {
        if let Some(lhsi) = lhs.as_fixnum() {
            Ok(Value::from_ord(arith::cmp_fixnum(lhsi, rhs)))
        } else if let Some(lhsf) = lhs.as_float() {
            Ok(Value::from_ord(arith::cmp_float(lhsf, rhs)))
        } else {
            self.exec_send1(IdentId::_CMP, lhs, rhs)?;
            Ok(self.stack_pop())
        }
    }

    pub(super) fn invoke_set_index(&mut self) -> Result<VMResKind, RubyError> {
        let (idx, val) = self.stack_pop2();
        let mut receiver = self.stack_pop();

        match receiver.as_mut_rvalue() {
            Some(oref) => {
                match oref.kind {
                    ObjKind::Array(ref mut aref) => {
                        aref.set_elem1(idx, val)?;
                        return Ok(VMResKind::Return);
                    }
                    ObjKind::Hash(ref mut href) => {
                        href.insert(idx, val);
                        return Ok(VMResKind::Return);
                    }
                    _ => {}
                };
            }
            None => {}
        }
        self.invoke_send2(IdentId::_INDEX_ASSIGN, receiver, idx, val, false)
    }

    pub(super) fn invoke_set_index_imm(&mut self, idx: u32) -> Result<VMResKind, RubyError> {
        let mut receiver = self.stack_pop();
        let val = self.stack_pop();
        match receiver.as_mut_rvalue() {
            Some(oref) => {
                match oref.kind {
                    ObjKind::Array(ref mut aref) => {
                        aref.set_elem_imm(idx as usize, val);
                        return Ok(VMResKind::Return);
                    }
                    ObjKind::Hash(ref mut href) => {
                        href.insert(Value::integer(idx as i64), val);
                        return Ok(VMResKind::Return);
                    }
                    _ => {}
                };
            }
            None => {}
        }
        self.invoke_send2(
            IdentId::_INDEX_ASSIGN,
            receiver,
            Value::integer(idx as i64),
            val,
            false,
        )
    }

    pub(super) fn invoke_get_index(
        &mut self,
        receiver: Value,
        idx: Value,
    ) -> Result<VMResKind, RubyError> {
        match receiver.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Array(aref) => {
                    let val = aref.get_elem1(idx)?;
                    self.stack_push(val);
                    return Ok(VMResKind::Return);
                }
                ObjKind::Hash(href) => {
                    let val = href.get(&idx).cloned().unwrap_or_default();
                    self.stack_push(val);
                    return Ok(VMResKind::Return);
                }
                _ => {}
            },
            _ => {}
        };
        self.invoke_send1(IdentId::_INDEX, receiver, idx, true)
    }

    pub(super) fn invoke_get_index_imm(
        &mut self,
        receiver: Value,
        idx: u32,
    ) -> Result<VMResKind, RubyError> {
        match receiver.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Array(aref) => {
                    let val = aref.get_elem_imm(idx as usize);
                    self.stack_push(val);
                    return Ok(VMResKind::Return);
                }
                ObjKind::Hash(href) => {
                    let val = href
                        .get(&Value::integer(idx as i64))
                        .cloned()
                        .unwrap_or_default();
                    self.stack_push(val);
                    return Ok(VMResKind::Return);
                }
                ObjKind::Method(mref) => {
                    if let Some(recv) = mref.receiver {
                        self.stack_push(Value::integer(idx as i64));
                        self.stack_push(recv);
                        let args = Args2::new(1);
                        return self.invoke_method(mref.method, &args);
                    }
                }
                _ => {}
            },
            None => {
                if let Some(i) = receiver.as_fixnum() {
                    let val = if 63 < idx { 0 } else { (i >> idx) & 1 };
                    self.stack_push(Value::integer(val));
                    return Ok(VMResKind::Return);
                }
            }
        };
        self.invoke_send1(IdentId::_INDEX, receiver, Value::integer(idx as i64), true)
    }
}

#[cfg(test)]
mod test {
    use crate::tests::*;
    use crate::Value;
    #[test]
    fn expr11() {
        let program = r#"
            assert(true, true || false && false)
            assert(false, (true or false and false))
            assert(false, true^5)
            assert(false, true^true)
            assert(true, true^false)
            assert(true, true^nil)
            assert(true, false^5)
            assert(true, false^true)
            assert(false, false^false)
            assert(false, false^nil)
        "#;
        assert_script(program);
    }

    #[test]
    fn op1() {
        let program = "4==5";
        let expected = Value::bool(false);
        eval_script(program, expected);
    }

    #[test]
    fn op2() {
        let program = "4!=5";
        let expected = Value::bool(true);
        eval_script(program, expected);
    }

    #[test]
    fn op3() {
        let program = "
        assert(true, nil==nil)
        assert(true, 4.0==4)
        assert(true, 4==4.0)
        assert(true, 12345678==12345678)
        assert(true, 1234.5678==1234.5678)
        ";
        assert_script(program);
    }

    #[test]
    fn op4() {
        let program = "
        assert(false, nil!=nil)
        assert(false, 4.0!=4)
        assert(false, 4!=4.0)
        assert(false, 12345678!=12345678)
        assert(false, 1234.5678!=1234.5678)
        ";
        assert_script(program);
    }

    #[test]
    fn op10() {
        let program = "4==4 && 4!=5 && 3<4 && 5>4 && 4<=4 && 4>=4";
        let expected = Value::bool(true);
        eval_script(program, expected);
    }

    #[test]
    fn op11() {
        let program = "
        assert(nil, a&&=4)
        a = 3
        assert(4, a&&=4)
        assert(4, b||=4)
        assert(4, b||=5)
        ";
        assert_script(program);
    }

    #[test]
    fn op5() {
        let program = "
        a = 42
        assert(true, a == 42)
        assert(false, a == 43)
        assert(false, a != 42)
        assert(true, a != 43)

        assert(true, a <= 43)
        assert(true, a <= 42)
        assert(false, a <= 41)
        assert(true, a < 43)
        assert(false, a < 42)
        assert(false, a < 41)
        assert(false, a >= 43)
        assert(true, a >= 42)
        assert(true, a >= 41)
        assert(false, a > 43)
        assert(false, a > 42)
        assert(true, a > 41)
        ";
        assert_script(program);
    }

    #[test]
    fn op6() {
        let program = "
        a = 42
        assert(true, a == 42.0)
        assert(false, a == 43.0)
        assert(false, a != 42.0)
        assert(true, a != 43.0)

        assert(true, a <= 43.0)
        assert(true, a <= 42.0)
        assert(false, a <= 41.0)
        assert(true, a < 43.0)
        assert(false, a < 42.0)
        assert(false, a < 41.0)
        assert(false, a >= 43.0)
        assert(true, a >= 42.0)
        assert(true, a >= 41.0)
        assert(false, a > 43.0)
        assert(false, a > 42.0)
        assert(true, a > 41.0)
        ";
        assert_script(program);
    }

    #[test]
    fn op9() {
        let program = "
        assert(4, 4 || 5)
        assert(4, 4 || nil)
        assert(5, nil || 5)
        assert(false, nil || false)
        assert(5, 4 && 5)
        assert(nil, 4 && nil)
        assert(nil, nil && 5)
        assert(nil, nil && false)

        assert(4, (4 or 5))
        assert(4, (4 or nil))
        assert(5, (nil or 5))
        assert(false, (nil or false))
        assert(5, (4 and 5))
        assert(nil, (4 and nil))
        assert(nil, (nil and 5))
        assert(nil, (nil and false))
        ";
        assert_script(program);
    }

    #[test]
    fn op_error() {
        let program = "
    assert_error { 4 / 0 }

    ";
        assert_script(program);
    }

    #[test]
    fn op_div() {
        let program = r#"
        assert 5, 17/3
        assert -6, -17/3
        assert -6, 17/-3
        assert 5, -17/-3
        assert 5.666666666666667, 17.0/3
        assert -5.666666666666667, -17.0/3
        assert -5.666666666666667, 17.0/-3
        assert 5.666666666666667, -17.0/-3
        assert 5.483870967741935, 17/3.1
        assert -5.483870967741935, -17/3.1
        assert -5.483870967741935, 17/-3.1
        assert 5.483870967741935, -17/-3.1
        assert 5.483870967741935, 17.0/3.1
        assert -5.483870967741935, -17.0/3.1
        assert -5.483870967741935, 17.0/-3.1
        assert 5.483870967741935, -17.0/-3.1

        assert 5, 17./3
        assert -6, -17./3
        assert -6, 17./ -3      # `17./-3` cause ArgumentError (wrong number of arguments (given 0, expected 1)) in CRuby.
        assert 5, -17./ -3
        assert 5.666666666666667, 17.0./3
        assert -5.666666666666667, -17.0./3
        assert -5.666666666666667, 17.0./ -3
        assert 5.666666666666667, -17.0./ -3
        assert 5.483870967741935, 17./3.1
        assert -5.483870967741935, -17./3.1
        assert -5.483870967741935, 17./ -3.1
        assert 5.483870967741935, -17./ -3.1
        assert 5.483870967741935, 17.0./3.1
        assert -5.483870967741935, -17.0./3.1
        assert -5.483870967741935, 17.0./ -3.1
        assert 5.483870967741935, -17.0./ -3.1

        assert 70707070707070707070707070707070707070707070, 777777777777777777777777777777777777777777777/11
        assert -70707070707070707070707070707070707070707071, 777777777777777777777777777777777777777777777/-11
        assert -70707070707070707070707070707070707070707071, -777777777777777777777777777777777777777777777/11
        assert 70707070707070707070707070707070707070707070, -777777777777777777777777777777777777777777777/-11

        assert 14000000, 777777777777777777777777777777777777777777777/55555555555555555555555555555555555555
        assert -14000001, 777777777777777777777777777777777777777777777/-55555555555555555555555555555555555555
        assert -14000001, -777777777777777777777777777777777777777777777/55555555555555555555555555555555555555
        assert 14000000, -777777777777777777777777777777777777777777777/-55555555555555555555555555555555555555

        assert_error { Object / 2 }
        assert_error { 4 / 0 }
        assert true, (0 / 0.0).nan?
        assert 1, (1 / 0.0).infinite?
        assert -1, (-1 / 0.0).infinite?
    "#;
        assert_script(program);
    }

    #[test]
    fn op_rem() {
        let program = r#"
        assert 2, 17%3
        assert 1, -17%3
        assert -1, 17%-3
        assert -2, -17%-3
        assert 2.0, 17.0%3
        assert 1.0, -17.0%3
        assert -1.0, 17.0%-3
        assert -2.0, -17.0%-3
        assert 1.4999999999999996, 17%3.1
        assert 1.6000000000000005, -17%3.1
        assert -1.6000000000000005, 17% -3.1
        assert -1.4999999999999996, -17% -3.1
        assert 1.4999999999999996, 17.0%3.1
        assert 1.6000000000000005, -17.0%3.1
        assert -1.6000000000000005, 17.0% -3.1
        assert -1.4999999999999996, -17.0% -3.1

        assert 2, 17.%3
        assert 1, -17.%3
        assert -1, 17.% -3
        assert -2, -17.% -3
        assert 2.0, 17.0.%3
        assert 1.0, -17.0.%3
        assert -1.0, 17.0.% -3
        assert -2.0, -17.0.% -3
        assert 1.4999999999999996, 17.%3.1
        assert 1.6000000000000005, -17.%3.1
        assert -1.6000000000000005, 17.% -3.1
        assert -1.4999999999999996, -17.% -3.1
        assert 1.4999999999999996, 17.0.%3.1
        assert 1.6000000000000005, -17.0.%3.1
        assert -1.6000000000000005, 17.0.% -3.1
        assert -1.4999999999999996, -17.0.% -3.1

        assert 7, 777777777777777777777777777777777777777777777%11
        assert -4, 777777777777777777777777777777777777777777777%-11
        assert 4, -777777777777777777777777777777777777777777777%11
        assert -7, -777777777777777777777777777777777777777777777%-11

        assert 7777777, 777777777777777777777777777777777777777777777%55555555555555555555555555555555555555
        assert -55555555555555555555555555555547777778, 777777777777777777777777777777777777777777777%-55555555555555555555555555555555555555
        assert 55555555555555555555555555555547777778, -777777777777777777777777777777777777777777777%55555555555555555555555555555555555555
        assert -7777777, -777777777777777777777777777777777777777777777%-55555555555555555555555555555555555555

        assert_error { Object % 2 }
        assert_error { 4 % 0 }
        assert_error { 4 % 0.0 }
    "#;
        assert_script(program);
    }

    #[test]
    fn op_exp() {
        let program = r#"
        assert 125, 5**3
        assert 125, 5.**3
        assert 125.0, 5.0**3
        assert 9.067685400621531e+229, 1.0000001**5294967295
        assert 9.067685400621531e+229, 1.0000001.**5294967295
        assert Float::INFINITY, 1.000001**5294967295
        assert 125.0, 5**3.0
        assert 125.0, 5.0**3.0
        assert 515377520732011331036461129765621272702107522001, 9**50
        assert 99004978424634758460788175959850231384062258785686870081613872498101581395313542495352149834069897992935729260289701142455224740103391181260004609302334341800039635568129970115442839391827996550424487303485396454012469580882905641175688141679838136482494020114955270207320538452763240956374406074149300671579864332715145709863383780863275927913264488065824964134097111327055578304715573574863170881233596435321677269161373766830326623290133505249820241363748144053712429443738904672676108823757514270791357700793478322575236984919703879832227808887128808505470052700016643446425074170977013556970105118016875781729296487167035893013296726868340192779006950627556250376138361427629874669567756024656175878424770926330104731840651602669084889253946964534434285751230028440603209032294832059498594606606468490926474436675016431713136210477879628847342170621587069455808669470061631374013948079436151002181084682267794749297291647300783641586353030320198431406765416672379011079564116794400764446410324094916351134737908653602495683238912513245986821277445046124727197197942423579770184795685132535857761152282171188192202463999960657594792551859851101152970213401448149592201319946570451550649407279550004010912317702582662181472912015632105302573317759484624893770032176820288320073387656761277331242679472701266587357621245770830211129511961579873634726931081078561636510231195575470672425647361586025682173480892977489661451594795814128509509425645150258743730505998190389830905255028489744158007353975434402844632903542388216950234846910950016933581693043083203563960250040595281298131076781209633994798067794829721721207145083723996432251964594130634775857069677713604946054761870522277052786475978919885081309661131596183958974187958947392337718547840397608643750387141913891905022918582008778172214728550032991223156662392055884257037505490649206578470786769953636143943051498762797391979510646398701737551122262121194721735729584817975610778353208070300016017612485098705051576372570889389027686826927021354003296996972665573694623452439914428134108186995838573281853260260551326272651319302345336109009831786294854579152053806172787670898486228532783960756125000641605920544762128696668174512951678298401612472654901833823129081536498009697722707089194770194193877396548801314133180710068341774272087948559511157789830567777953618554286629609941111782209145679840892062464993259272560188343372649137473128436248659282915448922388598822198689223744804226505034904892440418921082583888568415672372810941137657901830773534867103279838723919344958324622064850958685104590741641314653534945602042664695745242124195498480911070889250285648082147135922256776812674505771497985833102994253393058886858013555492101850119408040063312674607330296468178248191852338792536021661196063676965703392917466341809324961614149374121667498311047805075329809274593403255044719712585275782370848411272534179946277913147605644296772605179000588593167084990728433184244199937345280789889986778490418225973611557077576920711205233787631467895203243147667278831754482064015215145620944003285825981308597266457069315901943232835049284369344625229731093098848654988437316182505753827696967568193233319409751479959756294143780270618863985019423675624664466826362954599612271100389478231449079758828965901645002854500882422453215051596770367779957118881541171786660332752071193186931315074553921104991346831730522139995786011825303688705687369374468570644026499515521723627898502116472161625050121341535171586371822582278385434386412134649983941597520052275261284260041433150668882097819023591218445786995257658067092208295408904882259315218740664102165077617258492889308833958231513086067650550987023064744146282880623574397910052659984159730664681182603761618727792576813624445279110372547605073014406165024534364127717868222758569050284128214766731234248682362538166360968069750622771396453919091772981444414837768316016598851550667383324344996372695315026078867643401787226050063952173812346832419796853327241750401025370627381863851115572713096005781926902034419241532439310610871872106071373624827913435539719369145612268941223907538575473725882778413132245073970399569320440048652444176865541352299418375452061978961811913160269318340085764894859169138674135640397155909307168938442329101069754805488756528538184514974882678230313082822436690711468949404502157005024493342337507031630247012498114633269311319104558723488461327166350178123323760604947600478156981350411607579079298519612795554526271362156440629373483216063258438011287070470627913927237542047460279410706669563684230519118728348870081872174864317881564608948078235159267096996526482456666123925827556951269170734489824827548938946329935755344727748701743857902657752246637611024684444920732051157114796802091418649278785984231377660042945976690409253256815208765748841991865386853969919947152510451720079208375232626852648166437533801585035681025873707689159330754393166998690745093664523630337933821854934833743916488591639712308833004994999900000001, 99999**1000
        assert 999999999999999999999999999999998000000000000000000000000000000001, 999999999999999999999999999999999**2
        assert 1.0e+66, 999999999999999999999999999999999**2.0
        u32maxplus = 4294967295 + 1     # 4294967295 = std::u32::MAX
        assert Float::INFINITY, 9999999999999999999999999999999**u32maxplus 
        assert Float::INFINITY, 2**u32maxplus 
        assert 3.3732648683496757e+186, 1.0000001**u32maxplus 
        assert_error { Object ** 2 }
    "#;
        assert_script(program);
    }

    #[test]
    fn op_negate() {
        let program = "
    a = 3.5
    assert(-3.5, -a)
    a = 3
    assert(-3, -a)
    assert(-5, -a=5)
    assert(5, a)
    ";
        assert_script(program);
    }

    #[test]
    fn index_op() {
        let program = "
        assert_error{ :a[3] }
        assert_error{ Object[3] }
        assert_error{ :a[3] = 100 }
        assert_error{ Object[3] = 200 }
        h = {a:1, b:2}
        h[0] = 100
        assert 100, h[0]
        ";
        assert_script(program);
    }

    #[test]
    fn index_op2() {
        let program = "
        class C
          def [](idx)
            idx * 2
          end 
          def []=(idx, val)
            $a = idx * 2 + val * 100
          end
        end

        o = C.new
        assert 10, o[5]
        o[3]=7
        assert 706, $a
        i = 6
        assert 12, o[i]
        o[i]=7
        assert 712, $a
        ";
        assert_script(program);
    }

    #[test]
    fn int_index() {
        let program = "
        i = 0b0100_1101
        assert(0, i[-5])
        assert(1, i[0])
        assert(0, i[1])
        assert(1, i[2])
        assert(1, i[3])
        assert(0, i[4])
        assert(0, i[5])
        assert(1, i[6])
        assert(0, i[7])
        assert(0, i[700])
    ";
        assert_script(program);
    }

    #[test]
    fn triple_equal() {
        let program = r#"
        assert true, 1 === 1
        assert false, 1 === 2
        assert false, "a" === 2
        assert false, 2 === "a"
        assert false, "ruby" === "rust"
        assert true, "ruby" === "ruby"
        assert false, Integer === Integer
        assert true, Integer === 100
        assert false, Integer === "ruby"
        assert true, String === "ruby"
        assert false, String === 100
        assert true, /\A[A-Z]*\z/ === "HELLO"
        assert false, /\A[a-z]*\z/ === "HELLO"
        assert 4, "aabcdxafv" =~ /dx.f/
        assert 3, "sdrgbgbgbff" =~ /(gb)*f/
    "#;
        assert_script(program);
    }

    #[test]
    fn unary_minus_and_exponential() {
        let program = r#"
        assert -78125..0, -5**7..b=0
    "#;
        assert_script(program);
    }
}
