use crate::*;

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
            RV::Integer(num) => Value::fixnum(*num),
            RV::Float(num) => Value::flonum(*num),
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
                ObjKind::Invalid => panic!("Invalid rvalue. (maybe GC problem) {:?}", lhs),
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
            (ObjKind::Integer(lhs), ObjKind::Float(rhs)) => *lhs as f64 == *rhs,
            (ObjKind::Float(lhs), ObjKind::Integer(rhs)) => *lhs == *rhs as f64,
            (ObjKind::String(lhs), ObjKind::String(rhs)) => *lhs == *rhs,
            (ObjKind::Array(lhs), ObjKind::Array(rhs)) => lhs.elements == rhs.elements,
            (ObjKind::Range(lhs), ObjKind::Range(rhs)) => {
                lhs.start == rhs.start && lhs.end == rhs.end && lhs.exclude == rhs.exclude
            }
            (ObjKind::Hash(lhs), ObjKind::Hash(rhs)) => match (&**lhs, &**rhs) {
                (HashInfo::Map(lhs), HashInfo::Map(rhs)) => *lhs == *rhs,
                (HashInfo::IdentMap(lhs), HashInfo::IdentMap(rhs)) => *lhs == *rhs,
                _ => false,
            },
            (ObjKind::Invalid, _) => {
                panic!("Invalid rvalue. (maybe GC problem) {:?}", self.rvalue())
            }
            (_, ObjKind::Invalid) => {
                panic!("Invalid rvalue. (maybe GC problem) {:?}", other.rvalue())
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
            write!(f, ":\"{}\"", IdentId::get_name(self.as_packed_symbol()))
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

    pub fn as_gcbox(&self) -> Option<&GCBox<RValue>> {
        if self.is_packed_value() {
            None
        } else {
            Some(self.gcbox())
        }
    }

    /// Get reference of RValue from Value.
    /// This method works only if `self` is not a packed value.
    pub fn as_rvalue(&self) -> Option<&RValue> {
        if self.is_packed_value() {
            None
        } else {
            Some(self.rvalue())
        }
    }

    /// Get mutable reference of RValue from Value.
    /// This method works only if `self` is not a packed value.
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

    pub fn get_class_object_for_method(&self, globals: &Globals) -> Value {
        match self.as_rvalue() {
            None => {
                if self.is_packed_fixnum() {
                    globals.builtins.integer
                } else if self.is_packed_num() {
                    globals.builtins.float
                } else if self.is_packed_symbol() {
                    globals.builtins.object
                } else {
                    globals.builtins.object
                }
            }
            Some(info) => match &info.kind {
                ObjKind::Invalid => panic!("Invalid rvalue. (maybe GC problem) {:?}", info),
                ObjKind::Integer(_) => globals.builtins.integer,
                ObjKind::Float(_) => globals.builtins.float,
                _ => info.class(),
            },
        }
    }

    pub fn get_class_object(&self, globals: &Globals) -> Value {
        match self.unpack() {
            RV::Integer(_) => globals.builtins.integer,
            RV::Float(_) => globals.builtins.float,
            RV::Object(info) => info.search_class(),
            _ => globals.builtins.object,
        }
    }

    pub fn superclass(&self) -> Option<Value> {
        match self.as_module() {
            Some(class) => {
                let superclass = class.superclass;
                if superclass.is_nil() {
                    None
                } else {
                    Some(superclass)
                }
            }
            None => None,
        }
    }

    pub fn set_var(&mut self, id: IdentId, val: Value) {
        self.rvalue_mut().set_var(id, val);
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

    pub fn get_instance_method(&self, id: IdentId) -> Option<MethodRef> {
        let cref = self.as_module().unwrap();
        match cref.method_table.get(&id) {
            Some(method) => Some(*method),
            None => {
                for v in &cref.include {
                    match v.get_instance_method(id) {
                        Some(method) => return Some(method),
                        None => {}
                    }
                }
                None
            }
        }
    }
}

impl Value {
    pub fn is_packed_fixnum(&self) -> bool {
        self.0 & 0b1 == 1
    }

    pub fn is_packed_flonum(&self) -> bool {
        self.0 & 0b10 == 2
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

    pub fn expect_integer(&self, vm: &VM, msg: impl Into<String>) -> Result<i64, RubyError> {
        match self.as_fixnum() {
            Some(i) => Ok(i),
            None => {
                let val = vm.globals.get_class_name(*self);
                Err(vm.error_argument(format!("{} must be an Integer. {}", msg.into(), val)))
            }
        }
    }

    pub fn as_flonum(&self) -> Option<f64> {
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
                ObjKind::String(RString::Str(s)) => Some(s.as_bytes()),
                ObjKind::String(RString::Bytes(b)) => Some(b),
                _ => None,
            },
            None => None,
        }
    }

    pub fn expect_bytes(&self, vm: &mut VM, msg: &str) -> Result<&[u8], RubyError> {
        let rstring = self.as_rstring().ok_or_else(|| {
            let inspect = vm.val_inspect(self.clone());
            vm.error_type(format!("{} must be String. (given:{})", msg, inspect))
        })?;
        Ok(rstring.as_bytes())
    }

    pub fn as_string(&self) -> Option<&String> {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::String(RString::Str(s)) => Some(s),
                _ => None,
            },
            None => None,
        }
    }

    pub fn expect_string(&mut self, vm: &mut VM, msg: &str) -> Result<&String, RubyError> {
        let val = self.clone();
        let rstring = self.as_mut_rstring().ok_or_else(|| {
            let inspect = vm.val_inspect(val);
            vm.error_type(format!("{} must be String. (given:{})", msg, inspect))
        })?;
        rstring.as_string(vm)
    }

    pub fn as_class(&self) -> ClassRef {
        match self.is_class() {
            Some(class) => class,
            None => panic!(format!("Class is not class. {:?}", *self)),
        }
    }

    pub fn is_class(&self) -> Option<ClassRef> {
        match self.as_rvalue() {
            Some(oref) => match oref.kind {
                ObjKind::Class(cref) => Some(cref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_module(&self) -> Option<ClassRef> {
        match self.as_rvalue() {
            Some(oref) => match oref.kind {
                ObjKind::Class(cref) | ObjKind::Module(cref) => Some(cref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn is_module(&self) -> Option<ClassRef> {
        match self.as_rvalue() {
            Some(oref) => match oref.kind {
                ObjKind::Module(cref) => Some(cref),
                _ => None,
            },
            None => None,
        }
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

    pub fn expect_array(&mut self, vm: &mut VM, msg: &str) -> Result<&mut ArrayInfo, RubyError> {
        let val = self.clone();
        match self.as_mut_array() {
            Some(ary) => Ok(ary),
            None => Err(vm.error_type(format!("{} must be Array. (given:{:?})", msg, val))),
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

    pub fn expect_hash(&self, vm: &mut VM, msg: &str) -> Result<&HashInfo, RubyError> {
        let val = self.clone();
        self.as_hash().ok_or_else(|| {
            let inspect = vm.val_inspect(val);
            vm.error_type(format!("{} must be Hash. (given:{})", msg, inspect))
        })
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

    pub fn as_method(&self) -> Option<&MethodObjInfo> {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::Method(mref) => Some(mref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_fiber(&self) -> Option<FiberRef> {
        match self.as_rvalue() {
            Some(oref) => match oref.kind {
                ObjKind::Fiber(info) => Some(info),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_enumerator(&mut self) -> Option<&mut EnumInfo> {
        match self.as_mut_rvalue() {
            Some(oref) => match &mut oref.kind {
                ObjKind::Enumerator(eref) => Some(eref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn expect_enumerator(
        &mut self,
        vm: &mut VM,
        error_msg: &str,
    ) -> Result<&mut EnumInfo, RubyError> {
        match self.as_enumerator() {
            Some(e) => Ok(e),
            None => Err(vm.error_argument(error_msg)),
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
        let num = if self.0 & (0b1000u64 << 60) == 0 {
            self.0 //(self.0 & !(0b0011u64)) | 0b10
        } else {
            (self.0 & !(0b0011u64)) | 0b01
        }
        .rotate_right(3);
        //eprintln!("after  unpack:{:064b}", num);
        f64::from_bits(num)
    }

    pub fn as_packed_symbol(&self) -> IdentId {
        IdentId::from((self.0 >> 32) as u32)
    }

    pub fn uninitialized() -> Self {
        Value(UNINITIALIZED)
    }

    pub fn nil() -> Self {
        Value(NIL_VALUE)
    }

    pub fn true_val() -> Self {
        Value(TRUE_VALUE)
    }

    pub fn false_val() -> Self {
        Value(FALSE_VALUE)
    }

    pub fn bool(b: bool) -> Self {
        if b {
            Value(TRUE_VALUE)
        } else {
            Value(FALSE_VALUE)
        }
    }

    pub fn fixnum(num: i64) -> Self {
        let top = (num as u64) >> 62 ^ (num as u64) >> 63;
        if top & 0b1 == 0 {
            Value((num << 1) as u64 | 0b1)
        } else {
            RValue::new_fixnum(num).pack()
        }
    }

    pub fn flonum(num: f64) -> Self {
        if num == 0.0 {
            return Value(ZERO);
        }
        let unum = f64::to_bits(num);
        let exp = (unum >> 60) & 0b111;
        if exp == 4 || exp == 3 {
            Value((unum & MASK1 | MASK2).rotate_left(3))
        } else {
            RValue::new_flonum(num).pack()
        }
    }

    pub fn string(globals: &Globals, string: String) -> Self {
        RValue::new_string(globals, string).pack()
    }

    pub fn bytes(globals: &Globals, bytes: Vec<u8>) -> Self {
        RValue::new_bytes(globals, bytes).pack()
    }

    pub fn symbol(id: IdentId) -> Self {
        let id: u32 = id.into();
        Value((id as u64) << 32 | TAG_SYMBOL)
    }

    pub fn range(globals: &Globals, start: Value, end: Value, exclude: bool) -> Self {
        let info = RangeInfo::new(start, end, exclude);
        RValue::new_range(globals, info).pack()
    }

    pub fn bootstrap_class(classref: ClassRef) -> Self {
        RValue::new_bootstrap(classref).pack()
    }

    pub fn ordinary_object(class: Value) -> Self {
        RValue::new_ordinary(class).pack()
    }

    pub fn class(globals: &Globals, class_ref: ClassRef) -> Self {
        RValue::new_class(globals, class_ref).pack()
    }

    pub fn class_from(
        globals: &Globals,
        id: impl Into<Option<IdentId>>,
        superclass: impl Into<Option<Value>>,
    ) -> Self {
        RValue::new_class(globals, ClassRef::from(id, superclass)).pack()
    }

    pub fn module(globals: &Globals, class_ref: ClassRef) -> Self {
        RValue::new_module(globals, class_ref).pack()
    }

    pub fn array_from(globals: &Globals, ary: Vec<Value>) -> Self {
        RValue::new_array(globals, ArrayInfo::new(ary)).pack()
    }

    pub fn splat(globals: &Globals, val: Value) -> Self {
        RValue::new_splat(globals, val).pack()
    }

    pub fn hash_from(globals: &Globals, hash: HashInfo) -> Self {
        RValue::new_hash(globals, hash).pack()
    }

    pub fn hash_from_map(
        globals: &Globals,
        hash: std::collections::HashMap<HashKey, Value>,
    ) -> Self {
        RValue::new_hash(globals, HashInfo::new(hash)).pack()
    }

    pub fn regexp(globals: &Globals, regexp_ref: RegexpInfo) -> Self {
        RValue::new_regexp(globals, regexp_ref).pack()
    }

    pub fn procobj(globals: &Globals, context: ContextRef) -> Self {
        RValue::new_proc(globals, ProcInfo::new(context)).pack()
    }

    pub fn method(globals: &Globals, name: IdentId, receiver: Value, method: MethodRef) -> Self {
        RValue::new_method(globals, MethodObjInfo::new(name, receiver, method)).pack()
    }

    pub fn fiber(
        globals: &Globals,
        vm: VMRef,
        context: ContextRef,
        rec: std::sync::mpsc::Receiver<VMResult>,
        tx: std::sync::mpsc::SyncSender<usize>,
    ) -> Self {
        RValue::new_fiber(globals, vm, context, rec, tx).pack()
    }

    pub fn enumerator(
        globals: &Globals,
        method: IdentId,
        receiver: Value,
        args: Args,
        fiber: FiberInfo,
    ) -> Self {
        RValue::new_enumerator(globals, method, receiver, args, fiber).pack()
    }
}

impl Value {
    // ==
    pub fn equal(self, other: Value) -> bool {
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
            (ObjKind::String(lhs), ObjKind::String(rhs)) => *lhs == *rhs,
            (ObjKind::Array(lhs), ObjKind::Array(rhs)) => lhs.elements == rhs.elements,
            (ObjKind::Range(lhs), ObjKind::Range(rhs)) => lhs == rhs,
            (ObjKind::Hash(lhs), ObjKind::Hash(rhs)) => **lhs == **rhs,
            (ObjKind::Regexp(lhs), ObjKind::Regexp(rhs)) => *lhs == *rhs,
            (ObjKind::Invalid, _) => {
                panic!("Invalid rvalue. (maybe GC problem) {:?}", self.rvalue())
            }
            (_, ObjKind::Invalid) => {
                panic!("Invalid rvalue. (maybe GC problem) {:?}", other.rvalue())
            }
            (_, _) => false,
        }
    }

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
        match self.as_fixnum() {
            Some(1) => Ordering::Greater,
            Some(0) => Ordering::Equal,
            Some(-1) => Ordering::Less,
            _ => panic!("Illegal ordering value."),
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
            let got = match RV::Integer(*expect).pack().as_fixnum() {
                Some(int) => int,
                None => panic!("Expect:{:?} Got:Invalid RValue"),
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
        let globals = GlobalsRef::new_globals();
        let from = RV::Integer(7).pack();
        let to = RV::Integer(36).pack();
        let expect = Value::range(&globals, from, to, true);
        let got = expect.unpack().pack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_class() {
        let globals = GlobalsRef::new_globals();
        let expect = Value::class(&globals, ClassRef::from(IdentId::from(1), None));
        let got = expect.unpack().pack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_instance() {
        let globals = GlobalsRef::new_globals();
        let class = Value::class_from(&globals, IdentId::from(1), None);
        let expect = Value::ordinary_object(class);
        let got = expect.unpack().pack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_symbol() {
        let _globals = GlobalsRef::new_globals();
        let expect = RV::Symbol(IdentId::from(12345));
        let packed = expect.pack();
        let got = packed.unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }
}
