use crate::*;

/// Heap-allocated objects.
#[derive(Debug, PartialEq)]
pub struct RValue {
    class: Value,
    var_table: Option<Box<ValueTable>>,
    pub kind: ObjKind,
}

#[derive(PartialEq)]
pub enum ObjKind {
    Invalid,
    Ordinary,
    Integer(i64),
    Float(f64),
    Complex { r: Value, i: Value },
    Class(ClassRef),
    Module(ClassRef),
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
}

impl std::fmt::Debug for ObjKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjKind::Invalid => write!(f, "[Invalid]"),
            ObjKind::Ordinary => write!(f, "Ordinary"),
            ObjKind::String(rs) => write!(f, r#""{:?}""#, rs),
            ObjKind::Integer(i) => write!(f, "{}", *i),
            ObjKind::Float(i) => write!(f, "{}", *i),
            ObjKind::Complex { r, i } => {
                let (r, i) = (r.to_real().unwrap(), i.to_real().unwrap());
                if !i.is_negative() {
                    write!(f, "({:?}+{:?}i)", r, i)
                } else {
                    write!(f, "({:?}{:?}i)", r, i)
                }
            }
            ObjKind::Class(cref) => match cref.name {
                Some(id) => write!(f, "{:?}", id),
                None => write!(f, "#<Class:0x{:x}>", cref.id()),
            },
            ObjKind::Module(cref) => match cref.name {
                Some(id) => write!(f, "{:?}", id),
                None => write!(f, "#<Module:0x{:x}>", cref.id()),
            },
            ObjKind::Array(aref) => {
                write!(f, "[")?;
                match aref.elements.len() {
                    0 => {}
                    1 => write!(f, "{:#?}", aref.elements[0])?,
                    2 => write!(f, "{:#?}, {:#?}", aref.elements[0], aref.elements[1])?,
                    3 => write!(
                        f,
                        "{:#?}, {:#?}, {:#?}",
                        aref.elements[0], aref.elements[1], aref.elements[2]
                    )?,
                    n => write!(
                        f,
                        "{:#?}, {:#?}, {:#?}.. {} items",
                        aref.elements[0], aref.elements[1], aref.elements[2], n
                    )?,
                }
                write!(f, "]")
            }
            ObjKind::Hash(href) => {
                write!(f, "{{")?;
                let mut flag = false;
                for (k, v) in href.iter() {
                    if flag {
                        write!(f, ", ")?;
                    }
                    write!(f, "{:#?}=>{:#?}", k, v)?;
                    flag = true;
                }
                write!(f, "}}")
            }
            ObjKind::Range(RangeInfo { start, end, .. }) => {
                write!(f, "Range({:?}, {:?})", start, end)
            }
            ObjKind::Regexp(rref) => write!(f, "/{}/", rref.as_str()),
            ObjKind::Splat(v) => write!(f, "Splat[{:#?}]", v),
            ObjKind::Proc(_) => write!(f, "Proc"),
            ObjKind::Method(_) => write!(f, "Method"),
            ObjKind::Enumerator(_) => write!(f, "Enumerator"),
            ObjKind::Fiber(_) => write!(f, "Fiber"),
            ObjKind::Time(time) => write!(f, "{:?}", time),
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
            _ => {}
        }
    }
}

impl RValue {
    pub fn free(&mut self) -> bool {
        if self.kind == ObjKind::Invalid {
            return false;
        };
        match std::mem::replace(&mut self.kind, ObjKind::Invalid) {
            ObjKind::Invalid => return false,
            ObjKind::Class(c) | ObjKind::Module(c) => c.free(),
            ObjKind::Fiber(mut f) => f.free(),
            ObjKind::Enumerator(mut f) => f.free(),
            ObjKind::Ordinary => {}
            ObjKind::Integer(_) => {}
            ObjKind::Float(_) => {}
            ObjKind::Complex { .. } => {}
            ObjKind::String(_) => {}
            ObjKind::Array(_) => {}
            ObjKind::Range(_) => {}
            ObjKind::Splat(_) => {}
            ObjKind::Hash(_) => {}
            ObjKind::Proc(_) => {}
            ObjKind::Regexp(_) => {}
            ObjKind::Method(_) => {}
            ObjKind::Time(_) => {}
        }
        std::mem::take(&mut self.var_table);
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
                ObjKind::Class(cref) => ObjKind::Class(cref.dup()),
                ObjKind::Enumerator(_eref) => ObjKind::Ordinary,
                ObjKind::Fiber(_fref) => ObjKind::Ordinary,
                ObjKind::Integer(num) => ObjKind::Integer(*num),
                ObjKind::Float(num) => ObjKind::Float(*num),
                ObjKind::Hash(href) => ObjKind::Hash(href.clone()),
                ObjKind::Method(mref) => ObjKind::Method(mref.clone()),
                ObjKind::Module(cref) => ObjKind::Module(cref.dup()),
                ObjKind::Ordinary => ObjKind::Ordinary,
                ObjKind::Proc(pref) => ObjKind::Proc(pref.clone()),
                ObjKind::Range(info) => ObjKind::Range(info.clone()),
                ObjKind::Regexp(rref) => ObjKind::Regexp(rref.clone()),
                ObjKind::Splat(v) => ObjKind::Splat(*v),
                ObjKind::String(rstr) => ObjKind::String(rstr.clone()),
                ObjKind::Time(time) => ObjKind::Time(time.clone()),
            },
        }
    }

    pub fn class_name(&self) -> String {
        IdentId::get_ident_name(self.search_class().as_class().name)
    }

    pub fn to_s(&self) -> String {
        format! {"#<{}:{:?}>", self.class_name(), self}
    }

    pub fn inspect(&self, vm: &mut VM) -> String {
        let mut s = format! {"#<{}:0x{:x}", self.class_name(), self.id()};
        match self.var_table() {
            Some(table) => {
                for (k, v) in table {
                    let inspect = vm.val_to_s(*v);
                    s = format!("{} {:?}={}", s, k, inspect);
                }
            }
            None => {}
        }

        format!("{}>", s)
    }

    pub fn new_invalid() -> Self {
        RValue {
            class: Value::nil(),
            kind: ObjKind::Invalid,
            var_table: None,
        }
    }

    pub fn new_bootstrap(classref: ClassRef) -> Self {
        RValue {
            class: Value::nil(), // dummy for boot strapping
            kind: ObjKind::Class(classref),
            var_table: None,
        }
    }

    pub fn new_integer(i: i64) -> Self {
        RValue {
            class: Value::nil(),
            var_table: None,
            kind: ObjKind::Integer(i),
        }
    }

    pub fn new_float(f: f64) -> Self {
        RValue {
            class: Value::nil(),
            var_table: None,
            kind: ObjKind::Float(f),
        }
    }

    pub fn new_complex(r: Value, i: Value) -> Self {
        let class = BUILTINS.with(|b| b.borrow().unwrap().complex);
        RValue {
            class,
            var_table: None,
            kind: ObjKind::Complex { r, i },
        }
    }

    pub fn new_string(s: String) -> Self {
        RValue {
            class: BuiltinClass::string(),
            var_table: None,
            kind: ObjKind::String(RString::Str(s)),
        }
    }

    pub fn new_bytes(b: Vec<u8>) -> Self {
        RValue {
            class: BuiltinClass::string(),
            var_table: None,
            kind: ObjKind::String(RString::Bytes(b)),
        }
    }

    pub fn new_ordinary(class: Value) -> Self {
        RValue {
            class,
            var_table: None,
            kind: ObjKind::Ordinary,
        }
    }

    pub fn new_class(classref: ClassRef) -> Self {
        RValue {
            class: BuiltinClass::class(),
            var_table: None,
            kind: ObjKind::Class(classref),
        }
    }

    pub fn new_module(classref: ClassRef) -> Self {
        RValue {
            class: BuiltinClass::module(),
            var_table: None,
            kind: ObjKind::Module(classref),
        }
    }

    pub fn new_array(array_info: ArrayInfo) -> Self {
        let class = BUILTINS.with(|b| b.borrow().unwrap().array);
        RValue {
            class,
            var_table: None,
            kind: ObjKind::Array(array_info),
        }
    }

    pub fn new_range(range: RangeInfo) -> Self {
        let class = BUILTINS.with(|b| b.borrow().unwrap().range);
        RValue {
            class,
            var_table: None,
            kind: ObjKind::Range(range),
        }
    }

    pub fn new_splat(val: Value) -> Self {
        let class = BUILTINS.with(|b| b.borrow().unwrap().array);
        RValue {
            class,
            var_table: None,
            kind: ObjKind::Splat(val),
        }
    }

    pub fn new_hash(hash: HashInfo) -> Self {
        let class = BUILTINS.with(|b| b.borrow().unwrap().hash);
        RValue {
            class,
            var_table: None,
            kind: ObjKind::Hash(Box::new(hash)),
        }
    }

    pub fn new_regexp(regexp: RegexpInfo) -> Self {
        let class = BUILTINS.with(|b| b.borrow().unwrap().regexp);
        RValue {
            class,
            var_table: None,
            kind: ObjKind::Regexp(regexp),
        }
    }

    pub fn new_proc(proc_info: ProcInfo) -> Self {
        let class = BUILTINS.with(|b| b.borrow().unwrap().procobj);
        RValue {
            class,
            var_table: None,
            kind: ObjKind::Proc(proc_info),
        }
    }

    pub fn new_method(method_info: MethodObjInfo) -> Self {
        let class = BUILTINS.with(|b| b.borrow().unwrap().method);
        RValue {
            class,
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
        let class = BUILTINS.with(|b| b.borrow().unwrap().fiber);
        RValue {
            class,
            var_table: None,
            kind: ObjKind::Fiber(Box::new(fiber)),
        }
    }

    pub fn new_enumerator(fiber: FiberInfo) -> Self {
        let class = BUILTINS.with(|b| b.borrow().unwrap().enumerator);
        RValue {
            class,
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
}

pub type ObjectRef = Ref<RValue>;

impl RValue {
    /// Pack `self` into `Value`(64-bit data representation).
    /// This method consumes `self` and allocates it on the heap, returning `Value`,
    /// a wrapped raw pointer.  
    pub fn pack(self) -> Value {
        ALLOC.with(|a| {
            let mut alloc = *a.borrow().as_ref().unwrap();
            let ptr = alloc.alloc(self);
            Value::from_ptr(ptr)
        })
    }

    /// Return a class of the object. If the objetct has a sigleton class, return the singleton class.
    pub fn class(&self) -> Value {
        self.class
    }

    /// Return a "real" class of the object.
    pub fn search_class(&self) -> Value {
        let mut class = self.class;
        loop {
            if class.as_class().is_singleton {
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

    pub fn set_var(&mut self, id: IdentId, val: Value) {
        match &mut self.var_table {
            Some(table) => table.insert(id, val),
            None => {
                let mut table = FxHashMap::default();
                let v = table.insert(id, val);
                self.var_table = Some(Box::new(table));
                v
            }
        };
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

    pub fn get_instance_method(&self, id: IdentId) -> Option<MethodRef> {
        self.search_class()
            .as_class()
            .method_table
            .get(&id)
            .cloned()
    }
}
