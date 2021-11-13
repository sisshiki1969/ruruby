use crate::num::{Integer, Signed, ToPrimitive, Zero};
use crate::num_bigint::{BigInt, ToBigInt};
use crate::*;
use divrem::RemFloor;
use num::traits::Pow;
use std::convert::TryInto;

// modulus operation (%)
//
// Ruby use `floored division` for divide/modulus operation.

pub(crate) fn rem_fixnum(lhsi: i64, rhs: Value) -> VMResult {
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
        return Err(VMError::cant_coerse(rhs, "Integer"));
    };
    Ok(val)
}

pub(crate) fn rem_bignum(lhsb: &BigInt, rhs: Value) -> VMResult {
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
        return Err(VMError::cant_coerse(rhs, "Integer"));
    };
    Ok(val)
}

pub(crate) fn rem_float(lhsf: f64, rhs: Value) -> VMResult {
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

pub(crate) fn rem_floorf64(self_: f64, other: f64) -> f64 {
    if self_ > 0.0 && other < 0.0 {
        ((self_ - 1.0) % other) + other + 1.0
    } else if self_ < 0.0 && other > 0.0 {
        ((self_ + 1.0) % other) + other - 1.0
    } else {
        self_ % other
    }
}

// exponential operation (**)

pub(crate) fn exp_fixnum(lhsi: i64, rhs: Value) -> VMResult {
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
        return Err(VMError::cant_coerse(rhs, "Integer"));
    };
    Ok(val)
}

pub(crate) fn exp_float(lhsf: f64, rhs: Value) -> VMResult {
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
        return Err(VMError::cant_coerse(rhs, "Integer"));
    };
    Ok(Value::float(f))
}

// compare operation (<=>)

pub(crate) fn cmp_fixnum(lhsi: i64, rhs: Value) -> Option<std::cmp::Ordering> {
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

pub(crate) fn cmp_float(lhsf: f64, rhs: Value) -> Option<std::cmp::Ordering> {
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

/// Compare `lhsb` to `rhs` (`lhsb` <=> `rhs`).
///
/// # Examples
///
/// ~~~
/// # use ruruby::*;
/// # use ruruby::arith::*;
/// # use num::bigint::BigInt;
/// # use std::cmp::Ordering::*;
/// let big =  BigInt::from(2i64).pow(100u32);
/// let big_prev: BigInt = big.clone() - 1;
/// let big_next: BigInt = big.clone() + 1;
/// assert_eq!(cmp_bignum(&big, Value::bignum(big.clone())), Some(Equal));
/// assert_eq!(cmp_bignum(&big, Value::bignum(big_prev.clone())), Some(Greater));
/// assert_eq!(cmp_bignum(&big, Value::bignum(big_next.clone())), Some(Less));
///
/// assert_eq!(cmp_bignum(&big, Value::float(f64::INFINITY)), Some(Less));
/// assert_eq!(cmp_bignum(&big, Value::float(f64::NEG_INFINITY)), Some(Greater));
/// ~~~
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

/// Safe arithmetic shift left operation (`lhs` << `rhs`).
///
/// # Examples
///
/// ~~~
/// # use ruruby::*;
/// # use ruruby::arith::*;
/// assert_eq!(shl_fixnum(10, 10), Ok(Value::integer(10240)));
/// assert_eq!(shl_fixnum(10240, -10), Ok(Value::integer(10)));
/// ~~~
pub fn shl_fixnum(lhs: i64, rhs: i64) -> VMResult {
    if rhs >= 0 {
        Ok(shl_fixnum_sub(lhs, rhs))
    } else {
        Ok(shr_fixnum_sub(lhs, -rhs))
    }
}

/// Safe arithmetic shift right operation (`lhs` >> `rhs`).
///
/// # Examples
///
/// ~~~
/// # use ruruby::*;
/// # use ruruby::arith::*;
/// assert_eq!(shr_fixnum(10, -10), Ok(Value::integer(10240)));
/// assert_eq!(shr_fixnum(10240, 10), Ok(Value::integer(10)));
/// ~~~
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

/// Safe GCD calculation for i64.
///
/// # Examples
///
/// ~~~
/// # use ruruby::*;
/// # use ruruby::arith::*;
/// assert_eq!(safe_gcd(&6, &8), Value::integer(2));
/// assert_eq!(safe_gcd(&7, &3), Value::integer(1));
/// assert_eq!(safe_gcd(&-7, &0), Value::integer(7));
/// assert_eq!(safe_gcd(&0, &9), Value::integer(9));
/// assert_eq!(safe_gcd(&0, &0), Value::integer(0));
/// ~~~
///
/// This code is from num-integer crate, and modified by @sisshiki1969.
/// https://docs.rs/num-integer/0.1.44/src/num_integer/lib.rs.html#462
pub fn safe_gcd(self_: &i64, other: &i64) -> Value {
    // Use Stein's algorithm
    let mut m = *self_;
    let mut n = *other;
    if m == 0 || n == 0 {
        return Value::integer((m | n).abs());
    }

    // find common factors of 2
    let shift = (m | n).trailing_zeros();

    // The algorithm needs positive numbers, but the minimum value
    // can't be represented as a positive one.
    // It's also a power of two, so the gcd can be
    // calculated by bitshifting in that case

    // Assuming two's complement, the number created by the shift
    // is positive for all numbers except gcd = abs(min value)
    // The call to .abs() causes a panic in debug mode
    if m == i64::min_value() || n == i64::min_value() {
        return match 1i64.checked_shl(shift) {
            Some(i) => Value::integer(i.abs()),
            None => Value::bignum((BigInt::from(n) << shift).abs()),
        };
    }

    // guaranteed to be positive now, rest like unsigned algorithm
    m = m.abs();
    n = n.abs();

    // divide n and m by 2 until odd
    // m inside loop
    n >>= n.trailing_zeros();

    while m != 0 {
        m >>= m.trailing_zeros();
        if n > m {
            std::mem::swap(&mut n, &mut m)
        }
        m -= n;
    }

    match n.checked_shl(shift) {
        Some(i) => Value::integer(i),
        None => Value::bignum(BigInt::from(n) << shift),
    }
}

/// Safe LCM calculation for i64.
///
/// # Examples
///
/// ~~~
/// # use ruruby::*;
/// # use ruruby::arith::*;
/// assert_eq!(safe_lcm(&7, &3), Value::integer(21));
/// assert_eq!(safe_lcm(&2, &4), Value::integer(4));
/// assert_eq!(safe_lcm(&0, &0), Value::integer(0));
/// ~~~
///
/// This code is from num-integer crate, and modified by @sisshiki1969.
/// https://docs.rs/num-integer/0.1.44/src/num_integer/lib.rs.html#462
// This code is from num-integer crate, and modified by @sisshiki1969.
pub fn safe_lcm(self_: &i64, other: &i64) -> Value {
    if self_.is_zero() && other.is_zero() {
        return Value::integer(0);
    }
    // should not have to recalculate abs
    let gcd = safe_gcd(self_, other);
    if let Some(i) = gcd.as_fixnum() {
        match (*self_).checked_mul(*other / i) {
            Some(i) => Value::integer(i.abs()),
            None => Value::bignum((BigInt::from(*self_) * BigInt::from(*other / i)).abs()),
        }
    } else if let Some(b) = gcd.as_bignum() {
        Value::bignum((*self_ * (*other / b)).abs())
    } else {
        unreachable!()
    }
}

/// Safe GCD/LCM calculation for i64.
///
/// # Examples
///
/// ~~~
/// # use ruruby::*;
/// # use ruruby::arith::*;
/// assert_eq!(safe_gcd_lcm(&7, &3), (Value::integer(1), Value::integer(21)));
/// assert_eq!(safe_gcd_lcm(&2, &4), (Value::integer(2), Value::integer(4)));
/// assert_eq!(safe_gcd_lcm(&0, &0), (Value::integer(0), Value::integer(0)));
/// ~~~
///
pub fn safe_gcd_lcm(self_: &i64, other: &i64) -> (Value, Value) {
    (safe_gcd(self_, other), safe_lcm(self_, other))
}
