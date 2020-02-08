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
    String(String),
    Symbol(IdentId),
    Object(ObjectInfo),
    Char(u8),
}

const VEC_ARRAY_SIZE: usize = 8;

#[derive(Debug, Clone)]
pub struct Args {
    pub self_value: PackedValue,
    pub block: PackedValue,
    args: ArgsArray,
}

impl Args {
    pub fn new(len: usize) -> Self {
        Args {
            self_value: PackedValue::nil(),
            block: PackedValue::nil(),
            args: ArgsArray::new(len),
        }
    }

    pub fn push(&mut self, val: PackedValue) {
        self.args.push(val);
    }

    pub fn new0(self_value: PackedValue, block: impl Into<Option<PackedValue>>) -> Self {
        Args {
            self_value,
            block: block.into().unwrap_or_default(),
            args: ArgsArray::new0(),
        }
    }

    pub fn new1(
        self_value: PackedValue,
        block: impl Into<Option<PackedValue>>,
        arg: PackedValue,
    ) -> Self {
        Args {
            self_value,
            block: block.into().unwrap_or_default(),
            args: ArgsArray::new1(arg),
        }
    }

    pub fn new2(
        self_value: PackedValue,
        block: impl Into<Option<PackedValue>>,
        arg0: PackedValue,
        arg1: PackedValue,
    ) -> Self {
        Args {
            self_value,
            block: block.into().unwrap_or_default(),
            args: ArgsArray::new2(arg0, arg1),
        }
    }

    pub fn new3(
        self_value: PackedValue,
        block: impl Into<Option<PackedValue>>,
        arg0: PackedValue,
        arg1: PackedValue,
        arg2: PackedValue,
    ) -> Self {
        Args {
            self_value,
            block: block.into().unwrap_or_default(),
            args: ArgsArray::new3(arg0, arg1, arg2),
        }
    }

    pub fn new4(
        self_value: PackedValue,
        block: impl Into<Option<PackedValue>>,
        arg0: PackedValue,
        arg1: PackedValue,
        arg2: PackedValue,
        arg3: PackedValue,
    ) -> Self {
        Args {
            self_value,
            block: block.into().unwrap_or_default(),
            args: ArgsArray::new4(arg0, arg1, arg2, arg3),
        }
    }

    pub fn len(&self) -> usize {
        self.args.len()
    }

    pub fn get_slice(&self, start: usize, end: usize) -> &[PackedValue] {
        self.args.get_slice(start, end)
    }
}

impl Index<usize> for Args {
    type Output = PackedValue;

    fn index(&self, index: usize) -> &Self::Output {
        &self.args[index]
    }
}

impl IndexMut<usize> for Args {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.args.index_mut(index)
    }
}

impl Deref for Args {
    type Target = [PackedValue];

    fn deref(&self) -> &Self::Target {
        self.args.deref()
    }
}

#[derive(Debug, Clone)]
enum ArgsArray {
    Array {
        len: usize,
        ary: [PackedValue; VEC_ARRAY_SIZE],
    },
    Vec(Vec<PackedValue>),
}

impl ArgsArray {
    fn new(len: usize) -> Self {
        if len <= VEC_ARRAY_SIZE {
            ArgsArray::Array {
                len,
                ary: [PackedValue::uninitialized(); VEC_ARRAY_SIZE],
            }
        } else {
            ArgsArray::Vec(vec![PackedValue::uninitialized(); len])
        }
    }

    fn push(&mut self, val: PackedValue) {
        if self.len() == VEC_ARRAY_SIZE {
            let mut ary = self.get_slice(0, VEC_ARRAY_SIZE).to_vec();
            ary.push(val);
            unsafe { std::ptr::write(self, ArgsArray::Vec(ary)) };
        } else {
            match self {
                ArgsArray::Vec(ref mut v) => v.push(val),
                ArgsArray::Array {
                    ref mut len,
                    ref mut ary,
                } => {
                    ary[*len] = val;
                    *len += 1;
                }
            }
        }
    }

    fn new0() -> Self {
        ArgsArray::Array {
            len: 0,
            ary: [PackedValue::uninitialized(); VEC_ARRAY_SIZE],
        }
    }

    fn new1(arg: PackedValue) -> Self {
        let mut ary = [PackedValue::uninitialized(); VEC_ARRAY_SIZE];
        ary[0] = arg;
        ArgsArray::Array { len: 1, ary }
    }

    fn new2(arg0: PackedValue, arg1: PackedValue) -> Self {
        let mut ary = [PackedValue::uninitialized(); VEC_ARRAY_SIZE];
        ary[0] = arg0;
        ary[1] = arg1;
        ArgsArray::Array { len: 2, ary }
    }

    fn new3(arg0: PackedValue, arg1: PackedValue, arg2: PackedValue) -> Self {
        let mut ary = [PackedValue::uninitialized(); VEC_ARRAY_SIZE];
        ary[0] = arg0;
        ary[1] = arg1;
        ary[2] = arg2;
        ArgsArray::Array { len: 3, ary }
    }

    fn new4(arg0: PackedValue, arg1: PackedValue, arg2: PackedValue, arg3: PackedValue) -> Self {
        let mut ary = [PackedValue::uninitialized(); VEC_ARRAY_SIZE];
        ary[0] = arg0;
        ary[1] = arg1;
        ary[2] = arg2;
        ary[3] = arg3;
        ArgsArray::Array { len: 4, ary }
    }

    fn len(&self) -> usize {
        match self {
            ArgsArray::Array { len, .. } => *len,
            ArgsArray::Vec(v) => v.len(),
        }
    }

    fn get_slice(&self, start: usize, end: usize) -> &[PackedValue] {
        match self {
            ArgsArray::Array { ary, .. } => &ary[start..end],
            ArgsArray::Vec(v) => &v[start..end],
        }
    }
}

impl Index<usize> for ArgsArray {
    type Output = PackedValue;

    fn index(&self, index: usize) -> &Self::Output {
        match self {
            ArgsArray::Array { ary, .. } => &ary[index],
            ArgsArray::Vec(v) => &v[index],
        }
    }
}

impl IndexMut<usize> for ArgsArray {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        match self {
            ArgsArray::Array { ary, .. } => &mut ary[index],
            ArgsArray::Vec(v) => &mut v[index],
        }
    }
}

impl Deref for ArgsArray {
    type Target = [PackedValue];

    fn deref(&self) -> &Self::Target {
        match self {
            ArgsArray::Array { len, ary } => &ary[0..*len],
            ArgsArray::Vec(v) => &v,
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
        let mut top = (num as u64) >> 62;
        top = top ^ (top >> 1);
        if top & 0b1 == 0 {
            ((num << 1) as u64) | 0b1
        } else {
            Value::pack_as_boxed(Value::FixNum(num))
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
            let lhs = unsafe { &(*(self.0 as *mut Value)) };
            match lhs {
                Value::FixNum(lhs) => lhs.hash(state),
                Value::FloatNum(lhs) => (*lhs as u64).hash(state),
                Value::String(lhs) => lhs.hash(state),
                Value::Object(lhs) => match lhs.kind {
                    ObjKind::Array(lhs) => lhs.elements.hash(state),
                    ObjKind::Hash(lhs) => {
                        for (key, val) in lhs.map.iter() {
                            key.hash(state);
                            val.hash(state);
                        }
                    }
                    _ => self.0.hash(state),
                },
                _ => self.0.hash(state),
            };
        }
    }
}

impl PartialEq for PackedValue {
    // Object#eql?()
    // This type of equality is used for comparison for keys of Hash.
    // Regexp, Array, Hash must be implemented.
    fn eq(&self, other: &Self) -> bool {
        if self.is_packed_value() || other.is_packed_value() {
            self.0 == other.0
        } else {
            let lhs = unsafe { &(*(self.0 as *mut Value)) };
            let rhs = unsafe { &(*(other.0 as *mut Value)) };
            match (lhs, rhs) {
                (Value::FixNum(lhs), Value::FixNum(rhs)) => lhs == rhs,
                (Value::FloatNum(lhs), Value::FloatNum(rhs)) => lhs == rhs,
                (Value::String(lhs), Value::String(rhs)) => *lhs == *rhs,
                (Value::Object(lhs), Value::Object(rhs)) => match (&lhs.kind, &rhs.kind) {
                    (ObjKind::Array(lhs), ObjKind::Array(rhs)) => lhs.elements == rhs.elements,
                    (ObjKind::Hash(lhs), ObjKind::Hash(rhs)) => lhs.map == rhs.map,
                    _ => lhs.kind == rhs.kind,
                },
                _ => self.0 == other.0,
            }
        }
    }
}
impl Eq for PackedValue {}

impl Default for PackedValue {
    fn default() -> Self {
        PackedValue::nil()
    }
}

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
        } else {
            match self.0 {
                NIL_VALUE => Value::Nil,
                TRUE_VALUE => Value::Bool(true),
                FALSE_VALUE => Value::Bool(false),
                UNINITIALIZED => Value::Uninitialized,
                _ => unreachable!("Illegal packed value."),
            }
        }
    }

    pub fn id(&self) -> u64 {
        self.0
    }

    pub fn get_class_object_for_method(&self, globals: &Globals) -> PackedValue {
        match self.is_object() {
            Some(oref) => oref.class(),
            None => match self.unpack() {
                Value::FixNum(_) => globals.integer,
                Value::String(_) => globals.string,
                _ => globals.object,
            },
        }
    }

    pub fn get_class_object(&self, globals: &Globals) -> PackedValue {
        match self.is_object() {
            Some(oref) => oref.search_class(),
            None => match self.unpack() {
                Value::FixNum(_) => globals.integer,
                Value::String(_) => globals.string,
                _ => globals.object,
            },
        }
    }

    pub fn superclass(&self) -> Option<PackedValue> {
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

    pub fn set_var(&mut self, id: IdentId, val: PackedValue) {
        self.as_object().set_var(id, val);
    }

    pub fn get_var(&self, id: IdentId) -> Option<PackedValue> {
        self.as_object().get_var(id)
    }

    pub fn set_var_if_exists(&self, id: IdentId, val: PackedValue) -> bool {
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

impl PackedValue {
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

    pub fn as_object(&self) -> ObjectRef {
        self.is_object().unwrap()
    }

    pub fn is_object(&self) -> Option<ObjectRef> {
        if self.is_packed_value() {
            return None;
        }
        unsafe {
            match &*(self.0 as *mut Value) {
                Value::Object(oref) => Some(oref.as_ref()),
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

    pub fn as_array(&self) -> Option<ArrayRef> {
        match self.is_object() {
            Some(oref) => match oref.kind {
                ObjKind::Array(aref) => Some(aref),
                _ => None,
            },
            None => None,
        }
    }

    pub fn as_splat(&self) -> Option<ArrayRef> {
        match self.is_object() {
            Some(oref) => match oref.kind {
                ObjKind::SplatArray(aref) => Some(aref),
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
            match &*(self.0 as *mut Value) {
                Value::String(string) => Some(string),
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
        PackedValue(Value::pack_as_boxed(Value::String(string)))
    }

    pub fn symbol(id: IdentId) -> Self {
        let id: u32 = id.into();
        PackedValue((id as u64) << 32 | TAG_SYMBOL)
    }

    fn object(obj_info: ObjectInfo) -> Self {
        PackedValue(Value::pack_as_boxed(Value::Object(obj_info)))
    }

    pub fn bootstrap_class(classref: ClassRef) -> Self {
        PackedValue::object(ObjectInfo::new_bootstrap(classref))
    }

    pub fn ordinary_object(class: PackedValue) -> Self {
        PackedValue::object(ObjectInfo::new_ordinary(class))
    }

    pub fn class(globals: &Globals, class_ref: ClassRef) -> Self {
        PackedValue::object(ObjectInfo::new_class(globals, class_ref))
    }

    pub fn plain_class(globals: &Globals, class_ref: ClassRef) -> Self {
        PackedValue::object(ObjectInfo::new_class(globals, class_ref))
    }

    pub fn module(globals: &Globals, class_ref: ClassRef) -> Self {
        PackedValue::object(ObjectInfo::new_module(globals, class_ref))
    }

    pub fn array(globals: &Globals, array_ref: ArrayRef) -> Self {
        PackedValue::object(ObjectInfo::new_array(globals, array_ref))
    }

    pub fn array_from(globals: &Globals, ary: Vec<PackedValue>) -> Self {
        PackedValue::object(ObjectInfo::new_array(globals, ArrayRef::from(ary)))
    }

    pub fn splat(globals: &Globals, array_ref: ArrayRef) -> Self {
        PackedValue::object(ObjectInfo::new_splat(globals, array_ref))
    }

    pub fn hash(globals: &Globals, hash_ref: HashRef) -> Self {
        PackedValue::object(ObjectInfo::new_hash(globals, hash_ref))
    }

    pub fn regexp(globals: &Globals, regexp_ref: RegexpRef) -> Self {
        PackedValue::object(ObjectInfo::new_regexp(globals, regexp_ref))
    }

    pub fn range(globals: &Globals, start: PackedValue, end: PackedValue, exclude: bool) -> Self {
        let info = RangeInfo::new(start, end, exclude);
        PackedValue::object(ObjectInfo::new_range(globals, info))
    }

    pub fn procobj(globals: &Globals, context: ContextRef) -> Self {
        PackedValue::object(ObjectInfo::new_proc(globals, ProcRef::from(context)))
    }

    pub fn method(
        globals: &Globals,
        name: IdentId,
        receiver: PackedValue,
        method: MethodRef,
    ) -> Self {
        PackedValue::object(ObjectInfo::new_method(
            globals,
            MethodObjRef::from(name, receiver, method),
        ))
    }
}

impl PackedValue {
    // ==
    pub fn equal(self, other: PackedValue) -> bool {
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
            (Value::FixNum(lhs), Value::FixNum(rhs)) => lhs == rhs,
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => lhs == rhs,
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => *lhs as f64 == *rhs,
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => *lhs == *rhs as f64,
            (Value::String(lhs), Value::String(rhs)) => *lhs == *rhs,
            (Value::Object(lhs_o), Value::Object(rhs_o)) => match (&lhs_o.kind, &rhs_o.kind) {
                (ObjKind::Array(lhs), ObjKind::Array(rhs)) => {
                    let lhs = &lhs.elements;
                    let rhs = &rhs.elements;
                    if lhs.len() != rhs.len() {
                        return false;
                    }
                    for i in 0..lhs.len() {
                        if !lhs[i].equal(rhs[i]) {
                            return false;
                        }
                    }
                    true
                }
                (ObjKind::Range(lhs), ObjKind::Range(rhs)) => {
                    if lhs.start.equal(rhs.start)
                        && lhs.end.equal(rhs.end)
                        && lhs.exclude == rhs.exclude
                    {
                        true
                    } else {
                        false
                    }
                }
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
        let expect = Value::String("Ruby".to_string());
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
        let expect = Value::Object(ObjectInfo::new_range(
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
        let expect = Value::Object(ObjectInfo::new_class(
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
        let class = PackedValue::class(&globals, class_ref);
        let expect = Value::Object(ObjectInfo::new_ordinary(class));
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
