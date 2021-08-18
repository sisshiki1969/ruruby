use crate::coroutine::*;
use crate::*;
use num::BigInt;
use std::borrow::Cow;

/// Heap-allocated objects.
#[derive(Debug)]
pub struct RValue {
    class: Module,
    var_table: Option<Box<ValueTable>>,
    pub kind: ObjKind,
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
    Binding(ContextRef),
}

impl RValue {
    // This type of equality is used for comparison for keys of Hash.
    pub fn eql(&self, other: &Self) -> bool {
        match (&self.kind, &other.kind) {
            (ObjKind::Ordinary, ObjKind::Ordinary) => self.id() == other.id(),
            (ObjKind::BigNum(lhs), ObjKind::BigNum(rhs)) => *lhs == *rhs,
            (ObjKind::Float(lhs), ObjKind::Float(rhs)) => *lhs == *rhs,
            (ObjKind::Complex { r: r1, i: i1 }, ObjKind::Complex { r: r2, i: i2 }) => {
                r1.eql(r2) && i1.eql(i2)
            }
            (ObjKind::String(lhs), ObjKind::String(rhs)) => *lhs == *rhs,
            (ObjKind::Array(lhs), ObjKind::Array(rhs)) => lhs.eql(rhs),
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
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl RValue {
    pub fn free(&mut self) -> bool {
        if self.is_invalid() {
            return false;
        };
        self.kind = ObjKind::Invalid;
        self.var_table = None;
        true
    }
}

impl RValue {
    pub fn id(&self) -> u64 {
        self as *const RValue as u64
    }

    pub fn is_invalid(&self) -> bool {
        match self.kind {
            ObjKind::Invalid => true,
            _ => false,
        }
    }

    pub fn shallow_dup(&self) -> Self {
        RValue {
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

    pub fn class_name(&self) -> String {
        self.real_class().name()
    }

    pub fn inspect(&self) -> Result<String, RubyError> {
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

    pub fn to_s(&self) -> String {
        format! {"#<{}:0x{:016x}>", self.class_name(), self.id()}
    }

    pub fn new(class: Module, kind: ObjKind) -> Self {
        RValue {
            class,
            kind,
            var_table: None,
        }
    }

    pub fn new_invalid() -> Self {
        RValue::new(Module::default(), ObjKind::Invalid)
    }

    pub fn new_bootstrap(cinfo: ClassInfo) -> Self {
        RValue::new(
            Module::default(), // dummy for boot strapping
            ObjKind::Module(cinfo),
        )
    }

    pub fn new_bigint(bigint: BigInt) -> Self {
        RValue::new(BuiltinClass::integer(), ObjKind::BigNum(bigint))
    }

    pub fn new_float(f: f64) -> Self {
        RValue::new(BuiltinClass::float(), ObjKind::Float(f))
    }

    pub fn new_complex(r: Value, i: Value) -> Self {
        RValue::new(BuiltinClass::complex(), ObjKind::Complex { r, i })
    }

    pub fn new_string_from_rstring(rs: RString) -> Self {
        RValue::new(BuiltinClass::string(), ObjKind::String(rs))
    }

    pub fn new_string<'a>(s: impl Into<Cow<'a, str>>) -> Self {
        RValue::new_string_from_rstring(RString::from(s))
    }

    pub fn new_bytes(b: Vec<u8>) -> Self {
        RValue::new_string_from_rstring(RString::Bytes(b))
    }

    pub fn new_ordinary(class: Module) -> Self {
        RValue::new(class, ObjKind::Ordinary)
    }

    pub fn new_class(cinfo: ClassInfo) -> Self {
        RValue::new(BuiltinClass::class(), ObjKind::Module(cinfo))
    }

    pub fn new_module(cinfo: ClassInfo) -> Self {
        RValue::new(BuiltinClass::module(), ObjKind::Module(cinfo))
    }

    pub fn new_array(array_info: ArrayInfo) -> Self {
        RValue::new(BuiltinClass::array(), ObjKind::Array(array_info))
    }

    pub fn new_array_with_class(array_info: ArrayInfo, class: Module) -> Self {
        RValue::new(class, ObjKind::Array(array_info))
    }

    pub fn new_range(range: RangeInfo) -> Self {
        RValue::new(BuiltinClass::range(), ObjKind::Range(range))
    }

    pub fn new_splat(val: Value) -> Self {
        RValue::new(BuiltinClass::array(), ObjKind::Splat(val))
    }

    pub fn new_hash(hash: HashInfo) -> Self {
        RValue::new(BuiltinClass::hash(), ObjKind::Hash(Box::new(hash)))
    }

    pub fn new_regexp(regexp: RegexpInfo) -> Self {
        RValue::new(BuiltinClass::regexp(), ObjKind::Regexp(regexp))
    }

    pub fn new_proc(proc_info: ProcInfo) -> Self {
        RValue::new(BuiltinClass::procobj(), ObjKind::Proc(proc_info))
    }

    pub fn new_method(method_info: MethodObjInfo) -> Self {
        RValue::new(BuiltinClass::method(), ObjKind::Method(method_info))
    }

    pub fn new_unbound_method(method_info: MethodObjInfo) -> Self {
        RValue::new(BuiltinClass::unbound_method(), ObjKind::Method(method_info))
    }

    pub fn new_fiber(vm: VM, context: ContextRef) -> Self {
        let fiber = FiberContext::new_fiber(vm, context);
        RValue::new(BuiltinClass::fiber(), ObjKind::Fiber(Box::new(fiber)))
    }

    pub fn new_enumerator(fiber: FiberContext) -> Self {
        RValue::new(
            BuiltinClass::enumerator(),
            ObjKind::Enumerator(Box::new(fiber)),
        )
    }

    pub fn new_time(time_class: Module, time: TimeInfo) -> Self {
        RValue::new(time_class, ObjKind::Time(Box::new(time)))
    }

    pub fn new_exception(exception_class: Module, err: RubyError) -> Self {
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

    pub fn new_binding(ctx: ContextRef) -> Self {
        RValue::new(BuiltinClass::binding(), ObjKind::Binding(ctx))
    }
}

impl RValue {
    /// Pack `self` into `Value`(64-bit data representation).
    ///
    /// This method consumes `self` and allocates it on the heap, returning `Value`,
    /// a wrapped raw pointer.  
    pub fn pack(self) -> Value {
        let ptr = ALLOC.with(|alloc| {
            alloc.borrow_mut().alloc(self)
            //assert!((ptr as u64) & 0b111 == 0);
        });
        Value::from_ptr(ptr)
    }

    /// Return a class of the object.
    ///
    /// If the objetct has a sigleton class, return the singleton class.
    pub fn class(&self) -> Module {
        self.class
    }

    /// Return a "real" class of the object.
    pub fn real_class(&self) -> Module {
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
    pub fn set_class(&mut self, class: Module) {
        self.class = class;
    }

    pub fn get_var(&self, id: IdentId) -> Option<Value> {
        match &self.var_table {
            Some(table) => table.get(&id).cloned(),
            None => None,
        }
    }

    pub fn get_mut_var(&mut self, id: IdentId) -> Option<&mut Value> {
        match &mut self.var_table {
            Some(table) => table.get_mut(&id),
            None => None,
        }
    }

    /// Set `val` for `id` in variable table. <br>
    /// Return Some(old_value) or None if no old value exists.
    pub fn set_var(&mut self, id: IdentId, val: Value) -> Option<Value> {
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

    pub fn var_table(&self) -> Option<&ValueTable> {
        match &self.var_table {
            Some(table) => Some(table),
            None => None,
        }
    }

    pub fn var_table_mut(&mut self) -> &mut ValueTable {
        if self.var_table.is_none() {
            self.var_table = Some(Box::new(FxHashMap::default()));
        }
        self.var_table.as_deref_mut().unwrap()
    }
}
