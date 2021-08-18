use crate::num::bigint::ToBigInt;
use crate::num::{BigInt, Integer, Signed, ToPrimitive, Zero};
use crate::*;
use divrem::RemFloor;
use std::convert::TryInto;

// modulus operation (%)
//
// Ruby use `floored division` for divide/modulus operation.

pub fn rem_fixnum(lhsi: i64, rhs: Value) -> VMResult {
    let val = if let Some(i2) = rhs.as_fixnum() {
        if i2.is_zero() {
            return Err(RubyError::zero_div("Divided by zero."));
        }
        Value::integer(lhsi.rem_floor(&i2))
    } else if let Some(f2) = rhs.as_float() {
        if f2.is_zero() {
            return Err(RubyError::zero_div("Divided by zero."));
        }
        Value::float(rem_floorf64(lhsi as f64, f2))
    } else if let Some(b2) = rhs.as_bignum() {
        if b2.is_zero() {
            return Err(RubyError::zero_div("Divided by zero."));
        }
        Value::bignum(BigInt::from(lhsi).mod_floor(&b2))
    } else {
        return Err(RubyError::cant_coerse(rhs, "Integer"));
    };
    Ok(val)
}

pub fn rem_bignum(lhsb: &BigInt, rhs: Value) -> VMResult {
    let val = if let Some(i2) = rhs.as_fixnum() {
        if i2.is_zero() {
            return Err(RubyError::zero_div("Divided by zero."));
        }
        Value::bignum(lhsb.mod_floor(&BigInt::from(i2)))
    } else if let Some(f2) = rhs.as_float() {
        if f2.is_zero() {
            return Err(RubyError::zero_div("Divided by zero."));
        }
        Value::float(rem_floorf64(lhsb.to_f64().unwrap(), f2))
    } else if let Some(b2) = rhs.as_bignum() {
        if b2.is_zero() {
            return Err(RubyError::zero_div("Divided by zero."));
        }
        Value::bignum(lhsb.mod_floor(&b2))
    } else {
        return Err(RubyError::cant_coerse(rhs, "Integer"));
    };
    Ok(val)
}

pub fn rem_float(lhsf: f64, rhs: Value) -> VMResult {
    let val = if let Some(rhs) = rhs.as_fixnum() {
        if rhs.is_zero() {
            return Err(RubyError::zero_div("Divided by zero."));
        }
        Value::float(rem_floorf64(lhsf, rhs as f64))
    } else if let Some(rhs) = rhs.as_float() {
        if rhs.is_zero() {
            return Err(RubyError::zero_div("Divided by zero."));
        }
        Value::float(rem_floorf64(lhsf, rhs))
    } else if let Some(rhs) = rhs.as_bignum() {
        if rhs.is_zero() {
            return Err(RubyError::zero_div("Divided by zero."));
        }
        Value::float(rem_floorf64(lhsf, rhs.to_f64().unwrap()))
    } else {
        unreachable!()
    };
    Ok(val)
}

pub fn rem_floorf64(self_: f64, other: f64) -> f64 {
    if self_ > 0.0 && other < 0.0 {
        ((self_ - 1.0) % other) + other + 1.0
    } else if self_ < 0.0 && other > 0.0 {
        ((self_ + 1.0) % other) + other - 1.0
    } else {
        self_ % other
    }
}

// exponential operation (**)

pub fn exp_fixnum(lhsi: i64, rhs: Value) -> VMResult {
    let val = if let Some(rhsi) = rhs.as_fixnum() {
        // fixnum, fixnum
        if let Ok(rhsu) = rhsi.try_into() {
            // fixnum, u32
            match lhsi.checked_pow(rhsu) {
                Some(i) => Value::integer(i),
                None => Value::bignum(BigInt::from(lhsi).pow(rhsu)),
            }
        } else {
            Value::float((lhsi as f64).powf(rhsi as f64))
        }
    } else if let Some(rhsf) = rhs.as_float() {
        // fixnum, float
        Value::float((lhsi as f64).powf(rhsf))
    } else if let Some(rhsb) = rhs.as_bignum() {
        // fixnum, bignum
        Value::float((lhsi as f64).powf(rhsb.to_f64().unwrap()))
    } else {
        return Err(RubyError::cant_coerse(rhs, "Integer"));
    };
    Ok(val)
}

pub fn exp_float(lhsf: f64, rhs: Value) -> VMResult {
    let f = if let Some(rhsi) = rhs.as_fixnum() {
        match rhsi.try_into() {
            Ok(r) => lhsf.powi(r),
            Err(_) => lhsf.powf(rhsi as f64),
        }
    } else if let Some(rhsf) = rhs.as_float() {
        lhsf.powf(rhsf)
    } else if let Some(rhsb) = rhs.as_bignum() {
        lhsf.powf(rhsb.to_f64().unwrap())
    } else {
        return Err(RubyError::cant_coerse(rhs, "Integer"));
    };
    Ok(Value::float(f))
}

// compare operation (<=>)

pub fn cmp_fixnum(lhsi: i64, rhs: Value) -> Option<std::cmp::Ordering> {
    if let Some(rhsi) = rhs.as_fixnum() {
        lhsi.partial_cmp(&rhsi)
    } else if let Some(rhsf) = rhs.as_float() {
        (lhsi as f64).partial_cmp(&rhsf)
    } else if let Some(rhsb) = rhs.as_bignum() {
        if rhsb.is_positive() {
            Some(std::cmp::Ordering::Less)
        } else {
            Some(std::cmp::Ordering::Greater)
        }
    } else {
        None
    }
}

pub fn cmp_float(lhsf: f64, rhs: Value) -> Option<std::cmp::Ordering> {
    if let Some(rhsi) = rhs.as_fixnum() {
        lhsf.partial_cmp(&(rhsi as f64))
    } else if let Some(rhsf) = rhs.as_float() {
        lhsf.partial_cmp(&rhsf)
    } else if let Some(rhsb) = rhs.as_bignum() {
        if lhsf.is_infinite() {
            if lhsf == f64::INFINITY {
                Some(std::cmp::Ordering::Greater)
            } else if lhsf == f64::NEG_INFINITY {
                Some(std::cmp::Ordering::Less)
            } else {
                unreachable!()
            }
        } else {
            lhsf.partial_cmp(&rhsb.to_f64().unwrap())
        }
    } else {
        None
    }
}

pub fn cmp_bignum(lhsb: &BigInt, rhs: Value) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering::*;
    if let Some(_) = rhs.as_fixnum() {
        if lhsb.is_positive() {
            Some(Greater)
        } else {
            Some(Less)
        }
    } else if let Some(rhsf) = rhs.as_float() {
        if rhsf.is_infinite() {
            if rhsf == f64::INFINITY {
                Some(Less)
            } else if rhsf == f64::NEG_INFINITY {
                Some(Greater)
            } else {
                unreachable!()
            }
        } else {
            lhsb.to_f64().unwrap().partial_cmp(&rhsf)
        }
    } else if let Some(rhsb) = rhs.as_bignum() {
        lhsb.partial_cmp(&rhsb)
    } else {
        None
    }
}

// arithmetic shift operation (<<, >>)

pub fn shl_fixnum(lhs: i64, rhs: i64) -> VMResult {
    if rhs >= 0 {
        Ok(shl_fixnum_sub(lhs, rhs))
    } else {
        Ok(shr_fixnum_sub(lhs, -rhs))
    }
}

pub fn shr_fixnum(lhs: i64, rhs: i64) -> VMResult {
    if rhs >= 0 {
        Ok(shr_fixnum_sub(lhs, rhs))
    } else {
        Ok(shl_fixnum_sub(lhs, -rhs))
    }
}

/// rhs must be a non-negative value.
fn shr_fixnum_sub(lhs: i64, rhs: i64) -> Value {
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
fn shl_fixnum_sub(lhs: i64, rhs: i64) -> Value {
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
