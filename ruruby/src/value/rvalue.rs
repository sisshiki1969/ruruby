use crate::coroutine::*;
use crate::*;
use num::BigInt;
use std::borrow::Cow;
use std::default;

/// Heap-allocated objects.
pub struct RValue {
    flags: RVFlag,
    class: Module,
    var_table: Option<Box<ValueTable>>,
    pub kind: ObjKind,
}

#[derive(Clone, Copy)]
pub union RVFlag {
    flag: u64,
    next: Option<std::ptr::NonNull<RValue>>,
}

impl default::Default for RVFlag {
    #[inline(always)]
    fn default() -> Self {
        Self { flag: 1 }
    }
}

impl RVFlag {
    #[inline(always)]
    fn new(kind: u8) -> Self {
        RVFlag {
            flag: ((kind as u64) << 8) | 1,
        }
    }
}

impl std::fmt::Debug for RValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if unsafe { self.flags.flag & 0b1 } == 0 {
            writeln!(f, "FreeCell next:{:?}", unsafe { self.flags.next })?;
            return Ok(());
        }

        write!(f, "RValue {{ ")?;
        write!(f, "class: {:?} ", self.class)?;
        match &self.var_table {
            None => write!(f, "var_table: None ")?,
            Some(box t) => write!(f, "var_table: {:?} ", t)?,
        }
        write!(
            f,
            "kind: {} ",
            match self.kind() {
                ObjKind::INVALID => format!("Invalid"),
                ObjKind::ORDINARY => format!("Ordinary"),
                ObjKind::BIGNUM => format!("Bignum {:?}", *self.bignum()),
                ObjKind::FLOAT => format!("Float {}", self.float()),
                ObjKind::COMPLEX => format!("{:?}", *self.complex()),
                ObjKind::MODULE => format!("Module {:?}", *self.module()),
                ObjKind::CLASS => format!("Class {:?}", *self.module()),
                ObjKind::STRING => format!("String {:?}", *self.string()),
                ObjKind::ARRAY => format!("Array {:?}", *self.array()),
                ObjKind::RANGE => format!("Range {:?}", *self.range()),
                ObjKind::SPLAT => format!("Splat {:?}", self.splat()),
                ObjKind::HASH => format!("Hash {:?}", *self.rhash()),
                ObjKind::PROC => format!("Proc {:?}", *self.proc()),
                ObjKind::REGEXP => format!("Regexp {:?}", *self.regexp()),
                ObjKind::METHOD => format!("Method {:?}", *self.method()),
                ObjKind::FIBER => format!("Fiber {:?}", *self.fiber()),
                ObjKind::ENUMERATOR => format!("Enumerator {:?}", *self.enumerator()),
                ObjKind::TIME => format!("Time {:?}", *self.time()),
                ObjKind::EXCEPTION => format!("Exception {:?}", *self.exception()),
                ObjKind::BINDING => format!("Binding {:?}", *self.binding()),
                ObjKind::UNBOUND_METHOD => format!("UnboundMethod {:?}", *self.method()),
                k => panic!("invalid RValue kind. {}", k),
            }
        )?;
        writeln!(f, "}}")
    }
}

impl std::hash::Hash for RValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self.kind() {
            ObjKind::INVALID => unreachable!("Invalid rvalue. (maybe GC problem) {:?}", self),
            ObjKind::BIGNUM => self.bignum().hash(state),
            ObjKind::FLOAT => self.float().to_bits().hash(state),
            ObjKind::COMPLEX => self.complex().hash(state),
            ObjKind::STRING => self.string().hash(state),
            ObjKind::ARRAY => self.array().hash(state),
            ObjKind::RANGE => self.range().hash(state),
            ObjKind::HASH => self.rhash().hash(state),
            ObjKind::METHOD | ObjKind::UNBOUND_METHOD => self.method().hash(state),
            _ => self.id().hash(state),
        }
    }
}

use std::mem::ManuallyDrop;

#[repr(C)]
pub union ObjKind {
    pub bignum: ManuallyDrop<BigInt>,
    pub float: f64,
    pub complex: ManuallyDrop<RubyComplex>,
    pub module: ManuallyDrop<ClassInfo>,
    pub string: ManuallyDrop<RString>,
    pub array: ManuallyDrop<ArrayInfo>,
    pub range: ManuallyDrop<RangeInfo>,
    pub splat: Value, // internal use only.
    pub hash: ManuallyDrop<Box<HashInfo>>,
    pub proc: ManuallyDrop<ProcInfo>,
    pub regexp: ManuallyDrop<RegexpInfo>,
    pub method: ManuallyDrop<MethodObjInfo>,
    pub fiber: ManuallyDrop<Box<FiberContext>>,
    pub enumerator: ManuallyDrop<Box<FiberContext>>,
    pub time: ManuallyDrop<TimeInfo>,
    pub exception: ManuallyDrop<Box<RubyError>>,
    pub binding: HeapCtxRef,
    pub other: (),
}

impl ObjKind {
    pub const INVALID: u8 = 0;
    pub const ORDINARY: u8 = 1;
    pub const BIGNUM: u8 = 2;
    pub const FLOAT: u8 = 3;
    pub const COMPLEX: u8 = 4;
    pub const MODULE: u8 = 5;
    pub const CLASS: u8 = 20;
    pub const STRING: u8 = 6;
    pub const ARRAY: u8 = 7;
    pub const RANGE: u8 = 8;
    pub const SPLAT: u8 = 9;
    pub const HASH: u8 = 10;
    pub const PROC: u8 = 11;
    pub const REGEXP: u8 = 12;
    pub const METHOD: u8 = 13;
    pub const FIBER: u8 = 14;
    pub const ENUMERATOR: u8 = 15;
    pub const TIME: u8 = 16;
    pub const EXCEPTION: u8 = 17;
    pub const BINDING: u8 = 18;
    pub const UNBOUND_METHOD: u8 = 19;
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct RubyComplex {
    pub r: Value,
    pub i: Value,
}

impl ObjKind {
    #[inline(always)]
    fn other() -> Self {
        Self { other: () }
    }

    #[inline(always)]
    fn complex(r: Value, i: Value) -> Self {
        Self {
            complex: ManuallyDrop::new(RubyComplex { r, i }),
        }
    }

    #[inline(always)]
    fn bignum(b: BigInt) -> Self {
        Self {
            bignum: ManuallyDrop::new(b),
        }
    }

    #[inline(always)]
    fn range(info: RangeInfo) -> Self {
        Self {
            range: ManuallyDrop::new(info),
        }
    }

    #[inline(always)]
    fn string(rstr: RString) -> Self {
        Self {
            string: ManuallyDrop::new(rstr),
        }
    }

    #[inline(always)]
    fn array(info: ArrayInfo) -> Self {
        Self {
            array: ManuallyDrop::new(info),
        }
    }

    #[inline(always)]
    fn splat(val: Value) -> Self {
        Self { splat: val }
    }

    #[inline(always)]
    fn hash(info: HashInfo) -> Self {
        Self {
            hash: ManuallyDrop::new(Box::new(info)),
        }
    }

    #[inline(always)]
    fn module(info: ClassInfo) -> Self {
        Self {
            module: ManuallyDrop::new(info),
        }
    }

    #[inline(always)]
    fn enumerator(info: FiberContext) -> Self {
        Self {
            enumerator: ManuallyDrop::new(Box::new(info)),
        }
    }

    #[inline(always)]
    fn fiber(info: FiberContext) -> Self {
        Self {
            fiber: ManuallyDrop::new(Box::new(info)),
        }
    }

    #[inline(always)]
    fn proc(info: ProcInfo) -> Self {
        Self {
            proc: ManuallyDrop::new(info),
        }
    }

    #[inline(always)]
    fn method(info: MethodObjInfo) -> Self {
        Self {
            method: ManuallyDrop::new(info),
        }
    }

    #[inline(always)]
    fn regexp(info: RegexpInfo) -> Self {
        Self {
            regexp: ManuallyDrop::new(info),
        }
    }

    #[inline(always)]
    fn exception(info: RubyError) -> Self {
        Self {
            exception: ManuallyDrop::new(Box::new(info)),
        }
    }

    #[inline(always)]
    fn binding(info: HeapCtxRef) -> Self {
        Self { binding: info }
    }

    #[inline(always)]
    fn time(info: TimeInfo) -> Self {
        Self {
            time: ManuallyDrop::new(info),
        }
    }
}

impl RValue {
    #[inline(always)]
    pub fn complex(&self) -> &RubyComplex {
        unsafe { &*self.kind.complex }
    }

    #[inline(always)]
    pub fn float(&self) -> f64 {
        unsafe { self.kind.float }
    }

    #[inline(always)]
    pub fn bignum(&self) -> &BigInt {
        unsafe { &*self.kind.bignum }
    }

    #[inline(always)]
    pub fn range(&self) -> &RangeInfo {
        unsafe { &*self.kind.range }
    }

    #[inline(always)]
    pub fn string(&self) -> &RString {
        unsafe { &*self.kind.string }
    }

    #[inline(always)]
    pub fn string_mut(&mut self) -> &mut RString {
        unsafe { &mut *self.kind.string }
    }

    #[inline(always)]
    pub fn array(&self) -> &ArrayInfo {
        unsafe { &*self.kind.array }
    }

    #[inline(always)]
    pub fn array_mut(&mut self) -> &mut ArrayInfo {
        unsafe { &mut *self.kind.array }
    }

    #[inline(always)]
    pub fn splat(&self) -> Value {
        unsafe { self.kind.splat }
    }

    #[inline(always)]
    pub fn rhash(&self) -> &HashInfo {
        unsafe { &**self.kind.hash }
    }

    #[inline(always)]
    pub fn rhash_mut(&mut self) -> &mut HashInfo {
        unsafe { &mut **self.kind.hash }
    }

    #[inline(always)]
    pub fn module(&self) -> &ClassInfo {
        unsafe { &*self.kind.module }
    }

    #[inline(always)]
    pub fn module_mut(&mut self) -> &mut ClassInfo {
        unsafe { &mut *self.kind.module }
    }

    #[inline(always)]
    pub fn enumerator(&self) -> &FiberContext {
        unsafe { &*self.kind.enumerator }
    }

    #[inline(always)]
    pub fn enumerator_mut(&mut self) -> &mut FiberContext {
        unsafe { &mut *self.kind.enumerator }
    }

    #[inline(always)]
    pub fn fiber(&self) -> &FiberContext {
        unsafe { &*self.kind.fiber }
    }

    #[inline(always)]
    pub fn fiber_mut(&mut self) -> &mut FiberContext {
        unsafe { &mut *self.kind.fiber }
    }

    #[inline(always)]
    pub fn proc(&self) -> &ProcInfo {
        unsafe { &*self.kind.proc }
    }

    #[inline(always)]
    pub fn method(&self) -> &MethodObjInfo {
        unsafe { &*self.kind.method }
    }

    #[inline(always)]
    pub fn regexp(&self) -> &RegexpInfo {
        unsafe { &*self.kind.regexp }
    }

    #[inline(always)]
    pub fn exception(&self) -> &RubyError {
        unsafe { &**self.kind.exception }
    }

    #[inline(always)]
    pub fn binding(&self) -> HeapCtxRef {
        unsafe { self.kind.binding }
    }

    #[inline(always)]
    pub fn time(&self) -> &TimeInfo {
        unsafe { &*self.kind.time }
    }

    #[inline(always)]
    pub fn time_mut(&mut self) -> &mut TimeInfo {
        unsafe { &mut *self.kind.time }
    }
}

impl RValue {
    #[inline(always)]
    pub fn kind(&self) -> u8 {
        let flag = unsafe { self.flags.flag };
        assert!(flag & 0b1 == 1, "{:?}", self);
        (flag >> 8) as u8
    }

    #[inline(always)]
    pub fn kind_or_none(&self) -> Option<u8> {
        let flag = unsafe { self.flags.flag };
        if flag & 0b1 == 1 {
            Some((flag >> 8) as u8)
        } else {
            None
        }
    }
}

impl RValue {
    // This type of equality is used for comparison for keys of Hash.
    pub(crate) fn eql(&self, other: &Self) -> bool {
        match (self.kind(), other.kind()) {
            (ObjKind::ORDINARY, ObjKind::ORDINARY) => self.id() == other.id(),
            (ObjKind::BIGNUM, ObjKind::BIGNUM) => *self.bignum() == *other.bignum(),
            (ObjKind::FLOAT, ObjKind::FLOAT) => self.float() == other.float(),
            (ObjKind::COMPLEX, ObjKind::COMPLEX) => {
                self.complex().r.eql(&other.complex().r) && self.complex().i.eql(&other.complex().i)
            }
            (ObjKind::STRING, ObjKind::STRING) => *self.string() == *other.string(),
            (ObjKind::ARRAY, ObjKind::ARRAY) => {
                let lhs = &*self.array();
                let rhs = &*other.array();
                if lhs.len() != rhs.len() {
                    return false;
                }
                lhs.iter().zip(rhs.iter()).all(|(a1, a2)| {
                    // Support self-containing arrays.
                    if self.id() == a1.id() && other.id() == a2.id() {
                        true
                    } else if self.id() == a1.id() || other.id() == a2.id() {
                        false
                    } else {
                        a1.eql(a2)
                    }
                })
            }
            (ObjKind::RANGE, ObjKind::RANGE) => self.range().eql(&other.range()),
            (ObjKind::HASH, ObjKind::HASH) => *self.rhash() == *other.rhash(),
            (ObjKind::METHOD, ObjKind::METHOD) => *self.method() == *other.method(),
            (ObjKind::UNBOUND_METHOD, ObjKind::UNBOUND_METHOD) => *self.method() == *other.method(),
            (ObjKind::INVALID, _) => panic!("Invalid rvalue. (maybe GC problem) {:?}", self),
            (_, ObjKind::INVALID) => panic!("Invalid rvalue. (maybe GC problem) {:?}", other),
            _ => false,
        }
    }
}

impl GC<RValue> for RValue {
    fn mark(&self, alloc: &mut Allocator<RValue>) {
        if alloc.gc_check_and_mark(self) {
            return;
        }
        self.class.mark(alloc);
        match &self.var_table {
            Some(table) => table.values().for_each(|v| v.mark(alloc)),
            None => {}
        }
        match self.kind() {
            ObjKind::INVALID => panic!(
                "Invalid rvalue. (maybe GC problem) {:?} {:#?}",
                self as *const RValue, self
            ),
            ObjKind::ORDINARY
            | ObjKind::BIGNUM
            | ObjKind::FLOAT
            | ObjKind::STRING
            | ObjKind::REGEXP
            | ObjKind::TIME
            | ObjKind::EXCEPTION => {}
            ObjKind::COMPLEX => {
                let RubyComplex { r, i } = *self.complex();
                r.mark(alloc);
                i.mark(alloc);
            }
            ObjKind::MODULE | ObjKind::CLASS => self.module().mark(alloc),
            ObjKind::ARRAY => self.array().mark(alloc),
            ObjKind::HASH => self.rhash().mark(alloc),
            ObjKind::RANGE => {
                let RangeInfo { start, end, .. } = *self.range();
                start.mark(alloc);
                end.mark(alloc);
            }
            ObjKind::SPLAT => self.splat().mark(alloc),
            ObjKind::PROC => self.proc().mark(alloc),
            ObjKind::METHOD | ObjKind::UNBOUND_METHOD => self.method().mark(alloc),
            ObjKind::ENUMERATOR => self.enumerator().mark(alloc),
            ObjKind::FIBER => self.fiber().mark(alloc),
            ObjKind::BINDING => self.binding().mark(alloc),
            _ => unreachable!("{:?}", self),
        }
    }
}

impl PartialEq for RValue {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl GCBox for RValue {
    fn free(&mut self) {
        unsafe {
            if let Some(k) = self.kind_or_none() {
                match k {
                    ObjKind::INVALID => panic!("Invalid rvalue. (maybe GC problem) {:?}", &self),
                    ObjKind::BIGNUM => ManuallyDrop::drop(&mut self.kind.bignum),
                    ObjKind::MODULE | ObjKind::CLASS => ManuallyDrop::drop(&mut self.kind.module),
                    ObjKind::STRING => ManuallyDrop::drop(&mut self.kind.string),
                    ObjKind::ARRAY => ManuallyDrop::drop(&mut self.kind.array),
                    ObjKind::HASH => ManuallyDrop::drop(&mut self.kind.hash),
                    ObjKind::REGEXP => ManuallyDrop::drop(&mut self.kind.regexp),
                    ObjKind::FIBER => ManuallyDrop::drop(&mut self.kind.fiber),
                    ObjKind::ENUMERATOR => ManuallyDrop::drop(&mut self.kind.enumerator),
                    ObjKind::TIME => ManuallyDrop::drop(&mut self.kind.time),
                    ObjKind::EXCEPTION => ManuallyDrop::drop(&mut self.kind.exception),
                    ObjKind::BINDING => {}
                    _ => {}
                }
                self.set_next_none();
                self.var_table = None;
            }
        }
    }

    #[inline(always)]
    fn next(&self) -> Option<std::ptr::NonNull<RValue>> {
        let next = unsafe { self.flags.next };
        assert!(unsafe { std::mem::transmute::<_, u64>(next) } & 0b1 != 1);
        next
    }

    #[inline(always)]
    fn set_next_none(&mut self) {
        self.flags.next = None;
    }

    #[inline(always)]
    fn set_next(&mut self, next: *mut RValue) {
        self.flags.next = Some(std::ptr::NonNull::new(next).unwrap());
    }

    #[inline(always)]
    fn new_invalid() -> Self {
        RValue {
            flags: RVFlag { next: None },
            class: Module::default(),
            kind: ObjKind::other(),
            var_table: None,
        }
    }
}

impl RValue {
    #[inline(always)]
    pub(crate) fn id(&self) -> u64 {
        self as *const RValue as u64
    }

    pub(crate) fn shallow_dup(&self) -> Self {
        RValue {
            flags: self.flags,
            class: self.class,
            var_table: self.var_table.clone(),
            kind: match self.kind() {
                ObjKind::INVALID => panic!("Invalid rvalue. (maybe GC problem) {:?}", &self),
                ObjKind::COMPLEX => {
                    let RubyComplex { r, i } = *self.complex();
                    ObjKind::complex(r.shallow_dup(), i.shallow_dup())
                }
                ObjKind::ARRAY => ObjKind::array(self.array().clone()),
                ObjKind::MODULE | ObjKind::CLASS => ObjKind::module(self.module().clone()),
                ObjKind::ENUMERATOR => ObjKind::other(), //ObjKind::enumerator((**self.enumerator()).clone()),
                ObjKind::FIBER => ObjKind::other(),      //ObjKind::fiber((**self.fiber()).clone()),
                ObjKind::FLOAT => ObjKind {
                    float: self.float(),
                },
                ObjKind::BIGNUM => ObjKind::bignum(self.bignum().clone()),
                ObjKind::HASH => ObjKind::hash(self.rhash().clone()),
                ObjKind::METHOD | ObjKind::UNBOUND_METHOD => ObjKind::method(self.method().clone()),
                ObjKind::ORDINARY => ObjKind::other(),
                ObjKind::PROC => ObjKind::proc(self.proc().clone()),
                ObjKind::RANGE => ObjKind::range(self.range().clone()),
                ObjKind::REGEXP => ObjKind::regexp(self.regexp().clone()),
                ObjKind::SPLAT => ObjKind::splat(self.splat()),
                ObjKind::STRING => ObjKind::string(self.string().clone()),
                ObjKind::TIME => ObjKind::time(self.time().clone()),
                ObjKind::EXCEPTION => ObjKind::exception(self.exception().clone()),
                ObjKind::BINDING => ObjKind::binding(self.binding()),
                _ => unreachable!(),
            },
        }
    }

    pub(crate) fn class_name(&self) -> String {
        self.real_class().name()
    }

    pub(crate) fn inspect(&self) -> Result<String, RubyError> {
        let mut s = format! {"#<{}:0x{:016x}", self.class_name(), self.id()};
        match self.var_table() {
            Some(table) => {
                for (k, v) in table {
                    s = format!("{} {:?}={:?}", s, k, *v);
                }
            }
            None => {}
        }

        Ok(s + ">")
    }

    pub(crate) fn to_s(&self) -> String {
        format! {"#<{}:0x{:016x}>", self.class_name(), self.id()}
    }

    #[inline(always)]
    pub(crate) fn new(kind: u8, class: Module, objkind: ObjKind) -> Self {
        RValue {
            flags: RVFlag::new(kind),
            class,
            kind: objkind,
            var_table: None,
        }
    }

    pub(crate) fn new_bootstrap_class(cinfo: ClassInfo) -> Self {
        RValue::new(
            ObjKind::CLASS,
            Module::default(), // dummy for boot strapping
            ObjKind::module(cinfo),
        )
    }

    pub(crate) fn new_bigint(bigint: BigInt) -> Self {
        RValue::new(
            ObjKind::BIGNUM,
            BuiltinClass::integer(),
            ObjKind::bignum(bigint),
        )
    }

    pub(crate) fn new_float(f: f64) -> Self {
        RValue::new(ObjKind::FLOAT, BuiltinClass::float(), ObjKind { float: f })
    }

    pub(crate) fn new_complex(r: Value, i: Value) -> Self {
        RValue::new(
            ObjKind::COMPLEX,
            BuiltinClass::complex(),
            ObjKind::complex(r, i),
        )
    }

    pub(crate) fn new_string_from_rstring(rs: RString) -> Self {
        RValue::new(ObjKind::STRING, BuiltinClass::string(), ObjKind::string(rs))
    }

    pub(crate) fn new_string<'a>(s: impl Into<Cow<'a, str>>) -> Self {
        RValue::new_string_from_rstring(RString::from(s))
    }

    pub(crate) fn new_bytes(b: Vec<u8>) -> Self {
        RValue::new_string_from_rstring(RString::Bytes(b))
    }

    pub(crate) fn new_ordinary(class: Module) -> Self {
        RValue::new(ObjKind::ORDINARY, class, ObjKind::other())
    }

    pub(crate) fn new_class_with_class(class: Module, cinfo: ClassInfo) -> Self {
        RValue::new(ObjKind::CLASS, class, ObjKind::module(cinfo))
    }

    pub(crate) fn new_class(cinfo: ClassInfo) -> Self {
        RValue::new(
            ObjKind::CLASS,
            BuiltinClass::class(),
            ObjKind::module(cinfo),
        )
    }

    pub(crate) fn new_module(cinfo: ClassInfo) -> Self {
        RValue::new(
            ObjKind::MODULE,
            BuiltinClass::module(),
            ObjKind::module(cinfo),
        )
    }

    pub(crate) fn new_array(array_info: ArrayInfo) -> Self {
        RValue::new(
            ObjKind::ARRAY,
            BuiltinClass::array(),
            ObjKind::array(array_info),
        )
    }

    pub(crate) fn new_array_with_class(array_info: ArrayInfo, class: Module) -> Self {
        RValue::new(ObjKind::ARRAY, class, ObjKind::array(array_info))
    }

    pub(crate) fn new_range(range: RangeInfo) -> Self {
        RValue::new(ObjKind::RANGE, BuiltinClass::range(), ObjKind::range(range))
    }

    pub(crate) fn new_splat(val: Value) -> Self {
        RValue::new(ObjKind::SPLAT, BuiltinClass::array(), ObjKind::splat(val))
    }

    pub(crate) fn new_hash(hash: HashInfo) -> Self {
        RValue::new(ObjKind::HASH, BuiltinClass::hash(), ObjKind::hash(hash))
    }

    pub(crate) fn new_regexp(regexp: RegexpInfo) -> Self {
        RValue::new(
            ObjKind::REGEXP,
            BuiltinClass::regexp(),
            ObjKind::regexp(regexp),
        )
    }

    pub(crate) fn new_proc(proc_info: ProcInfo) -> Self {
        RValue::new(
            ObjKind::PROC,
            BuiltinClass::procobj(),
            ObjKind::proc(proc_info),
        )
    }

    pub(crate) fn new_method(method_info: MethodObjInfo) -> Self {
        RValue::new(
            ObjKind::METHOD,
            BuiltinClass::method(),
            ObjKind::method(method_info),
        )
    }

    pub(crate) fn new_unbound_method(method_info: MethodObjInfo) -> Self {
        RValue::new(
            ObjKind::UNBOUND_METHOD,
            BuiltinClass::unbound_method(),
            ObjKind::method(method_info),
        )
    }

    pub(crate) fn new_fiber(vm: VM, context: HeapCtxRef) -> Self {
        let fiber = FiberContext::new_fiber(vm, context);
        RValue::new(ObjKind::FIBER, BuiltinClass::fiber(), ObjKind::fiber(fiber))
    }

    pub(crate) fn new_enumerator(fiber: FiberContext) -> Self {
        RValue::new(
            ObjKind::ENUMERATOR,
            BuiltinClass::enumerator(),
            ObjKind::enumerator(fiber),
        )
    }

    pub(crate) fn new_time(time_class: Module, time: TimeInfo) -> Self {
        RValue::new(ObjKind::TIME, time_class, ObjKind::time(time))
    }

    pub(crate) fn new_exception(exception_class: Module, err: RubyError) -> Self {
        let message = Value::string(err.message());
        let mut backtrace = vec![];
        for pos in 0..err.info.len() {
            backtrace.push(Value::string(err.get_location(pos)));
        }
        let backtrace = Value::array_from(backtrace);
        let mut rval = RValue::new(ObjKind::EXCEPTION, exception_class, ObjKind::exception(err));
        rval.set_var(IdentId::get_id("@message"), message);
        rval.set_var(IdentId::get_id("@backtrace"), backtrace);
        rval
    }

    pub(crate) fn new_binding(ctx: HeapCtxRef) -> Self {
        RValue::new(
            ObjKind::BINDING,
            BuiltinClass::binding(),
            ObjKind::binding(ctx),
        )
    }
}

impl RValue {
    /// Pack `self` into `Value`(64-bit data representation).
    ///
    /// This method consumes `self` and allocates it on the heap, returning `Value`,
    /// a wrapped raw pointer.  
    pub(crate) fn pack(self) -> Value {
        let ptr = ALLOC.with(|alloc| alloc.borrow_mut().alloc(self));
        Value::from_ptr(ptr)
    }

    /// Return a class of the object.
    ///
    /// If the objetct has a sigleton class, return the singleton class.
    #[inline(always)]
    pub(crate) fn class(&self) -> Module {
        self.class
    }

    /// Return a "real" class of the object.
    pub(crate) fn real_class(&self) -> Module {
        let mut class = self.class;
        loop {
            if class.is_singleton() {
                class = class.superclass().unwrap();
            } else {
                return class;
            }
        }
    }

    /// Set a class of the object.
    pub(crate) fn set_class(&mut self, class: Module) {
        self.class = class;
    }

    #[inline(always)]
    pub(crate) fn get_var(&self, id: IdentId) -> Option<Value> {
        match &self.var_table {
            Some(table) => table.get(&id).cloned(),
            None => None,
        }
    }

    #[inline(always)]
    pub(crate) fn get_mut_var(&mut self, id: IdentId) -> Option<&mut Value> {
        match &mut self.var_table {
            Some(table) => table.get_mut(&id),
            None => None,
        }
    }

    /// Set `val` for `id` in variable table. <br>
    /// Return Some(old_value) or None if no old value exists.
    pub(crate) fn set_var(&mut self, id: IdentId, val: Value) -> Option<Value> {
        match &mut self.var_table {
            Some(table) => table.insert(id, val),
            None => {
                let mut table = FxHashMap::default();
                let v = table.insert(id, val);
                self.var_table = Some(Box::new(table));
                v
            }
        }
    }

    pub(crate) fn var_table(&self) -> Option<&ValueTable> {
        match &self.var_table {
            Some(table) => Some(table),
            None => None,
        }
    }
}
