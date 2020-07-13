use std::collections::HashMap;
//#[macro_use]
use crate::*;

/// Heap-allocated objects.
#[derive(Debug, Clone, PartialEq)]
pub struct RValue {
    class: Value,
    var_table: Option<Box<ValueTable>>,
    pub kind: ObjKind,
}

#[derive(Clone, PartialEq)]
pub enum ObjKind {
    Invalid,
    Ordinary,
    Integer(i64),
    Float(f64),
    Class(ClassRef),
    Module(ClassRef),
    String(RString),
    Array(Box<ArrayInfo>),
    Range(RangeInfo),
    Splat(Value), // internal use only.
    Hash(Box<HashInfo>),
    Proc(Box<ProcInfo>),
    Regexp(RegexpInfo),
    Method(Box<MethodObjInfo>),
    Fiber(FiberRef),
    Enumerator(Box<EnumInfo>),
}

impl std::fmt::Debug for ObjKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjKind::Invalid => write!(f, "[Invalid]"),
            ObjKind::Ordinary => write!(f, "Ordinary"),
            ObjKind::String(rs) => write!(f, "String:{:?}", rs),
            ObjKind::Integer(i) => write!(f, "{}", *i),
            ObjKind::Float(i) => write!(f, "{}", *i),
            ObjKind::Class(cref) => match cref.name {
                Some(id) => write!(f, "{}", IdentId::get_ident_name(id)),
                None => write!(f, "#<Class:0x{:x}>", cref.id()),
            },
            ObjKind::Module(cref) => match cref.name {
                Some(id) => write!(f, "{}", IdentId::get_ident_name(id)),
                None => write!(f, "#<Module:0x{:x}>", cref.id()),
            },
            ObjKind::Array(aref) => {
                write!(f, "Array[")?;
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
            ObjKind::Enumerator(eref) => eref.mark(alloc),
            ObjKind::Fiber(fref) => fref.mark(alloc),
            _ => {}
        }
    }
}

impl RValue {
    pub fn free(&mut self) -> bool {
        match &mut self.kind {
            ObjKind::Invalid => return false,
            ObjKind::Class(c) | ObjKind::Module(c) => c.free(),
            ObjKind::String(rs) => drop(rs),
            ObjKind::Array(a) => drop(a),
            ObjKind::Range(r) => drop(r),
            ObjKind::Hash(h) => drop(h),
            ObjKind::Proc(p) => drop(p),
            ObjKind::Regexp(r) => drop(r),
            ObjKind::Method(m) => drop(m),
            ObjKind::Fiber(f) => f.free(),
            ObjKind::Enumerator(e) => drop(e),
            _ => {}
        }
        match &mut self.var_table {
            Some(t) => drop(t),
            None => {}
        }
        *self = RValue::new_invalid();
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
                ObjKind::Array(aref) => ObjKind::Array(aref.clone()),
                ObjKind::Class(cref) => ObjKind::Class(cref.dup()),
                ObjKind::Enumerator(eref) => ObjKind::Enumerator(eref.clone()),
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
                    let id = IdentId::get_ident_name(*k);
                    s = format!("{} {}={}", s, id, inspect);
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

    pub fn new_fixnum(i: i64) -> Self {
        RValue {
            class: Value::nil(),
            var_table: None,
            kind: ObjKind::Integer(i),
        }
    }

    pub fn new_flonum(f: f64) -> Self {
        RValue {
            class: Value::nil(),
            var_table: None,
            kind: ObjKind::Float(f),
        }
    }

    pub fn new_string(globals: &Globals, s: String) -> Self {
        RValue {
            class: globals.builtins.string,
            var_table: None,
            kind: ObjKind::String(RString::Str(s)),
        }
    }

    pub fn new_bytes(globals: &Globals, b: Vec<u8>) -> Self {
        RValue {
            class: globals.builtins.string,
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

    pub fn new_class(globals: &Globals, classref: ClassRef) -> Self {
        RValue {
            class: globals.builtins.class,
            var_table: None,
            kind: ObjKind::Class(classref),
        }
    }

    pub fn new_module(globals: &Globals, classref: ClassRef) -> Self {
        RValue {
            class: globals.builtins.module,
            var_table: None,
            kind: ObjKind::Module(classref),
        }
    }

    pub fn new_array(globals: &Globals, array_info: ArrayInfo) -> Self {
        RValue {
            class: globals.builtins.array,
            var_table: None,
            kind: ObjKind::Array(Box::new(array_info)),
        }
    }

    pub fn new_range(globals: &Globals, range: RangeInfo) -> Self {
        RValue {
            class: globals.builtins.range,
            var_table: None,
            kind: ObjKind::Range(range),
        }
    }

    pub fn new_splat(globals: &Globals, val: Value) -> Self {
        RValue {
            class: globals.builtins.array,
            var_table: None,
            kind: ObjKind::Splat(val),
        }
    }

    pub fn new_hash(globals: &Globals, hash: HashInfo) -> Self {
        RValue {
            class: globals.builtins.hash,
            var_table: None,
            kind: ObjKind::Hash(Box::new(hash)),
        }
    }

    pub fn new_regexp(globals: &Globals, regexpref: RegexpInfo) -> Self {
        RValue {
            class: globals.builtins.regexp,
            var_table: None,
            kind: ObjKind::Regexp(regexpref),
        }
    }

    pub fn new_proc(globals: &Globals, proc_info: ProcInfo) -> Self {
        RValue {
            class: globals.builtins.procobj,
            var_table: None,
            kind: ObjKind::Proc(Box::new(proc_info)),
        }
    }

    pub fn new_method(globals: &Globals, method_info: MethodObjInfo) -> Self {
        RValue {
            class: globals.builtins.method,
            var_table: None,
            kind: ObjKind::Method(Box::new(method_info)),
        }
    }

    pub fn new_fiber(
        globals: &Globals,
        vm: VMRef,
        context: ContextRef,
        rec: std::sync::mpsc::Receiver<VMResult>,
        tx: std::sync::mpsc::SyncSender<usize>,
    ) -> Self {
        let fiber = FiberInfo::new(vm, context, rec, tx);
        RValue {
            class: globals.builtins.fiber,
            var_table: None,
            kind: ObjKind::Fiber(FiberRef::new(fiber)),
        }
    }

    pub fn new_enumerator(globals: &Globals, fiber: FiberInfo) -> Self {
        let enum_info = EnumInfo::new(fiber);
        RValue {
            class: globals.builtins.enumerator,
            var_table: None,
            kind: ObjKind::Enumerator(Box::new(enum_info)),
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
                let mut table = HashMap::new();
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
            self.var_table = Some(Box::new(HashMap::new()));
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
