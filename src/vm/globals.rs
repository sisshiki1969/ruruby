use crate::vm::*;

#[derive(Debug, Clone)]
pub struct Globals {
    // Global info
    pub ident_table: IdentifierTable,
    method_table: GlobalMethodTable,
    method_cache: MethodCache,
    pub instant: std::time::Instant,
    /// version counter: increment when new instance / class methods are defined.
    pub class_version: usize,
    pub main_object: Value,
    pub builtins: BuiltinClass,
    pub class_class: ClassRef,
    pub module_class: ClassRef,
    pub object_class: ClassRef,

    case_dispatch: CaseDispatchMap,
}

#[derive(Debug, Clone)]
pub struct BuiltinClass {
    pub integer: Value,
    pub array: Value,
    pub class: Value,
    pub module: Value,
    pub procobj: Value,
    pub method: Value,
    pub range: Value,
    pub hash: Value,
    pub regexp: Value,
    pub string: Value,
    pub object: Value,
}

impl BuiltinClass {
    fn new(object: Value, module: Value, class: Value) -> Self {
        let nil = Value::nil();
        BuiltinClass {
            integer: nil,
            array: nil,
            class,
            module,
            procobj: nil,
            method: nil,
            range: nil,
            hash: nil,
            regexp: nil,
            string: nil,
            object,
        }
    }
}

impl Globals {
    pub fn new() -> Self {
        let mut ident_table = IdentifierTable::new();
        let object_id = IdentId::OBJECT;
        let module_id = ident_table.get_ident_id("Module");
        let class_id = ident_table.get_ident_id("Class");
        let object_class = ClassRef::from(object_id, None);
        let object = Value::bootstrap_class(object_class);
        let module_class = ClassRef::from(module_id, object);
        let module = Value::bootstrap_class(module_class);
        let class_class = ClassRef::from(class_id, module);
        let class = Value::bootstrap_class(class_class);

        object.as_object().set_class(class);
        module.as_object().set_class(class);
        class.as_object().set_class(class);
        let builtins = BuiltinClass::new(object, module, class);

        let main_object = Value::ordinary_object(object);
        let mut globals = Globals {
            ident_table,
            method_table: GlobalMethodTable::new(),
            method_cache: MethodCache::new(),
            instant: std::time::Instant::now(),
            class_version: 0,
            main_object,
            object_class,
            module_class,
            class_class,
            builtins,
            case_dispatch: CaseDispatchMap::new(),
        };
        // Generate singleton class for Object
        let mut singleton_class = ClassRef::from(None, globals.builtins.class);
        singleton_class.is_singleton = true;
        let singleton_obj = Value::class(&globals, singleton_class);
        globals.builtins.object.as_object().set_class(singleton_obj);

        module::init_module(&mut globals);
        class::init_class(&mut globals);
        globals.builtins.integer = init_integer(&mut globals);
        globals.builtins.array = init_array(&mut globals);
        globals.builtins.procobj = init_proc(&mut globals);
        globals.builtins.method = init_method(&mut globals);
        globals.builtins.range = init_range(&mut globals);
        globals.builtins.string = init_string(&mut globals);
        globals.builtins.hash = init_hash(&mut globals);
        globals.builtins.regexp = init_regexp(&mut globals);
        init_object(&mut globals);
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

    pub fn get_object_method(&self, id: IdentId) -> Option<MethodRef> {
        self.builtins.object.get_instance_method(id)
    }

    pub fn add_method(&mut self, info: MethodInfo) -> MethodRef {
        self.method_table.add_method(info)
    }

    pub fn new_method(&mut self) -> MethodRef {
        self.method_table.new_method()
    }

    pub fn set_method(&mut self, method: MethodRef, info: MethodInfo) {
        self.method_table.set_method(method, info);
    }

    pub fn get_method_info(&self, method: MethodRef) -> &MethodInfo {
        self.method_table.get_method(method)
    }

    pub fn get_mut_method_info(&mut self, method: MethodRef) -> &mut MethodInfo {
        self.method_table.get_mut_method(method)
    }

    pub fn get_singleton_class(&self, obj: Value) -> Result<Value, ()> {
        match obj.is_object() {
            Some(mut oref) => {
                let class = oref.class();
                if class.as_class().is_singleton {
                    Ok(class)
                } else {
                    let mut singleton_class = if let ObjKind::Class(cref) = oref.kind {
                        let superclass = cref.superclass;
                        if superclass.is_nil() {
                            ClassRef::from(None, None)
                        } else {
                            ClassRef::from(None, self.get_singleton_class(superclass)?)
                        }
                    } else {
                        ClassRef::from(None, None)
                    };
                    singleton_class.is_singleton = true;
                    let singleton_obj = Value::class(&self, singleton_class);
                    singleton_obj.as_object().set_class(class);
                    oref.set_class(singleton_obj);
                    Ok(singleton_obj)
                }
            }
            _ => Err(()),
        }
    }

    pub fn add_builtin_class_method(
        &mut self,
        obj: Value,
        name: impl Into<String>,
        func: BuiltinFunc,
    ) {
        let name = name.into();
        let id = self.get_ident_id(&name);
        let info = MethodInfo::BuiltinFunc { name, func };
        let func_ref = self.add_method(info);
        let singleton = self.get_singleton_class(obj).unwrap();
        singleton.as_class().method_table.insert(id, func_ref);
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

    pub fn get_class_name(&self, val: Value) -> String {
        match val.unpack() {
            RValue::Uninitialized => "[Uninitialized]".to_string(),
            RValue::Nil => "NilClass".to_string(),
            RValue::Bool(true) => "TrueClass".to_string(),
            RValue::Bool(false) => "FalseClass".to_string(),
            RValue::FixNum(_) => "Integer".to_string(),
            RValue::FloatNum(_) => "Float".to_string(),
            RValue::String(_) => "String".to_string(),
            RValue::Symbol(_) => "Symbol".to_string(),
            RValue::Char(_) => "Char".to_string(),
            RValue::Object(oref) => match oref.kind {
                ObjKind::Array(_) => "Array".to_string(),
                ObjKind::Splat(_) => "[Splat]".to_string(),
                ObjKind::Hash(_) => "Hash".to_string(),
                ObjKind::Regexp(_) => "Regexp".to_string(),
                ObjKind::Range(_) => "Range".to_string(),
                ObjKind::Class(_) => "Class".to_string(),
                ObjKind::Module(_) => "Module".to_string(),
                ObjKind::Proc(_) => "Proc".to_string(),
                ObjKind::Method(_) => "Method".to_string(),
                ObjKind::Ordinary => self
                    .get_ident_name(oref.as_ref().search_class().as_class().name)
                    .to_string(),
            },
        }
    }
}

impl Globals {
    pub fn set_method_cache_entry(&mut self, id: u32, class: Value, method: MethodRef) {
        self.method_cache.table[id as usize] = Some(MethodCacheEntry {
            class,
            version: self.class_version,
            method,
        });
    }

    pub fn add_method_cache_entry(&mut self) -> u32 {
        self.method_cache.add_entry()
    }

    fn get_method_cache_entry(&self, id: u32) -> &Option<MethodCacheEntry> {
        self.method_cache.get_entry(id)
    }

    pub fn get_method_from_cache(
        &mut self,
        cache_slot: u32,
        rec_class: Value,
    ) -> Option<MethodRef> {
        match self.get_method_cache_entry(cache_slot) {
            Some(MethodCacheEntry {
                class,
                version,
                method,
            }) if class.id() == rec_class.id() && *version == self.class_version => Some(*method),
            _ => None,
        }
    }
}

impl Globals {
    pub fn new_case_dispatch_map(&mut self) -> u32 {
        self.case_dispatch.new_entry()
    }

    pub fn get_case_dispatch_map(&self, id: u32) -> &HashMap<Value, i32> {
        self.case_dispatch.get_entry(id)
    }

    pub fn get_mut_case_dispatch_map(&mut self, id: u32) -> &mut HashMap<Value, i32> {
        self.case_dispatch.get_mut_entry(id)
    }
}

#[derive(Debug, Clone)]
pub struct MethodCacheEntry {
    class: Value,
    version: usize,
    //is_class_method: bool,
    method: MethodRef,
}

#[derive(Debug, Clone)]
pub struct MethodCache {
    table: Vec<Option<MethodCacheEntry>>,
    id: u32,
}

impl MethodCache {
    fn new() -> Self {
        MethodCache {
            table: vec![],
            id: 0,
        }
    }
    fn add_entry(&mut self) -> u32 {
        self.id += 1;
        self.table.push(None);
        self.id - 1
    }

    fn get_entry(&self, id: u32) -> &Option<MethodCacheEntry> {
        &self.table[id as usize]
    }
}

#[derive(Debug, Clone)]
pub struct CaseDispatchMap {
    table: Vec<HashMap<Value, i32>>,
    id: u32,
}

impl CaseDispatchMap {
    fn new() -> Self {
        CaseDispatchMap {
            table: vec![],
            id: 0,
        }
    }
    fn new_entry(&mut self) -> u32 {
        self.id += 1;
        self.table.push(HashMap::new());
        self.id - 1
    }
    fn get_entry(&self, id: u32) -> &HashMap<Value, i32> {
        &self.table[id as usize]
    }
    fn get_mut_entry(&mut self, id: u32) -> &mut HashMap<Value, i32> {
        &mut self.table[id as usize]
    }
}
