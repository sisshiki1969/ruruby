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

    pub integer: PackedValue,
    pub array: PackedValue,
    pub class: PackedValue,
    pub module: PackedValue,
    pub procobj: PackedValue,
    pub method: PackedValue,
    pub range: PackedValue,
    pub hash: PackedValue,
    pub regexp: PackedValue,
    pub string: PackedValue,
    pub object: PackedValue,

    //pub integer_class: ClassRef,
    //pub array_class: ClassRef,
    pub class_class: ClassRef,
    pub module_class: ClassRef,
    //pub proc_class: ClassRef,
    //pub method_class: ClassRef,
    //pub range_class: ClassRef,
    //pub hash_class: ClassRef,
    //pub regexp_class: ClassRef,
    //pub string_class: ClassRef,
    pub object_class: ClassRef,
}

impl Globals {
    pub fn new() -> Self {
        let mut ident_table = IdentifierTable::new();
        let object_id = IdentId::OBJECT;
        let module_id = ident_table.get_ident_id("Module");
        let class_id = ident_table.get_ident_id("Class");
        let object_class = ClassRef::from_no_superclass(object_id);
        let nil = PackedValue::nil();
        let object = PackedValue::bootstrap_class(object_class);
        let module_class = ClassRef::from(module_id, object);
        let module = PackedValue::bootstrap_class(module_class);
        let class_class = ClassRef::from(class_id, module);
        let class = PackedValue::bootstrap_class(class_class);
        object.as_object().unwrap().class = class;
        module.as_object().unwrap().class = class;
        class.as_object().unwrap().class = class;

        let main_object = ObjectRef::from(object);
        let mut globals = Globals {
            ident_table,
            method_table: GlobalMethodTable::new(),
            method_cache: MethodCache::new(),
            class_version: 0,
            main_object,
            object_class,
            module_class,
            class_class,
            object,
            module,
            class,
            integer: nil, // dummy
            array: nil,   // dummy
            procobj: nil, // dummy
            method: nil,  // dummy
            range: nil,   // dummy
            hash: nil,    // dummy
            regexp: nil,  // dummy
            string: nil,  // dummy
        };
        // Generate singleton class for Object
        let mut singleton_class = ClassRef::from(None, globals.class);
        singleton_class.is_singleton = true;
        let singleton_obj = PackedValue::class(&globals, singleton_class);
        globals.object.as_object().unwrap().singleton = Some(singleton_obj);

        object::init_object(&mut globals);
        module::init_module(&mut globals);
        class::init_class(&mut globals);
        globals.integer = integer::init_integer(&mut globals);
        globals.array = array::init_array(&mut globals);
        globals.procobj = procobj::init_proc(&mut globals);
        globals.method = method::init_method(&mut globals);
        globals.range = range::init_range(&mut globals);
        globals.string = string::init_string(&mut globals);
        globals.hash = hash::init_hash(&mut globals);
        globals.regexp = regexp::init_regexp(&mut globals);
        globals
    }
    pub fn add_builtin_method(&mut self, name: impl Into<String>, func: BuiltinFunc) {
        let name = name.into();
        let id = self.get_ident_id(&name);
        let info = MethodInfo::BuiltinFunc { name, func };
        let methodref = self.add_method(info);
        self.add_object_method(id, methodref);
    }

    pub fn get_ident_name(&self, id: impl Into<Option<IdentId>>) -> &str {
        let id = id.into();
        match id {
            Some(id) => self.ident_table.get_name(id),
            None => &"",
        }
    }

    pub fn get_ident_id(&mut self, name: impl Into<String>) -> IdentId {
        self.ident_table.get_ident_id(name)
    }

    pub fn add_object_method(&mut self, id: IdentId, info: MethodRef) {
        self.object_class.method_table.insert(id, info);
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

    pub fn get_singleton_class(&self, obj: PackedValue) -> Result<PackedValue, ()> {
        match obj.unpack() {
            Value::Object(mut obj) => match obj.singleton {
                Some(class) => Ok(class),
                None => {
                    let mut singleton_class = if let ObjKind::Class(cref) = obj.kind {
                        let superclass = cref.superclass;
                        if superclass.is_nil() {
                            ClassRef::from_no_superclass(None)
                        } else {
                            ClassRef::from(None, self.get_singleton_class(superclass)?)
                        }
                    } else {
                        ClassRef::from_no_superclass(None)
                    };
                    singleton_class.is_singleton = true;
                    let singleton_obj = PackedValue::class(&self, singleton_class);
                    obj.singleton = Some(singleton_obj);
                    Ok(singleton_obj)
                }
            },
            _ => Err(()),
        }
    }

    pub fn add_builtin_class_method(
        &mut self,
        obj: PackedValue,
        name: impl Into<String>,
        func: BuiltinFunc,
    ) {
        let name = name.into();
        let id = self.get_ident_id(&name);
        let info = MethodInfo::BuiltinFunc { name, func };
        let func_ref = self.add_method(info);
        let singleton = self.get_singleton_class(obj).unwrap();
        singleton
            .as_class()
            .unwrap()
            .method_table
            .insert(id, func_ref);
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
        classref.method_table.insert(id, methodref);
    }

    pub fn get_class_name(&self, val: PackedValue) -> String {
        match val.unpack() {
            Value::Uninitialized => "[Uninitialized]".to_string(),
            Value::Nil => "NilClass".to_string(),
            Value::Bool(true) => "TrueClass".to_string(),
            Value::Bool(false) => "FalseClass".to_string(),
            Value::FixNum(_) => "Integer".to_string(),
            Value::FloatNum(_) => "Float".to_string(),
            Value::String(_) => "String".to_string(),
            Value::Symbol(_) => "Symbol".to_string(),
            Value::Range(_) => "Range".to_string(),
            Value::Char(_) => "Char".to_string(),
            Value::Object(oref) => match oref.kind {
                ObjKind::Array(_) => "Array".to_string(),
                ObjKind::SplatArray(_) => "[SplatArray]".to_string(),
                ObjKind::Hash(_) => "Hash".to_string(),
                ObjKind::Regexp(_) => "Regexp".to_string(),

                ObjKind::Class(_) => "Class".to_string(),
                ObjKind::Module(_) => "Module".to_string(),
                ObjKind::Proc(_) => "Proc".to_string(),
                ObjKind::Method(_) => "Method".to_string(),
                ObjKind::Ordinary => self.get_ident_name(oref.class().name).to_string(),
            },
        }
    }
}

impl Globals {
    pub fn set_method_cache_entry(
        &mut self,
        id: usize,
        class: ClassRef,
        //is_class_method: bool,
        method: MethodRef,
    ) {
        self.method_cache.table[id] = Some(MethodCacheEntry {
            class,
            version: self.class_version,
            //is_class_method,
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
        rec_class: ClassRef,
    ) -> Option<MethodRef> {
        match self.get_method_cache_entry(cache_slot) {
            Some(MethodCacheEntry {
                class,
                version,
                method,
            }) if *class == rec_class && *version == self.class_version => Some(*method),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MethodCacheEntry {
    class: ClassRef,
    version: usize,
    //is_class_method: bool,
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
