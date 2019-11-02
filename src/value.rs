use crate::class::ClassRef;
use crate::instance::InstanceRef;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Nil,
    Bool(bool),
    FixNum(i64),
    FloatNum(f64),
    String(String),
    Class(ClassRef),
    Instance(InstanceRef),
    Range(Box<Value>, Box<Value>, bool),
    Char(u8),
}

impl Value {
    pub fn pack(self) -> PackedValue {
        match self {
            Value::Nil => PackedValue(0x08),
            Value::Bool(b) if b => PackedValue(0x14),
            Value::Bool(_) => PackedValue(0x00),
            Value::FixNum(num) => PackedValue(Value::pack_fixnum(num)),
            Value::FloatNum(num) => PackedValue(Value::pack_flonum(num)),
            _ => {
                let val = Box::new(self);
                PackedValue(Box::into_raw(val) as u64)
            }
        }
    }

    fn pack_fixnum(num: i64) -> u64 {
        ((num << 1) as u64) | 0b1
    }

    fn pack_flonum(num: f64) -> u64 {
        let unum: u64 = unsafe { std::mem::transmute(num) };
        ((unum & !(0b0110u64 << 60)) | (0b0100u64 << 60)).rotate_left(3)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PackedValue(u64);

impl PackedValue {
    pub fn unpack(self) -> Box<Value> {
        if self.0 & 0b1 == 1 {
            Box::new(Value::FixNum((self.0 as i64) >> 1))
        } else if self.0 & 0b11 == 0b10 {
            let num = if self.0 & (0b1000u64 << 60) == 0 {
                self.0 //(self.0 & !(0b0011u64)) | 0b10
            } else {
                (self.0 & !(0b0011u64)) | 0b01
            }
            .rotate_right(3);
            Box::new(Value::FloatNum(unsafe { std::mem::transmute(num) }))
        } else if self.0 == 0x08 {
            Box::new(Value::Nil)
        } else if self.0 == 0x14 {
            Box::new(Value::Bool(true))
        } else if self.0 == 0x00 {
            Box::new(Value::Bool(false))
        } else {
            unsafe { Box::from_raw(self.0 as *mut Value) }
        }
    }

    pub fn nil() -> Self {
        PackedValue(0x08)
    }

    pub fn true_val() -> Self {
        PackedValue(0x14)
    }

    pub fn false_val() -> Self {
        PackedValue(0x00)
    }

    pub fn fixnum(num: i64) -> Self {
        PackedValue(Value::pack_fixnum(num))
    }
}

#[allow(unused_imports)]
mod tests {
    use super::Value;

    #[test]
    fn pack_bool1() {
        let expect = Value::Bool(true);
        let got = expect.clone().pack().unpack();
        if expect != *got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_bool2() {
        let expect = Value::Bool(false);
        let got = expect.clone().pack().unpack();
        if expect != *got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_nil() {
        let expect = Value::Nil;
        let got = expect.clone().pack().unpack();
        if expect != *got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer1() {
        let expect = Value::FixNum(12054);
        let got = expect.clone().pack().unpack();
        if expect != *got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer2() {
        let expect = Value::FixNum(-58993);
        let got = expect.clone().pack().unpack();
        if expect != *got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float1() {
        let expect = Value::FloatNum(100.0);
        let got = expect.clone().pack().unpack();
        if expect != *got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float2() {
        let expect = Value::FloatNum(13859.628547);
        let got = expect.clone().pack().unpack();
        if expect != *got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float3() {
        let expect = Value::FloatNum(-5282.2541156);
        let got = expect.clone().pack().unpack();
        if expect != *got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_char() {
        let expect = Value::Char(123);
        let got = expect.clone().pack().unpack();
        if expect != *got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_string() {
        let expect = Value::String("Ruby".to_string());
        let got = expect.clone().pack().unpack();
        if expect != *got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_range() {
        let from = Box::new(Value::FixNum(7));
        let to = Box::new(Value::FixNum(36));
        let expect = Value::Range(from, to, false);
        let got = expect.clone().pack().unpack();
        if expect != *got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }
}
