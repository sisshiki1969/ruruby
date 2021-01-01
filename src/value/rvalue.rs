use crate::*;
use std::borrow::Cow;

/// Heap-allocated objects.
#[derive(Debug, PartialEq)]
pub struct RValue {
    class: Value,
    var_table: Option<Box<ValueTable>>,
    pub kind: ObjKind,
}

#[derive(Debug, PartialEq)]
pub enum ObjKind {
    Invalid,
    Ordinary,
    Integer(i64),
    Float(f64),
    Complex { r: Value, i: Value },
    Class(ClassInfo),
    Module(ClassInfo),
    String(RString),
    Array(ArrayInfo),
    Range(RangeInfo),
    Splat(Value), // internal use only.
    Hash(Box<HashInfo>),
    Proc(ProcInfo),
    Regexp(RegexpInfo),
    Method(MethodObjInfo),
    Fiber(Box<FiberInfo>),
    Enumerator(Box<FiberInfo>),
    Time(TimeInfo),
    Exception(RubyError),
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
            ObjKind::Complex { r, i } => {
                r.mark(alloc);
                i.mark(alloc);
            }
            ObjKind::Class(cref) | ObjKind::Module(cref) => cref.mark(alloc),
            ObjKind::Array(aref) => aref.mark(alloc),
            ObjKind::Hash(href) => href.mark(alloc),
            ObjKind::Range(RangeInfo { start, end, .. }) => {
                start.mark(alloc);
                end.mark(alloc);
            }
            ObjKind::Splat(v) => v.mark(alloc),
            ObjKind::Proc(pref) => pref.context.mark(alloc),
            ObjKind::Method(mref) => mref.mark(alloc),
            ObjKind::Enumerator(fref) | ObjKind::Fiber(fref) => fref.mark(alloc),
            ObjKind::Exception(err) => match &err.kind {
                RubyErrorKind::Value(val) => val.mark(alloc),
                RubyErrorKind::BlockReturn(val) => val.mark(alloc),
                RubyErrorKind::MethodReturn(val) => val.mark(alloc),
                _ => {}
            },
            _ => {}
        }
    }
}

impl RValue {
    pub fn free(&mut self) -> bool {
        if self.kind == ObjKind::Invalid {
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

    pub fn as_ref(&self) -> ObjectRef {
        Ref::from_ref(self)
    }

    pub fn dup(&self) -> Self {
        RValue {
            class: self.class,
            var_table: self.var_table.clone(),
            kind: match &self.kind {
                ObjKind::Invalid => panic!("Invalid rvalue. (maybe GC problem) {:?}", &self),
                ObjKind::Complex { r, i } => ObjKind::Complex {
                    r: r.dup(),
                    i: i.dup(),
                },
                ObjKind::Array(aref) => ObjKind::Array(aref.clone()),
                ObjKind::Class(cinfo) => ObjKind::Class(cinfo.clone()),
                ObjKind::Module(cinfo) => ObjKind::Module(cinfo.clone()),
                ObjKind::Enumerator(_eref) => ObjKind::Ordinary,
                ObjKind::Fiber(_fref) => ObjKind::Ordinary,
                ObjKind::Integer(num) => ObjKind::Integer(*num),
                ObjKind::Float(num) => ObjKind::Float(*num),
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
            },
        }
    }

    pub fn class_name(&self) -> String {
        IdentId::get_ident_name(self.search_class().as_class().name())
    }

    pub fn inspect(&self, vm: &mut VM) -> Result<String, RubyError> {
        let mut s = format! {"#<{}:0x{:x}", self.class_name(), self.id()};
        match self.var_table() {
            Some(table) => {
                for (k, v) in table {
                    let inspect = v.val_to_s(vm)?;
                    s = format!("{} {:?}={}", s, k, inspect);
                }
            }
            None => {}
        }

        Ok(s + ">")
    }

    pub fn new_invalid() -> Self {
        RValue {
            class: Value::nil(),
            kind: ObjKind::Invalid,
            var_table: None,
        }
    }

    pub fn new_bootstrap(cinfo: ClassInfo) -> Self {
        RValue {
            class: Value::nil(), // dummy for boot strapping
            kind: ObjKind::Class(cinfo),
            var_table: None,
        }
    }

    pub fn new_integer(i: i64) -> Self {
        RValue {
            class: BuiltinClass::integer(),
            var_table: None,
            kind: ObjKind::Integer(i),
        }
    }

    pub fn new_float(f: f64) -> Self {
        RValue {
            class: BuiltinClass::float(),
            var_table: None,
            kind: ObjKind::Float(f),
        }
    }

    pub fn new_complex(r: Value, i: Value) -> Self {
        let class = BuiltinClass::complex();
        RValue {
            class,
            var_table: None,
            kind: ObjKind::Complex { r, i },
        }
    }

    pub fn new_string_from_rstring(rs: RString) -> Self {
        RValue {
            class: BuiltinClass::string(),
            var_table: None,
            kind: ObjKind::String(rs),
        }
    }

    pub fn new_string<'a>(s: impl Into<Cow<'a, str>>) -> Self {
        RValue::new_string_from_rstring(RString::from(s))
    }

    pub fn new_bytes(b: Vec<u8>) -> Self {
        RValue::new_string_from_rstring(RString::Bytes(b))
    }

    pub fn new_ordinary(class: Value) -> Self {
        RValue {
            class,
            var_table: None,
            kind: ObjKind::Ordinary,
        }
    }

    pub fn new_class(cinfo: ClassInfo) -> Self {
        RValue {
            class: BuiltinClass::class(),
            var_table: None,
            kind: ObjKind::Class(cinfo),
        }
    }

    pub fn new_module(cinfo: ClassInfo) -> Self {
        RValue {
            class: BuiltinClass::module(),
            var_table: None,
            kind: ObjKind::Module(cinfo),
        }
    }

    pub fn new_array(array_info: ArrayInfo) -> Self {
        RValue {
            class: BuiltinClass::array(),
            var_table: None,
            kind: ObjKind::Array(array_info),
        }
    }

    pub fn new_array_with_class(array_info: ArrayInfo, class: Value) -> Self {
        RValue {
            class,
            var_table: None,
            kind: ObjKind::Array(array_info),
        }
    }

    pub fn new_range(range: RangeInfo) -> Self {
        RValue {
            class: BuiltinClass::range(),
            var_table: None,
            kind: ObjKind::Range(range),
        }
    }

    pub fn new_splat(val: Value) -> Self {
        RValue {
            class: BuiltinClass::array(),
            var_table: None,
            kind: ObjKind::Splat(val),
        }
    }

    pub fn new_hash(hash: HashInfo) -> Self {
        RValue {
            class: BuiltinClass::hash(),
            var_table: None,
            kind: ObjKind::Hash(Box::new(hash)),
        }
    }

    pub fn new_regexp(regexp: RegexpInfo) -> Self {
        RValue {
            class: BuiltinClass::regexp(),
            var_table: None,
            kind: ObjKind::Regexp(regexp),
        }
    }

    pub fn new_proc(proc_info: ProcInfo) -> Self {
        RValue {
            class: BuiltinClass::procobj(),
            var_table: None,
            kind: ObjKind::Proc(proc_info),
        }
    }

    pub fn new_method(method_info: MethodObjInfo) -> Self {
        RValue {
            class: BuiltinClass::method(),
            var_table: None,
            kind: ObjKind::Method(method_info),
        }
    }

    pub fn new_fiber(
        vm: VM,
        context: ContextRef,
        rec: std::sync::mpsc::Receiver<VMResult>,
        tx: std::sync::mpsc::SyncSender<FiberMsg>,
    ) -> Self {
        let fiber = FiberInfo::new(vm, context, rec, tx);
        RValue {
            class: BuiltinClass::fiber(),
            var_table: None,
            kind: ObjKind::Fiber(Box::new(fiber)),
        }
    }

    pub fn new_enumerator(fiber: FiberInfo) -> Self {
        RValue {
            class: BuiltinClass::enumerator(),
            var_table: None,
            kind: ObjKind::Enumerator(Box::new(fiber)),
        }
    }

    pub fn new_time(time_class: Value, time: TimeInfo) -> Self {
        RValue {
            class: time_class,
            var_table: None,
            kind: ObjKind::Time(time),
        }
    }

    pub fn new_exception(exception_class: Value, err: RubyError) -> Self {
        let message = Value::string(err.message());
        let mut backtrace = vec![];
        for pos in 0..err.info.len() {
            backtrace.push(Value::string(err.get_location(pos)));
        }
        let backtrace = Value::array_from(backtrace);
        let mut rval = RValue {
            class: exception_class,
            var_table: None,
            kind: ObjKind::Exception(err),
        };
        rval.set_var(IdentId::get_id("@message"), message);
        rval.set_var(IdentId::get_id("@backtrace"), backtrace);
        rval
    }
}

pub type ObjectRef = Ref<RValue>;

impl RValue {
    /// Pack `self` into `Value`(64-bit data representation).
    ///
    /// This method consumes `self` and allocates it on the heap, returning `Value`,
    /// a wrapped raw pointer.  
    pub fn pack(self) -> Value {
        ALLOC.with(|a| {
            let mut alloc = *a.borrow().as_ref().unwrap();
            let ptr = alloc.alloc(self);
            Value::from_ptr(ptr)
        })
    }

    /// Return a class of the object.
    ///
    /// If the objetct has a sigleton class, return the singleton class.
    pub fn class(&self) -> Value {
        self.class
    }

    /// Return a "real" class of the object.
    pub fn search_class(&self) -> Value {
        let mut class = self.class;
        loop {
            if class.is_singleton() {
                class = class.rvalue().class;
            } else {
                return class;
            }
        }
    }

    /// Set a class of the object.
    pub fn set_class(&mut self, class: Value) {
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
