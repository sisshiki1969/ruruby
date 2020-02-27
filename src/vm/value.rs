use crate::vm::*;

const FALSE_VALUE: u64 = 0x00;
const UNINITIALIZED: u64 = 0x04;
const NIL_VALUE: u64 = 0x08;
const TAG_SYMBOL: u64 = 0x0c;
const TRUE_VALUE: u64 = 0x14;

const ZERO: u64 = (0b1000 << 60) | 0b10;

#[derive(Debug, Clone, PartialEq)]
pub enum RValue {
    Uninitialized,
    Nil,
    Bool(bool),
    FixNum(i64),
    FloatNum(f64),
    String(RString),
    Symbol(IdentId),
    Object(ObjectInfo),
    Char(u8),
}

impl RValue {
    pub fn pack(self) -> Value {
        match self {
            RValue::Uninitialized => Value::uninitialized(),
            RValue::Nil => Value::nil(),
            RValue::Bool(b) if b => Value::true_val(),
            RValue::Bool(_) => Value::false_val(),
            RValue::FixNum(num) => Value::fixnum(num),
            RValue::FloatNum(num) => Value::flonum(num),
            RValue::Symbol(id) => Value::symbol(id),
            _ => Value(RValue::pack_as_boxed(self)),
        }
    }

    fn pack_fixnum(num: i64) -> u64 {
        let mut top = (num as u64) >> 62;
        top = top ^ (top >> 1);
        if top & 0b1 == 0 {
            ((num << 1) as u64) | 0b1
        } else {
            RValue::pack_as_boxed(RValue::FixNum(num))
        }
    }

    fn pack_flonum(num: f64) -> u64 {
        if num == 0.0 {
            return ZERO;
        }
        let unum = f64::to_bits(num);
        let exp = (unum >> 60) & 0b111;
        if exp == 4 || exp == 3 {
            ((unum & !(0b0110u64 << 60)) | (0b0100u64 << 60)).rotate_left(3)
        } else {
            RValue::pack_as_boxed(RValue::FloatNum(num))
        }
    }

    fn pack_as_boxed(val: RValue) -> u64 {
        Box::into_raw(Box::new(val)) as u64
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
        if self.is_packed_value() {
            self.0.hash(state);
        } else {
            let lhs = unsafe { &*(self.0 as *mut RValue) };
            match lhs {
                RValue::FixNum(lhs) => lhs.hash(state),
                RValue::FloatNum(lhs) => (*lhs as u64).hash(state),
                RValue::String(lhs) => lhs.hash(state),
                RValue::Object(lhs) => match lhs.kind {
                    ObjKind::Array(lhs) => lhs.elements.hash(state),
                    ObjKind::Hash(lhs) => match lhs.inner() {
                        HashInfo::Map(map) => {
                            for (key, val) in map {
                                key.hash(state);
                                val.hash(state);
                            }
                        }
                        HashInfo::IdentMap(map) => {
                            for (key, val) in map {
                                key.hash(state);
                                val.hash(state);
                            }
                        }
                    },
                    ObjKind::Method(lhs) => lhs.inner().hash(state),
                    _ => self.0.hash(state),
                },
                _ => self.0.hash(state),
            };
        }
    }
}

impl PartialEq for Value {
    // Object#eql?()
    // This type of equality is used for comparison for keys of Hash.
    // Regexp, Range etc must be implemented.
    fn eq(&self, other: &Self) -> bool {
        if self.is_packed_value() || other.is_packed_value() {
            self.0 == other.0
        } else {
            let lhs = unsafe { &(*(self.0 as *mut RValue)) };
            let rhs = unsafe { &(*(other.0 as *mut RValue)) };
            match (lhs, rhs) {
                (RValue::FixNum(lhs), RValue::FixNum(rhs)) => lhs == rhs,
                (RValue::FloatNum(lhs), RValue::FloatNum(rhs)) => lhs == rhs,
                (RValue::String(lhs), RValue::String(rhs)) => *lhs == *rhs,
                (RValue::Object(lhs), RValue::Object(rhs)) => match (&lhs.kind, &rhs.kind) {
                    (ObjKind::Array(lhs), ObjKind::Array(rhs)) => lhs.elements == rhs.elements,
                    (ObjKind::Hash(lhs), ObjKind::Hash(rhs)) => match (lhs.inner(), rhs.inner()) {
                        (HashInfo::Map(lhs), HashInfo::Map(rhs)) => lhs == rhs,
                        (HashInfo::IdentMap(lhs), HashInfo::IdentMap(rhs)) => lhs == rhs,
                        _ => false,
                    },
                    (ObjKind::Method(lhs), ObjKind::Method(rhs)) => lhs.inner() == rhs.inner(),
                    _ => lhs.kind == rhs.kind,
                },
                _ => self.0 == other.0,
            }
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
    pub fn unpack(self) -> RValue {
        if !self.is_packed_value() {
            unsafe { (*(self.0 as *mut RValue)).clone() }
        } else if self.is_packed_fixnum() {
            RValue::FixNum(self.as_packed_fixnum())
        } else if self.is_packed_num() {
            RValue::FloatNum(self.as_packed_flonum())
        } else if self.is_packed_symbol() {
            RValue::Symbol(self.as_packed_symbol())
        } else {
            match self.0 {
                NIL_VALUE => RValue::Nil,
                TRUE_VALUE => RValue::Bool(true),
                FALSE_VALUE => RValue::Bool(false),
                UNINITIALIZED => RValue::Uninitialized,
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

    pub fn get_class_object_for_method(&self, globals: &Globals) -> Value {
        match self.is_object() {
            Some(oref) => oref.class(),
            None => match self.unpack() {
                RValue::FixNum(_) => globals.builtins.integer,
                RValue::String(_) => globals.builtins.string,
                _ => globals.builtins.object,
            },
        }
    }

    pub fn get_class_object(&self, globals: &Globals) -> Value {
        match self.is_object() {
            Some(oref) => oref.search_class(),
            None => match self.unpack() {
                RValue::FixNum(_) => globals.builtins.integer,
                RValue::String(_) => globals.builtins.string,
                _ => globals.builtins.object,
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
        self.as_class().method_table.get(&id).cloned()
    }
}

impl Value {
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
                match *(self.0 as *mut RValue) {
                    RValue::FixNum(i) => Some(i),
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
                match *(self.0 as *mut RValue) {
                    RValue::FixNum(i) => Ok(i),
                    _ => Err(vm.error_argument(msg.into() + " must be an Integer.")),
                }
            }
        }
    }

    pub fn as_object(&self) -> ObjectRef {
        self.is_object().unwrap()
    }

    pub fn is_object(&self) -> Option<ObjectRef> {
        if self.is_packed_value() {
            return None;
        }
        unsafe {
            match &*(self.0 as *mut RValue) {
                RValue::Object(oref) => Some(oref.as_ref()),
                _ => None,
            }
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

    pub fn as_range(&self) -> Option<RangeInfo> {
        match self.is_object() {
            Some(oref) => match &oref.kind {
                ObjKind::Range(info) => Some(info.clone()),
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

    pub fn as_string(&self) -> Option<&String> {
        if self.is_packed_value() {
            return None;
        }
        unsafe {
            match &*(self.0 as *mut RValue) {
                RValue::String(RString::Str(s)) => Some(s),
                _ => None,
            }
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        if self.is_packed_value() {
            return None;
        }
        unsafe {
            match &*(self.0 as *mut RValue) {
                RValue::String(RString::Bytes(b)) => Some(b),
                RValue::String(RString::Str(s)) => Some(s.as_bytes()),
                _ => None,
            }
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
        Value(RValue::pack_fixnum(num))
    }

    pub fn flonum(num: f64) -> Self {
        Value(RValue::pack_flonum(num))
    }

    pub fn string(string: String) -> Self {
        Value(RValue::pack_as_boxed(RValue::String(RString::Str(string))))
    }

    pub fn bytes(bytes: Vec<u8>) -> Self {
        Value(RValue::pack_as_boxed(RValue::String(RString::Bytes(bytes))))
    }

    pub fn symbol(id: IdentId) -> Self {
        let id: u32 = id.into();
        Value((id as u64) << 32 | TAG_SYMBOL)
    }

    fn object(obj_info: ObjectInfo) -> Self {
        Value(RValue::pack_as_boxed(RValue::Object(obj_info)))
    }

    pub fn bootstrap_class(classref: ClassRef) -> Self {
        Value::object(ObjectInfo::new_bootstrap(classref))
    }

    pub fn ordinary_object(class: Value) -> Self {
        Value::object(ObjectInfo::new_ordinary(class))
    }

    pub fn class(globals: &Globals, class_ref: ClassRef) -> Self {
        Value::object(ObjectInfo::new_class(globals, class_ref))
    }

    pub fn plain_class(globals: &Globals, class_ref: ClassRef) -> Self {
        Value::object(ObjectInfo::new_class(globals, class_ref))
    }

    pub fn module(globals: &Globals, class_ref: ClassRef) -> Self {
        Value::object(ObjectInfo::new_module(globals, class_ref))
    }

    pub fn array(globals: &Globals, array_ref: ArrayRef) -> Self {
        Value::object(ObjectInfo::new_array(globals, array_ref))
    }

    pub fn array_from(globals: &Globals, ary: Vec<Value>) -> Self {
        Value::object(ObjectInfo::new_array(globals, ArrayRef::from(ary)))
    }

    pub fn splat(globals: &Globals, val: Value) -> Self {
        Value::object(ObjectInfo::new_splat(globals, val))
    }

    pub fn hash(globals: &Globals, hash_ref: HashRef) -> Self {
        Value::object(ObjectInfo::new_hash(globals, hash_ref))
    }

    pub fn regexp(globals: &Globals, regexp_ref: RegexpRef) -> Self {
        Value::object(ObjectInfo::new_regexp(globals, regexp_ref))
    }

    pub fn range(globals: &Globals, start: Value, end: Value, exclude: bool) -> Self {
        let info = RangeInfo::new(start, end, exclude);
        Value::object(ObjectInfo::new_range(globals, info))
    }

    pub fn procobj(globals: &Globals, context: ContextRef) -> Self {
        Value::object(ObjectInfo::new_proc(globals, ProcRef::from(context)))
    }

    pub fn method(globals: &Globals, name: IdentId, receiver: Value, method: MethodRef) -> Self {
        Value::object(ObjectInfo::new_method(
            globals,
            MethodObjRef::from(name, receiver, method),
        ))
    }
}

impl Value {
    // ==
    pub fn equal(self, other: Value) -> bool {
        if self.id() == other.id() {
            return true;
        };
        match (self.is_packed_num(), other.is_packed_num()) {
            (false, false) => {}
            (true, true) => match (self.is_packed_fixnum(), other.is_packed_fixnum()) {
                (true, false) => return self.as_packed_fixnum() as f64 == other.as_packed_flonum(),
                (false, true) => return self.as_packed_flonum() == other.as_packed_fixnum() as f64,
                _ => return false,
            },
            _ => return false,
        }
        if self.is_packed_symbol() || other.is_packed_symbol() {
            return false;
        }
        match (&self.unpack(), &other.unpack()) {
            (RValue::FixNum(lhs), RValue::FixNum(rhs)) => lhs == rhs,
            (RValue::FloatNum(lhs), RValue::FloatNum(rhs)) => lhs == rhs,
            (RValue::FixNum(lhs), RValue::FloatNum(rhs)) => *lhs as f64 == *rhs,
            (RValue::FloatNum(lhs), RValue::FixNum(rhs)) => *lhs == *rhs as f64,
            (RValue::String(lhs), RValue::String(rhs)) => *lhs == *rhs,
            (RValue::Object(lhs_o), RValue::Object(rhs_o)) => match (&lhs_o.kind, &rhs_o.kind) {
                (ObjKind::Array(lhs), ObjKind::Array(rhs)) => lhs.elements == rhs.elements,
                (ObjKind::Range(lhs), ObjKind::Range(rhs)) => {
                    lhs.start.equal(rhs.start)
                        && lhs.end.equal(rhs.end)
                        && lhs.exclude == rhs.exclude
                }
                (ObjKind::Hash(lhs), ObjKind::Hash(rhs)) => match (lhs.inner(), rhs.inner()) {
                    (HashInfo::Map(lhs), HashInfo::Map(rhs)) => lhs == rhs,
                    (HashInfo::IdentMap(lhs), HashInfo::IdentMap(rhs)) => lhs == rhs,
                    _ => false,
                },
                (_, _) => false,
            },
            _ => false,
        }
    }
}

#[allow(unused_imports)]
mod tests {
    use crate::vm::*;

    #[test]
    fn pack_bool1() {
        let expect = RValue::Bool(true);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_bool2() {
        let expect = RValue::Bool(false);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_nil() {
        let expect = RValue::Nil;
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_uninit() {
        let expect = RValue::Uninitialized;
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer1() {
        let expect = RValue::FixNum(12054);
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
            let got = match RValue::FixNum(*expect).pack().as_fixnum() {
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
        let expect = RValue::FixNum(-58993);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer3() {
        let expect = RValue::FixNum(0x8000_0000_0000_0000 as u64 as i64);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer4() {
        let expect = RValue::FixNum(0x4000_0000_0000_0000 as u64 as i64);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_integer5() {
        let expect = RValue::FixNum(0x7fff_ffff_ffff_ffff as u64 as i64);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float0() {
        let expect = RValue::FloatNum(0.0);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float1() {
        let expect = RValue::FloatNum(100.0);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float2() {
        let expect = RValue::FloatNum(13859.628547);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_float3() {
        let expect = RValue::FloatNum(-5282.2541156);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_char() {
        let expect = RValue::Char(123);
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_string() {
        let expect = RValue::String(RString::Str("Ruby".to_string()));
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_range() {
        let globals = Globals::new();
        let from = RValue::FixNum(7).pack();
        let to = RValue::FixNum(36).pack();
        let expect = RValue::Object(ObjectInfo::new_range(
            &globals,
            RangeInfo::new(from, to, false),
        ));
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_class() {
        let globals = Globals::new();
        let expect = RValue::Object(ObjectInfo::new_class(
            &globals,
            ClassRef::from(IdentId::from(1), None),
        ));
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_instance() {
        let globals = Globals::new();
        let class_ref = ClassRef::from(IdentId::from(1), None);
        let class = Value::class(&globals, class_ref);
        let expect = RValue::Object(ObjectInfo::new_ordinary(class));
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }

    #[test]
    fn pack_symbol() {
        let expect = RValue::Symbol(IdentId::from(12345));
        let got = expect.clone().pack().unpack();
        if expect != got {
            panic!("Expect:{:?} Got:{:?}", expect, got)
        }
    }
}
