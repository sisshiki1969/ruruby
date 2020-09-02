use crate::*;
use fancy_regex::Regex;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Globals {
    // Global info
    pub allocator: AllocatorRef,
    pub builtins: BuiltinRef,
    pub const_values: ConstantValues,
    pub global_var: ValueTable,
    pub method_cache: MethodCache,
    pub inline_cache: InlineCache,
    pub case_dispatch: CaseDispatchMap,

    main_fiber: Option<VMRef>,
    pub instant: std::time::Instant,
    /// version counter: increment when new instance / class methods are defined.
    pub class_version: u32,
    pub main_object: Value,
    pub gc_enabled: bool,

    pub fibers: Vec<VMRef>,
    pub regexp_cache: FxHashMap<String, Rc<Regex>>,
}

pub type GlobalsRef = Ref<Globals>;

thread_local!(
    pub static BUILTINS: RefCell<Option<BuiltinRef>> = RefCell::new(None);
);

#[derive(Debug, Clone)]
pub struct BuiltinClass {
    pub integer: Value,
    pub float: Value,
    pub complex: Value,
    pub array: Value,
    pub class: Value,
    pub module: Value,
    pub procobj: Value,
    pub method: Value,
    pub range: Value,
    pub hash: Value,
    pub regexp: Value,
    pub string: Value,
    pub fiber: Value,
    pub object: Value,
    pub enumerator: Value,
}

type BuiltinRef = Ref<BuiltinClass>;

impl BuiltinClass {
    fn new() -> Self {
        let object_id = IdentId::OBJECT;
        let module_id = IdentId::get_id("Module");
        let class_id = IdentId::get_id("Class");
        let object_class = ClassRef::from(object_id, None);
        let mut object = Value::bootstrap_class(object_class);
        let module_class = ClassRef::from(module_id, object);
        let mut module = Value::bootstrap_class(module_class);
        let class_class = ClassRef::from(class_id, module);
        let mut class = Value::bootstrap_class(class_class);

        object.set_class(class);
        module.set_class(class);
        class.set_class(class);

        let nil = Value::nil();
        BuiltinClass {
            integer: nil,
            float: nil,
            complex: nil,
            array: nil,
            class,
            module,
            procobj: nil,
            method: nil,
            range: nil,
            hash: nil,
            regexp: nil,
            string: nil,
            fiber: nil,
            enumerator: nil,
            object,
        }
    }

    pub fn object() -> Value {
        BUILTINS.with(|b| b.borrow().unwrap().object)
    }

    pub fn class() -> Value {
        BUILTINS.with(|b| b.borrow().unwrap().class)
    }

    pub fn module() -> Value {
        BUILTINS.with(|b| b.borrow().unwrap().module)
    }

    pub fn string() -> Value {
        BUILTINS.with(|b| b.borrow().unwrap().string)
    }

    pub fn integer() -> Value {
        BUILTINS.with(|b| b.borrow().unwrap().integer)
    }

    pub fn float() -> Value {
        BUILTINS.with(|b| b.borrow().unwrap().float)
    }

    /// Bind `class_object` to the constant `class_name` of the root object.
    fn set_class(class_name: &str, class_object: Value) {
        Self::object().set_var_by_str(class_name, class_object);
    }
}

impl GC for BuiltinClass {
    fn mark(&self, alloc: &mut Allocator) {
        self.object.mark(alloc);
    }
}

impl GC for Globals {
    fn mark(&self, alloc: &mut Allocator) {
        self.const_values.mark(alloc);
        self.global_var.values().for_each(|v| v.mark(alloc));
        self.method_cache.0.keys().for_each(|(v, _)| v.mark(alloc));
        for t in &self.case_dispatch.table {
            t.keys().for_each(|k| k.mark(alloc));
        }
        if let Some(vm) = self.main_fiber {
            vm.mark(alloc);
        }
    }
}

impl GlobalsRef {
    pub fn new_globals() -> Self {
        Ref::new(Globals::new())
    }

    pub fn new_vm(&mut self) -> VMRef {
        let vm = VMRef::new(VM::new(self.to_owned()));
        self.main_fiber = Some(vm);
        self.fibers.push(vm);
        vm
    }
}

impl Globals {
    fn new() -> Self {
        use builtin::*;
        let allocator = AllocatorRef::new(Allocator::new());
        ALLOC.with(|alloc| *alloc.borrow_mut() = Some(allocator));
        let mut builtins = BuiltinRef::new(BuiltinClass::new());
        BUILTINS.with(|b| *b.borrow_mut() = Some(builtins));
        let mut object = builtins.object;
        let module = builtins.module;
        let class = builtins.class;
        let main_object = Value::ordinary_object(object);
        let mut globals = Globals {
            builtins,
            allocator,
            const_values: ConstantValues::new(),
            global_var: FxHashMap::default(),
            method_cache: MethodCache::new(),
            inline_cache: InlineCache::new(),
            main_fiber: None,
            instant: std::time::Instant::now(),
            class_version: 0,
            main_object,
            case_dispatch: CaseDispatchMap::new(),
            gc_enabled: true,
            fibers: vec![],
            regexp_cache: FxHashMap::default(),
        };
        // Generate singleton class for Object
        let mut singleton_class = ClassRef::from(None, class);
        singleton_class.is_singleton = true;
        let singleton_obj = Value::class(singleton_class);
        object.set_class(singleton_obj);

        module::init(&mut globals);
        class::init(&mut globals);
        object::init(&mut globals);

        macro_rules! set_builtin_class {
            ($name:expr, $class_object:ident) => {
                object.set_var_by_str($name, $class_object);
            };
        }

        macro_rules! init_builtin_class {
            ($name:expr, $module_name:ident) => {
                let class_obj = $module_name::init(&mut globals);
                builtins.$module_name = class_obj;
                object.set_var_by_str($name, class_obj);
            };
        }

        macro_rules! init_class {
            ($name:expr, $module_name:ident) => {
                let class_obj = $module_name::init(&mut globals);
                object.set_var_by_str($name, class_obj);
            };
        }

        set_builtin_class!("Object", object);
        set_builtin_class!("Module", module);
        set_builtin_class!("Class", class);

        init_builtin_class!("Integer", integer);
        init_builtin_class!("Complex", complex);
        init_builtin_class!("Float", float);
        init_builtin_class!("Array", array);
        init_builtin_class!("Proc", procobj);
        init_builtin_class!("Range", range);
        init_builtin_class!("String", string);
        init_builtin_class!("Hash", hash);
        init_builtin_class!("Method", method);
        init_builtin_class!("Regexp", regexp);
        init_builtin_class!("Fiber", fiber);
        init_builtin_class!("Enumerator", enumerator);

        let kernel = kernel::init(&mut globals);
        object.as_class().include.push(kernel);
        BuiltinClass::set_class("Kernel", kernel);

        init_class!("Math", math);
        init_class!("File", file);
        init_class!("Process", process);
        init_class!("GC", gc);
        init_class!("Struct", structobj);
        init_class!("Time", time);
        init_class!("IO", io);

        BuiltinClass::set_class("StandardError", Value::class(class.as_class()));
        let id = IdentId::get_id("StopIteration");
        let class = ClassRef::from(id, object);
        BuiltinClass::set_class("StopIteration", Value::class(class));
        let errorobj = errorobj::init();
        BuiltinClass::set_class("RuntimeError", errorobj);

        let mut env_map = HashInfo::new(FxHashMap::default());
        let home_dir = dirs::home_dir()
            .unwrap_or(std::path::PathBuf::new())
            .to_string_lossy()
            .to_string();
        env_map.insert(Value::string("HOME".to_string()), Value::string(home_dir));
        let env = Value::hash_from(env_map);
        object.set_var_by_str("ENV", env);

        globals
            .global_var
            .insert(IdentId::get_id("$:"), Value::array_from(vec![]));

        globals
    }

    pub fn gc(&self) {
        let mut alloc = self.allocator;
        alloc.gc(self);
    }

    #[cfg(feature = "gc-debug")]
    pub fn print_mark(&self) {
        self.allocator.print_mark();
    }
}

/*
impl Globals {
    pub fn new_case_dispatch_map(&mut self) -> u32 {
        self.case_dispatch.new_entry()
    }

    pub fn get_case_dispatch_map(&self, id: u32) -> &FxHashMap<Value, i32> {
        self.case_dispatch.get_entry(id)
    }

    pub fn get_mut_case_dispatch_map(&mut self, id: u32) -> &mut FxHashMap<Value, i32> {
        self.case_dispatch.get_mut_entry(id)
    }
}
*/

///
/// Contant value
///
/// A table which holds constant values.
///
#[derive(Debug, Clone)]
pub struct ConstantValues {
    table: Vec<Value>,
}

impl ConstantValues {
    pub fn new() -> Self {
        Self { table: vec![] }
    }

    pub fn insert(&mut self, val: Value) -> usize {
        let id = self.table.len();
        self.table.push(val);
        id
    }

    pub fn get(&self, id: usize) -> Value {
        self.table[id].dup()
    }
}

impl GC for ConstantValues {
    fn mark(&self, alloc: &mut Allocator) {
        self.table.iter().for_each(|v| v.mark(alloc));
    }
}

///
/// Global method cache
///
/// This module supports global method cache.
///
#[derive(Debug, Clone)]
pub struct MethodCache(FxHashMap<(Value, IdentId), MethodCacheEntry>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodCacheEntry {
    pub method: MethodRef,
    pub version: u32,
}

impl MethodCache {
    fn new() -> Self {
        MethodCache(FxHashMap::default())
    }

    fn add_entry(&mut self, class: Value, id: IdentId, version: u32, method: MethodRef) {
        self.0
            .insert((class, id), MethodCacheEntry { method, version });
    }

    fn get_entry(&self, class: Value, id: IdentId) -> Option<&MethodCacheEntry> {
        self.0.get(&(class, id))
    }

    /// Get corresponding instance method(MethodRef) for the class object `class` and `method`.
    ///
    /// If an entry for `class` and `method` exists in global method cache and the entry is not outdated,
    /// return MethodRef of the entry.
    /// If not, search `method` by scanning a class chain.
    /// `class` must be a Class.
    pub fn get_method(
        &mut self,
        class_version: u32,
        rec_class: Value,
        method: IdentId,
    ) -> Option<MethodRef> {
        //let class_version = self.class_version;
        if let Some(MethodCacheEntry { version, method }) = self.get_entry(rec_class, method) {
            if *version == class_version {
                return Some(*method);
            }
        };
        let mut temp_class = rec_class;
        let mut singleton_flag = rec_class.as_class().is_singleton;
        loop {
            match temp_class.get_instance_method(method) {
                Some(methodref) => {
                    self.add_entry(rec_class, method, class_version, methodref);
                    return Some(methodref);
                }
                None => match temp_class.superclass() {
                    Some(superclass) => temp_class = superclass,
                    None => {
                        if singleton_flag {
                            singleton_flag = false;
                            temp_class = rec_class.rvalue().class();
                        } else {
                            return None;
                        }
                    }
                },
            };
        }
    }
}

///
///  Inline method cache
///
///  This module supports inline method cache which is embedded in the instruction sequence directly.
///
#[derive(Debug, Clone)]
pub struct InlineCache {
    table: Vec<InlineCacheEntry>,
    id: u32,
}

#[derive(Debug, Clone)]
pub struct InlineCacheEntry {
    pub version: u32,
    pub entries: Option<(Value, MethodRef)>,
}

impl InlineCacheEntry {
    fn new() -> Self {
        InlineCacheEntry {
            version: 0,
            entries: None,
        }
    }
}

impl InlineCache {
    fn new() -> Self {
        InlineCache {
            table: vec![],
            id: 0,
        }
    }
    pub fn add_entry(&mut self) -> u32 {
        self.id += 1;
        self.table.push(InlineCacheEntry::new());
        self.id - 1
    }

    pub fn get_entry(&mut self, id: u32) -> &mut InlineCacheEntry {
        &mut self.table[id as usize]
    }
}

///
/// Case dispatch map.
///
/// This module supports optimization for case-when syntax when all of the when-conditions were integer literals.
///
#[derive(Debug, Clone)]
pub struct CaseDispatchMap {
    table: Vec<FxHashMap<Value, i32>>,
    id: u32,
}

impl CaseDispatchMap {
    fn new() -> Self {
        CaseDispatchMap {
            table: vec![],
            id: 0,
        }
    }

    pub fn new_entry(&mut self) -> u32 {
        self.id += 1;
        self.table.push(FxHashMap::default());
        self.id - 1
    }

    pub fn get_entry(&self, id: u32) -> &FxHashMap<Value, i32> {
        &self.table[id as usize]
    }

    pub fn get_mut_entry(&mut self, id: u32) -> &mut FxHashMap<Value, i32> {
        &mut self.table[id as usize]
    }
}
