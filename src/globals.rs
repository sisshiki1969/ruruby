use crate::*;
use fancy_regex::Regex;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Globals {
    // Global info
    pub allocator: AllocatorRef,
    pub builtins: BuiltinRef,
    pub const_values: ConstantValues,
    global_var: ValueTable,
    pub method_cache: MethodCache,
    inline_cache: InlineCache,
    const_cache: ConstCache,
    pub case_dispatch: CaseDispatchMap,

    main_fiber: Option<VMRef>,
    pub instant: std::time::Instant,
    /// version counter: increment when new instance / class methods are defined.
    pub class_version: u32,
    pub const_version: u32,
    pub main_object: Value,
    pub gc_enabled: bool,

    pub fibers: Vec<VMRef>,
    pub regexp_cache: FxHashMap<String, Rc<Regex>>,
    source_files: Vec<PathBuf>,
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
    pub exception: Value,
}

type BuiltinRef = Ref<BuiltinClass>;

impl BuiltinClass {
    fn new() -> Self {
        let basic_class = ClassInfo::from(None);
        let mut basic = Value::bootstrap_class(basic_class);
        let object_class = ClassInfo::from(basic);
        let mut object = Value::bootstrap_class(object_class);
        let module_class = ClassInfo::from(object);
        let mut module = Value::bootstrap_class(module_class);
        let class_class = ClassInfo::from(module);
        let mut class = Value::bootstrap_class(class_class);

        basic.set_class(class);
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
            exception: nil,
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
        self.method_cache
            .cache
            .keys()
            .for_each(|(v, _)| v.mark(alloc));
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

    pub fn create_main_fiber(&mut self) -> VMRef {
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
        let basic = object.superclass().unwrap();
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
            const_cache: ConstCache::new(),
            main_fiber: None,
            instant: std::time::Instant::now(),
            class_version: 0,
            const_version: 0,
            main_object,
            case_dispatch: CaseDispatchMap::new(),
            gc_enabled: true,
            fibers: vec![],
            regexp_cache: FxHashMap::default(),
            source_files: vec![],
        };
        // Generate singleton class for Object
        let singleton_class = ClassInfo::singleton_from(class);
        let singleton_obj = Value::class(singleton_class);
        object.set_class(singleton_obj);

        module::init(&mut globals);
        class::init(&mut globals);
        object::init(&mut globals);
        basicobject::init(&mut globals);

        macro_rules! set_builtin_class {
            ($name:expr, $class_object:ident) => {
                globals.set_toplevel_constant($name, $class_object);
            };
        }

        macro_rules! init_builtin_class {
            ($name:expr, $module_name:ident) => {
                let class_obj = $module_name::init(&mut globals);
                builtins.$module_name = class_obj;
                globals.set_toplevel_constant($name, class_obj);
            };
        }

        macro_rules! init_class {
            ($name:expr, $module_name:ident) => {
                let class_obj = $module_name::init(&mut globals);
                globals.set_toplevel_constant($name, class_obj);
            };
        }

        set_builtin_class!("BasicObject", basic);
        set_builtin_class!("Object", object);
        set_builtin_class!("Module", module);
        set_builtin_class!("Class", class);

        init_class!("Numeric", numeric);

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
        init_builtin_class!("Exception", exception);

        let kernel = kernel::init(&mut globals);
        object.as_mut_class().append_include(kernel, &mut globals);
        globals.set_toplevel_constant("Kernel", kernel);

        init_class!("Math", math);
        init_class!("IO", io);
        init_class!("File", file);
        init_class!("Dir", dir);
        init_class!("Process", process);
        init_class!("GC", gc);
        init_class!("Struct", structobj);
        init_class!("Time", time);
        init_class!("Comparable", comparable);

        globals.set_toplevel_constant("StopIteration", Value::class_from(object));

        let mut env_map = HashInfo::new(FxHashMap::default());
        /*
            let home_dir = dirs::home_dir()
                .unwrap_or(std::path::PathBuf::new())
                .to_string_lossy()
                .to_string();
            env_map.insert(Value::string("HOME".to_string()), Value::string(home_dir));
        */
        std::env::vars()
            .for_each(|(var, val)| env_map.insert(Value::string(var), Value::string(val)));

        let env = Value::hash_from(env_map);
        globals.set_toplevel_constant("ENV", env);
        globals
    }

    pub fn gc(&self) {
        let mut alloc = self.allocator;
        alloc.gc(self);
    }

    pub fn add_source_file(&mut self, file_path: &PathBuf) -> Option<usize> {
        if self.source_files.contains(file_path) {
            None
        } else {
            let i = self.source_files.len();
            self.source_files.push(file_path.to_owned());
            Some(i)
        }
    }

    #[cfg(feature = "perf")]
    pub fn inc_inline_hit(&mut self) {
        self.method_cache.inc_inline_hit();
    }

    #[cfg(feature = "gc-debug")]
    pub fn print_mark(&self) {
        self.allocator.print_mark();
    }
}

impl Globals {
    pub fn get_global_var(&self, id: IdentId) -> Option<Value> {
        self.global_var.get(&id).cloned()
    }

    pub fn set_global_var(&mut self, id: IdentId, val: Value) {
        self.global_var.insert(id, val);
    }

    pub fn set_global_var_by_str(&mut self, name: &str, val: Value) {
        let id = IdentId::get_id(name);
        self.global_var.insert(id, val);
    }

    /// Bind `class_object` to the constant `class_name` of the root object.
    pub fn set_const(&mut self, mut class_obj: Value, id: IdentId, val: Value) {
        class_obj.as_mut_module().set_const(id, val);
        self.const_version += 1;
    }

    /// Bind `class_object` to the constant `class_name` of the root object.
    pub fn set_toplevel_constant(&mut self, class_name: &str, class_object: Value) {
        let mut object = self.builtins.object;
        object
            .as_mut_module()
            .set_const_by_str(class_name, class_object);
        self.const_version += 1;
    }

    pub fn get_toplevel_constant(&self, class_name: &str) -> Option<Value> {
        let object = self.builtins.object;
        object.as_module().get_const_by_str(class_name)
    }

    /// Search method for receiver class.
    ///
    /// If the method was not found, return None.
    pub fn find_method(&mut self, rec_class: Value, method_id: IdentId) -> Option<MethodRef> {
        let class_version = self.class_version;
        self.method_cache
            .get_method(class_version, rec_class, method_id)
    }

    /// Search method(MethodRef) for receiver object.
    ///
    /// If the method was not found, return None.
    pub fn find_method_from_receiver(
        &mut self,
        receiver: Value,
        method_id: IdentId,
    ) -> Option<MethodRef> {
        let rec_class = receiver.get_class_for_method();
        self.find_method(rec_class, method_id)
    }
}

impl Globals {
    pub fn add_const_cache_entry(&mut self) -> u32 {
        self.const_cache.add_entry()
    }

    pub fn get_const_cache_entry(&mut self, id: u32) -> &mut ConstCacheEntry {
        self.const_cache.get_entry(id)
    }

    pub fn set_const_cache(&mut self, id: u32, version: u32, val: Value) {
        self.const_cache.set(id, version, val)
    }

    pub fn add_inline_cache_entry(&mut self) -> u32 {
        self.inline_cache.add_entry()
    }

    fn get_inline_cache_entry(&mut self, id: u32) -> &mut InlineCacheEntry {
        self.inline_cache.get_entry(id)
    }
}

impl GlobalsRef {
    /// Search method(MethodRef) for receiver object using inline method cache.
    ///
    /// If the method was not found, return None.
    pub fn find_method_from_icache(
        &mut self,
        cache: u32,
        receiver: Value,
        method_id: IdentId,
    ) -> Option<MethodRef> {
        let mut globals = self.clone();
        let rec_class = receiver.get_class_for_method();
        let version = self.class_version;
        let icache = self.get_inline_cache_entry(cache);
        if icache.version == version {
            match icache.entries {
                Some((class, method)) if class.id() == rec_class.id() => {
                    #[cfg(feature = "perf")]
                    {
                        self.inc_inline_hit();
                    }
                    return Some(method);
                }
                _ => {}
            }
        };
        let method = match globals.find_method(rec_class, method_id) {
            Some(method) => method,
            None => return None,
        };
        icache.version = version;
        icache.entries = Some((rec_class, method));
        Some(method)
    }
}

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

    pub fn dump(&self) {
        for (i, val) in self.table.iter().enumerate() {
            eprintln!("{}:{:?}", i, val);
        }
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
pub struct MethodCache {
    cache: FxHashMap<(Value, IdentId), MethodCacheEntry>,
    #[cfg(feature = "perf")]
    inline_hit: usize,
    #[cfg(feature = "perf")]
    total: usize,
    #[cfg(feature = "perf")]
    missed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodCacheEntry {
    pub method: MethodRef,
    pub version: u32,
}

impl MethodCache {
    fn new() -> Self {
        MethodCache {
            cache: FxHashMap::default(),
            #[cfg(feature = "perf")]
            inline_hit: 0,
            #[cfg(feature = "perf")]
            total: 0,
            #[cfg(feature = "perf")]
            missed: 0,
        }
    }

    fn add_entry(&mut self, class: Value, id: IdentId, version: u32, method: MethodRef) {
        self.cache
            .insert((class, id), MethodCacheEntry { method, version });
    }

    fn get_entry(&self, class: Value, id: IdentId) -> Option<&MethodCacheEntry> {
        self.cache.get(&(class, id))
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
        #[cfg(feature = "perf")]
        {
            self.total += 1;
        }
        if let Some(MethodCacheEntry { version, method }) = self.get_entry(rec_class, method) {
            if *version == class_version {
                return Some(*method);
            }
        };
        #[cfg(feature = "perf")]
        {
            self.missed += 1;
        }
        match rec_class.get_method(method) {
            Some(methodref) => {
                self.add_entry(rec_class, method, class_version, methodref);
                Some(methodref)
            }
            None => None,
        }
    }
}

#[cfg(feature = "perf")]
impl MethodCache {
    fn inc_inline_hit(&mut self) {
        self.inline_hit += 1;
    }

    pub fn print_stats(&self) {
        eprintln!("+-------------------------------------------+");
        eprintln!("| Method cache stats:                       |");
        eprintln!("+-------------------------------------------+");
        eprintln!("  hit inline cache : {:>10}", self.inline_hit);
        eprintln!("  hit global cache : {:>10}", self.total - self.missed);
        eprintln!("  missed           : {:>10}", self.missed);
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
    fn add_entry(&mut self) -> u32 {
        self.id += 1;
        self.table.push(InlineCacheEntry::new());
        self.id - 1
    }

    fn get_entry(&mut self, id: u32) -> &mut InlineCacheEntry {
        &mut self.table[id as usize]
    }
}

///
///  Inline constant cache
///
///  This module supports inline constant cache which is embedded in the instruction sequence directly.
///
#[derive(Debug, Clone)]
struct ConstCache {
    table: Vec<ConstCacheEntry>,
    id: u32,
}

#[derive(Debug, Clone)]
pub struct ConstCacheEntry {
    pub version: u32,
    pub val: Option<Value>,
}

impl ConstCacheEntry {
    fn new() -> Self {
        ConstCacheEntry {
            version: 0,
            val: None,
        }
    }
}

impl ConstCache {
    fn new() -> Self {
        ConstCache {
            table: vec![],
            id: 0,
        }
    }
    fn add_entry(&mut self) -> u32 {
        self.id += 1;
        self.table.push(ConstCacheEntry::new());
        self.id - 1
    }

    fn get_entry(&mut self, id: u32) -> &mut ConstCacheEntry {
        &mut self.table[id as usize]
    }

    fn set(&mut self, id: u32, version: u32, val: Value) {
        self.table[id as usize] = ConstCacheEntry {
            version,
            val: Some(val),
        };
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
