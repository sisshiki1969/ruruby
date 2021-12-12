use num::bigint::{Sign, ToBigInt};
use num::{BigInt, ToPrimitive};

use crate::coroutine::*;
use crate::*;
use std::borrow::Cow;

const UNINITIALIZED: u64 = 0x04; // 0000_0100
const FALSE_VALUE: u64 = 0x14; // 0001_0100
const NIL_VALUE: u64 = 0x24; // 0010_0100
const TRUE_VALUE: u64 = 0x1c; // 0001_1100
const TAG_SYMBOL: u64 = 0x0c; // 0000_1100
const BOOL_MASK1: u64 = 0b0011_0000;
const BOOL_MASK2: u64 = 0xffff_ffff_ffff_ffcf;
const FLOAT_MASK1: u64 = !(0b0110u64 << 60);
const FLOAT_MASK2: u64 = 0b0100u64 << 60;

const ZERO: u64 = (0b1000 << 60) | 0b10;

#[derive(Debug, Clone, PartialEq)]
pub enum RV<'a> {
    Uninitialized,
    Nil,
    True,
    False,
    Integer(i64),
    Float(f64),
    Symbol(IdentId),
    Object(&'a RValue),
}

impl<'a> RV<'a> {
    pub fn pack(&'a self) -> Value {
        match self {
            RV::Uninitialized => Value::uninitialized(),
            RV::Nil => Value::nil(),
            RV::True => Value::true_val(),
            RV::False => Value::false_val(),
            RV::Integer(num) => Value::integer(*num),
            RV::Float(num) => Value::float(*num),
            RV::Symbol(id) => Value::symbol(*id),
            RV::Object(info) => Value::from(info.id()),
        }
    }
}

#[derive(Clone, Copy, Eq)]
#[repr(transparent)]
pub struct Value(std::num::NonZeroU64);

impl std::ops::Deref for Value {
    type Target = std::num::NonZeroU64;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self.as_rvalue() {
            None => self.0.hash(state),
            Some(lhs) => lhs.hash(state),
        }
    }
}

impl PartialEq for Value {
    /// Equality of value.
    ///
    /// This kind of equality is used for `==` operator of Ruby.
    /// Generally, two objects which all of properties are `eq` are defined as `eq`.
    /// Some classes have original difinitions of `eq`.
    ///
    /// ex. 3.0 == 3.
    fn eq(&self, other: &Self) -> bool {
        if self.id() == other.id() {
            return true;
        };
        if self.is_packed_value() || other.is_packed_value() {
            if let Some(lhsi) = self.as_fixnum() {
                if let Some(rhsf) = other.as_flonum() {
                    return lhsi as f64 == rhsf;
                }
            } else if let Some(lhsf) = self.as_flonum() {
                if let Some(rhsi) = other.as_fixnum() {
                    return rhsi as f64 == lhsf;
                }
            }
            return false;
        }
        let (lhs, rhs) = (self.rvalue(), other.rvalue());
        match (lhs.kind(), rhs.kind()) {
            (ObjKind::BIGNUM, ObjKind::BIGNUM) => *lhs.bignum() == *rhs.bignum(),
            (ObjKind::BIGNUM, ObjKind::FLOAT) => lhs.bignum().to_f64().unwrap() == rhs.float(),
            (ObjKind::FLOAT, ObjKind::FLOAT) => lhs.float() == rhs.float(),
            (ObjKind::FLOAT, ObjKind::BIGNUM) => lhs.float() == rhs.bignum().to_f64().unwrap(),
            (ObjKind::COMPLEX, ObjKind::COMPLEX) => {
                let RubyComplex { r: r1, i: i1 } = *lhs.complex();
                let RubyComplex { r: r2, i: i2 } = *rhs.complex();
                r1.eq(&r2) && i1.eq(&i2)
            }
            (ObjKind::STRING, ObjKind::STRING) => {
                lhs.string().as_bytes() == rhs.string().as_bytes()
            }
            (ObjKind::ARRAY, ObjKind::ARRAY) => **lhs.array() == **rhs.array(),
            (ObjKind::RANGE, ObjKind::RANGE) => {
                let (lhs, rhs) = (&*lhs.range(), &*rhs.range());
                lhs.exclude == rhs.exclude && lhs.start == rhs.start && rhs.end == lhs.end
            }
            (ObjKind::HASH, ObjKind::HASH) => *lhs.rhash() == *rhs.rhash(),
            (ObjKind::REGEXP, ObjKind::REGEXP) => *lhs.regexp() == *rhs.regexp(),
            (ObjKind::TIME, ObjKind::TIME) => *lhs.time() == *rhs.time(),
            (ObjKind::PROC, ObjKind::PROC) => *lhs.proc() == *rhs.proc(),
            (ObjKind::METHOD, ObjKind::METHOD) => *lhs.method() == *rhs.method(),
            (ObjKind::UNBOUND_METHOD, ObjKind::UNBOUND_METHOD) => *lhs.method() == *rhs.method(),
            (ObjKind::INVALID, _) => {
                unreachable!("Invalid rvalue. (maybe GC problem) {:?}", self.rvalue())
            }
            (_, ObjKind::INVALID) => {
                unreachable!("Invalid rvalue. (maybe GC problem) {:?}", other.rvalue())
            }
            (_, _) => false,
        }
    }
}

//impl Eq for Value {}

impl Default for Value {
    #[inline(always)]
    fn default() -> Self {
        Value::nil()
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format(3))
    }
}

impl GC<RValue> for Value {
    fn mark(&self, alloc: &mut Allocator<RValue>) {
        match self.as_rvalue() {
            Some(rvalue) => rvalue.mark(alloc),
            None => {}
        }
    }
}

impl Value {
    /// Convert `self` to `RV`.
    ///
    /// `RV` is a struct for convenience in handling `Value`.
    /// Both of packed integer and ObjKind::Integer are converted to RV::Integer.
    /// Packed float and ObjKind::Float are converted to RV::Float.
    pub(crate) fn unpack(&self) -> RV {
        if !self.is_packed_value() {
            let info = self.rvalue();
            match info.kind() {
                ObjKind::INVALID => panic!(
                    "Invalid rvalue. (maybe GC problem) {:?} {:#?}",
                    &*info as *const RValue, info
                ),
                ObjKind::FLOAT => RV::Float(info.float()),
                _ => RV::Object(info),
            }
        } else if let Some(i) = self.as_fixnum() {
            RV::Integer(i)
        } else if let Some(f) = self.as_flonum() {
            RV::Float(f)
        } else if self.is_packed_symbol() {
            RV::Symbol(self.as_packed_symbol())
        } else {
            match self.get() {
                NIL_VALUE => RV::Nil,
                TRUE_VALUE => RV::True,
                FALSE_VALUE => RV::False,
                UNINITIALIZED => RV::Uninitialized,
                _ => unreachable!("Illegal packed value. {:x}", self.0),
            }
        }
    }

    pub(crate) fn eql(&self, other: &Self) -> bool {
        HashKey(*self) == HashKey(*other)
    }

    fn format(&self, level: usize) -> String {
        match self.unpack() {
            RV::Nil => format!("nil"),
            RV::True => format!("true"),
            RV::False => format!("false"),
            RV::Uninitialized => "[Uninitialized]".to_string(),
            RV::Integer(i) => format!("{}", i),
            RV::Float(f) => Self::float_format(f),
            RV::Symbol(id) => format!(":\"{:?}\"", id),
            RV::Object(rval) => match rval.kind() {
                ObjKind::INVALID => format!("[Invalid]"),
                ObjKind::ORDINARY => {
                    if let Some(name) = self.get_var(IdentId::_NAME) {
                        format!("{}", name.as_string().unwrap())
                    } else {
                        format!("#<{}:0x{:016x}>", self.get_class_name(), self.id())
                    }
                }
                ObjKind::STRING => format!(r#""{:?}""#, *rval.string()),
                ObjKind::BIGNUM => format!("{}", *rval.bignum()),
                ObjKind::FLOAT => Self::float_format(rval.float()),
                ObjKind::RANGE => {
                    let RangeInfo {
                        start,
                        end,
                        exclude,
                    } = *rval.range();
                    let sym = if exclude { "..." } else { ".." };
                    format!("{}{}{}", start.format(0), sym, end.format(0))
                }
                ObjKind::COMPLEX => {
                    let RubyComplex { r, i } = *rval.complex();
                    let (r, i) = (r.to_real().unwrap(), i.to_real().unwrap());
                    if !i.is_negative() {
                        format!("({:?}+{:?}i)", r, i)
                    } else {
                        format!("({:?}{:?}i)", r, i)
                    }
                }
                ObjKind::MODULE | ObjKind::CLASS => rval.module().inspect(),
                ObjKind::ARRAY => {
                    let aref = &*rval.array();
                    if level == 0 {
                        format!("[Array]")
                    } else {
                        let s = match aref.len() {
                            0 => String::new(),
                            n if n < 10 => {
                                let mut s = format!("{}", aref[0].format(level - 1));
                                for i in 1..n {
                                    s += &format!(", {}", aref[i].format(level - 1));
                                }
                                s
                            }
                            n => {
                                let mut s = format!("{}", aref[0].format(level - 1));
                                for i in 1..10 {
                                    s += &format!(", {}", aref[i].format(level - 1));
                                }
                                s += &format!(" .. {} items", n);
                                s
                            }
                        };
                        format!("[{}]", s)
                    }
                }
                ObjKind::HASH => {
                    let href = rval.rhash();
                    if level == 0 {
                        format!("[Hash]")
                    } else {
                        let mut s = String::new();
                        let mut flag = false;
                        for (k, v) in href.iter() {
                            if flag {
                                s += ", ";
                            }
                            s += &format!("{}=>{}", k.format(level - 1), v.format(level - 1));
                            flag = true;
                        }
                        format!("{{{}}}", s)
                    }
                }
                ObjKind::REGEXP => format!("/{}/", rval.regexp().as_str()),
                ObjKind::SPLAT => format!("Splat[{}]", rval.splat().format(level - 1)),
                ObjKind::METHOD | ObjKind::UNBOUND_METHOD => {
                    let m = rval.method();
                    match m.receiver {
                        Some(_) => format!("#<Method: {:?}#{:?}>", m.owner.name(), m.name),
                        None => format!("#<UnboundMethod: {:?}#{:?}>", m.owner.name(), m.name),
                    }
                }
                ObjKind::TIME => format!("{:?}", *rval.time()),
                ObjKind::EXCEPTION => {
                    format!(
                        "#<{}: {}>",
                        self.get_class_name(),
                        rval.exception().message()
                    )
                }
                ObjKind::ENUMERATOR => {
                    let info = match &rval.enumerator().kind {
                        FiberKind::Enum(info) => info,
                        _ => unreachable!(),
                    };
                    format!(
                        "#<{}: {:?}:{:?}>",
                        self.get_class_name(),
                        info.receiver,
                        info.method
                    )
                }
                _ => {
                    format!("#<{}:0x{:x}>", self.get_class_name(), self.id())
                }
            },
        }
    }

    fn float_format(f: f64) -> String {
        let fabs = f.abs();
        if fabs < 0.0001 || fabs >= 1000000000000000.0 {
            format!("{:.1e}", f)
        } else {
            format!("{}", f)
        }
    }

    #[inline(always)]
    pub(crate) fn id(&self) -> u64 {
        self.get()
    }

    #[inline(always)]
    pub(crate) fn from(id: u64) -> Self {
        Value(std::num::NonZeroU64::new(id).unwrap())
    }

    #[inline(always)]
    pub(crate) fn from_ptr(ptr: *mut RValue) -> Self {
        Value::from(ptr as u64)
    }

    #[inline(always)]
    pub(crate) fn into_module(self) -> Module {
        Module::new_unchecked(self)
    }

    #[inline(always)]
    pub(crate) fn into_array(self) -> Array {
        Array::new_unchecked(self)
    }

    pub(crate) fn shallow_dup(&self) -> Self {
        match self.as_rvalue() {
            Some(rv) => rv.shallow_dup().pack(),
            None => *self,
        }
    }

    pub(crate) fn is_real(&self) -> bool {
        match self.unpack() {
            RV::Float(_) | RV::Integer(_) => true,
            _ => false,
        }
    }

    /*pub(crate) fn is_zero(&self) -> bool {
        match self.unpack() {
            RV::Float(f) => f == 0.0,
            RV::Integer(i) => i == 0,
            _ => false,
        }
    }*/

    /// If `self` is Class or Module, return `self`.
    /// Otherwise, return 'real' class of `self`.
    pub(crate) fn get_class_if_object(self) -> Module {
        match self.if_mod_class() {
            Some(class) => class,
            None => self.get_class(),
        }
    }
    /// Get reference of RValue from `self`.
    ///
    /// return None if `self` was not a packed value.
    #[inline(always)]
    pub(crate) fn as_rvalue(&self) -> Option<&RValue> {
        if self.is_packed_value() {
            None
        } else {
            Some(self.rvalue())
        }
    }

    /// Get mutable reference of RValue from `self`.
    ///
    /// Return None if `self` was not a packed value.
    #[inline(always)]
    pub(crate) fn as_mut_rvalue(&mut self) -> Option<&mut RValue> {
        if self.is_packed_value() {
            None
        } else {
            Some(self.rvalue_mut())
        }
    }

    #[inline(always)]
    pub(crate) fn rvalue(&self) -> &RValue {
        unsafe { &*(self.get() as *const RValue) }
    }

    #[inline(always)]
    pub(crate) fn rvalue_mut(&self) -> &mut RValue {
        unsafe { &mut *(self.get() as *mut RValue) }
    }
}

impl Value {
    pub(crate) fn val_to_s(&self, vm: &mut VM) -> Result<Cow<str>, RubyError> {
        let s = match self.unpack() {
            RV::Uninitialized => Cow::from("[Uninitialized]"),
            RV::Nil => Cow::from(""),
            RV::True => Cow::from("true"),
            RV::False => Cow::from("false"),
            RV::Integer(i) => Cow::from(i.to_string()),
            RV::Float(f) => {
                if f.fract() == 0.0 {
                    Cow::from(format!("{:.1}", f))
                } else {
                    Cow::from(f.to_string())
                }
            }
            RV::Symbol(i) => Cow::from(format!("{:?}", i)),
            RV::Object(oref) => match oref.kind() {
                ObjKind::INVALID => panic!("Invalid rvalue. (maybe GC problem) {:?}", *oref),
                ObjKind::STRING => oref.string().to_s(),
                _ => {
                    let val = vm.eval_send0(IdentId::TO_S, *self)?;
                    Cow::from(val.as_string().unwrap().to_owned())
                }
            },
        };
        Ok(s)
    }

    /// Change class of `self`.
    ///
    /// ### panic
    /// panic if `self` was a primitive type (integer, float, etc.).
    pub(crate) fn set_class(mut self, class: Module) {
        match self.as_mut_rvalue() {
            Some(rvalue) => rvalue.set_class(class),
            None => unreachable!(
                "set_class(): can not change class of primitive type. {:?}",
                self.get_class()
            ),
        }
    }

    /// Get class of `self` for method exploration.
    /// If a direct class of `self` was a singleton class, returns the singleton class.
    ///
    /// ### panic
    /// panic if `self` was Invalid.
    pub(crate) fn get_class_for_method(&self) -> Module {
        if !self.is_packed_value() {
            self.rvalue().class()
        } else if self.as_fixnum().is_some() {
            BuiltinClass::integer()
        } else if self.is_packed_num() {
            BuiltinClass::float()
        } else if self.is_packed_symbol() {
            BuiltinClass::symbol()
        } else {
            match self.get() {
                NIL_VALUE => BuiltinClass::nilclass(),
                TRUE_VALUE => BuiltinClass::trueclass(),
                FALSE_VALUE => BuiltinClass::falseclass(),
                _ => unreachable!("Illegal packed value. {:x}", self.0),
            }
        }
    }

    /// Get class of `self`.
    /// If a direct class of `self` was a singleton class, returns a class of the singleton class.
    pub(crate) fn get_class(&self) -> Module {
        match self.unpack() {
            RV::Integer(_) => BuiltinClass::integer(),
            RV::Float(_) => BuiltinClass::float(),
            RV::Symbol(_) => BuiltinClass::symbol(),
            RV::Nil => BuiltinClass::nilclass(),
            RV::True => BuiltinClass::trueclass(),
            RV::False => BuiltinClass::falseclass(),
            RV::Object(info) => info.real_class(),
            RV::Uninitialized => unreachable!("[Uninitialized]"),
        }
    }

    pub(crate) fn get_class_name(&self) -> String {
        match self.unpack() {
            RV::Uninitialized => "[Uninitialized]".to_string(),
            RV::Nil => "NilClass".to_string(),
            RV::True => "TrueClass".to_string(),
            RV::False => "FalseClass".to_string(),
            RV::Integer(_) => "Integer".to_string(),
            RV::Float(_) => "Float".to_string(),
            RV::Symbol(_) => "Symbol".to_string(),
            RV::Object(oref) => match oref.kind() {
                ObjKind::INVALID => panic!("Invalid rvalue. (maybe GC problem) {:?}", *oref),
                ObjKind::SPLAT => "[Splat]".to_string(),
                _ => oref.real_class().name(),
            },
        }
    }

    pub(crate) fn kind_of(&self, class: Value) -> bool {
        let mut val = self.get_class();
        loop {
            if val.id() == class.id() {
                return true;
            }
            val = match val.upper() {
                Some(val) => val,
                None => break,
            };
        }
        false
    }

    pub(crate) fn is_exception_class(&self) -> bool {
        let mut val = Module::new(*self);
        let ex = BuiltinClass::exception();
        loop {
            if val.id() == ex.id() {
                return true;
            }
            val = match val.superclass() {
                Some(val) => val,
                None => break,
            };
        }
        false
    }

    /// Examine whether `self` has a singleton class.
    /// Panic if `self` is not a class object.
    /*pub(crate) fn has_singleton(&self) -> bool {
        self.get_class_for_method().is_singleton()
    }*/

    #[inline(always)]
    pub(crate) fn set_var(self, id: IdentId, val: Value) -> Option<Value> {
        self.rvalue_mut().set_var(id, val)
    }

    pub(crate) fn set_var_by_str(self, name: &str, val: Value) {
        let id = IdentId::get_id(name);
        self.set_var(id, val);
    }

    #[inline(always)]
    pub(crate) fn get_var(&self, id: IdentId) -> Option<Value> {
        self.rvalue().get_var(id)
    }

    #[inline(always)]
    pub(crate) fn set_var_if_exists(&self, id: IdentId, val: Value) -> bool {
        match self.rvalue_mut().get_mut_var(id) {
            Some(entry) => {
                *entry = val;
                true
            }
            None => false,
        }
    }
}

impl Value {
    #[inline(always)]
    pub(crate) fn is_uninitialized(&self) -> bool {
        self.get() == UNINITIALIZED
    }

    #[inline(always)]
    pub(crate) fn is_nil(&self) -> bool {
        self.get() == NIL_VALUE
    }

    #[inline(always)]
    pub(crate) fn is_packed_value(&self) -> bool {
        self.get() & 0b0111 != 0
    }

    #[inline(always)]
    pub(crate) fn as_fnum(&self) -> i64 {
        (self.get() as i64) >> 1
    }

    #[inline(always)]
    pub(crate) fn is_fnum(&self) -> bool {
        self.get() & 0b1 == 1
    }

    #[inline(always)]
    pub(crate) fn as_fixnum(&self) -> Option<i64> {
        if self.is_fnum() {
            Some(self.as_fnum())
        } else {
            None
        }
    }

    #[inline(always)]
    pub(crate) fn as_flonum(&self) -> Option<f64> {
        let u = self.get();
        if u & 0b11 == 2 {
            if u == ZERO {
                return Some(0.0);
            }
            let bit = 0b10 - ((u >> 63) & 0b1);
            let num = ((u & !(0b0011u64)) | bit).rotate_right(3);
            //eprintln!("after  unpack:{:064b}", num);
            Some(f64::from_bits(num))
        } else {
            None
        }
    }

    #[inline(always)]
    pub(crate) fn is_packed_num(&self) -> bool {
        self.get() & 0b11 != 0
    }

    #[inline(always)]
    pub(crate) fn is_packed_symbol(&self) -> bool {
        self.get() & 0xff == TAG_SYMBOL
    }

    #[inline(always)]
    pub(crate) fn as_packed_symbol(&self) -> IdentId {
        IdentId::from((self.get() >> 32) as u32)
    }

    pub(crate) fn coerce_to_fixnum(&self, _msg: &str) -> Result<i64, RubyError> {
        match self.unpack() {
            RV::Integer(i) => Ok(i),
            RV::Float(f) => Ok(f.trunc() as i64),
            _ => Err(VMError::cant_coerse(*self, "Fixnum")),
        }
    }

    pub(crate) fn as_bignum(&self) -> Option<&BigInt> {
        match self.as_rvalue() {
            Some(info) => match info.kind() {
                ObjKind::BIGNUM => Some(&*info.bignum()),
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn as_float(&self) -> Option<f64> {
        if let Some(f) = self.as_flonum() {
            Some(f)
        } else {
            match self.as_rvalue() {
                Some(info) => match info.kind() {
                    ObjKind::FLOAT => Some(info.float()),
                    _ => None,
                },
                _ => None,
            }
        }
    }

    pub(crate) fn as_complex(&self) -> Option<(Value, Value)> {
        match self.as_rvalue() {
            Some(info) => match info.kind() {
                ObjKind::COMPLEX => {
                    let RubyComplex { r, i } = *info.complex();
                    Some((r, i))
                }
                _ => None,
            },
            _ => None,
        }
    }

    pub(crate) fn as_rstring(&self) -> Option<&RString> {
        match self.as_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::STRING => Some(&*oref.string()),
                _ => None,
            },
            None => None,
        }
    }

    pub(crate) fn as_mut_rstring(&mut self) -> Option<&mut RString> {
        match self.as_mut_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::STRING => Some(oref.string_mut()),
                _ => None,
            },
            None => None,
        }
    }

    pub(crate) fn as_bytes(&self) -> Option<&[u8]> {
        match self.as_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::STRING => Some(oref.string().as_bytes()),
                _ => None,
            },
            None => None,
        }
    }

    pub(crate) fn expect_bytes(&self, msg: &str) -> Result<&[u8], RubyError> {
        match self.as_rstring() {
            Some(rs) => Ok(rs.as_bytes()),
            None => Err(VMError::wrong_type(msg, "String", *self)),
        }
    }

    pub(crate) fn as_string(&self) -> Option<&str> {
        match self.as_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::STRING => Some(oref.string().as_str()),
                _ => None,
            },
            None => None,
        }
    }

    pub(crate) fn expect_string(&mut self, msg: &str) -> Result<&str, RubyError> {
        let val = *self;
        match self.as_mut_rstring() {
            Some(rs) => rs.as_string(),
            None => Err(VMError::wrong_type(msg, "String", val)),
        }
    }

    pub(crate) fn expect_string_or_symbol(&self, msg: &str) -> Result<IdentId, RubyError> {
        let mut val = *self;
        if let Some(id) = val.as_symbol() {
            return Ok(id);
        };
        let str = val
            .as_mut_rstring()
            .ok_or_else(|| VMError::wrong_type(msg, "String or Symbol", *self))?
            .as_string()?;
        Ok(IdentId::get_id(str))
    }

    pub(crate) fn expect_symbol_or_string(&self, msg: &str) -> Result<IdentId, RubyError> {
        let val = *self;
        match self.as_symbol() {
            Some(symbol) => Ok(symbol),
            None => match self.as_string() {
                Some(s) => Ok(IdentId::get_id(s)),
                None => Err(VMError::wrong_type(msg, "Symbol or String", val)),
            },
        }
    }

    pub(crate) fn expect_regexp_or_string(
        &self,
        vm: &mut VM,
        msg: &str,
    ) -> Result<RegexpInfo, RubyError> {
        let val = *self;
        if let Some(re) = self.as_regexp() {
            Ok(re)
        } else if let Some(string) = self.as_string() {
            vm.regexp_from_string(string)
        } else {
            Err(VMError::wrong_type(msg, "RegExp or String.", val))
        }
    }

    pub(crate) fn as_class(&self) -> &ClassInfo {
        match self.as_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::MODULE | ObjKind::CLASS => oref.module(),
                _ => unreachable!("Not a module/class. {:?} {:?}", self, self.rvalue()),
            },
            None => unreachable!("Not a module/class. {:?}", self),
        }
    }

    pub(crate) fn as_mut_class(&mut self) -> &mut ClassInfo {
        match self.as_mut_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::MODULE | ObjKind::CLASS => oref.module_mut(),
                _ => unreachable!(),
            },
            None => unreachable!(),
        }
    }

    /// Check whether `self` is a Class.
    pub(crate) fn is_class(&self) -> bool {
        match self.as_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::CLASS => {
                    assert!(!oref.module().is_module());
                    true
                }
                _ => false,
            },
            None => false,
        }
    }

    /// Check whether `self` is a Module.
    pub(crate) fn is_module(&self) -> bool {
        match self.as_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::MODULE => {
                    assert!(oref.module().is_module());
                    true
                }
                _ => false,
            },
            None => false,
        }
    }

    pub(crate) fn if_mod_class(self) -> Option<Module> {
        match self.as_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::MODULE | ObjKind::CLASS => Some(self.into_module()),
                _ => None,
            },
            None => None,
        }
    }

    /// Returns `ClassRef` if `self` is a Class.
    /// When `self` is not a Class, returns `TypeError`.
    pub(crate) fn expect_class(self, msg: &str) -> Result<Module, RubyError> {
        //let self_ = self.clone();
        if self.is_class() {
            Ok(Module::new(self))
        } else {
            Err(VMError::wrong_type(msg, "Class", self))
        }
    }

    /// Returns `&ClassInfo` if `self` is a Module.
    /// When `self` is not a Module, returns `TypeError`.
    pub(crate) fn expect_module(self, msg: &str) -> Result<Module, RubyError> {
        if self.is_module() {
            Ok(Module::new(self))
        } else {
            Err(VMError::wrong_type(msg, "Module", self))
        }
    }

    /// Returns `ClassRef` if `self` is a Module / Class.
    /// When `self` is not a Module, returns `TypeError`.
    pub(crate) fn expect_mod_class(self) -> Result<Module, RubyError> {
        if self.if_mod_class().is_some() {
            Ok(Module::new(self))
        } else {
            Err(RubyError::typeerr(format!(
                "Must be Module or Class. (given:{:?})",
                self
            )))
        }
    }

    pub(crate) fn is_array(&self) -> bool {
        match self.as_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::ARRAY => true,
                _ => false,
            },
            None => false,
        }
    }

    pub(crate) fn as_array(&self) -> Option<Array> {
        if self.is_array() {
            Some(Array::new_unchecked(*self))
        } else {
            None
        }
    }

    pub(crate) fn expect_array(&self, msg: &str) -> Result<Array, RubyError> {
        match self.as_array() {
            Some(_) => Ok(self.into_array()),
            None => Err(VMError::wrong_type(msg, "Array", *self)),
        }
    }

    pub(crate) fn as_range(&self) -> Option<&RangeInfo> {
        match self.as_rvalue() {
            Some(rval) => match rval.kind() {
                ObjKind::RANGE => Some(&*rval.range()),
                _ => None,
            },
            None => None,
        }
    }

    pub(crate) fn as_splat(&self) -> Option<Value> {
        match self.as_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::SPLAT => Some(oref.splat()),
                _ => None,
            },
            None => None,
        }
    }

    pub(crate) fn as_hash(&self) -> Option<&HashInfo> {
        match self.as_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::HASH => Some(oref.rhash()),
                _ => None,
            },
            None => None,
        }
    }

    pub(crate) fn as_mut_hash(&mut self) -> Option<&mut HashInfo> {
        match self.as_mut_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::HASH => Some(oref.rhash_mut()),
                _ => None,
            },
            None => None,
        }
    }

    pub(crate) fn expect_hash(&self, msg: &str) -> Result<&HashInfo, RubyError> {
        let val = *self;
        match self.as_hash() {
            Some(hash) => Ok(hash),
            None => Err(VMError::wrong_type(msg, "Hash", val)),
        }
    }

    pub(crate) fn as_regexp(&self) -> Option<RegexpInfo> {
        match self.as_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::REGEXP => Some((*oref.regexp()).clone()),
                _ => None,
            },
            None => None,
        }
    }

    pub(crate) fn as_proc(&self) -> Option<&ProcInfo> {
        match self.as_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::PROC => Some(&*oref.proc()),
                _ => None,
            },
            None => None,
        }
    }

    pub(crate) fn as_method(&self) -> Option<&MethodObjInfo> {
        match self.as_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::METHOD => Some(oref.method()),
                _ => None,
            },
            None => None,
        }
    }

    pub(crate) fn as_unbound_method(&self) -> Option<&MethodObjInfo> {
        match self.as_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::UNBOUND_METHOD => Some(oref.method()),
                _ => None,
            },
            None => None,
        }
    }

    pub(crate) fn as_enumerator(&mut self) -> Option<&mut FiberContext> {
        match self.as_mut_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::ENUMERATOR => Some(oref.enumerator_mut()),
                _ => None,
            },
            None => None,
        }
    }

    pub(crate) fn expect_fiber(&mut self, error_msg: &str) -> Result<&mut FiberContext, RubyError> {
        match self.as_mut_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::FIBER => Ok(oref.fiber_mut()),
                _ => Err(RubyError::argument(error_msg)),
            },
            None => Err(RubyError::argument(error_msg)),
        }
    }

    pub(crate) fn if_exception(&self) -> Option<&RubyError> {
        match self.as_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::EXCEPTION => Some(oref.exception()),
                _ => None,
            },
            None => None,
        }
    }

    pub(crate) fn as_time(&self) -> &TimeInfo {
        let rval = self.rvalue();
        match rval.kind() {
            ObjKind::TIME => rval.time(),
            _ => unreachable!(),
        }
    }

    pub(crate) fn as_binding(&self) -> HeapCtxRef {
        let rval = self.rvalue();
        match rval.kind() {
            ObjKind::BINDING => rval.binding(),
            _ => unreachable!(),
        }
    }

    pub(crate) fn expect_binding(&self, error_msg: &str) -> Result<HeapCtxRef, RubyError> {
        match self.as_rvalue() {
            Some(oref) => match oref.kind() {
                ObjKind::BINDING => Ok(oref.binding()),
                _ => Err(RubyError::argument(error_msg)),
            },
            None => Err(RubyError::argument(error_msg)),
        }
    }

    pub(crate) fn as_mut_time(&mut self) -> &mut TimeInfo {
        let rval = self.rvalue_mut();
        match rval.kind() {
            ObjKind::TIME => rval.time_mut(),
            _ => unreachable!(),
        }
    }

    pub(crate) fn as_symbol(&self) -> Option<IdentId> {
        if self.is_packed_symbol() {
            Some(self.as_packed_symbol())
        } else {
            None
        }
    }
}

impl Value {
    #[inline(always)]
    pub const fn uninitialized() -> Self {
        Value(unsafe { std::num::NonZeroU64::new_unchecked(UNINITIALIZED) })
    }
    #[inline(always)]
    pub const fn nil() -> Self {
        Value(unsafe { std::num::NonZeroU64::new_unchecked(NIL_VALUE) })
    }

    #[inline(always)]
    pub const fn true_val() -> Self {
        Value(unsafe { std::num::NonZeroU64::new_unchecked(TRUE_VALUE) })
    }

    #[inline(always)]
    pub const fn false_val() -> Self {
        Value(unsafe { std::num::NonZeroU64::new_unchecked(FALSE_VALUE) })
    }

    #[inline(always)]
    pub fn bool(b: bool) -> Self {
        if b {
            Value::from(TRUE_VALUE)
        } else {
            Value::from(FALSE_VALUE)
        }
    }

    #[inline(always)]
    pub(crate) fn fixnum(num: i64) -> Self {
        Value::from((num << 1) as u64 | 0b1)
    }

    #[inline(always)]
    pub(crate) fn is_i63(num: i64) -> bool {
        let top = (num as u64) >> 62 ^ (num as u64) >> 63;
        top & 0b1 == 0
    }

    pub fn integer(num: i64) -> Self {
        if Value::is_i63(num) {
            Value::fixnum(num)
        } else {
            RValue::new_bigint(num.to_bigint().unwrap()).pack()
        }
    }

    pub fn bignum(num: BigInt) -> Self {
        if let Some(i) = num.to_i64() {
            Value::integer(i)
        } else {
            RValue::new_bigint(num).pack()
        }
    }

    pub fn float(num: f64) -> Self {
        if num == 0.0 {
            return Value::from(ZERO);
        }
        let unum = f64::to_bits(num);
        let exp = ((unum >> 60) & 0b111) + 1;
        if (exp & 0b0110) == 0b0100 {
            Value::from((unum & FLOAT_MASK1 | FLOAT_MASK2).rotate_left(3))
        } else {
            RValue::new_float(num).pack()
        }
    }

    pub fn complex(r: Value, i: Value) -> Self {
        RValue::new_complex(r, i).pack()
    }

    pub(crate) fn string_from_rstring(rs: RString) -> Self {
        RValue::new_string_from_rstring(rs).pack()
    }

    pub fn string<'a>(string: impl Into<Cow<'a, str>>) -> Self {
        RValue::new_string(string).pack()
    }

    pub fn bytes(bytes: Vec<u8>) -> Self {
        match std::str::from_utf8(&bytes) {
            Ok(s) => RValue::new_string(s).pack(),
            Err(_) => RValue::new_bytes(bytes).pack(),
        }
    }

    pub fn symbol(id: IdentId) -> Self {
        let id: u32 = id.into();
        Value::from((id as u64) << 32 | TAG_SYMBOL)
    }

    pub fn symbol_from_str(sym: &str) -> Self {
        Value::symbol(IdentId::get_id(sym))
    }

    pub fn range(start: Value, end: Value, exclude: bool) -> Self {
        let info = RangeInfo::new(start, end, exclude);
        RValue::new_range(info).pack()
    }

    pub fn ordinary_object(class: Module) -> Self {
        RValue::new_ordinary(class).pack()
    }

    pub(crate) fn array_empty() -> Value {
        Value::array_from(vec![])
    }

    pub fn array_from(ary: Vec<Value>) -> Value {
        RValue::new_array(ArrayInfo::new(ary)).pack()
    }

    pub fn array_from_slice(slice: &[Value]) -> Value {
        RValue::new_array(ArrayInfo::new_from_slice(slice)).pack()
    }

    pub fn array_from_with_class(ary: Vec<Value>, class: Module) -> Value {
        RValue::new_array_with_class(ArrayInfo::new(ary), class).pack()
    }

    pub(crate) fn splat(val: Value) -> Self {
        RValue::new_splat(val).pack()
    }

    pub(crate) fn hash_from(hash: HashInfo) -> Self {
        RValue::new_hash(hash).pack()
    }

    pub fn hash_from_map(hash: FxIndexMap<HashKey, Value>) -> Self {
        RValue::new_hash(HashInfo::new(hash)).pack()
    }

    pub(crate) fn regexp(regexp: RegexpInfo) -> Self {
        RValue::new_regexp(regexp).pack()
    }

    pub fn regexp_from(vm: &mut VM, string: &str) -> Result<Self, RubyError> {
        Ok(RValue::new_regexp(vm.regexp_from_string(string)?).pack())
    }

    pub(crate) fn procobj(
        vm: &mut VM,
        self_val: Value,
        method: FnId,
        outer: Option<Frame>,
    ) -> Self {
        let outer = if let Some(outer) = outer {
            Some(vm.move_frame_to_heap(outer))
        } else {
            None
        };
        RValue::new_proc(ProcInfo::new(self_val, method, outer)).pack()
    }

    pub(crate) fn method(name: IdentId, receiver: Value, method: FnId, owner: Module) -> Self {
        RValue::new_method(MethodObjInfo::new(name, receiver, method, owner)).pack()
    }

    pub(crate) fn unbound_method(name: IdentId, method: FnId, owner: Module) -> Self {
        RValue::new_unbound_method(MethodObjInfo::new_unbound(name, method, owner)).pack()
    }

    pub(crate) fn fiber(parent_vm: &mut VM, context: HeapCtxRef) -> Self {
        let new_fiber = parent_vm.create_fiber();
        RValue::new_fiber(new_fiber, context).pack()
    }

    pub(crate) fn enumerator(fiber: FiberContext) -> Self {
        RValue::new_enumerator(fiber).pack()
    }

    pub(crate) fn time(time_class: Module, time: TimeInfo) -> Self {
        RValue::new_time(time_class, time).pack()
    }

    pub(crate) fn exception(exception_class: Module, err: RubyError) -> Self {
        RValue::new_exception(exception_class, err).pack()
    }

    pub(crate) fn binding(ctx: HeapCtxRef) -> Self {
        RValue::new_binding(ctx).pack()
    }

    pub(crate) fn from_ord(ord: Option<std::cmp::Ordering>) -> Self {
        ord.map_or(Value::nil(), |ord| Value::integer(ord as i64))
    }
}

impl Value {
    pub(crate) fn to_ordering(&self) -> Result<std::cmp::Ordering, RubyError> {
        use std::cmp::Ordering;
        if let Some(i) = self.as_fixnum() {
            match i {
                0 => Ok(Ordering::Equal),
                i if i > 0 => Ok(Ordering::Greater),
                _ => Ok(Ordering::Less),
            }
        } else if let Some(b) = self.as_bignum() {
            match b.sign() {
                Sign::Plus => Ok(Ordering::Greater),
                Sign::Minus => Ok(Ordering::Less),
                _ => Ok(Ordering::Equal),
            }
        } else {
            Err(RubyError::argument("Ordering value must be Integer."))
        }
    }
}

impl Value {
    /// Get singleton class object of `self`.
    ///
    /// When `self` already has a singleton class, simply return it.  
    /// If not, generate a new singleton class object.  
    /// Return None when `self` was a primitive (i.e. Integer, Symbol, Float) which can not have a singleton class.
    pub(crate) fn get_singleton_class(self) -> Result<Module, RubyError> {
        match self.clone().as_mut_rvalue() {
            Some(oref) => {
                let class = oref.class();
                if class.is_singleton() {
                    Ok(class)
                } else {
                    let singleton = match oref.kind() {
                        ObjKind::CLASS => {
                            assert!(!oref.module().is_module());
                            let superclass = match oref.module().superclass() {
                                None => None,
                                Some(superclass) => Some(superclass.get_singleton_class()),
                            };
                            Module::singleton_class_from(superclass, self)
                        }
                        ObjKind::INVALID => {
                            panic!("Invalid rvalue. (maybe GC problem) {:?}", *oref)
                        }
                        _ => Module::singleton_class_from(class, self),
                    };
                    oref.set_class(singleton);
                    Ok(singleton)
                }
            }
            _ => Err(RubyError::typeerr(format!(
                "Can not define singleton for {:?}:{}",
                self,
                self.get_class_name()
            ))),
        }
    }

    /// Get method(MethodId) for receiver.
    pub(crate) fn get_method_or_nomethod(
        self,
        globals: &mut Globals,
        method_name: IdentId,
    ) -> Result<FnId, RubyError> {
        let rec_class = self.get_class_for_method();
        rec_class.get_method_or_nomethod(globals, method_name)
    }
}

impl Value {
    /// Convert `self` to boolean value.
    #[inline(always)]
    pub(crate) fn to_bool(&self) -> bool {
        self.get() & BOOL_MASK2 | BOOL_MASK1 != 0x34
    }

    pub(crate) fn expect_bool_nil_num(self) -> Result<bool, RubyError> {
        match self.unpack() {
            RV::True | RV::Integer(0) => Ok(true),
            RV::Float(f) if f == 0.0 => Ok(true),
            RV::False | RV::Nil | RV::Integer(_) | RV::Float(_) => Ok(false),
            _ => Err(RubyError::typeerr(format!(
                "Wrong argument type {} (must be numeric, true, false or nil)",
                self.get_class_name()
            ))),
        }
    }

    /// Convert `self` to `Option<Real>`.
    /// If `self` was not a integer nor a float, return `None`.
    pub(crate) fn to_real(&self) -> Option<Real> {
        match self.unpack() {
            RV::Integer(i) => Some(Real::Integer(i)),
            RV::Float(f) => Some(Real::Float(f)),
            RV::Object(obj) => match obj.kind() {
                ObjKind::BIGNUM => Some(Real::Bignum((*obj.bignum()).clone())),
                _ => None,
            },
            _ => None,
        }
    }

    /// Convert `self` to `Option<(real:Real, imaginary:Real)>`.
    /// If `self` was not a integer nor a float nor a complex, return `None`.
    pub(crate) fn to_complex(&self) -> Option<(Real, Real)> {
        match self.unpack() {
            RV::Integer(i) => Some((Real::Integer(i), Real::Integer(0))),
            RV::Float(f) => Some((Real::Float(f), Real::Integer(0))),
            RV::Object(obj) => match obj.kind() {
                ObjKind::COMPLEX => {
                    let RubyComplex { r, i } = *obj.complex();
                    Some((r.to_real().unwrap(), i.to_real().unwrap()))
                }
                _ => None,
            },
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn pack_bool1() {
        let expect = RV::True;
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_bool2() {
        let expect = RV::False;
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_nil() {
        let expect = RV::Nil;
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_uninit() {
        let expect = RV::Uninitialized;
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer1() {
        let expect = RV::Integer(12054);
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float0() {
        let expect = RV::Float(0.0);
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float1() {
        let expect = RV::Float(100.0);
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float2() {
        let expect = RV::Float(13859.628547);
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float3() {
        let expect = RV::Float(-5282.2541156);
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_range() {
        let from = RV::Integer(7).pack();
        let to = RV::Integer(36).pack();
        let expect = Value::range(from, to, true);
        let got = expect.unpack().pack();
        if expect.id() != got.id() {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_class() {
        let expect: Value = Module::class_under(None).into();
        let got = expect.unpack().pack();
        if expect.id() != got.id() {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_instance() {
        let expect = Value::ordinary_object(BuiltinClass::class());
        let got = expect.unpack().pack();
        if expect.id() != got.id() {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_symbol() {
        let expect = RV::Symbol(IdentId::from(12345));
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }
}
