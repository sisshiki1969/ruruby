use crate::*;

/// This module represents real values and their basic calculations.
#[derive(Clone, Copy)]
pub enum Real {
    Integer(i64),
    Float(f64),
}

impl Real {
    pub fn to_val(self) -> Value {
        match self {
            Real::Integer(i) => Value::integer(i),
            Real::Float(f) => Value::float(f),
        }
    }
    pub fn is_negative(self) -> bool {
        match self {
            Real::Integer(i) => i < 0,
            Real::Float(f) => f < 0.0,
        }
    }
    pub fn is_zero(self) -> bool {
        match self {
            Real::Integer(i) => i == 0,
            Real::Float(f) => f == 0.0,
        }
    }
    pub fn sqrt(self) -> Self {
        match self {
            Real::Integer(i) => Real::Float((i as f64).sqrt()),
            Real::Float(f) => Real::Float(f.sqrt()),
        }
    }

    pub fn quo(self, other: Self) -> Self {
        match (self, other) {
            (Real::Integer(lhs), Real::Integer(rhs)) => Real::Integer(lhs.div_euclid(rhs)),
            (Real::Integer(lhs), Real::Float(rhs)) => {
                let quo = ((lhs as f64) / rhs).floor() as i64;
                Real::Integer(quo)
            }
            (Real::Float(lhs), Real::Integer(rhs)) => {
                let quo = (lhs / (rhs as f64)).floor() as i64;
                Real::Integer(quo)
            }
            (Real::Float(lhs), Real::Float(rhs)) => {
                let quo = (lhs / rhs).floor() as i64;
                Real::Integer(quo)
            }
        }
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
            Real::Integer(num) => write!(f, "{}", *num),
            Real::Float(num) => write!(f, "{}", *num),
        }
    }
}

impl Add for Real {
    type Output = Real;
    fn add(self, other: Real) -> Real {
        match (self, other) {
            (Real::Integer(i1), Real::Integer(i2)) => Real::Integer(i1 + i2),
            (Real::Integer(i1), Real::Float(f2)) => Real::Float(i1 as f64 + f2),
            (Real::Float(f1), Real::Integer(i2)) => Real::Float(f1 + i2 as f64),
            (Real::Float(f1), Real::Float(f2)) => Real::Float(f1 + f2),
        }
    }
}

impl Sub for Real {
    type Output = Real;
    fn sub(self, other: Real) -> Real {
        match (self, other) {
            (Real::Integer(i1), Real::Integer(i2)) => Real::Integer(i1 - i2),
            (Real::Integer(i1), Real::Float(f2)) => Real::Float(i1 as f64 - f2),
            (Real::Float(f1), Real::Integer(i2)) => Real::Float(f1 - i2 as f64),
            (Real::Float(f1), Real::Float(f2)) => Real::Float(f1 - f2),
        }
    }
}

impl Mul for Real {
    type Output = Real;
    fn mul(self, other: Real) -> Real {
        match (self, other) {
            (Real::Integer(i1), Real::Integer(i2)) => Real::Integer(i1 * i2),
            (Real::Integer(i1), Real::Float(f2)) => Real::Float(i1 as f64 * f2),
            (Real::Float(f1), Real::Integer(i2)) => Real::Float(f1 * i2 as f64),
            (Real::Float(f1), Real::Float(f2)) => Real::Float(f1 * f2),
        }
    }
}

impl Div for Real {
    type Output = Real;
    fn div(self, other: Real) -> Real {
        match (self, other) {
            (Real::Integer(i1), Real::Integer(i2)) => Real::Float((i1 as f64) / (i2 as f64)),
            (Real::Integer(i1), Real::Float(f2)) => Real::Float(i1 as f64 / f2),
            (Real::Float(f1), Real::Integer(i2)) => Real::Float(f1 / i2 as f64),
            (Real::Float(f1), Real::Float(f2)) => Real::Float(f1 / f2),
        }
    }
}

impl PartialEq for Real {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Real::Integer(i1), Real::Integer(i2)) => *i1 == *i2,
            (Real::Integer(i1), Real::Float(f2)) => *i1 as f64 == *f2,
            (Real::Float(f1), Real::Integer(i2)) => *f1 == *i2 as f64,
            (Real::Float(f1), Real::Float(f2)) => *f1 == *f2,
        }
    }
}

impl Neg for Real {
    type Output = Real;
    fn neg(self) -> Self {
        match self {
            Real::Integer(i) => Real::Integer(-i),
            Real::Float(f) => Real::Float(-f),
        }
    }
}

impl PartialOrd for Real {
    fn partial_cmp(&self, other: &Real) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Real::Integer(i1), Real::Integer(i2)) => i1.partial_cmp(i2),
            (Real::Integer(i1), Real::Float(f2)) => (*i1 as f64).partial_cmp(f2),
            (Real::Float(f1), Real::Integer(i2)) => f1.partial_cmp(&(*i2 as f64)),
            (Real::Float(f1), Real::Float(f2)) => f1.partial_cmp(f2),
        }
    }
}
