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
    method_cache: MethodCache,
    inline_cache: InlineCache,
    const_cache: ConstCache,
    pub case_dispatch: CaseDispatchMap,

    main_fiber: Option<VMRef>,
    pub instant: std::time::Instant,
    /// version counter: increment when new instance / class methods are defined.
    pub class_version: u32,
    pub const_version: u32,
    pub main_object: Value,
    pub regexp_cache: FxHashMap<String, Rc<Regex>>,
    source_files: Vec<PathBuf>,
    #[cfg(feature = "perf")]
    pub perf: Perf,
}

pub type GlobalsRef = Ref<Globals>;

thread_local!(
    static BUILTINS: RefCell<BuiltinRef> = RefCell::new(BuiltinRef::new(BuiltinClass::new()));
);

#[derive(Debug, Clone)]
pub struct BuiltinClass {
    pub integer: Value,
    pub float: Value,
    pub complex: Value,
    pub array: Value,
    pub symbol: Value,
    pub class: Module,
    pub module: Module,
    pub procobj: Value,
    pub method: Value,
    pub range: Value,
    pub hash: Value,
    pub regexp: Value,
    pub string: Value,
    pub fiber: Value,
    pub object: Module,
    pub enumerator: Value,
    pub exception: Value,
    pub standard: Value,
    pub nilclass: Value,
    pub trueclass: Value,
    pub falseclass: Value,
    pub kernel: Module,
    pub comparable: Module,
    pub numeric: Module,
}

type BuiltinRef = Ref<BuiltinClass>;

impl BuiltinClass {
    fn new() -> Self {
        let basic_class = ClassInfo::class_from(None);
        let basic = Module::bootstrap_class(basic_class);
        let object_class = ClassInfo::class_from(basic);
        let object = Module::bootstrap_class(object_class);
        let module_class = ClassInfo::class_from(object);
        let module = Module::bootstrap_class(module_class);
        let class_class = ClassInfo::class_from(module);
        let class = Module::bootstrap_class(class_class);

        basic.set_class(class);
        object.set_class(class);
        module.set_class(class);
        class.set_class(class);

        let nil = Value::nil();
        let nilmod = Module::default();
        BuiltinClass {
            integer: nil,
            float: nil,
            complex: nil,
            array: nil,
            symbol: nil,
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
            standard: nil,
            nilclass: nil,
            trueclass: nil,
            falseclass: nil,
            kernel: nilmod,
            comparable: nilmod,
            numeric: nilmod,
        }
    }

    pub fn object() -> Module {
        BUILTINS.with(|b| b.borrow().object)
    }

    pub fn class() -> Module {
        BUILTINS.with(|b| b.borrow().class)
    }

    pub fn module() -> Module {
        BUILTINS.with(|b| b.borrow().module)
    }

    pub fn string() -> Module {
        BUILTINS.with(|b| b.borrow().string).into_module()
    }

    pub fn integer() -> Module {
        BUILTINS.with(|b| b.borrow().integer).into_module()
    }

    pub fn float() -> Module {
        BUILTINS.with(|b| b.borrow().float).into_module()
    }

    pub fn symbol() -> Module {
        BUILTINS.with(|b| b.borrow().symbol).into_module()
    }

    pub fn complex() -> Module {
        BUILTINS.with(|b| b.borrow().complex).into_module()
    }

    pub fn range() -> Module {
        BUILTINS.with(|b| b.borrow().range).into_module()
    }

    pub fn array() -> Module {
        BUILTINS.with(|b| b.borrow().array).into_module()
    }

    pub fn hash() -> Module {
        BUILTINS.with(|b| b.borrow().hash).into_module()
    }

    pub fn fiber() -> Module {
        BUILTINS.with(|b| b.borrow().fiber).into_module()
    }

    pub fn enumerator() -> Module {
        BUILTINS.with(|b| b.borrow().enumerator).into_module()
    }

    pub fn procobj() -> Module {
        BUILTINS.with(|b| b.borrow().procobj).into_module()
    }

    pub fn regexp() -> Module {
        BUILTINS.with(|b| b.borrow().regexp).into_module()
    }

    pub fn method() -> Module {
        BUILTINS.with(|b| b.borrow().method).into_module()
    }

    pub fn exception() -> Module {
        BUILTINS.with(|b| b.borrow().exception).into_module()
    }

    pub fn standard() -> Module {
        BUILTINS.with(|b| b.borrow().standard).into_module()
    }

    pub fn nilclass() -> Module {
        BUILTINS.with(|b| b.borrow().nilclass).into_module()
    }

    pub fn trueclass() -> Module {
        BUILTINS.with(|b| b.borrow().trueclass).into_module()
    }

    pub fn falseclass() -> Module {
        BUILTINS.with(|b| b.borrow().falseclass).into_module()
    }

    pub fn kernel() -> Module {
        BUILTINS.with(|b| b.borrow().kernel)
    }

    pub fn numeric() -> Module {
        BUILTINS.with(|b| b.borrow().numeric)
    }

    pub fn comparable() -> Module {
        BUILTINS.with(|b| b.borrow().comparable)
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
        self.main_object.mark(alloc);
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
        vm
    }
}

impl Globals {
    fn new() -> Self {
        use builtin::*;
        let allocator = ALLOC.with(|alloc| *alloc.borrow());
        let mut builtins = BUILTINS.with(|b| *b.borrow());
        let object = builtins.object;
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
            regexp_cache: FxHashMap::default(),
            source_files: vec![],
            #[cfg(feature = "perf")]
            perf: Perf::new(),
        };
        // Generate singleton class for BasicObject
        let singleton_class = ClassInfo::singleton_from(class, basic);
        let singleton_obj = RValue::new_class(singleton_class).pack();
        basic.set_class(Module::new(singleton_obj));

        builtins.comparable = comparable::init(&mut globals);
        builtins.numeric = numeric::init(&mut globals);
        builtins.kernel = kernel::init(&mut globals);

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

        init_builtin_class!("Integer", integer);
        init_builtin_class!("Complex", complex);
        builtins.float = float::init(&mut globals);
        builtins.nilclass = nilclass::init(&mut globals);
        builtins.trueclass = trueclass::init(&mut globals);
        builtins.falseclass = falseclass::init(&mut globals);
        init_builtin_class!("Array", array);
        init_builtin_class!("Symbol", symbol);
        init_builtin_class!("Proc", procobj);
        init_builtin_class!("Range", range);
        init_builtin_class!("String", string);
        init_builtin_class!("Hash", hash);
        init_builtin_class!("Method", method);
        init_builtin_class!("Regexp", regexp);
        init_builtin_class!("Fiber", fiber);
        init_builtin_class!("Enumerator", enumerator);
        init_builtin_class!("Exception", exception);

        globals.builtins.standard = globals.get_toplevel_constant("StandardError").unwrap();

        math::init(&mut globals);
        io::init(&mut globals);
        file::init(&mut globals);
        dir::init(&mut globals);
        process::init(&mut globals);
        init_class!("GC", gc);
        init_class!("Struct", structobj);
        init_class!("Time", time);

        let mut env_map = HashInfo::new(FxHashMap::default());
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
    pub fn set_const(&mut self, mut class_obj: Module, class_name: IdentId, val: impl Into<Value>) {
        let val = val.into();
        class_obj.set_const(class_name, val);
        self.const_version += 1;
    }

    /// Search inline constant cache for `slot`.
    ///
    /// Return None if not found.
    pub fn find_const_cache(&mut self, slot: u32) -> Option<Value> {
        let const_version = self.const_version;
        #[cfg(feature = "perf")]
        {
            self.const_cache.total += 1;
        }
        match &self.const_cache.get_entry(slot) {
            ConstCacheEntry {
                version,
                val: Some(val),
            } if *version == const_version => Some(*val),
            _ => {
                #[cfg(feature = "perf")]
                {
                    self.const_cache.missed += 1;
                }
                None
            }
        }
    }

    /// Bind `object` to the constant `name` of the root object.
    pub fn set_toplevel_constant(&mut self, name: &str, object: impl Into<Value>) {
        self.builtins.object.set_const_by_str(name, object.into());
        self.const_version += 1;
    }

    pub fn get_toplevel_constant(&self, class_name: &str) -> Option<Value> {
        self.builtins.object.get_const_by_str(class_name)
    }

    /// Search global method cache with receiver class and method name.
    ///
    /// If the method was not found, return None.
    pub fn find_method(&mut self, rec_class: Module, method_id: IdentId) -> Option<MethodRef> {
        let class_version = self.class_version;
        self.method_cache
            .get_method(class_version, rec_class, method_id)
    }

    /// Search global method cache with receiver object and method class_name.
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
        self.const_cache.get_mut_entry(id)
    }

    pub fn set_const_cache(&mut self, id: u32, val: Value) {
        let version = self.const_version;
        self.const_cache.set(id, version, val)
    }

    pub fn add_inline_cache_entry(&mut self) -> u32 {
        self.inline_cache.add_entry()
    }

    fn get_inline_cache_entry(&mut self, id: u32) -> &mut InlineCacheEntry {
        self.inline_cache.get_entry(id)
    }
}

#[cfg(feature = "perf")]
impl Globals {
    pub fn print_method_cache_stats(&self) {
        self.method_cache.print_stats()
    }

    pub fn print_constant_cache_stats(&self) {
        self.const_cache.print_stats()
    }
}

impl GlobalsRef {
    /// Search inline method cache for receiver object and method name.
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
    cache: FxHashMap<(Module, IdentId), MethodCacheEntry>,
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

    fn add_entry(&mut self, class: Module, id: IdentId, version: u32, method: MethodRef) {
        self.cache
            .insert((class, id), MethodCacheEntry { method, version });
    }

    fn get_entry(&self, class: Module, id: IdentId) -> Option<&MethodCacheEntry> {
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
        rec_class: Module,
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
    pub entries: Option<(Module, MethodRef)>,
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
    #[cfg(feature = "perf")]
    total: usize,
    #[cfg(feature = "perf")]
    missed: usize,
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
            #[cfg(feature = "perf")]
            total: 0,
            #[cfg(feature = "perf")]
            missed: 0,
        }
    }
    fn add_entry(&mut self) -> u32 {
        self.id += 1;
        self.table.push(ConstCacheEntry::new());
        self.id - 1
    }

    fn get_entry(&self, id: u32) -> &ConstCacheEntry {
        &self.table[id as usize]
    }

    fn get_mut_entry(&mut self, id: u32) -> &mut ConstCacheEntry {
        &mut self.table[id as usize]
    }

    fn set(&mut self, id: u32, version: u32, val: Value) {
        self.table[id as usize] = ConstCacheEntry {
            version,
            val: Some(val),
        };
    }
}

#[cfg(feature = "perf")]
impl ConstCache {
    pub fn print_stats(&self) {
        eprintln!("+-------------------------------------------+");
        eprintln!("| Constant cache stats:                     |");
        eprintln!("+-------------------------------------------+");
        eprintln!("  hit              : {:>10}", self.total - self.missed);
        eprintln!("  missed           : {:>10}", self.missed);
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
