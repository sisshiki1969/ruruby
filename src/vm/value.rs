use crate::vm::*;
use std::ops::Deref;
use std::ops::{Index, IndexMut};

const FALSE_VALUE: u64 = 0x00;
const UNINITIALIZED: u64 = 0x04;
const NIL_VALUE: u64 = 0x08;
const TAG_SYMBOL: u64 = 0x0c;
const TRUE_VALUE: u64 = 0x14;

const ZERO: u64 = (0b1000 << 60) | 0b10;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Uninitialized,
    Nil,
    Bool(bool),
    FixNum(i64),
    FloatNum(f64),
    String(Box<String>),
    Symbol(IdentId),
    Object(ObjectRef),
    Char(u8),
}

const VEC_ARRAY_SIZE: usize = 8;

pub enum VecArray {
    Array {
        len: usize,
        ary: [PackedValue; VEC_ARRAY_SIZE],
    },
    Vec(Vec<PackedValue>),
}

impl VecArray {
    pub fn new(len: usize) -> Self {
        if len <= VEC_ARRAY_SIZE {
            VecArray::Array {
                len,
                ary: [PackedValue::uninitialized(); VEC_ARRAY_SIZE],
            }
        } else {
            VecArray::Vec(vec![PackedValue::uninitialized(); len])
        }
    }

    pub fn push(&mut self, val: PackedValue) {
        if self.len() == VEC_ARRAY_SIZE {
            let mut ary = self.get_slice(0, VEC_ARRAY_SIZE).to_vec();
            ary.push(val);
            unsafe { std::ptr::write(self, VecArray::Vec(ary)) };
        } else {
            match self {
                VecArray::Vec(ref mut v) => v.push(val),
                VecArray::Array {
                    ref mut len,
                    ref mut ary,
                } => {
                    ary[*len] = val;
                    *len += 1;
                }
            }
        }
    }

    pub fn new0() -> Self {
        VecArray::Array {
            len: 0,
            ary: [PackedValue::uninitialized(); VEC_ARRAY_SIZE],
        }
    }

    pub fn new1(arg: PackedValue) -> Self {
        let mut ary = [PackedValue::uninitialized(); VEC_ARRAY_SIZE];
        ary[0] = arg;
        VecArray::Array { len: 1, ary }
    }

    pub fn new2(arg0: PackedValue, arg1: PackedValue) -> Self {
        let mut ary = [PackedValue::uninitialized(); VEC_ARRAY_SIZE];
        ary[0] = arg0;
        ary[1] = arg1;
        VecArray::Array { len: 2, ary }
    }

    pub fn len(&self) -> usize {
        match self {
            VecArray::Array { len, .. } => *len,
            VecArray::Vec(v) => v.len(),
        }
    }

    pub fn get_slice(&self, start: usize, end: usize) -> &[PackedValue] {
        match self {
            VecArray::Array { ary, .. } => &ary[start..end],
            VecArray::Vec(v) => &v[start..end],
        }
    }
}

impl Index<usize> for VecArray {
    type Output = PackedValue;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            VecArray::Array { ary, .. } => &ary[index],
            VecArray::Vec(v) => &v[index],
        }
    }
}

impl IndexMut<usize> for VecArray {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match self {
            VecArray::Array { ary, .. } => &mut ary[index],
            VecArray::Vec(v) => &mut v[index],
        }
    }
}

impl Deref for VecArray {
    type Target = [PackedValue];

    fn deref(&self) -> &Self::Target {
        match self {
            VecArray::Array { len, ary } => &ary[0..*len],
            VecArray::Vec(v) => &v,
        }
    }
}

impl Value {
    pub fn pack(self) -> PackedValue {
        match self {
            Value::Uninitialized => PackedValue::uninitialized(),
            Value::Nil => PackedValue::nil(),
            Value::Bool(b) if b => PackedValue::true_val(),
            Value::Bool(_) => PackedValue::false_val(),
            Value::FixNum(num) => PackedValue::fixnum(num),
            Value::FloatNum(num) => PackedValue::flonum(num),
            Value::Symbol(id) => PackedValue::symbol(id),
            _ => PackedValue(Value::pack_as_boxed(self)),
        }
    }

    fn pack_fixnum(num: i64) -> u64 {
        let top = ((num as u64) >> 62) & 0b11;
        if top == 0b00 || top == 0b11 {
            ((num << 1) as u64) | 0b1
        } else {
            Value::pack_as_boxed(Value::FixNum(num))
        }
    }

    fn pack_flonum(num: f64) -> u64 {
        if num == 0.0 {
            return ZERO;
        }
        let unum: u64 = unsafe { std::mem::transmute(num) };
        let exp = (unum >> 60) & 0b111;
        if exp == 4 || exp == 3 {
            ((unum & !(0b0110u64 << 60)) | (0b0100u64 << 60)).rotate_left(3)
        } else {
            Value::pack_as_boxed(Value::FloatNum(num))
        }
    }

    fn pack_as_boxed(val: Value) -> u64 {
        Box::into_raw(Box::new(val)) as u64
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PackedValue(u64);

impl std::ops::Deref for PackedValue {
    type Target = u64;
    fn deref(&self) -> &u64 {
        &self.0
    }
}

impl std::hash::Hash for PackedValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if self.is_packed_value() {
            self.0.hash(state);
        } else {
            let lhs = unsafe { (*(self.0 as *mut Value)).clone() };
            match lhs {
                Value::String(lhs) => lhs.hash(state),
                _ => self.0.hash(state),
            };
        }
    }
}

impl PartialEq for PackedValue {
    fn eq(&self, other: &Self) -> bool {
        if self.0 == other.0 {
            return true;
        };
        match (self.unpack(), other.unpack()) {
            (Value::Nil, Value::Nil) => true,
            (Value::FixNum(lhs), Value::FixNum(rhs)) => lhs == rhs,
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => lhs == rhs,
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => lhs as f64 == rhs,
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => lhs == rhs as f64,
            (Value::String(lhs), Value::String(rhs)) => lhs == rhs,
            (Value::Bool(lhs), Value::Bool(rhs)) => lhs == rhs,
            (Value::Symbol(lhs), Value::Symbol(rhs)) => lhs == rhs,

            (Value::Object(lhs), Value::Object(rhs)) => match (&lhs.kind, &rhs.kind) {
                (ObjKind::Array(lhs), ObjKind::Array(rhs)) => {
                    if lhs.elements.len() != rhs.elements.len() {
                        false
                    } else {
                        for i in 0..lhs.elements.len() {
                            if lhs.elements[i] != rhs.elements[i] {
                                return false;
                            };
                        }
                        true
                    }
                }
                _ => lhs == rhs,
            },
            _ => false,
        }
    }
}

impl Eq for PackedValue {}

impl PackedValue {
    pub fn unpack(self) -> Value {
        if !self.is_packed_value() {
            unsafe { (*(self.0 as *mut Value)).clone() }
        } else if self.is_packed_fixnum() {
            Value::FixNum(self.as_packed_fixnum())
        } else if self.is_packed_num() {
            Value::FloatNum(self.as_packed_flonum())
        } else if self.is_packed_symbol() {
            Value::Symbol(self.as_packed_symbol())
        } else if self.0 == NIL_VALUE {
            Value::Nil
        } else if self.0 == TRUE_VALUE {
            Value::Bool(true)
        } else if self.0 == FALSE_VALUE {
            Value::Bool(false)
        } else if self.0 == UNINITIALIZED {
            Value::Uninitialized
        } else {
            unreachable!("Illegal packed value.")
        }
    }

    pub fn get_class(&self, globals: &Globals) -> ClassRef {
        match self.unpack() {
            Value::FixNum(_) => globals.integer_class,
            Value::String(_) => globals.string_class,
            Value::Object(oref) => oref.classref,
            _ => globals.object_class,
        }
    }

    pub fn is_packed_fixnum(&self) -> bool {
        self.0 & 0b1 == 1
    }

    pub fn is_packed_num(&self) -> bool {
        self.0 & 0b11 != 0
    }

    pub fn is_packed_symbol(&self) -> bool {
        self.0 & 0xff == TAG_SYMBOL
    }

    pub fn is_uninitialized(&self) -> bool {
        self.0 == UNINITIALIZED
    }

    pub fn is_nil(&self) -> bool {
        self.0 == NIL_VALUE
    }

    pub fn is_true_val(&self) -> bool {
        self.0 == TRUE_VALUE
    }

    pub fn is_false_val(&self) -> bool {
        self.0 == FALSE_VALUE
    }

    pub fn is_packed_value(&self) -> bool {
        self.0 & 0b0111 != 0 || self.0 <= 0x20
    }

    pub fn as_fixnum(&self) -> Option<i64> {
        if self.is_packed_fixnum() {
            Some(self.as_packed_fixnum())
        } else if self.is_packed_value() {
            None
        } else {
            unsafe {
                match *(self.0 as *mut Value) {
                    Value::FixNum(i) => Some(i),
                    _ => None,
                }
            }
        }
    }

    pub fn expect_fixnum(&self, vm: &VM, msg: impl Into<String>) -> Result<i64, RubyError> {
        if self.is_packed_fixnum() {
            Ok(self.as_packed_fixnum())
        } else if self.is_packed_value() {
            Err(vm.error_argument(msg.into() + " must be an Integer."))
        } else {
            unsafe {
                match *(self.0 as *mut Value) {
                    Value::FixNum(i) => Ok(i),
                    _ => Err(vm.error_argument(msg.into() + " must be an Integer.")),
                }
            }
        }
    }

    pub fn as_object(&self) -> Option<ObjectRef> {
        if self.is_packed_value() {
            return None;
        }
        unsafe {
            match *(self.0 as *mut Value) {
                Value::Object(oref) => Some(oref),
                _ => None,
            }
        }
    }

    pub fn as_class(&self) -> Option<ClassRef> {
        match self.as_object() {
            Some(oref) => match oref.kind {
                ObjKind::Class(cref) => Some(cref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_module(&self) -> Option<ClassRef> {
        match self.as_object() {
            Some(oref) => match oref.kind {
                ObjKind::Class(cref) | ObjKind::Module(cref) => Some(cref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_array(&self) -> Option<ArrayRef> {
        match self.as_object() {
            Some(oref) => match oref.kind {
                ObjKind::Array(aref) => Some(aref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_splat(&self) -> Option<ArrayRef> {
        match self.as_object() {
            Some(oref) => match oref.kind {
                ObjKind::SplatArray(aref) => Some(aref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_hash(&self) -> Option<HashRef> {
        match self.as_object() {
            Some(oref) => match oref.kind {
                ObjKind::Hash(href) => Some(href),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_range(&self) -> Option<RangeRef> {
        match self.as_object() {
            Some(oref) => match oref.kind {
                ObjKind::Range(rref) => Some(rref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_proc(&self) -> Option<procobj::ProcRef> {
        match self.as_object() {
            Some(oref) => match oref.kind {
                ObjKind::Proc(pref) => Some(pref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_string(&self) -> Option<String> {
        if self.is_packed_value() {
            return None;
        }
        unsafe {
            match &*(self.0 as *mut Value) {
                Value::String(string) => Some(string.to_string()),
                _ => None,
            }
        }
    }

    pub fn as_packed_fixnum(&self) -> i64 {
        (self.0 as i64) >> 1
    }

    pub fn as_packed_flonum(&self) -> f64 {
        if self.0 == ZERO {
            return 0.0;
        }
        let num = if self.0 & (0b1000u64 << 60) == 0 {
            self.0 //(self.0 & !(0b0011u64)) | 0b10
        } else {
            (self.0 & !(0b0011u64)) | 0b01
        }
        .rotate_right(3);
        //eprintln!("after  unpack:{:064b}", num);
        unsafe { std::mem::transmute(num) }
    }

    pub fn as_packed_symbol(&self) -> IdentId {
        IdentId::from((self.0 >> 32) as u32)
    }

    pub fn uninitialized() -> Self {
        PackedValue(UNINITIALIZED)
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

    pub fn bool(b: bool) -> Self {
        if b {
            PackedValue(TRUE_VALUE)
        } else {
            PackedValue(FALSE_VALUE)
        }
    }

    pub fn fixnum(num: i64) -> Self {
        PackedValue(Value::pack_fixnum(num))
    }

    pub fn flonum(num: f64) -> Self {
        PackedValue(Value::pack_flonum(num))
    }

    pub fn string(string: String) -> Self {
        PackedValue(Value::pack_as_boxed(Value::String(Box::new(string))))
    }

    pub fn symbol(id: IdentId) -> Self {
        let id: u32 = id.into();
        PackedValue((id as u64) << 32 | TAG_SYMBOL)
    }

    pub fn object(obj_ref: ObjectRef) -> Self {
        PackedValue(Value::pack_as_boxed(Value::Object(obj_ref)))
    }

    pub fn class(globals: &Globals, class_ref: ClassRef) -> Self {
        PackedValue::object(ObjectRef::new_class(globals, class_ref))
    }

    pub fn module(globals: &Globals, class_ref: ClassRef) -> Self {
        PackedValue::object(ObjectRef::new_module(globals, class_ref))
    }

    pub fn array(globals: &Globals, array_ref: ArrayRef) -> Self {
        PackedValue::object(ObjectRef::new_array(globals, array_ref))
    }

    pub fn splat(globals: &Globals, array_ref: ArrayRef) -> Self {
        PackedValue::object(ObjectRef::new_splat(globals, array_ref))
    }

    pub fn hash(globals: &Globals, hash_ref: HashRef) -> Self {
        PackedValue::object(ObjectRef::new_hash(globals, hash_ref))
    }

    pub fn range(globals: &Globals, start: PackedValue, end: PackedValue, exclude: bool) -> Self {
        let rref = range::RangeRef::new_range(start, end, exclude);
        PackedValue::object(ObjectRef::new_range(globals, rref))
    }

    pub fn procobj(globals: &Globals, context: ContextRef) -> Self {
        PackedValue::object(ObjectRef::new_proc(globals, context))
    }
}

#[allow(unused_imports)]
mod tests {
    use crate::vm::*;

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
    fn pack_uninit() {
        let expect = Value::Uninitialized;
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
    fn pack_integer11() {
        let expect_ary = [
            12054,
            -58993,
            0x8000_0000_0000_0000 as u64 as i64,
            0x4000_0000_0000_0000 as u64 as i64,
            0x7fff_ffff_ffff_ffff as u64 as i64,
        ];
        for expect in expect_ary.iter() {
            let got = match Value::FixNum(*expect).pack().as_fixnum() {
                Some(int) => int,
                None => panic!("Expect:{:?} Got:Invalid Value"),
            };
            if *expect != got {
                panic!("Expect:{:?} Got:{:?}", *expect, got)
            };
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
    fn pack_integer3() {
        let expect = Value::FixNum(0x8000_0000_0000_0000 as u64 as i64);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer4() {
        let expect = Value::FixNum(0x4000_0000_0000_0000 as u64 as i64);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer5() {
        let expect = Value::FixNum(0x7fff_ffff_ffff_ffff as u64 as i64);
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
        let expect = Value::String(Box::new("Ruby".to_string()));
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_range() {
        let globals = Globals::new();
        let from = Value::FixNum(7).pack();
        let to = Value::FixNum(36).pack();
        let expect = Value::Object(ObjectRef::new_range(
            &globals,
            range::RangeRef::new_range(from, to, false),
        ));
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_class() {
        let globals = Globals::new();
        let expect = Value::Object(ObjectRef::new_class(
            &globals,
            ClassRef::from_no_superclass(IdentId::from(0)),
        ));
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_instance() {
        let class_ref = ClassRef::from_no_superclass(IdentId::from(0));
        let expect = Value::Object(ObjectRef::from(class_ref));
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_symbol() {
        let expect = Value::Symbol(IdentId::from(12345));
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }
}
