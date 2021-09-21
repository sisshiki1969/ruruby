use crate::*;
use num::{BigInt, FromPrimitive, Integer, Signed, ToPrimitive, Zero};

/// This module represents real values and their basic calculations.
#[derive(Clone)]
pub enum Real {
    Bignum(BigInt),
    Integer(i64),
    Float(f64),
}

impl Real {
    pub fn to_val(self) -> Value {
        match self {
            Real::Bignum(n) => Value::bignum(n),
            Real::Integer(i) => Value::integer(i),
            Real::Float(f) => Value::float(f),
        }
    }

    pub fn integer(n: BigInt) -> Self {
        match n.to_i64() {
            Some(i) => Self::Integer(i),
            None => Self::Bignum(n),
        }
    }

    pub fn is_negative(&self) -> bool {
        match self {
            Real::Bignum(n) => !n.is_positive(),
            Real::Integer(i) => i.is_negative(),
            Real::Float(f) => f.is_sign_negative(),
        }
    }
    pub fn is_zero(&self) -> bool {
        match self {
            Real::Bignum(n) => n.is_zero(),
            Real::Integer(i) => i.is_zero(),
            Real::Float(f) => f.is_zero(),
        }
    }

    pub fn to_f64(&self) -> f64 {
        match self {
            Real::Bignum(n) => n.to_f64().unwrap(),
            Real::Integer(i) => *i as f64,
            Real::Float(f) => *f,
        }
    }

    pub fn sqrt(&self) -> Self {
        Real::Float(self.to_f64().sqrt())
    }

    pub fn exp2(self) -> Self {
        self.clone() * self
    }

    pub fn quotient(self, other: Self) -> Self {
        let quo = (self.to_f64() / other.to_f64()).floor();
        match ToPrimitive::to_i64(&quo) {
            Some(i) => Real::Integer(i),
            None => Real::Bignum(FromPrimitive::from_f64(quo).unwrap()),
        }
    }

    pub fn divide(self, other: Real) -> Real {
        Real::Float(self.to_f64() / other.to_f64())
    }

    pub fn included(&self, start: &Self, end: &Self, exclude: bool) -> bool {
        start <= self && (if exclude { self < end } else { self <= end })
    }
}

use std::cmp::*;
use std::fmt::{Debug, Formatter, Result};
use std::ops::*;

impl Debug for Real {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Real::Bignum(n) => write!(f, "{}", *n),
            Real::Integer(num) => write!(f, "{}", *num),
            Real::Float(num) => write!(f, "{}", *num),
        }
    }
}

macro_rules! impl_ops {
    ($trait:ident, $fname:ident, $check_fname:ident) => {
        impl $trait for Real {
            type Output = Real;
            fn $fname(self, other: Real) -> Real {
                match (self, other) {
                    (Real::Bignum(n1), Real::Bignum(n2)) => Real::integer(n1.$fname(n2)),
                    (Real::Bignum(n1), Real::Integer(i2)) => Real::integer(n1.$fname(i2)),
                    (Real::Bignum(n1), Real::Float(f2)) => {
                        Real::Float(n1.to_f64().unwrap().$fname(f2))
                    }
                    (Real::Integer(i1), Real::Bignum(n2)) => {
                        Real::integer(BigInt::from(i1).$fname(n2))
                    }
                    (Real::Integer(i1), Real::Integer(i2)) => match i1.$check_fname(i2) {
                        Some(i) => Real::Integer(i),
                        None => Real::Bignum(BigInt::from(i1).$fname(i2)),
                    },
                    (Real::Integer(i1), Real::Float(f2)) => Real::Float((i1 as f64).$fname(f2)),
                    (Real::Float(f1), Real::Bignum(n2)) => {
                        Real::Float(f1.$fname(n2.to_f64().unwrap()))
                    }
                    (Real::Float(f1), Real::Integer(i2)) => Real::Float(f1.$fname(i2 as f64)),
                    (Real::Float(f1), Real::Float(f2)) => Real::Float(f1.$fname(f2)),
                }
            }
        }
    };
}

impl_ops!(Add, add, checked_add);
impl_ops!(Sub, sub, checked_sub);
impl_ops!(Mul, mul, checked_mul);
//impl_ops!(Div, div_floor, checked_div);
//impl_ops!(Rem, rem, checked_rem);

impl Div for Real {
    type Output = Real;
    fn div(self, other: Real) -> Real {
        match (self, other) {
            (Real::Bignum(n1), Real::Bignum(n2)) => Real::integer(n1.div_floor(&n2)),
            (Real::Bignum(n1), Real::Integer(i2)) => Real::integer(n1.div_floor(&BigInt::from(i2))),
            (Real::Bignum(n1), Real::Float(f2)) => Real::Float(n1.to_f64().unwrap() / f2),
            (Real::Integer(i1), Real::Bignum(n2)) => Real::integer(BigInt::from(i1).div_floor(&n2)),
            (Real::Integer(i1), Real::Integer(i2)) => Real::Integer(i1.div_floor(i2)),
            (Real::Integer(i1), Real::Float(f2)) => Real::Float(i1 as f64 / f2),
            (Real::Float(f1), Real::Bignum(n2)) => Real::Float(f1 / n2.to_f64().unwrap()),
            (Real::Float(f1), Real::Integer(i2)) => Real::Float(f1 / i2 as f64),
            (Real::Float(f1), Real::Float(f2)) => Real::Float(f1 / f2),
        }
    }
}

impl PartialEq for Real {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Real::Bignum(n1), Real::Bignum(n2)) => n1 == n2,
            (Real::Bignum(_), Real::Integer(_)) => false,
            (Real::Bignum(n1), Real::Float(f2)) => n1.to_f64().unwrap() == *f2,
            (Real::Integer(_), Real::Bignum(_)) => false,
            (Real::Integer(i1), Real::Integer(i2)) => i1 == i2,
            (Real::Integer(i1), Real::Float(f2)) => *i1 as f64 == *f2,
            (Real::Float(f1), Real::Bignum(n2)) => *f1 == n2.to_f64().unwrap(),
            (Real::Float(f1), Real::Integer(i2)) => *f1 == *i2 as f64,
            (Real::Float(f1), Real::Float(f2)) => f1 == f2,
        }
    }
}

impl Neg for Real {
    type Output = Real;
    fn neg(self) -> Self {
        match self {
            Real::Bignum(n) => Real::Bignum(-n),
            Real::Integer(i) => Real::Integer(-i),
            Real::Float(f) => Real::Float(-f),
        }
    }
}

impl PartialOrd for Real {
    fn partial_cmp(&self, other: &Real) -> Option<std::cmp::Ordering> {
        match self {
            Real::Bignum(n1) => arith::cmp_bignum(n1, other.clone().to_val()),
            Real::Integer(i1) => arith::cmp_fixnum(*i1, other.clone().to_val()),
            Real::Float(f1) => arith::cmp_float(*f1, other.clone().to_val()),
        }
    }
}
