use crate::class::ClassRef;
use crate::instance::InstanceRef;

const NIL_VALUE: u64 = 0x08;
const TRUE_VALUE: u64 = 0x14;
const FALSE_VALUE: u64 = 0x00;

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
            Value::Nil => PackedValue(NIL_VALUE),
            Value::Bool(b) if b => PackedValue(TRUE_VALUE),
            Value::Bool(_) => PackedValue(FALSE_VALUE),
            Value::FixNum(num) => PackedValue(Value::pack_fixnum(num)),
            Value::FloatNum(num) => PackedValue(Value::pack_flonum(num)),
            _ => PackedValue(Value::pack_as_boxed(self)),
        }
    }

    fn pack_fixnum(num: i64) -> u64 {
        ((num << 1) as u64) | 0b1
    }

    fn pack_flonum(num: f64) -> u64 {
        let unum: u64 = unsafe { std::mem::transmute(num) };
        let exp = (unum >> 60) & 0b111;
        //eprintln!("before   pack:{:064b}", unum);
        let res = if exp == 4 || exp == 3 {
            ((unum & !(0b0110u64 << 60)) | (0b0100u64 << 60)).rotate_left(3)
        } else {
            //eprintln!("{}", num);
            Value::pack_as_boxed(Value::FloatNum(num))
        };
        //eprintln!("after    pack:{:064b}", res);
        res
    }
    fn pack_as_boxed(val: Value) -> u64 {
        Box::into_raw(Box::new(val)) as u64
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PackedValue(u64);

impl std::ops::Deref for PackedValue {
    type Target = u64;
    fn deref(&self) -> &u64 {
        &self.0
    }
}

impl PackedValue {
    pub fn unpack(self) -> Value {
        if self.0 & 0b1 == 1 {
            Value::FixNum((self.0 as i64) >> 1)
        } else if self.0 & 0b11 == 0b10 {
            //eprintln!("before unpack:{:064b}", self.0);
            let num = if self.0 & (0b1000u64 << 60) == 0 {
                self.0 //(self.0 & !(0b0011u64)) | 0b10
            } else {
                (self.0 & !(0b0011u64)) | 0b01
            }
            .rotate_right(3);
            //eprintln!("after  unpack:{:064b}", num);
            Value::FloatNum(unsafe { std::mem::transmute(num) })
        } else if self.0 == NIL_VALUE {
            Value::Nil
        } else if self.0 == TRUE_VALUE {
            Value::Bool(true)
        } else if self.0 == FALSE_VALUE {
            Value::Bool(false)
        } else {
            unsafe { (*(self.0 as *mut Value)).clone() }
        }
    }

    pub fn is_packed_fixnum(&self) -> bool {
        self.0 & 0b1 == 1
    }

    pub fn is_packed_num(&self) -> bool {
        self.0 & 0b11 != 0
    }

    pub fn as_packed_fixnum(&self) -> i64 {
        (self.0 as i64) >> 1
    }

    pub fn as_packed_flonum(&self) -> f64 {
        let num = if self.0 & (0b1000u64 << 60) == 0 {
            self.0 //(self.0 & !(0b0011u64)) | 0b10
        } else {
            (self.0 & !(0b0011u64)) | 0b01
        }
        .rotate_right(3);
        //eprintln!("after  unpack:{:064b}", num);
        unsafe { std::mem::transmute(num) }
    }

    pub fn nil() -> Self {
        PackedValue(NIL_VALUE)
    }

    pub fn true_val() -> Self {
        PackedValue(TRUE_VALUE)
    }

    pub fn false_val() -> Self {
        PackedValue(FALSE_VALUE)
    }

    pub fn fixnum(num: i64) -> Self {
        PackedValue(Value::pack_fixnum(num))
    }

    pub fn flonum(num: f64) -> Self {
        PackedValue(Value::pack_flonum(num))
    }
}

#[allow(unused_imports)]
mod tests {
    use super::Value;

    #[test]
    fn pack_bool1() {
        let expect = Value::Bool(true);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_bool2() {
        let expect = Value::Bool(false);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_nil() {
        let expect = Value::Nil;
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer1() {
        let expect = Value::FixNum(12054);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer2() {
        let expect = Value::FixNum(-58993);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float0() {
        let expect = Value::FloatNum(0.0);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float1() {
        let expect = Value::FloatNum(100.0);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float2() {
        let expect = Value::FloatNum(13859.628547);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float3() {
        let expect = Value::FloatNum(-5282.2541156);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_char() {
        let expect = Value::Char(123);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_string() {
        let expect = Value::String("Ruby".to_string());
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_range() {
        let from = Box::new(Value::FixNum(7));
        let to = Box::new(Value::FixNum(36));
        let expect = Value::Range(from, to, false);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }
}
