use crate::*;
use std::borrow::Cow;
use std::sync::mpsc::{Receiver, SyncSender};

const FALSE_VALUE: u64 = 0x00;
const UNINITIALIZED: u64 = 0x04;
const NIL_VALUE: u64 = 0x08;
const TAG_SYMBOL: u64 = 0x0c;
const TRUE_VALUE: u64 = 0x14;
const MASK1: u64 = !(0b0110u64 << 60);
const MASK2: u64 = 0b0100u64 << 60;

const ZERO: u64 = (0b1000 << 60) | 0b10;

#[derive(Debug, Clone, PartialEq)]
pub enum RV<'a> {
    Uninitialized,
    Nil,
    Bool(bool),
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
            RV::Bool(true) => Value::true_val(),
            RV::Bool(false) => Value::false_val(),
            RV::Integer(num) => Value::integer(*num),
            RV::Float(num) => Value::float(*num),
            RV::Symbol(id) => Value::symbol(*id),
            RV::Object(info) => Value(info.id()),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Value(u64);

impl std::ops::Deref for Value {
    type Target = u64;
    fn deref(&self) -> &u64 {
        &self.0
    }
}

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self.as_rvalue() {
            None => self.0.hash(state),
            Some(lhs) => match &lhs.kind {
                ObjKind::Invalid => unreachable!("Invalid rvalue. (maybe GC problem) {:?}", lhs),
                ObjKind::Integer(lhs) => lhs.hash(state),
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
                *r1 == *r2 && *i1 == *i2
            }
            (ObjKind::String(lhs), ObjKind::String(rhs)) => lhs.as_bytes() == rhs.as_bytes(),
            (ObjKind::Array(lhs), ObjKind::Array(rhs)) => lhs.elements == rhs.elements,
            (ObjKind::Range(lhs), ObjKind::Range(rhs)) => lhs == rhs,
            (ObjKind::Hash(lhs), ObjKind::Hash(rhs)) => **lhs == **rhs,
            (ObjKind::Regexp(lhs), ObjKind::Regexp(rhs)) => *lhs == *rhs,
            (ObjKind::Time(lhs), ObjKind::Time(rhs)) => *lhs == *rhs,
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
        if !self.is_packed_value() {
            write!(f, "{:?}", self.rvalue().kind)
        } else if self.is_packed_fixnum() {
            write!(f, "{}", self.as_packed_fixnum())
        } else if self.is_packed_num() {
            write!(f, "{}", self.as_packed_flonum())
        } else if self.is_packed_symbol() {
            write!(f, ":\"{:?}\"", self.as_packed_symbol())
        } else {
            match self.0 {
                NIL_VALUE => write!(f, "Nil"),
                TRUE_VALUE => write!(f, "True"),
                FALSE_VALUE => write!(f, "False"),
                UNINITIALIZED => write!(f, "[Uninitialized]"),
                _ => write!(f, "[ILLEGAL]"),
            }
        }
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
            match self.0 {
                NIL_VALUE => RV::Nil,
                TRUE_VALUE => RV::Bool(true),
                FALSE_VALUE => RV::Bool(false),
                UNINITIALIZED => RV::Uninitialized,
                _ => unreachable!("Illegal packed value."),
            }
        }
    }

    pub fn id(&self) -> u64 {
        self.0
    }

    pub fn from(id: u64) -> Self {
        Value(id)
    }

    pub fn from_ptr<T: GC>(ptr: *mut GCBox<T>) -> Self {
        Value(ptr as u64)
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
        unsafe { &*(self.0 as *const GCBox<RValue>) }
    }

    pub fn rvalue(&self) -> &RValue {
        unsafe { &*(self.0 as *const GCBox<RValue>) }.inner()
    }

    pub fn rvalue_mut(&self) -> &mut RValue {
        unsafe { &mut *(self.0 as *mut GCBox<RValue>) }.inner_mut()
    }
}

impl Value {
    pub fn val_to_s(&self, vm: &mut VM) -> Result<Cow<str>, RubyError> {
        let s = match self.unpack() {
            RV::Uninitialized => Cow::from("[Uninitialized]"),
            RV::Nil => Cow::from(""),
            RV::Bool(b) => match b {
                true => Cow::from("true"),
                false => Cow::from("false"),
            },
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
    pub fn set_class(&mut self, class: Value) {
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
    pub fn get_class_for_method(&self) -> Value {
        match self.as_rvalue() {
            None => {
                if self.is_packed_fixnum() {
                    BuiltinClass::integer()
                } else if self.is_packed_num() {
                    BuiltinClass::float()
                } else if self.is_packed_symbol() {
                    BuiltinClass::object()
                } else {
                    BuiltinClass::object()
                }
            }
            Some(info) => info.class(),
        }
    }

    /// Get class of `self`.
    /// If a direct class of `self` was a singleton class, returns a class of the singleton class.
    pub fn get_class(&self) -> Value {
        match self.as_rvalue() {
            None => {
                if self.is_packed_fixnum() {
                    BuiltinClass::integer()
                } else if self.is_packed_num() {
                    BuiltinClass::float()
                } else if self.is_packed_symbol() {
                    BuiltinClass::object()
                } else {
                    BuiltinClass::object()
                }
            }
            Some(info) => info.search_class(),
        }
    }

    pub fn get_class_name(&self) -> String {
        match self.unpack() {
            RV::Uninitialized => "[Uninitialized]".to_string(),
            RV::Nil => "NilClass".to_string(),
            RV::Bool(true) => "TrueClass".to_string(),
            RV::Bool(false) => "FalseClass".to_string(),
            RV::Integer(_) => "Integer".to_string(),
            RV::Float(_) => "Float".to_string(),
            RV::Symbol(_) => "Symbol".to_string(),
            RV::Object(oref) => match oref.kind {
                ObjKind::Invalid => panic!("Invalid rvalue. (maybe GC problem) {:?}", *oref),
                ObjKind::String(_) => "String".to_string(),
                ObjKind::Array(_) => "Array".to_string(),
                ObjKind::Range(_) => "Range".to_string(),
                ObjKind::Splat(_) => "[Splat]".to_string(),
                ObjKind::Hash(_) => "Hash".to_string(),
                ObjKind::Regexp(_) => "Regexp".to_string(),
                ObjKind::Class(_) => "Class".to_string(),
                ObjKind::Module(_) => "Module".to_string(),
                ObjKind::Proc(_) => "Proc".to_string(),
                ObjKind::Method(_) => "Method".to_string(),
                ObjKind::Ordinary => oref.class_name().to_string(),
                ObjKind::Integer(_) => "Integer".to_string(),
                ObjKind::Float(_) => "Float".to_string(),
                ObjKind::Complex { .. } => "Complex".to_string(),
                ObjKind::Fiber(_) => "Fiber".to_string(),
                ObjKind::Enumerator(_) => "Enumerator".to_string(),
                ObjKind::Time(_) => "Time".to_string(),
            },
        }
    }

    /// Get superclass of `self`.
    ///
    /// If `self` was a module/class which has no superclass or `self` was not a module/class, return None.
    pub fn superclass(&self) -> Option<Value> {
        match self.if_mod_class() {
            Some(cinfo) => Some(cinfo.superclass()),
            None => None,
        }
    }

    /// Get an upper module/class of `self`.
    ///
    /// If `self` has no upper module/class, return None.
    /// Panic if `self` is not Class/Module.
    pub fn upper(&self) -> Option<Value> {
        match self.if_mod_class() {
            Some(class) => {
                let superclass = class.upper();
                if superclass.is_nil() {
                    None
                } else {
                    Some(superclass)
                }
            }
            None => unreachable!("upper(): Not a Class / Module."),
        }
    }

    /// Examine whether `self` is a singleton class.
    /// Panic if `self` is not a class object.
    pub fn is_singleton(&self) -> bool {
        self.as_module().is_singleton()
    }

    /// Examine whether `self` has a singleton class.
    /// Panic if `self` is not a class object.
    pub fn has_singleton(&self) -> bool {
        self.get_class_for_method().is_singleton()
    }

    pub fn set_var(&mut self, id: IdentId, val: Value) -> Option<Value> {
        self.rvalue_mut().set_var(id, val)
    }

    pub fn set_var_by_str(&mut self, name: &str, val: Value) {
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

    /// Find method `id` from method tables of `self` class and all of its superclasses including their included modules.
    /// Return None if no method found.
    pub fn get_instance_method(&self, id: IdentId) -> Option<MethodRef> {
        let cref = self.as_module();
        cref.method_table().get(&id).cloned()
    }

    pub fn add_builtin_class_method(&mut self, name: &str, func: BuiltinFunc) {
        let mut singleton = self.get_singleton_class().unwrap();
        let classinfo = singleton.as_mut_class();
        classinfo.add_builtin_method_by_str(name, func);
    }

    pub fn add_builtin_method_by_str(&mut self, name: &str, func: BuiltinFunc) {
        let name = IdentId::get_id(name);
        self.as_mut_module().add_builtin_method(name, func);
    }

    /// Add module function to `self`.
    /// `self` must be Module or Class.
    pub fn add_builtin_module_func(&mut self, name: &str, func: BuiltinFunc) {
        let classref = self.if_mut_mod_class().unwrap();
        classref.add_builtin_method_by_str(name, func);
        let mut singleton = self.get_singleton_class().unwrap();
        let classref = singleton.as_mut_class();
        classref.add_builtin_method_by_str(name, func);
    }
}

impl Value {
    pub fn is_packed_fixnum(&self) -> bool {
        self.0 & 0b1 == 1
    }

    pub fn is_packed_flonum(&self) -> bool {
        self.0 & 0b11 == 2
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
            _ => Err(VM::error_argument(format!(
                "{} must be an Integer. (given:{})",
                msg.into(),
                self.get_class_name()
            ))),
        }
    }

    pub fn expect_flonum(&self, msg: &str) -> Result<f64, RubyError> {
        match self.as_float() {
            Some(f) => Ok(f),
            None => Err(VM::error_argument(format!(
                "{} must be Float. (given:{})",
                msg,
                self.get_class_name()
            ))),
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
            None => Err(VM::error_type(format!(
                "{} must be String. (given:{:?})",
                msg, *self
            ))),
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
            None => Err(VM::error_type(format!(
                "{} must be String. (given:{:?})",
                msg, val
            ))),
        }
    }

    pub fn expect_string_or_symbol(&mut self, msg: &str) -> Result<IdentId, RubyError> {
        let val = *self;
        if let Some(id) = val.as_symbol() {
            return Ok(id);
        };
        let str = self
            .as_mut_rstring()
            .ok_or_else(|| {
                VM::error_type(format!(
                    "{} must be String or Symbol. (given:{:?})",
                    msg, val
                ))
            })?
            .as_string()?;
        Ok(IdentId::get_id(str))
    }

    /// Take &ClassInfo from `self`.
    /// Panic if `self` is not Class.
    pub fn as_class(&self) -> &ClassInfo {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Class(cinfo) => cinfo,
                _ => panic!(format!("as_class(): Not a class object. {:?}", *self)),
            },
            None => panic!(format!("as_class(): Not a class object. {:?}", *self)),
        }
    }

    /// Take &mut ClassInfo from `self`.
    /// Panic if `self` is not a Class.
    pub fn as_mut_class(&mut self) -> &mut ClassInfo {
        let self_ = *self;
        match self.if_mut_class() {
            Some(cinfo) => cinfo,
            None => panic!(format!("as_mut_class(): Not a class object. {:?}", self_)),
        }
    }

    /// Check whether `self` is a Class.
    pub fn is_class(&self) -> bool {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Class(_) => true,
                _ => false,
            },
            None => false,
        }
    }

    pub fn if_mut_class(&mut self) -> Option<&mut ClassInfo> {
        match self.as_mut_rvalue() {
            Some(oref) => match &mut oref.kind {
                ObjKind::Class(cinfo) => Some(cinfo),
                _ => None,
            },
            None => None,
        }
    }

    /// Take &ClassInfo from `self`.
    /// Panic if `self` is not a Class nor Module.
    pub fn as_module(&self) -> &ClassInfo {
        match self.if_mod_class() {
            Some(cinfo) => cinfo,
            None => panic!(format!(
                "as_module(): Not a class or module object. {:?}",
                self
            )),
        }
    }

    /// Take &mut ClassInfo from `self`.
    /// Panic if `self` is not a Class nor Module.
    pub fn as_mut_module(&mut self) -> &mut ClassInfo {
        let self_ = *self;
        match self.if_mut_mod_class() {
            Some(cinfo) => cinfo,
            None => panic!(format!(
                "as_mut_module(): Not a class or module object. {:?}",
                self_
            )),
        }
    }

    pub fn if_mod_class(&self) -> Option<&ClassInfo> {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Class(cinfo) | ObjKind::Module(cinfo) => Some(cinfo),
                _ => None,
            },
            None => None,
        }
    }

    pub fn if_mut_mod_class(&mut self) -> Option<&mut ClassInfo> {
        match self.as_mut_rvalue() {
            Some(oref) => match &mut oref.kind {
                ObjKind::Class(cinfo) | ObjKind::Module(cinfo) => Some(cinfo),
                _ => None,
            },
            None => None,
        }
    }

    /// Check whether `self` is a Module.
    pub fn is_module(&self) -> bool {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Module(_) => true,
                _ => false,
            },
            None => false,
        }
    }

    pub fn if_mut_module(&mut self) -> Option<&mut ClassInfo> {
        match self.as_mut_rvalue() {
            Some(oref) => match &mut oref.kind {
                ObjKind::Module(cinfo) => Some(cinfo),
                _ => None,
            },
            None => None,
        }
    }

    /// Returns `ClassRef` if `self` is a Class.
    /// When `self` is not a Class, returns `TypeError`.
    pub fn expect_class(&mut self, vm: &mut VM, msg: &str) -> Result<&mut ClassInfo, RubyError> {
        let self_ = self.clone();
        if let Some(cinfo) = self.if_mut_class() {
            Ok(cinfo)
        } else {
            let val = vm.val_inspect(self_)?;
            Err(VM::error_type(format!(
                "{} must be Class. (given:{})",
                msg, val
            )))
        }
    }

    /// Returns `ClassRef` if `self` is a Module.
    /// When `self` is not a Class, returns `TypeError`.
    pub fn expect_module(&mut self, vm: &mut VM, msg: &str) -> Result<&mut ClassInfo, RubyError> {
        let self_ = self.clone();
        if let Some(cinfo) = self.if_mut_module() {
            Ok(cinfo)
        } else {
            let val = vm.val_inspect(self_)?;
            Err(VM::error_type(format!(
                "{} must be Module. (given:{})",
                msg, val
            )))
        }
    }

    /// Returns `ClassRef` if `self` is a Module / Class.
    /// When `self` is not a Module, returns `TypeError`.
    pub fn expect_mod_class(&mut self, vm: &mut VM) -> Result<&mut ClassInfo, RubyError> {
        let self_ = self.clone();
        if let Some(cinfo) = self.if_mut_mod_class() {
            Ok(cinfo)
        } else {
            let val = vm.val_inspect(self_)?;
            Err(VM::error_type(format!(
                "Must be Module or Class. (given:{})",
                val
            )))
        }
    }

    pub fn generate_included(&self) -> Value {
        let origin = if self.as_module().is_included() {
            self.as_module().origin()
        } else {
            *self
        };
        let mut imodule = self.dup();
        imodule.as_mut_module().set_include(origin);
        imodule
    }

    pub fn as_array(&self) -> Option<&ArrayInfo> {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Array(aref) => Some(aref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_mut_array(&mut self) -> Option<&mut ArrayInfo> {
        match self.as_mut_rvalue() {
            Some(oref) => match &mut oref.kind {
                ObjKind::Array(aref) => Some(aref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn expect_array(&mut self, msg: &str) -> Result<&mut ArrayInfo, RubyError> {
        let val = *self;
        match self.as_mut_array() {
            Some(ary) => Ok(ary),
            None => Err(VM::error_type(format!(
                "{} must be Array. (given:{:?})",
                msg, val
            ))),
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
            None => Err(VM::error_type(format!(
                "{} must be Hash. (given:{:?})",
                msg, val
            ))),
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
            None => Err(VM::error_argument("Must be Proc.")),
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
    pub fn as_enumerator(&mut self) -> Option<&mut FiberInfo> {
        match self.as_mut_rvalue() {
            Some(oref) => match &mut oref.kind {
                ObjKind::Enumerator(info) => Some(info.as_mut()),
                _ => None,
            },
            None => None,
        }
    }

    pub fn expect_enumerator(&mut self, error_msg: &str) -> Result<&mut FiberInfo, RubyError> {
        match self.as_enumerator() {
            Some(e) => Ok(e),
            None => Err(VM::error_argument(error_msg)),
        }
    }

    pub fn expect_fiber(&mut self, error_msg: &str) -> Result<&mut FiberInfo, RubyError> {
        match self.as_mut_rvalue() {
            Some(oref) => match &mut oref.kind {
                ObjKind::Fiber(f) => Ok(f.as_mut()),
                _ => Err(VM::error_argument(error_msg)),
            },
            None => Err(VM::error_argument(error_msg)),
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
        (self.0 as i64) >> 1
    }

    pub fn as_packed_flonum(&self) -> f64 {
        if self.0 == ZERO {
            return 0.0;
        }
        let bit = 0b10 - ((self.0 >> 63) & 0b1);
        let num = ((self.0 & !(0b0011u64)) | bit).rotate_right(3);
        //eprintln!("after  unpack:{:064b}", num);
        f64::from_bits(num)
    }

    pub fn as_packed_symbol(&self) -> IdentId {
        IdentId::from((self.0 >> 32) as u32)
    }

    pub const fn uninitialized() -> Self {
        Value(UNINITIALIZED)
    }

    pub const fn nil() -> Self {
        Value(NIL_VALUE)
    }

    pub const fn true_val() -> Self {
        Value(TRUE_VALUE)
    }

    pub const fn false_val() -> Self {
        Value(FALSE_VALUE)
    }

    pub fn bool(b: bool) -> Self {
        if b {
            Value(TRUE_VALUE)
        } else {
            Value(FALSE_VALUE)
        }
    }

    pub fn integer(num: i64) -> Self {
        let top = (num as u64) >> 62 ^ (num as u64) >> 63;
        if top & 0b1 == 0 {
            Value((num << 1) as u64 | 0b1)
        } else {
            RValue::new_integer(num).pack()
        }
    }

    pub fn float(num: f64) -> Self {
        if num == 0.0 {
            return Value(ZERO);
        }
        let unum = f64::to_bits(num);
        let exp = ((unum >> 60) & 0b111) + 1;
        if (exp & 0b0110) == 0b0100 {
            Value((unum & MASK1 | MASK2).rotate_left(3))
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
        Value((id as u64) << 32 | TAG_SYMBOL)
    }

    pub fn range(start: Value, end: Value, exclude: bool) -> Self {
        let info = RangeInfo::new(start, end, exclude);
        RValue::new_range(info).pack()
    }

    pub fn bootstrap_class(cinfo: ClassInfo) -> Self {
        RValue::new_bootstrap(cinfo).pack()
    }

    pub fn ordinary_object(class: Value) -> Self {
        RValue::new_ordinary(class).pack()
    }

    pub fn class(cinfo: ClassInfo) -> Self {
        RValue::new_class(cinfo).pack()
    }

    pub fn class_from(superclass: impl Into<Option<Value>>) -> Self {
        RValue::new_class(ClassInfo::from(superclass)).pack()
    }

    pub fn singleton_class_from(superclass: impl Into<Option<Value>>) -> Self {
        RValue::new_class(ClassInfo::singleton_from(superclass)).pack()
    }

    pub fn module() -> Self {
        RValue::new_module(ClassInfo::from(None)).pack()
    }

    pub fn array_from(ary: Vec<Value>) -> Self {
        RValue::new_array(ArrayInfo::new(ary)).pack()
    }

    pub fn splat(val: Value) -> Self {
        RValue::new_splat(val).pack()
    }

    pub fn hash_from(hash: HashInfo) -> Self {
        RValue::new_hash(hash).pack()
    }

    pub fn hash_from_map(hash: FxHashMap<HashKey, Value>) -> Self {
        RValue::new_hash(HashInfo::new(hash)).pack()
    }

    pub fn regexp(regexp: RegexpInfo) -> Self {
        RValue::new_regexp(regexp).pack()
    }

    pub fn procobj(context: ContextRef) -> Self {
        RValue::new_proc(ProcInfo::new(context)).pack()
    }

    pub fn method(name: IdentId, receiver: Value, method: MethodRef) -> Self {
        RValue::new_method(MethodObjInfo::new(name, receiver, method)).pack()
    }

    pub fn fiber(
        vm: VM,
        context: ContextRef,
        rec: Receiver<VMResult>,
        tx: SyncSender<FiberMsg>,
    ) -> Self {
        RValue::new_fiber(vm, context, rec, tx).pack()
    }

    pub fn enumerator(fiber: FiberInfo) -> Self {
        RValue::new_enumerator(fiber).pack()
    }

    pub fn time(time_class: Value, time: TimeInfo) -> Self {
        RValue::new_time(time_class, time).pack()
    }
}

impl Value {
    pub fn equal_i(self, other: i32) -> bool {
        if self.is_packed_fixnum() {
            self.as_packed_fixnum() == other as i64
        } else if self.is_packed_num() {
            self.as_packed_flonum() == other as f64
        } else {
            false
        }
    }

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
    /// TODO: nil=>NilClass, true=>TrueClass, false=>FalseClass
    pub fn get_singleton_class(&mut self) -> Option<Value> {
        match self.as_mut_rvalue() {
            Some(oref) => {
                let class = oref.class();
                if class.is_singleton() {
                    Some(class)
                } else {
                    let singleton = match &oref.kind {
                        ObjKind::Class(cinfo) | ObjKind::Module(cinfo) => {
                            let mut superclass = cinfo.superclass();
                            let mut singleton = if superclass.is_nil() {
                                Value::singleton_class_from(None)
                            } else {
                                Value::singleton_class_from(superclass.get_singleton_class()?)
                            };
                            singleton.set_class(class);
                            singleton
                        }
                        ObjKind::Invalid => {
                            panic!("Invalid rvalue. (maybe GC problem) {:?}", *oref)
                        }
                        _ => Value::singleton_class_from(class),
                    };
                    oref.set_class(singleton);
                    Some(singleton)
                }
            }
            _ => None,
        }
    }

    /// Get method for a receiver class (`self`) and method (IdentId).
    pub fn get_method(&self, method: IdentId) -> Option<MethodRef> {
        let mut temp_class = *self;
        let mut singleton_flag = self.is_singleton();
        loop {
            match temp_class.get_instance_method(method) {
                Some(method) => {
                    return Some(method);
                }
                None => match temp_class.upper() {
                    Some(superclass) => temp_class = superclass,
                    None => {
                        if singleton_flag {
                            singleton_flag = false;
                            temp_class = self.rvalue().class();
                        } else {
                            return None;
                        }
                    }
                },
            };
        }
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
        let expect = RV::Bool(true);
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_bool2() {
        let _globals = GlobalsRef::new_globals();
        let expect = RV::Bool(false);
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
        let expect = Value::class_from(None);
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
