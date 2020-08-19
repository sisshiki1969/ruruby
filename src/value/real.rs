use crate::*;

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
}

impl std::fmt::Debug for Real {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Real::Integer(num) => write!(f, "{}", *num),
            Real::Float(num) => write!(f, "{}", *num),
        }
    }
}

impl std::ops::Add for Real {
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

impl std::ops::Sub for Real {
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

impl std::ops::Mul for Real {
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

impl std::ops::Div for Real {
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

impl std::cmp::PartialEq for Real {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Real::Integer(i1), Real::Integer(i2)) => *i1 == *i2,
            (Real::Integer(i1), Real::Float(f2)) => *i1 as f64 == *f2,
            (Real::Float(f1), Real::Integer(i2)) => *f1 == *i2 as f64,
            (Real::Float(f1), Real::Float(f2)) => *f1 == *f2,
        }
    }
}

impl std::ops::Neg for Real {
    type Output = Real;
    fn neg(self) -> Self {
        match self {
            Real::Integer(i) => Real::Integer(-i),
            Real::Float(f) => Real::Float(-f),
        }
    }
}
