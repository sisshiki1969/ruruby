use crate::*;

const FALSE_VALUE: u64 = 0x00;
const UNINITIALIZED: u64 = 0x04;
const NIL_VALUE: u64 = 0x08;
const TAG_SYMBOL: u64 = 0x0c;
const TRUE_VALUE: u64 = 0x14;
const MASK1: u64 = !(0b0110u64 << 60);
const MASK2: u64 = 0b0100u64 << 60;

const ZERO: u64 = (0b1000 << 60) | 0b10;

#[macro_export]
macro_rules! expect_string {
    ($var:ident, $vm:ident, $val:expr) => {
        let oref = match $val.as_rvalue() {
            Some(oref) => oref,
            None => return Err($vm.error_argument("Must be a String.")),
        };
        let $var: &str = match &oref.kind {
            ObjKind::String(RString::Str(s)) => s,
            ObjKind::String(RString::Bytes(b)) => match String::from_utf8_lossy(b) {
                std::borrow::Cow::Borrowed(s) => s,
                std::borrow::Cow::Owned(_) => return Err($vm.error_argument("Must be a String.")),
            },
            _ => return Err($vm.error_argument("Must be a String.")),
        };
    };
}

#[macro_export]
macro_rules! expect_bytes {
    ($var:ident, $vm:ident, $val:expr) => {
        let oref = match $val.as_rvalue() {
            Some(oref) => oref,
            None => return Err($vm.error_argument("Must be a String.")),
        };
        let $var = match &oref.kind {
            ObjKind::String(RString::Str(s)) => s.as_bytes(),
            ObjKind::String(RString::Bytes(b)) => b,
            _ => return Err($vm.error_argument("Must be a String.")),
        };
    };
}

#[derive(Debug, Clone, PartialEq)]
pub enum RV {
    Uninitialized,
    Nil,
    Bool(bool),
    FixNum(i64),
    FloatNum(f64),
    Symbol(IdentId),
    Object(ObjectRef),
}

impl RV {
    pub fn pack(self) -> Value {
        match self {
            RV::Uninitialized => Value::uninitialized(),
            RV::Nil => Value::nil(),
            RV::Bool(true) => Value::true_val(),
            RV::Bool(false) => Value::false_val(),
            RV::FixNum(num) => Value::fixnum(num),
            RV::FloatNum(num) => Value::flonum(num),
            RV::Symbol(id) => Value::symbol(id),
            RV::Object(info) => Value(info.id()),
        }
    }
}

#[derive(Debug, Clone, Copy)]
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
                ObjKind::FixNum(lhs) => lhs.hash(state),
                ObjKind::FloatNum(lhs) => lhs.to_bits().hash(state),
                ObjKind::String(lhs) => lhs.hash(state),
                ObjKind::Array(lhs) => lhs.elements.hash(state),
                ObjKind::Range(lhs) => lhs.hash(state),
                ObjKind::Hash(lhs) => {
                    for (key, val) in lhs.iter() {
                        key.hash(state);
                        val.hash(state);
                    }
                }
                ObjKind::Method(lhs) => lhs.inner().hash(state),
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
            (ObjKind::FixNum(lhs), ObjKind::FixNum(rhs)) => *lhs == *rhs,
            (ObjKind::FloatNum(lhs), ObjKind::FloatNum(rhs)) => *lhs == *rhs,
            (ObjKind::FixNum(lhs), ObjKind::FloatNum(rhs)) => *lhs as f64 == *rhs,
            (ObjKind::FloatNum(lhs), ObjKind::FixNum(rhs)) => *lhs == *rhs as f64,
            (ObjKind::String(lhs), ObjKind::String(rhs)) => *lhs == *rhs,
            (ObjKind::Array(lhs), ObjKind::Array(rhs)) => lhs.elements == rhs.elements,
            (ObjKind::Range(lhs), ObjKind::Range(rhs)) => {
                lhs.start == rhs.start && lhs.end == rhs.end && lhs.exclude == rhs.exclude
            }
            (ObjKind::Hash(lhs), ObjKind::Hash(rhs)) => match (lhs.inner(), rhs.inner()) {
                (HashInfo::Map(lhs), HashInfo::Map(rhs)) => *lhs == *rhs,
                (HashInfo::IdentMap(lhs), HashInfo::IdentMap(rhs)) => *lhs == *rhs,
                _ => false,
            },
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

impl Value {
    pub fn unpack(self) -> RV {
        if !self.is_packed_value() {
            let info = self.rvalue();
            match &info.kind {
                ObjKind::FixNum(i) => RV::FixNum(*i),
                ObjKind::FloatNum(f) => RV::FloatNum(*f),
                _ => RV::Object(Ref::from_ref(info)),
            }
        } else if self.is_packed_fixnum() {
            RV::FixNum(self.as_packed_fixnum())
        } else if self.is_packed_num() {
            RV::FloatNum(self.as_packed_flonum())
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

    /// Get RValue from Value.
    /// This method works only if `self` is not a packed value.
    pub fn as_rvalue(&self) -> Option<&RValue> {
        if self.is_packed_value() {
            None
        } else {
            Some(self.rvalue())
        }
    }

    pub fn rvalue(&self) -> &RValue {
        unsafe { &*(self.0 as *mut RValue) }
    }

    pub fn rvalue_mut(&self) -> &mut RValue {
        unsafe { &mut *(self.0 as *mut RValue) }
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
                ObjKind::FixNum(_) => globals.builtins.integer,
                ObjKind::FloatNum(_) => globals.builtins.float,
                _ => info.class(),
            },
        }
    }

    pub fn get_class_object(&self, globals: &Globals) -> Value {
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
                ObjKind::FixNum(_) => globals.builtins.integer,
                ObjKind::FloatNum(_) => globals.builtins.float,
                _ => info.search_class(),
            },
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
        self.as_object().set_var(id, val);
    }

    pub fn get_var(&self, id: IdentId) -> Option<Value> {
        self.as_object().get_var(id)
    }

    pub fn set_var_if_exists(&self, id: IdentId, val: Value) -> bool {
        match self.as_object().get_mut_var(id) {
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
                    ObjKind::FixNum(f) => Some(*f),
                    _ => None,
                },
                _ => None,
            }
        }
    }

    pub fn expect_fixnum(&self, vm: &VM, msg: impl Into<String>) -> Result<i64, RubyError> {
        match self.as_fixnum() {
            Some(i) => Ok(i),
            None => Err(vm.error_argument(msg.into() + " must be an Integer.")),
        }
    }

    pub fn as_flonum(&self) -> Option<f64> {
        if self.is_packed_flonum() {
            Some(self.as_packed_flonum())
        } else {
            match self.as_rvalue() {
                Some(info) => match &info.kind {
                    ObjKind::FloatNum(f) => Some(*f),
                    _ => None,
                },
                _ => None,
            }
        }
    }

    pub fn expect_flonum(&self, vm: &VM, msg: impl Into<String>) -> Result<f64, RubyError> {
        match self.as_flonum() {
            Some(f) => Ok(f),
            None => Err(vm.error_argument(msg.into() + " must be an Integer.")),
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self.as_rvalue() {
            Some(oref) => match &oref.kind {
                ObjKind::String(RString::Str(s)) => Some(s.as_bytes()),
                ObjKind::String(RString::Bytes(b)) => Some(b.as_slice()),
                _ => None,
            },
            None => None,
        }
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

    pub fn as_object(&self) -> ObjectRef {
        self.is_object().unwrap()
    }

    pub fn is_object(&self) -> Option<ObjectRef> {
        match self.as_rvalue() {
            Some(info) => Some(info.as_ref()),
            _ => None,
        }
    }

    pub fn as_class(&self) -> ClassRef {
        self.is_class().unwrap()
    }

    pub fn is_class(&self) -> Option<ClassRef> {
        match self.is_object() {
            Some(oref) => match oref.kind {
                ObjKind::Class(cref) => Some(cref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_module(&self) -> Option<ClassRef> {
        match self.is_object() {
            Some(oref) => match oref.kind {
                ObjKind::Class(cref) | ObjKind::Module(cref) => Some(cref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn is_module(&self) -> Option<ClassRef> {
        match self.is_object() {
            Some(oref) => match oref.kind {
                ObjKind::Module(cref) => Some(cref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_array(&self) -> Option<ArrayRef> {
        match self.is_object() {
            Some(oref) => match oref.kind {
                ObjKind::Array(aref) => Some(aref),
                _ => None,
            },
            None => None,
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
        match self.is_object() {
            Some(oref) => match oref.kind {
                ObjKind::Splat(val) => Some(val),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_hash(&self) -> Option<HashRef> {
        match self.is_object() {
            Some(oref) => match oref.kind {
                ObjKind::Hash(href) => Some(href),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_regexp(&self) -> Option<RegexpRef> {
        match self.is_object() {
            Some(oref) => match oref.kind {
                ObjKind::Regexp(regref) => Some(regref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_proc(&self) -> Option<ProcRef> {
        match self.is_object() {
            Some(oref) => match oref.kind {
                ObjKind::Proc(pref) => Some(pref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_method(&self) -> Option<MethodObjRef> {
        match self.is_object() {
            Some(oref) => match oref.kind {
                ObjKind::Method(mref) => Some(mref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_fiber(&self) -> Option<FiberRef> {
        match self.is_object() {
            Some(oref) => match oref.kind {
                ObjKind::Fiber(info) => Some(info),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_enumerator(&self) -> Option<EnumRef> {
        match self.is_object() {
            Some(oref) => match oref.kind {
                ObjKind::Enumerator(eref) => Some(eref),
                _ => None,
            },
            None => None,
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
        Value::object(RValue::new_string(globals, string))
    }

    pub fn bytes(globals: &Globals, bytes: Vec<u8>) -> Self {
        Value::object(RValue::new_bytes(globals, bytes))
    }

    pub fn symbol(id: IdentId) -> Self {
        let id: u32 = id.into();
        Value((id as u64) << 32 | TAG_SYMBOL)
    }

    pub fn range(globals: &Globals, start: Value, end: Value, exclude: bool) -> Self {
        let info = RangeInfo::new(start, end, exclude);
        Value::object(RValue::new_range(globals, info))
    }

    fn object(obj_info: RValue) -> Self {
        obj_info.pack()
    }

    pub fn bootstrap_class(classref: ClassRef) -> Self {
        Value::object(RValue::new_bootstrap(classref))
    }

    pub fn ordinary_object(class: Value) -> Self {
        Value::object(RValue::new_ordinary(class))
    }

    pub fn class(globals: &Globals, class_ref: ClassRef) -> Self {
        Value::object(RValue::new_class(globals, class_ref))
    }

    pub fn plain_class(globals: &Globals, class_ref: ClassRef) -> Self {
        Value::object(RValue::new_class(globals, class_ref))
    }

    pub fn module(globals: &Globals, class_ref: ClassRef) -> Self {
        Value::object(RValue::new_module(globals, class_ref))
    }

    pub fn array(globals: &Globals, array_ref: ArrayRef) -> Self {
        Value::object(RValue::new_array(globals, array_ref))
    }

    pub fn array_from(globals: &Globals, ary: Vec<Value>) -> Self {
        Value::object(RValue::new_array(globals, ArrayRef::from(ary)))
    }

    pub fn splat(globals: &Globals, val: Value) -> Self {
        Value::object(RValue::new_splat(globals, val))
    }

    pub fn hash(globals: &Globals, hash_ref: HashRef) -> Self {
        Value::object(RValue::new_hash(globals, hash_ref))
    }

    pub fn regexp(globals: &Globals, regexp_ref: RegexpRef) -> Self {
        Value::object(RValue::new_regexp(globals, regexp_ref))
    }

    pub fn procobj(globals: &Globals, context: ContextRef) -> Self {
        Value::object(RValue::new_proc(globals, ProcRef::from(context)))
    }

    pub fn method(globals: &Globals, name: IdentId, receiver: Value, method: MethodRef) -> Self {
        Value::object(RValue::new_method(
            globals,
            MethodObjRef::from(name, receiver, method),
        ))
    }

    pub fn fiber(
        globals: &Globals,
        vm: VMRef,
        context: ContextRef,
        rec: std::sync::mpsc::Receiver<VMResult>,
        tx: std::sync::mpsc::SyncSender<usize>,
    ) -> Self {
        Value::object(RValue::new_fiber(globals, vm, context, rec, tx))
    }

    pub fn enumerator(globals: &Globals, method: IdentId, receiver: Value, args: Args) -> Self {
        Value::object(RValue::new_enumerator(globals, method, receiver, args))
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
            (ObjKind::FixNum(lhs), ObjKind::FixNum(rhs)) => *lhs == *rhs,
            (ObjKind::FloatNum(lhs), ObjKind::FloatNum(rhs)) => *lhs == *rhs,
            (ObjKind::FixNum(lhs), ObjKind::FloatNum(rhs)) => *lhs as f64 == *rhs,
            (ObjKind::FloatNum(lhs), ObjKind::FixNum(rhs)) => *lhs == *rhs as f64,
            (ObjKind::String(lhs), ObjKind::String(rhs)) => *lhs == *rhs,
            (ObjKind::Array(lhs), ObjKind::Array(rhs)) => lhs.elements == rhs.elements,
            (ObjKind::Range(lhs), ObjKind::Range(rhs)) => {
                lhs.start.equal(rhs.start) && lhs.end.equal(rhs.end) && lhs.exclude == rhs.exclude
            }
            (ObjKind::Hash(lhs), ObjKind::Hash(rhs)) => lhs.inner() == rhs.inner(),
            (_, _) => false,
        }
    }
}

#[allow(unused_imports)]
mod tests {
    use crate::*;

    #[test]
    fn pack_bool1() {
        let expect = RV::Bool(true);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_bool2() {
        let expect = RV::Bool(false);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_nil() {
        let expect = RV::Nil;
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_uninit() {
        let expect = RV::Uninitialized;
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer1() {
        let expect = RV::FixNum(12054);
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
            let got = match RV::FixNum(*expect).pack().as_fixnum() {
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
        let expect = RV::FixNum(-58993);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer3() {
        let expect = RV::FixNum(0x8000_0000_0000_0000 as u64 as i64);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer4() {
        let expect = RV::FixNum(0x4000_0000_0000_0000 as u64 as i64);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer5() {
        let expect = RV::FixNum(0x7fff_ffff_ffff_ffff as u64 as i64);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float0() {
        let expect = RV::FloatNum(0.0);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float1() {
        let expect = RV::FloatNum(100.0);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float2() {
        let expect = RV::FloatNum(13859.628547);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float3() {
        let expect = RV::FloatNum(-5282.2541156);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_range() {
        let globals = Globals::new();
        let from = RV::FixNum(7).pack();
        let to = RV::FixNum(36).pack();
        let expect = Value::range(&globals, from, to, true);
        let got = expect.unpack().pack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_class() {
        let globals = Globals::new();
        let expect = Value::class(&globals, ClassRef::from(IdentId::from(1), None));
        let got = expect.unpack().pack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_instance() {
        let globals = Globals::new();
        let class_ref = ClassRef::from(IdentId::from(1), None);
        let class = Value::class(&globals, class_ref);
        let expect = Value::ordinary_object(class);
        let got = expect.unpack().pack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_symbol() {
        let expect = RV::Symbol(IdentId::from(12345));
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }
}
