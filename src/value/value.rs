use crate::coroutine::*;
use crate::*;
use std::borrow::Cow;

const UNINITIALIZED: u64 = 0x04;
const TAG_SYMBOL: u64 = 0x0c;
const TRUE_VALUE: u64 = 0x14;
const FALSE_VALUE: u64 = 0x1c;
const NIL_VALUE: u64 = 0x24;
const MASK1: u64 = !(0b0110u64 << 60);
const MASK2: u64 = 0b0100u64 << 60;

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

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Value(std::num::NonZeroU64);

impl std::ops::Deref for Value {
    type Target = std::num::NonZeroU64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self.as_rvalue() {
            None => self.0.hash(state),
            Some(lhs) => match &lhs.kind {
                ObjKind::Invalid => unreachable!("Invalid rvalue. (maybe GC problem) {:?}", lhs),
                ObjKind::Integer(lhs) => (*lhs as f64).to_bits().hash(state),
                ObjKind::Float(lhs) => lhs.to_bits().hash(state),
                ObjKind::String(lhs) => lhs.hash(state),
                ObjKind::Array(lhs) => lhs.elements.hash(state),
                ObjKind::Range(lhs) => lhs.hash(state),
                ObjKind::Hash(lhs) => {
                    for (key, val) in lhs.iter() {
                        key.hash(state);
                        val.hash(state);
                    }
                }
                ObjKind::Method(lhs) => (*lhs).hash(state),
                _ => self.0.hash(state),
            },
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
            if self.is_packed_num() && other.is_packed_num() {
                match (self.is_packed_fixnum(), other.is_packed_fixnum()) {
                    (true, false) => {
                        return self.as_packed_fixnum() as f64 == other.as_packed_flonum()
                    }
                    (false, true) => {
                        return self.as_packed_flonum() == other.as_packed_fixnum() as f64
                    }
                    _ => return false,
                }
            }
            return false;
        };
        match (&self.rvalue().kind, &other.rvalue().kind) {
            (ObjKind::Integer(lhs), ObjKind::Integer(rhs)) => *lhs == *rhs,
            (ObjKind::Float(lhs), ObjKind::Float(rhs)) => *lhs == *rhs,
            (ObjKind::Integer(lhs), ObjKind::Float(rhs)) => *lhs as f64 == *rhs,
            (ObjKind::Float(lhs), ObjKind::Integer(rhs)) => *lhs == *rhs as f64,
            (ObjKind::Complex { r: r1, i: i1 }, ObjKind::Complex { r: r2, i: i2 }) => {
                r1.eq(r2) && i1.eq(i2)
            }
            (ObjKind::String(lhs), ObjKind::String(rhs)) => lhs.as_bytes() == rhs.as_bytes(),
            (ObjKind::Array(lhs), ObjKind::Array(rhs)) => lhs.elements == rhs.elements,
            (ObjKind::Range(lhs), ObjKind::Range(rhs)) => lhs == rhs,
            (ObjKind::Hash(lhs), ObjKind::Hash(rhs)) => **lhs == **rhs,
            (ObjKind::Regexp(lhs), ObjKind::Regexp(rhs)) => *lhs == *rhs,
            (ObjKind::Time(lhs), ObjKind::Time(rhs)) => *lhs == *rhs,
            (ObjKind::Proc(lhs), ObjKind::Proc(rhs)) => lhs.context.id() == rhs.context.id(),
            (ObjKind::Invalid, _) => {
                unreachable!("Invalid rvalue. (maybe GC problem) {:?}", self.rvalue())
            }
            (_, ObjKind::Invalid) => {
                unreachable!("Invalid rvalue. (maybe GC problem) {:?}", other.rvalue())
            }
            (_, _) => false,
        }
    }
}

impl Eq for Value {}

impl Default for Value {
    fn default() -> Self {
        Value::nil()
    }
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format(3))
    }
}

impl GC for Value {
    fn mark(&self, alloc: &mut Allocator) {
        match self.as_gcbox() {
            Some(rvalue) => {
                rvalue.gc_mark(alloc);
            }
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
    pub fn unpack(&self) -> RV {
        if !self.is_packed_value() {
            let info = self.rvalue();
            match &info.kind {
                ObjKind::Invalid => panic!(
                    "Invalid rvalue. (maybe GC problem) {:?} {:#?}",
                    &*info as *const RValue, info
                ),
                ObjKind::Integer(i) => RV::Integer(*i),
                ObjKind::Float(f) => RV::Float(*f),
                _ => RV::Object(info),
            }
        } else if self.is_packed_fixnum() {
            RV::Integer(self.as_packed_fixnum())
        } else if self.is_packed_num() {
            RV::Float(self.as_packed_flonum())
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

    fn format(&self, level: usize) -> String {
        match self.unpack() {
            RV::Nil => format!("nil"),
            RV::True => format!("true"),
            RV::False => format!("false"),
            RV::Uninitialized => "[Uninitialized]".to_string(),
            RV::Integer(i) => format!("{}", i),
            RV::Float(f) => format!("{}", f),
            RV::Symbol(id) => format!(":\"{:?}\"", id),
            RV::Object(rval) => match &rval.kind {
                ObjKind::Invalid => format!("[Invalid]"),
                ObjKind::Ordinary => format!("#<{}:0x{:016x}>", self.get_class_name(), self.id()),
                ObjKind::String(rs) => format!(r#""{:?}""#, rs),
                ObjKind::Integer(i) => format!("{}", i),
                ObjKind::Float(f) => format!("{}", f),
                ObjKind::Range(r) => {
                    let sym = if r.exclude { "..." } else { ".." };
                    format!("{}{}{}", r.start.format(0), sym, r.end.format(0))
                }
                ObjKind::Complex { r, i } => {
                    let (r, i) = (r.to_real().unwrap(), i.to_real().unwrap());
                    if !i.is_negative() {
                        format!("({:?}+{:?}i)", r, i)
                    } else {
                        format!("({:?}{:?}i)", r, i)
                    }
                }
                ObjKind::Module(cinfo) => cinfo.inspect(),
                ObjKind::Array(aref) => {
                    if level == 0 {
                        format!("[Array]")
                    } else {
                        let s = match aref.elements.len() {
                            0 => String::new(),
                            1 => format!("{}", aref.elements[0].format(level - 1)),
                            2 => format!(
                                "{}, {}",
                                aref.elements[0].format(level - 1),
                                aref.elements[1].format(level - 1)
                            ),
                            3 => format!(
                                "{}, {}, {}",
                                aref.elements[0].format(level - 1),
                                aref.elements[1].format(level - 1),
                                aref.elements[2].format(level - 1)
                            ),
                            4 => format!(
                                "{}, {}, {}, {}",
                                aref.elements[0].format(level - 1),
                                aref.elements[1].format(level - 1),
                                aref.elements[2].format(level - 1),
                                aref.elements[3].format(level - 1)
                            ),
                            n => format!(
                                "{}, {}, {}, {}.. {} items",
                                aref.elements[0].format(level - 1),
                                aref.elements[1].format(level - 1),
                                aref.elements[2].format(level - 1),
                                aref.elements[3].format(level - 1),
                                n
                            ),
                        };
                        format!("[{}]", s)
                    }
                }
                ObjKind::Hash(href) => {
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
                ObjKind::Regexp(rref) => format!("/{}/", rref.as_str()),
                ObjKind::Splat(v) => format!("Splat[{}]", v.format(level - 1)),
                ObjKind::Proc(p) => format!("#<Proc:0x{:x}>", p.context.id()),
                ObjKind::Method(m) => match m.receiver {
                    Some(_) => format!("#<Method: {:?}#{:?}>", m.owner.name(), m.name),
                    None => format!("#<UnboundMethod: {:?}#{:?}>", m.owner.name(), m.name),
                },
                ObjKind::Enumerator(_) => format!("Enumerator"),
                ObjKind::Fiber(_) => format!("Fiber"),
                ObjKind::Time(time) => format!("{:?}", time),
                ObjKind::Exception(err) => {
                    format!("#<{}: {}>", self.get_class_name(), err.message())
                }
            },
        }
    }

    pub fn id(&self) -> u64 {
        self.get()
    }

    pub fn from(id: u64) -> Self {
        Value(std::num::NonZeroU64::new(id).unwrap())
    }

    pub fn from_ptr<T: GC>(ptr: *mut GCBox<T>) -> Self {
        Value::from(ptr as u64)
    }

    pub fn into_module(self) -> Module {
        Module::new_unchecked(self)
    }

    pub fn into_array(self) -> Array {
        Array::new_unchecked(self)
    }

    pub fn dup(&self) -> Self {
        match self.as_rvalue() {
            Some(rv) => rv.dup().pack(),
            None => *self,
        }
    }

    pub fn is_real(&self) -> bool {
        match self.unpack() {
            RV::Float(_) | RV::Integer(_) => true,
            _ => false,
        }
    }

    pub fn is_zero(&self) -> bool {
        match self.unpack() {
            RV::Float(f) => f == 0.0,
            RV::Integer(i) => i == 0,
            _ => false,
        }
    }

    pub fn as_gcbox(&self) -> Option<&GCBox<RValue>> {
        if self.is_packed_value() {
            None
        } else {
            Some(self.gcbox())
        }
    }

    /// Get reference of RValue from `self`.
    ///
    /// return None if `self` was not a packed value.
    pub fn as_rvalue(&self) -> Option<&RValue> {
        if self.is_packed_value() {
            None
        } else {
            Some(self.rvalue())
        }
    }

    /// Get mutable reference of RValue from `self`.
    ///
    /// Return None if `self` was not a packed value.
    pub fn as_mut_rvalue(&mut self) -> Option<&mut RValue> {
        if self.is_packed_value() {
            None
        } else {
            Some(self.rvalue_mut())
        }
    }

    pub fn gcbox(&self) -> &GCBox<RValue> {
        unsafe { &*(self.get() as *const GCBox<RValue>) }
    }

    pub fn rvalue(&self) -> &RValue {
        unsafe { &*(self.get() as *const GCBox<RValue>) }.inner()
    }

    pub fn rvalue_mut(&self) -> &mut RValue {
        unsafe { &mut *(self.get() as *mut GCBox<RValue>) }.inner_mut()
    }
}

impl Value {
    pub fn val_to_s(&self, vm: &mut VM) -> Result<Cow<str>, RubyError> {
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
            RV::Object(oref) => match &oref.kind {
                ObjKind::Invalid => panic!("Invalid rvalue. (maybe GC problem) {:?}", *oref),
                ObjKind::String(s) => s.to_s(),
                _ => {
                    let val = vm.send0(IdentId::TO_S, *self)?;
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
    pub fn set_class(mut self, class: Module) {
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
    pub fn get_class_for_method(&self) -> Module {
        if !self.is_packed_value() {
            self.rvalue().class()
        } else if self.is_packed_fixnum() {
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
    pub fn get_class(&self) -> Module {
        match self.unpack() {
            RV::Integer(_) => BuiltinClass::integer(),
            RV::Float(_) => BuiltinClass::float(),
            RV::Symbol(_) => BuiltinClass::symbol(),
            RV::Nil => BuiltinClass::nilclass(),
            RV::True => BuiltinClass::trueclass(),
            RV::False => BuiltinClass::falseclass(),
            RV::Object(info) => info.search_class(),
            RV::Uninitialized => unreachable!("[Uninitialized]"),
        }
    }

    pub fn get_class_name(&self) -> String {
        match self.unpack() {
            RV::Uninitialized => "[Uninitialized]".to_string(),
            RV::Nil => "NilClass".to_string(),
            RV::True => "TrueClass".to_string(),
            RV::False => "FalseClass".to_string(),
            RV::Integer(_) => "Integer".to_string(),
            RV::Float(_) => "Float".to_string(),
            RV::Symbol(_) => "Symbol".to_string(),
            RV::Object(oref) => match &oref.kind {
                ObjKind::Invalid => panic!("Invalid rvalue. (maybe GC problem) {:?}", *oref),
                ObjKind::Splat(_) => "[Splat]".to_string(),
                _ => oref.class().name(),
            },
        }
    }

    pub fn kind_of(&self, class: Value) -> bool {
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

    pub fn is_exception_class(&self) -> bool {
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
    pub fn has_singleton(&self) -> bool {
        self.get_class_for_method().is_singleton()
    }

    pub fn set_var(self, id: IdentId, val: Value) -> Option<Value> {
        self.rvalue_mut().set_var(id, val)
    }

    pub fn set_var_by_str(self, name: &str, val: Value) {
        let id = IdentId::get_id(name);
        self.set_var(id, val);
    }

    pub fn get_var(&self, id: IdentId) -> Option<Value> {
        self.rvalue().get_var(id)
    }

    pub fn set_var_if_exists(&self, id: IdentId, val: Value) -> bool {
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
    pub fn is_packed_fixnum(&self) -> bool {
        self.get() & 0b1 == 1
    }

    pub fn is_packed_flonum(&self) -> bool {
        self.get() & 0b11 == 2
    }

    pub fn is_packed_num(&self) -> bool {
        self.get() & 0b11 != 0
    }

    pub fn is_packed_symbol(&self) -> bool {
        self.get() & 0xff == TAG_SYMBOL
    }

    pub fn is_uninitialized(&self) -> bool {
        self.get() == UNINITIALIZED
    }

    pub fn is_nil(&self) -> bool {
        self.get() == NIL_VALUE
    }

    pub fn is_true_val(&self) -> bool {
        self.get() == TRUE_VALUE
    }

    pub fn is_false_val(&self) -> bool {
        self.get() == FALSE_VALUE
    }

    pub fn is_packed_value(&self) -> bool {
        self.get() & 0b0111 != 0
    }

    pub fn as_integer(&self) -> Option<i64> {
        if self.is_packed_fixnum() {
            Some(self.as_packed_fixnum())
        } else {
            match self.as_rvalue() {
                Some(info) => match &info.kind {
                    ObjKind::Integer(f) => Some(*f),
                    _ => None,
                },
                _ => None,
            }
        }
    }

    pub fn expect_integer(&self, msg: impl Into<String>) -> Result<i64, RubyError> {
        match self.unpack() {
            RV::Integer(i) => Ok(i),
            RV::Float(f) => Ok(f.trunc() as i64),
            _ => Err(RubyError::wrong_type(msg.into(), "Integer", *self)),
        }
    }

    pub fn expect_flonum(&self, msg: &str) -> Result<f64, RubyError> {
        match self.as_float() {
            Some(f) => Ok(f),
            None => Err(RubyError::wrong_type(msg, "Float", *self)),
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        if self.is_packed_flonum() {
            Some(self.as_packed_flonum())
        } else {
            match self.as_rvalue() {
                Some(info) => match &info.kind {
                    ObjKind::Float(f) => Some(*f),
                    _ => None,
                },
                _ => None,
            }
        }
    }

    pub fn as_complex(&self) -> Option<(Value, Value)> {
        match self.as_rvalue() {
            Some(info) => match &info.kind {
                ObjKind::Complex { r, i } => Some((*r, *i)),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn as_rstring(&self) -> Option<&RString> {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::String(rstr) => Some(rstr),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_mut_rstring(&mut self) -> Option<&mut RString> {
        match self.as_mut_rvalue() {
            Some(oref) => match &mut oref.kind {
                ObjKind::String(ref mut rstr) => Some(rstr),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::String(rs) => Some(rs.as_bytes()),
                _ => None,
            },
            None => None,
        }
    }

    pub fn expect_bytes(&self, msg: &str) -> Result<&[u8], RubyError> {
        match self.as_rstring() {
            Some(rs) => Ok(rs.as_bytes()),
            None => Err(RubyError::wrong_type(msg, "String", *self)),
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::String(rs) => Some(rs.as_str()),
                _ => None,
            },
            None => None,
        }
    }

    pub fn expect_string(&mut self, msg: &str) -> Result<&str, RubyError> {
        let val = *self;
        match self.as_mut_rstring() {
            Some(rs) => rs.as_string(),
            None => Err(RubyError::wrong_type(msg, "String", val)),
        }
    }

    pub fn expect_string_or_symbol(&self, msg: &str) -> Result<IdentId, RubyError> {
        let mut val = *self;
        if let Some(id) = val.as_symbol() {
            return Ok(id);
        };
        let str = val
            .as_mut_rstring()
            .ok_or_else(|| RubyError::wrong_type(msg, "String or Symbol", *self))?
            .as_string()?;
        Ok(IdentId::get_id(str))
    }

    pub fn expect_symbol_or_string(&self, msg: &str) -> Result<IdentId, RubyError> {
        let val = *self;
        match self.as_symbol() {
            Some(symbol) => Ok(symbol),
            None => match self.as_string() {
                Some(s) => Ok(IdentId::get_id(s)),
                None => Err(RubyError::wrong_type(msg, "Symbol or String", val)),
            },
        }
    }

    pub fn expect_regexp_or_string(&self, vm: &mut VM, msg: &str) -> Result<RegexpInfo, RubyError> {
        let val = *self;
        if let Some(re) = self.as_regexp() {
            Ok(re)
        } else if let Some(string) = self.as_string() {
            vm.regexp_from_string(string)
        } else {
            Err(RubyError::wrong_type(msg, "RegExp or String.", val))
        }
    }

    pub fn as_class(&self) -> &ClassInfo {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Module(cinfo) => cinfo,
                _ => unreachable!(),
            },
            None => unreachable!(),
        }
    }

    pub fn as_mut_class(&mut self) -> &mut ClassInfo {
        match self.as_mut_rvalue() {
            Some(oref) => match &mut oref.kind {
                ObjKind::Module(cinfo) => cinfo,
                _ => unreachable!(),
            },
            None => unreachable!(),
        }
    }

    /// Check whether `self` is a Class.
    pub fn is_class(&self) -> bool {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Module(cinfo) => !cinfo.is_module(),
                _ => false,
            },
            None => false,
        }
    }

    /// Check whether `self` is a Module.
    pub fn is_module(&self) -> bool {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Module(cinfo) => cinfo.is_module(),
                _ => false,
            },
            None => false,
        }
    }

    pub fn if_mod_class(self) -> Option<Module> {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Module(_) => Some(self.into_module()),
                _ => None,
            },
            None => None,
        }
    }

    /// Returns `ClassRef` if `self` is a Class.
    /// When `self` is not a Class, returns `TypeError`.
    pub fn expect_class(self, msg: &str) -> Result<Module, RubyError> {
        //let self_ = self.clone();
        if self.is_class() {
            Ok(Module::new(self))
        } else {
            Err(RubyError::wrong_type(msg, "Class", self))
        }
    }

    /// Returns `&ClassInfo` if `self` is a Module.
    /// When `self` is not a Module, returns `TypeError`.
    pub fn expect_module(self, msg: &str) -> Result<Module, RubyError> {
        if self.is_module() {
            Ok(Module::new(self))
        } else {
            Err(RubyError::wrong_type(msg, "Module", self))
        }
    }

    /// Returns `ClassRef` if `self` is a Module / Class.
    /// When `self` is not a Module, returns `TypeError`.
    pub fn expect_mod_class(self) -> Result<Module, RubyError> {
        if self.if_mod_class().is_some() {
            Ok(Module::new(self))
        } else {
            Err(RubyError::typeerr(format!(
                "Must be Module or Class. (given:{:?})",
                self
            )))
        }
    }

    pub fn is_array(&self) -> bool {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Array(_) => true,
                _ => false,
            },
            None => false,
        }
    }

    pub fn as_array(&self) -> Option<Array> {
        if self.is_array() {
            Some(Array::new_unchecked(*self))
        } else {
            None
        }
    }

    pub fn expect_array(&mut self, msg: &str) -> Result<Array, RubyError> {
        match self.as_array() {
            Some(_) => Ok(self.into_array()),
            None => Err(RubyError::wrong_type(msg, "Array", *self)),
        }
    }

    pub fn as_range(&self) -> Option<&RangeInfo> {
        match self.as_rvalue() {
            Some(rval) => match &rval.kind {
                ObjKind::Range(info) => Some(info),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_splat(&self) -> Option<Value> {
        match self.as_rvalue() {
            Some(oref) => match oref.kind {
                ObjKind::Splat(val) => Some(val),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_hash(&self) -> Option<&HashInfo> {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Hash(hash) => Some(hash),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_mut_hash(&mut self) -> Option<&mut HashInfo> {
        match self.as_mut_rvalue() {
            Some(oref) => match &mut oref.kind {
                ObjKind::Hash(hash) => Some(hash),
                _ => None,
            },
            None => None,
        }
    }

    pub fn expect_hash(&self, msg: &str) -> Result<&HashInfo, RubyError> {
        let val = *self;
        match self.as_hash() {
            Some(hash) => Ok(hash),
            None => Err(RubyError::wrong_type(msg, "Hash", val)),
        }
    }

    pub fn as_regexp(&self) -> Option<RegexpInfo> {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Regexp(regref) => Some(regref.clone()),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_proc(&self) -> Option<&ProcInfo> {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Proc(pref) => Some(pref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn expect_proc(&self, _: &mut VM) -> Result<&ProcInfo, RubyError> {
        match self.as_proc() {
            Some(e) => Ok(e),
            None => Err(RubyError::argument("Must be Proc.")),
        }
    }

    pub fn as_method(&self) -> Option<&MethodObjInfo> {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Method(mref) => Some(mref),
                _ => None,
            },
            None => None,
        }
    }
    /*
        pub fn as_fiber(&mut self) -> Option<&mut FiberInfo> {
            match self.as_mut_rvalue() {
                Some(oref) => match &mut oref.kind {
                    ObjKind::Fiber(info) => Some(info.as_mut()),
                    _ => None,
                },
                None => None,
            }
        }
    */
    pub fn as_enumerator(&mut self) -> Option<&mut FiberContext> {
        match self.as_mut_rvalue() {
            Some(oref) => match &mut oref.kind {
                ObjKind::Enumerator(info) => Some(info.as_mut()),
                _ => None,
            },
            None => None,
        }
    }

    pub fn expect_enumerator(&mut self, error_msg: &str) -> Result<&mut FiberContext, RubyError> {
        match self.as_enumerator() {
            Some(e) => Ok(e),
            None => Err(RubyError::argument(error_msg)),
        }
    }

    pub fn expect_fiber(&mut self, error_msg: &str) -> Result<&mut FiberContext, RubyError> {
        match self.as_mut_rvalue() {
            Some(oref) => match &mut oref.kind {
                ObjKind::Fiber(f) => Ok(f.as_mut()),
                _ => Err(RubyError::argument(error_msg)),
            },
            None => Err(RubyError::argument(error_msg)),
        }
    }

    pub fn if_exception(&self) -> Option<&RubyError> {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Exception(err) => Some(err),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_time(&self) -> &TimeInfo {
        match &self.rvalue().kind {
            ObjKind::Time(time) => &**time,
            _ => unreachable!(),
        }
    }

    pub fn as_mut_time(&mut self) -> &mut TimeInfo {
        match &mut self.rvalue_mut().kind {
            ObjKind::Time(time) => &mut **time,
            _ => unreachable!(),
        }
    }

    pub fn as_symbol(&self) -> Option<IdentId> {
        if self.is_packed_symbol() {
            Some(self.as_packed_symbol())
        } else {
            None
        }
    }

    pub fn as_packed_fixnum(&self) -> i64 {
        (self.get() as i64) >> 1
    }

    pub fn as_packed_flonum(&self) -> f64 {
        if self.get() == ZERO {
            return 0.0;
        }
        let bit = 0b10 - ((self.get() >> 63) & 0b1);
        let num = ((self.get() & !(0b0011u64)) | bit).rotate_right(3);
        //eprintln!("after  unpack:{:064b}", num);
        f64::from_bits(num)
    }

    pub fn as_packed_symbol(&self) -> IdentId {
        IdentId::from((self.get() >> 32) as u32)
    }
}

impl Value {
    pub const fn uninitialized() -> Self {
        Value(unsafe { std::num::NonZeroU64::new_unchecked(UNINITIALIZED) })
    }

    pub const fn nil() -> Self {
        Value(unsafe { std::num::NonZeroU64::new_unchecked(NIL_VALUE) })
    }

    pub const fn true_val() -> Self {
        Value(unsafe { std::num::NonZeroU64::new_unchecked(TRUE_VALUE) })
    }

    pub const fn false_val() -> Self {
        Value(unsafe { std::num::NonZeroU64::new_unchecked(FALSE_VALUE) })
    }

    pub fn bool(b: bool) -> Self {
        if b {
            Value::from(TRUE_VALUE)
        } else {
            Value::from(FALSE_VALUE)
        }
    }

    pub fn integer(num: i64) -> Self {
        let top = (num as u64) >> 62 ^ (num as u64) >> 63;
        if top & 0b1 == 0 {
            Value::from((num << 1) as u64 | 0b1)
        } else {
            RValue::new_integer(num).pack()
        }
    }

    pub fn float(num: f64) -> Self {
        if num == 0.0 {
            return Value::from(ZERO);
        }
        let unum = f64::to_bits(num);
        let exp = ((unum >> 60) & 0b111) + 1;
        if (exp & 0b0110) == 0b0100 {
            Value::from((unum & MASK1 | MASK2).rotate_left(3))
        } else {
            RValue::new_float(num).pack()
        }
    }

    pub fn complex(r: Value, i: Value) -> Self {
        RValue::new_complex(r, i).pack()
    }

    pub fn string_from_rstring(rs: RString) -> Self {
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

    pub fn array_empty() -> Value {
        Value::array_from(vec![])
    }

    pub fn array_from(ary: Vec<Value>) -> Value {
        RValue::new_array(ArrayInfo::new(ary)).pack()
    }

    pub fn array_from_with_class(ary: Vec<Value>, class: Module) -> Value {
        RValue::new_array_with_class(ArrayInfo::new(ary), class).pack()
    }

    pub fn splat(val: Value) -> Self {
        RValue::new_splat(val).pack()
    }

    pub fn hash_from(hash: HashInfo) -> Self {
        RValue::new_hash(hash).pack()
    }

    pub fn hash_from_map(hash: FxIndexMap<HashKey, Value>) -> Self {
        RValue::new_hash(HashInfo::new(hash)).pack()
    }

    pub fn regexp(regexp: RegexpInfo) -> Self {
        RValue::new_regexp(regexp).pack()
    }

    pub fn regexp_from(vm: &mut VM, string: &str) -> Result<Self, RubyError> {
        Ok(RValue::new_regexp(vm.regexp_from_string(string)?).pack())
    }

    pub fn procobj(context: ContextRef) -> Self {
        RValue::new_proc(ProcInfo::new(context)).pack()
    }

    pub fn method(name: IdentId, receiver: Value, method: MethodId, owner: Module) -> Self {
        RValue::new_method(MethodObjInfo::new(name, receiver, method, owner)).pack()
    }

    pub fn unbound_method(name: IdentId, method: MethodId, owner: Module) -> Self {
        RValue::new_unbound_method(MethodObjInfo::new_unbound(name, method, owner)).pack()
    }

    pub fn fiber(parent_vm: &mut VM, context: ContextRef) -> Self {
        let new_fiber = parent_vm.create_fiber();
        RValue::new_fiber(new_fiber, context).pack()
    }

    pub fn enumerator(fiber: Box<FiberContext>) -> Self {
        RValue::new_enumerator(fiber).pack()
    }

    pub fn time(time_class: Module, time: TimeInfo) -> Self {
        RValue::new_time(time_class, time).pack()
    }

    pub fn exception(exception_class: Module, err: RubyError) -> Self {
        RValue::new_exception(exception_class, err).pack()
    }
}

impl Value {
    pub fn to_ordering(&self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        match self.as_integer() {
            Some(1) => Ordering::Greater,
            Some(0) => Ordering::Equal,
            Some(-1) => Ordering::Less,
            _ => panic!("Illegal ordering value."),
        }
    }
}

impl Value {
    /// Get singleton class object of `self`.
    ///
    /// When `self` already has a singleton class, simply return it.  
    /// If not, generate a new singleton class object.  
    /// Return None when `self` was a primitive (i.e. Integer, Symbol, Float) which can not have a singleton class.
    pub fn get_singleton_class(self) -> Result<Module, RubyError> {
        match self.clone().as_mut_rvalue() {
            Some(oref) => {
                let class = oref.class();
                if class.is_singleton() {
                    Ok(class)
                } else {
                    let singleton = match &oref.kind {
                        ObjKind::Module(cinfo) => {
                            let superclass = match cinfo.superclass() {
                                None => None,
                                Some(superclass) => Some(superclass.get_singleton_class()),
                            };
                            Module::singleton_class_from(superclass, self)
                        }
                        ObjKind::Invalid => {
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
    pub fn get_method_or_nomethod(self, method_id: IdentId) -> Result<MethodId, RubyError> {
        let rec_class = self.get_class_for_method();
        rec_class.get_method_or_nomethod(method_id)
    }
}

impl Value {
    /// Convert `self` to boolean value.
    pub fn to_bool(&self) -> bool {
        !self.is_nil() && !self.is_false_val() && !self.is_uninitialized()
    }

    /// Convert `self` to `Option<Real>`.
    /// If `self` was not a integer nor a float, return `None`.
    pub fn to_real(&self) -> Option<Real> {
        match self.unpack() {
            RV::Integer(i) => Some(Real::Integer(i)),
            RV::Float(f) => Some(Real::Float(f)),
            _ => None,
        }
    }

    /// Convert `self` to `Option<(real:Real, imaginary:Real)>`.
    /// If `self` was not a integer nor a float nor a complex, return `None`.
    pub fn to_complex(&self) -> Option<(Real, Real)> {
        match self.unpack() {
            RV::Integer(i) => Some((Real::Integer(i), Real::Integer(0))),
            RV::Float(f) => Some((Real::Float(f), Real::Integer(0))),
            RV::Object(obj) => match obj.kind {
                ObjKind::Complex { r, i } => Some((r.to_real().unwrap(), i.to_real().unwrap())),
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
        let _globals = GlobalsRef::new_globals();
        let expect = RV::True;
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_bool2() {
        let _globals = GlobalsRef::new_globals();
        let expect = RV::False;
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_nil() {
        let _globals = GlobalsRef::new_globals();
        let expect = RV::Nil;
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_uninit() {
        let _globals = GlobalsRef::new_globals();
        let expect = RV::Uninitialized;
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer1() {
        let _globals = GlobalsRef::new_globals();
        let expect = RV::Integer(12054);
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer11() {
        let _globals = GlobalsRef::new_globals();
        let expect_ary = [
            12054,
            -58993,
            0x8000_0000_0000_0000 as u64 as i64,
            0x4000_0000_0000_0000 as u64 as i64,
            0x7fff_ffff_ffff_ffff as u64 as i64,
        ];
        for expect in expect_ary.iter() {
            let got = match RV::Integer(*expect).pack().as_integer() {
                Some(int) => int,
                None => panic!("Expect:{:?} Got:Invalid RValue", *expect),
            };
            if *expect != got {
                panic!("Expect:{:?} Got:{:?}", *expect, got)
            };
        }
    }

    #[test]
    fn pack_integer2() {
        let _globals = GlobalsRef::new_globals();
        let expect = RV::Integer(-58993);
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer3() {
        let _globals = GlobalsRef::new_globals();
        let expect = RV::Integer(0x8000_0000_0000_0000 as u64 as i64);
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer4() {
        let _globals = GlobalsRef::new_globals();
        let expect = RV::Integer(0x4000_0000_0000_0000 as u64 as i64);
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer5() {
        let _globals = GlobalsRef::new_globals();
        let expect = RV::Integer(0x7fff_ffff_ffff_ffff as u64 as i64);
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float0() {
        let _globals = GlobalsRef::new_globals();
        let expect = RV::Float(0.0);
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float1() {
        let _globals = GlobalsRef::new_globals();
        let expect = RV::Float(100.0);
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float2() {
        let _globals = GlobalsRef::new_globals();
        let expect = RV::Float(13859.628547);
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float3() {
        let _globals = GlobalsRef::new_globals();
        let expect = RV::Float(-5282.2541156);
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_range() {
        GlobalsRef::new_globals();
        let from = RV::Integer(7).pack();
        let to = RV::Integer(36).pack();
        let expect = Value::range(from, to, true);
        let got = expect.unpack().pack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_class() {
        GlobalsRef::new_globals();
        let expect: Value = Module::class_under(None).into();
        let got = expect.unpack().pack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_instance() {
        GlobalsRef::new_globals();
        let expect = Value::ordinary_object(BuiltinClass::class());
        let got = expect.unpack().pack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_symbol() {
        GlobalsRef::new_globals();
        let expect = RV::Symbol(IdentId::from(12345));
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }
}
