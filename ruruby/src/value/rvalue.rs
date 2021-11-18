use crate::coroutine::*;
use crate::*;
use num::BigInt;
use std::borrow::Cow;
use std::default;

/// Heap-allocated objects.
#[derive(Debug)]
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

impl std::fmt::Debug for RVFlag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        unsafe {
            if self.flag & 0b1 == 1 {
                write!(f, "FLAG {}", self.flag)
            } else {
                write!(f, "NEXT {:?}", self.next)
            }
        }
    }
}

impl default::Default for RVFlag {
    #[inline(always)]
    fn default() -> Self {
        Self { flag: 1 }
    }
}

#[derive(Debug)]
pub enum ObjKind {
    Invalid,
    Ordinary,
    BigNum(BigInt),
    Float(f64),
    Complex { r: Value, i: Value },
    Module(ClassInfo),
    String(RString),
    Array(ArrayInfo),
    Range(RangeInfo),
    Splat(Value), // internal use only.
    Hash(Box<HashInfo>),
    Proc(ProcInfo),
    Regexp(RegexpInfo),
    Method(MethodObjInfo),
    Fiber(Box<FiberContext>),
    Enumerator(Box<FiberContext>),
    Time(Box<TimeInfo>),
    Exception(Box<RubyError>),
    Binding(HeapCtxRef),
}

impl RValue {
    // This type of equality is used for comparison for keys of Hash.
    pub(crate) fn eql(&self, other: &Self) -> bool {
        match (&self.kind, &other.kind) {
            (ObjKind::Ordinary, ObjKind::Ordinary) => self.id() == other.id(),
            (ObjKind::BigNum(lhs), ObjKind::BigNum(rhs)) => *lhs == *rhs,
            (ObjKind::Float(lhs), ObjKind::Float(rhs)) => *lhs == *rhs,
            (ObjKind::Complex { r: r1, i: i1 }, ObjKind::Complex { r: r2, i: i2 }) => {
                r1.eql(r2) && i1.eql(i2)
            }
            (ObjKind::String(lhs), ObjKind::String(rhs)) => *lhs == *rhs,
            (ObjKind::Array(lhs), ObjKind::Array(rhs)) => {
                if lhs.len() != rhs.len() {
                    return false;
                }
                lhs.elements
                    .iter()
                    .zip(rhs.elements.iter())
                    .all(|(a1, a2)| {
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
            (ObjKind::Range(lhs), ObjKind::Range(rhs)) => lhs.eql(rhs),
            (ObjKind::Hash(lhs), ObjKind::Hash(rhs)) => lhs == rhs,
            (ObjKind::Method(lhs), ObjKind::Method(rhs)) => *lhs == *rhs,
            (ObjKind::Invalid, _) => panic!("Invalid rvalue. (maybe GC problem) {:?}", self),
            (_, ObjKind::Invalid) => panic!("Invalid rvalue. (maybe GC problem) {:?}", other),
            _ => false,
        }
    }
}

impl GC for RValue {
    fn mark(&self, alloc: &mut Allocator) {
        if alloc.gc_check_and_mark(self) {
            return;
        }
        self.class.mark(alloc);
        match &self.var_table {
            Some(table) => table.values().for_each(|v| v.mark(alloc)),
            None => {}
        }
        match &self.kind {
            ObjKind::Invalid => panic!(
                "Invalid rvalue. (maybe GC problem) {:?} {:#?}",
                self as *const RValue, self
            ),
            &ObjKind::Ordinary
            | ObjKind::BigNum(_)
            | ObjKind::Float(_)
            | ObjKind::String(_)
            | ObjKind::Regexp(_)
            | ObjKind::Time(_)
            | ObjKind::Exception(_) => {}
            ObjKind::Complex { r, i } => {
                r.mark(alloc);
                i.mark(alloc);
            }
            ObjKind::Module(cref) => cref.mark(alloc),
            ObjKind::Array(aref) => aref.mark(alloc),
            ObjKind::Hash(href) => href.mark(alloc),
            ObjKind::Range(RangeInfo { start, end, .. }) => {
                start.mark(alloc);
                end.mark(alloc);
            }
            ObjKind::Splat(v) => v.mark(alloc),
            ObjKind::Proc(pref) => pref.mark(alloc),
            ObjKind::Method(mref) => mref.mark(alloc),
            ObjKind::Enumerator(fref) | ObjKind::Fiber(fref) => fref.mark(alloc),
            ObjKind::Binding(cref) => cref.mark(alloc),
        }
    }
}

impl PartialEq for RValue {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl RValue {
    #[inline(always)]
    pub(crate) fn free(&mut self) {
        //#[cfg(feature = "gc-debug")]
        //assert!(self.is_invalid());
        self.kind = ObjKind::Invalid;
        self.var_table = None;
    }

    #[inline(always)]
    pub(crate) fn next(&self) -> Option<std::ptr::NonNull<RValue>> {
        let next = unsafe { self.flags.next };
        assert!(unsafe { std::mem::transmute::<_, u64>(next) } & 0b1 != 1);
        next
    }

    #[inline(always)]
    pub(crate) fn set_next_none(&mut self) {
        self.flags.next = None;
    }

    #[inline(always)]
    pub(crate) fn set_next(&mut self, next: *mut RValue) {
        self.flags.next = Some(std::ptr::NonNull::new(next).unwrap());
    }
}

impl RValue {
    #[inline(always)]
    pub(crate) fn id(&self) -> u64 {
        self as *const RValue as u64
    }

    #[cfg(feature = "gc-debug")]
    pub(crate) fn is_invalid(&self) -> bool {
        matches!(self.kind, ObjKind::Invalid)
    }

    pub(crate) fn shallow_dup(&self) -> Self {
        RValue {
            flags: self.flags,
            class: self.class,
            var_table: self.var_table.clone(),
            kind: match &self.kind {
                ObjKind::Invalid => panic!("Invalid rvalue. (maybe GC problem) {:?}", &self),
                ObjKind::Complex { r, i } => ObjKind::Complex {
                    r: r.shallow_dup(),
                    i: i.shallow_dup(),
                },
                ObjKind::Array(aref) => ObjKind::Array(aref.clone()),
                ObjKind::Module(cinfo) => ObjKind::Module(cinfo.clone()),
                ObjKind::Enumerator(_eref) => ObjKind::Ordinary,
                ObjKind::Fiber(_fref) => ObjKind::Ordinary,
                ObjKind::Float(num) => ObjKind::Float(*num),
                ObjKind::BigNum(bigint) => ObjKind::BigNum(bigint.clone()),
                ObjKind::Hash(hinfo) => ObjKind::Hash(hinfo.clone()),
                ObjKind::Method(hinfo) => ObjKind::Method(hinfo.clone()),
                ObjKind::Ordinary => ObjKind::Ordinary,
                ObjKind::Proc(pref) => ObjKind::Proc(pref.clone()),
                ObjKind::Range(info) => ObjKind::Range(info.clone()),
                ObjKind::Regexp(rref) => ObjKind::Regexp(rref.clone()),
                ObjKind::Splat(v) => ObjKind::Splat(*v),
                ObjKind::String(rstr) => ObjKind::String(rstr.clone()),
                ObjKind::Time(time) => ObjKind::Time(time.clone()),
                ObjKind::Exception(err) => ObjKind::Exception(err.clone()),
                ObjKind::Binding(ctx) => ObjKind::Binding(*ctx),
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
    pub(crate) fn new(class: Module, kind: ObjKind) -> Self {
        RValue {
            flags: RVFlag::default(),
            class,
            kind,
            var_table: None,
        }
    }

    #[inline(always)]
    pub(crate) fn new_invalid() -> Self {
        RValue {
            flags: RVFlag { next: None },
            class: Module::default(),
            kind: ObjKind::Invalid,
            var_table: None,
        }
    }

    pub(crate) fn new_bootstrap(cinfo: ClassInfo) -> Self {
        RValue::new(
            Module::default(), // dummy for boot strapping
            ObjKind::Module(cinfo),
        )
    }

    pub(crate) fn new_bigint(bigint: BigInt) -> Self {
        RValue::new(BuiltinClass::integer(), ObjKind::BigNum(bigint))
    }

    pub(crate) fn new_float(f: f64) -> Self {
        RValue::new(BuiltinClass::float(), ObjKind::Float(f))
    }

    pub(crate) fn new_complex(r: Value, i: Value) -> Self {
        RValue::new(BuiltinClass::complex(), ObjKind::Complex { r, i })
    }

    pub(crate) fn new_string_from_rstring(rs: RString) -> Self {
        RValue::new(BuiltinClass::string(), ObjKind::String(rs))
    }

    pub(crate) fn new_string<'a>(s: impl Into<Cow<'a, str>>) -> Self {
        RValue::new_string_from_rstring(RString::from(s))
    }

    pub(crate) fn new_bytes(b: Vec<u8>) -> Self {
        RValue::new_string_from_rstring(RString::Bytes(b))
    }

    pub(crate) fn new_ordinary(class: Module) -> Self {
        RValue::new(class, ObjKind::Ordinary)
    }

    pub(crate) fn new_class(cinfo: ClassInfo) -> Self {
        RValue::new(BuiltinClass::class(), ObjKind::Module(cinfo))
    }

    pub(crate) fn new_module(cinfo: ClassInfo) -> Self {
        RValue::new(BuiltinClass::module(), ObjKind::Module(cinfo))
    }

    pub(crate) fn new_array(array_info: ArrayInfo) -> Self {
        RValue::new(BuiltinClass::array(), ObjKind::Array(array_info))
    }

    pub(crate) fn new_array_with_class(array_info: ArrayInfo, class: Module) -> Self {
        RValue::new(class, ObjKind::Array(array_info))
    }

    pub(crate) fn new_range(range: RangeInfo) -> Self {
        RValue::new(BuiltinClass::range(), ObjKind::Range(range))
    }

    pub(crate) fn new_splat(val: Value) -> Self {
        RValue::new(BuiltinClass::array(), ObjKind::Splat(val))
    }

    pub(crate) fn new_hash(hash: HashInfo) -> Self {
        RValue::new(BuiltinClass::hash(), ObjKind::Hash(Box::new(hash)))
    }

    pub(crate) fn new_regexp(regexp: RegexpInfo) -> Self {
        RValue::new(BuiltinClass::regexp(), ObjKind::Regexp(regexp))
    }

    pub(crate) fn new_proc(proc_info: ProcInfo) -> Self {
        RValue::new(BuiltinClass::procobj(), ObjKind::Proc(proc_info))
    }

    pub(crate) fn new_method(method_info: MethodObjInfo) -> Self {
        RValue::new(BuiltinClass::method(), ObjKind::Method(method_info))
    }

    pub(crate) fn new_unbound_method(method_info: MethodObjInfo) -> Self {
        RValue::new(BuiltinClass::unbound_method(), ObjKind::Method(method_info))
    }

    pub(crate) fn new_fiber(vm: VM, context: HeapCtxRef) -> Self {
        let fiber = FiberContext::new_fiber(vm, context);
        RValue::new(BuiltinClass::fiber(), ObjKind::Fiber(Box::new(fiber)))
    }

    pub(crate) fn new_enumerator(fiber: FiberContext) -> Self {
        RValue::new(
            BuiltinClass::enumerator(),
            ObjKind::Enumerator(Box::new(fiber)),
        )
    }

    pub(crate) fn new_time(time_class: Module, time: TimeInfo) -> Self {
        RValue::new(time_class, ObjKind::Time(Box::new(time)))
    }

    pub(crate) fn new_exception(exception_class: Module, err: RubyError) -> Self {
        let message = Value::string(err.message());
        let mut backtrace = vec![];
        for pos in 0..err.info.len() {
            backtrace.push(Value::string(err.get_location(pos)));
        }
        let backtrace = Value::array_from(backtrace);
        let mut rval = RValue::new(exception_class, ObjKind::Exception(Box::new(err)));
        rval.set_var(IdentId::get_id("@message"), message);
        rval.set_var(IdentId::get_id("@backtrace"), backtrace);
        rval
    }

    pub(crate) fn new_binding(ctx: HeapCtxRef) -> Self {
        RValue::new(BuiltinClass::binding(), ObjKind::Binding(ctx))
    }
}

impl RValue {
    /// Pack `self` into `Value`(64-bit data representation).
    ///
    /// This method consumes `self` and allocates it on the heap, returning `Value`,
    /// a wrapped raw pointer.  
    pub(crate) fn pack(self) -> Value {
        let ptr = ALLOC.with(|alloc| {
            alloc.borrow_mut().alloc(self)
            //assert!((ptr as u64) & 0b111 == 0);
        });
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

    pub(crate) fn get_var(&self, id: IdentId) -> Option<Value> {
        match &self.var_table {
            Some(table) => table.get(&id).cloned(),
            None => None,
        }
    }

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
