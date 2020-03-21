use crate::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Globals {
    // Global info
    pub ident_table: IdentifierTable,
    method_table: GlobalMethodTable,
    inline_cache: InlineCache,
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
            inline_cache: InlineCache::new(),
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

        init_module(&mut globals);
        init_class(&mut globals);
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
                    let mut singleton_class = match oref.kind {
                        ObjKind::Class(cref) | ObjKind::Module(cref) => {
                            let superclass = cref.superclass;
                            if superclass.is_nil() {
                                ClassRef::from(None, None)
                            } else {
                                ClassRef::from(None, self.get_singleton_class(superclass)?)
                            }
                        }
                        _ => ClassRef::from(None, None),
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
            RV::Uninitialized => "[Uninitialized]".to_string(),
            RV::Nil => "NilClass".to_string(),
            RV::Bool(true) => "TrueClass".to_string(),
            RV::Bool(false) => "FalseClass".to_string(),
            RV::FixNum(_) => "Integer".to_string(),
            RV::FloatNum(_) => "Float".to_string(),
            RV::Symbol(_) => "Symbol".to_string(),
            RV::Object(oref) => match oref.kind {
                ObjKind::String(_) => "String".to_string(),
                ObjKind::Array(_) => "Array".to_string(),
                ObjKind::Range(_) => "Range".to_string(),
                ObjKind::Splat(_) => "[Splat]".to_string(),
                ObjKind::Hash(_) => "Hash".to_string(),
                ObjKind::Regexp(_) => "Regexp".to_string(),
                ObjKind::Class(_) => "Class".to_string(),
                ObjKind::Module(_) => "Module".to_string(),
                ObjKind::Proc(_) => "Proc".to_string(),
                ObjKind::Method(_) => "Method".to_string(),
                ObjKind::Ordinary => self
                    .get_ident_name(oref.search_class().as_class().name)
                    .to_string(),
                ObjKind::FixNum(_) => "Integer".to_string(),
                ObjKind::FloatNum(_) => "Float".to_string(),
            },
        }
    }

    pub fn val_inspect(&self, val: Value) -> String {
        match val.is_object() {
            Some(mut oref) => match &oref.kind {
                ObjKind::String(s) => match s {
                    RString::Str(s) => format!("\"{}\"", s.replace("\\", "\\\\")),
                    RString::Bytes(b) => match String::from_utf8(b.clone()) {
                        Ok(s) => format!("\"{}\"", s.replace("\\", "\\\\")),
                        Err(_) => "<ByteArray>".to_string(),
                    },
                },
                ObjKind::Class(cref) => match cref.name {
                    Some(id) => format! {"{}", self.get_ident_name(id)},
                    None => format! {"#<Class:0x{:x}>", cref.id()},
                },
                ObjKind::Module(cref) => match cref.name {
                    Some(id) => format! {"{}", self.get_ident_name(id)},
                    None => format! {"#<Module:0x{:x}>", cref.id()},
                },
                ObjKind::Array(aref) => match aref.elements.len() {
                    0 => "[]".to_string(),
                    1 => format!("[{}]", self.val_inspect(aref.elements[0])),
                    len => {
                        let mut result = self.val_inspect(aref.elements[0]);
                        for i in 1..len {
                            result = format!("{}, {}", result, self.val_inspect(aref.elements[i]));
                        }
                        format! {"[{}]", result}
                    }
                },
                ObjKind::Hash(href) => match href.len() {
                    0 => "{}".to_string(),
                    _ => {
                        let mut result = "".to_string();
                        let mut first = true;
                        for (k, v) in href.iter() {
                            result = if first {
                                format!("{} => {}", self.val_inspect(k), self.val_inspect(v))
                            } else {
                                format!(
                                    "{}, {} => {}",
                                    result,
                                    self.val_inspect(k),
                                    self.val_inspect(v)
                                )
                            };
                            first = false;
                        }

                        format! {"{{{}}}", result}
                    }
                },
                ObjKind::Regexp(rref) => format!("/{}/", rref.regexp.as_str().to_string()),
                ObjKind::Ordinary => {
                    let mut s = format! {"#<{}:0x{:x}", self.get_ident_name(oref.search_class().as_class().name), oref.id()};
                    for (k, v) in oref.var_table() {
                        s = format!("{} {}={}", s, self.get_ident_name(*k), self.val_inspect(*v));
                    }
                    format!("{}>", s)
                }
                _ => format!("{:?}", oref.kind),
            },
            None => match val.unpack() {
                RV::Nil => "nil".to_string(),
                _ => format!("{:?}", val),
            },
        }
    }
}

impl Globals {
    pub fn set_inline_cache_entry(&mut self, id: u32, class: Value, method: MethodRef) {
        self.inline_cache.table[id as usize] = Some(InlineCacheEntry {
            class,
            version: self.class_version,
            method,
        });
    }

    pub fn add_inline_cache_entry(&mut self) -> u32 {
        self.inline_cache.add_entry()
    }

    fn get_inline_cache_entry(&self, id: u32) -> &Option<InlineCacheEntry> {
        self.inline_cache.get_entry(id)
    }

    pub fn get_method_from_inline_cache(
        &mut self,
        cache_slot: u32,
        rec_class: Value,
    ) -> Option<MethodRef> {
        match self.get_inline_cache_entry(cache_slot) {
            Some(InlineCacheEntry {
                class,
                version,
                method,
            }) if class.id() == rec_class.id() && *version == self.class_version => Some(*method),
            _ => None,
        }
    }
}

impl Globals {
    pub fn add_method_cache_entry(&mut self, class: Value, id: IdentId, method: MethodRef) {
        self.method_cache
            .add_entry(class, id, self.class_version, method);
    }

    pub fn get_method_cache_entry(&self, class: Value, id: IdentId) -> Option<&MethodCacheEntry> {
        self.method_cache.get_entry(class, id)
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

//-------------------------------------------------------------------------------------------------------------
//
//  Global method cache
//  This supports method cache.
//
//-------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MethodCache(HashMap<(Value, IdentId), MethodCacheEntry>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodCacheEntry {
    pub method: MethodRef,
    pub version: usize,
}

impl MethodCache {
    fn new() -> Self {
        MethodCache(HashMap::new())
    }

    fn add_entry(&mut self, class: Value, id: IdentId, version: usize, method: MethodRef) {
        self.0
            .insert((class, id), MethodCacheEntry { method, version });
    }

    fn get_entry(&self, class: Value, id: IdentId) -> Option<&MethodCacheEntry> {
        self.0.get(&(class, id))
    }
}

//-------------------------------------------------------------------------------------------------------------
//
//  Inline method cache
//  This supports method cache embedded in the instruction sequence directly.
//
//-------------------------------------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct InlineCache {
    table: Vec<Option<InlineCacheEntry>>,
    id: u32,
}

#[derive(Debug, Clone)]
pub struct InlineCacheEntry {
    class: Value,
    version: usize,
    //is_class_method: bool,
    method: MethodRef,
}

impl InlineCache {
    fn new() -> Self {
        InlineCache {
            table: vec![],
            id: 0,
        }
    }
    fn add_entry(&mut self) -> u32 {
        self.id += 1;
        self.table.push(None);
        self.id - 1
    }

    fn get_entry(&self, id: u32) -> &Option<InlineCacheEntry> {
        &self.table[id as usize]
    }
}

//-------------------------------------------------------------------------------------------------------------
//
//  Case dispatch map
//  This supports optimization for case syntax when all of the when-conditions were fixnum literals.
//
//-------------------------------------------------------------------------------------------------------------

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
