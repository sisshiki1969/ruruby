use crate::vm::*;

#[derive(Debug, Clone)]
pub struct Globals {
    // Global info
    pub ident_table: IdentifierTable,
    method_table: GlobalMethodTable,
    method_cache: MethodCache,
    /// version counter: increment when new instance / class methods are defined.
    pub class_version: usize,
    pub main_object: ObjectRef,
    pub integer_class: ClassRef,
    pub array_class: ClassRef,
    pub class_class: ClassRef,
    pub module_class: ClassRef,
    pub proc_class: ClassRef,
    pub range_class: ClassRef,
    pub object_class: ClassRef,
}

impl Globals {
    pub fn new(ident_table: Option<IdentifierTable>) -> Self {
        let mut ident_table = match ident_table {
            Some(table) => table,
            None => IdentifierTable::new(),
        };
        let object_id = ident_table.get_ident_id("Object");
        let object_class = ClassRef::from_no_superclass(object_id);
        let main_object = ObjectRef::from(object_class);
        let mut globals = Globals {
            ident_table,
            method_table: GlobalMethodTable::new(),
            method_cache: MethodCache::new(),
            class_version: 0,
            main_object,
            integer_class: object_class,
            array_class: object_class,
            module_class: object_class,
            class_class: object_class,
            proc_class: object_class,
            range_class: object_class,
            object_class,
        };
        object::init_object(&mut globals);
        globals.integer_class = integer::init_integer(&mut globals);
        globals.array_class = array::init_array(&mut globals);
        globals.module_class = module::init_module(&mut globals);
        globals.class_class = class::init_class(&mut globals);
        globals.proc_class = procobj::init_proc(&mut globals);
        globals.range_class = range::init_range(&mut globals);
        globals
    }
    pub fn add_builtin_method(&mut self, name: impl Into<String>, func: BuiltinFunc) {
        let name = name.into();
        let id = self.get_ident_id(&name);
        let info = MethodInfo::BuiltinFunc { name, func };
        let methodref = self.add_method(info);
        self.add_object_method(id, methodref);
    }

    pub fn get_ident_name(&self, id: IdentId) -> &String {
        self.ident_table.get_name(id)
    }

    pub fn get_ident_id(&mut self, name: impl Into<String>) -> IdentId {
        self.ident_table.get_ident_id(&name.into())
    }

    pub fn add_object_method(&mut self, id: IdentId, info: MethodRef) {
        self.object_class.instance_method.insert(id, info);
    }

    pub fn get_object_method(&self, id: IdentId) -> Option<&MethodRef> {
        self.object_class.get_instance_method(id)
    }

    pub fn add_method(&mut self, info: MethodInfo) -> MethodRef {
        self.method_table.add_method(info)
    }

    pub fn get_method_info(&self, method: MethodRef) -> &MethodInfo {
        self.method_table.get_method(method)
    }

    pub fn get_mut_method_info(&mut self, method: MethodRef) -> &mut MethodInfo {
        self.method_table.get_mut_method(method)
    }

    pub fn add_builtin_class_method(
        &mut self,
        mut classref: ClassRef,
        name: impl Into<String>,
        func: BuiltinFunc,
    ) {
        let name = name.into();
        let id = self.get_ident_id(&name);
        let info = MethodInfo::BuiltinFunc { name, func };
        let func_ref = self.add_method(info);
        classref.class_method.insert(id, func_ref);
        //classref.clone().add_class_method(id, func_ref);
    }

    pub fn add_builtin_instance_method(
        &mut self,
        mut classref: ClassRef,
        name: impl Into<String>,
        func: BuiltinFunc,
    ) {
        let name = name.into();
        let id = self.get_ident_id(&name);
        let info = MethodInfo::BuiltinFunc { name, func };
        let methodref = self.add_method(info);
        classref.instance_method.insert(id, methodref);
    }

    pub fn get_class_name(&self, val: PackedValue) -> String {
        match val.unpack() {
            Value::Nil => "NilClass".to_string(),
            Value::Bool(true) => "TrueClass".to_string(),
            Value::Bool(false) => "FalseClass".to_string(),
            Value::FixNum(_) => "Integer".to_string(),
            Value::FloatNum(_) => "Float".to_string(),
            Value::String(_) => "String".to_string(),
            Value::Symbol(_) => "Symbol".to_string(),
            Value::Char(_) => "Char".to_string(),
            Value::Object(oref) => match oref.kind {
                ObjKind::Array(_) => "Array".to_string(),
                ObjKind::Hash(_) => "Hash".to_string(),
                ObjKind::Range(_) => "Range".to_string(),
                ObjKind::Class(_) => "Class".to_string(),
                ObjKind::Module(_) => "Module".to_string(),
                ObjKind::Proc(_) => "Proc".to_string(),
                ObjKind::Ordinary => self.get_ident_name(oref.classref.id).clone(),
            },
        }
    }
}

impl Globals {
    pub fn set_method_cache_entry(
        &mut self,
        id: usize,
        class: ClassRef,
        is_class_method: bool,
        method: MethodRef,
    ) {
        self.method_cache.table[id] = Some(MethodCacheEntry {
            class,
            version: self.class_version,
            is_class_method,
            method,
        });
    }

    pub fn add_method_cache_entry(&mut self) -> usize {
        self.method_cache.add_entry()
    }

    fn get_method_cache_entry(&self, id: usize) -> &Option<MethodCacheEntry> {
        self.method_cache.get_entry(id)
    }

    pub fn get_method_from_cache(
        &mut self,
        cache_slot: usize,
        receiver: PackedValue,
    ) -> Option<MethodRef> {
        let (rec_class, class_method) = match receiver.as_class() {
            Some(cref) => (cref, true),
            None => (receiver.get_class(&self), false),
        };
        match self.get_method_cache_entry(cache_slot) {
            Some(MethodCacheEntry {
                class,
                version,
                is_class_method,
                method,
            }) if *class == rec_class
                && *version == self.class_version
                && *is_class_method == class_method =>
            {
                Some(*method)
            }
            _ => {
                /*
                eprintln!(
                    "cache miss! {:?} {:?} {:?}",
                    receiver.unpack(),
                    rec_class,
                    class_method
                );*/
                None
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct MethodCacheEntry {
    class: ClassRef,
    version: usize,
    is_class_method: bool,
    method: MethodRef,
}

#[derive(Debug, Clone)]
pub struct MethodCache {
    table: Vec<Option<MethodCacheEntry>>,
    id: usize,
}

impl MethodCache {
    fn new() -> Self {
        MethodCache {
            table: vec![],
            id: 0,
        }
    }
    fn add_entry(&mut self) -> usize {
        self.id += 1;
        self.table.push(None);
        self.id - 1
    }

    fn get_entry(&self, id: usize) -> &Option<MethodCacheEntry> {
        &self.table[id]
    }
}
