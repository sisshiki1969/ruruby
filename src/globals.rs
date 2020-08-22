use crate::*;
use fancy_regex::Regex;
use std::cell::RefCell;
use std::rc::Rc;

const INLINE_SIZE: usize = 1;

#[derive(Debug, Clone)]
pub struct Globals {
    // Global info
    pub allocator: AllocatorRef,
    pub builtins: BuiltinRef,
    pub const_values: ConstantValues,
    pub global_var: ValueTable,
    inline_cache: InlineCache,
    method_cache: MethodCache,
    pub case_dispatch: CaseDispatchMap,

    main_fiber: Option<VMRef>,
    pub instant: std::time::Instant,
    /// version counter: increment when new instance / class methods are defined.
    pub class_version: usize,
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
            inline_cache: InlineCache::new(),
            method_cache: MethodCache::new(),
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
                let id = IdentId::get_id($name);
                object.set_var(id, $class_object);
            };
        }

        macro_rules! init_builtin_class {
            ($name:expr, $module_name:ident) => {
                let class_obj = $module_name::init(&mut globals);
                builtins.$module_name = class_obj;
                let id = IdentId::get_id($name);
                object.set_var(id, class_obj);
            };
        }

        macro_rules! init_class {
            ($name:expr, $module_name:ident) => {
                let class_obj = $module_name::init(&mut globals);
                let id = IdentId::get_id($name);
                object.set_var(id, class_obj);
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

impl Globals {
    pub fn set_inline_cache(&mut self, id: u32, class: Value, method: MethodRef) {
        let entry = &mut self.inline_cache.table[id as usize];
        for e in entry.entries.iter_mut() {
            if e.is_none() {
                *e = Some((class, method));
                return;
            }
        }
        entry.entries[INLINE_SIZE - 1] = Some((class, method));
    }

    pub fn add_inline_cache_entry(&mut self) -> u32 {
        self.inline_cache.add_entry()
    }

    pub fn get_method_from_inline_cache(
        &mut self,
        cache_id: u32,
        rec_class: Value,
    ) -> Option<MethodRef> {
        self.inline_cache
            .get_method(cache_id, rec_class, self.class_version)
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
    pub version: usize,
}

impl MethodCache {
    fn new() -> Self {
        MethodCache(FxHashMap::default())
    }

    fn add_entry(&mut self, class: Value, id: IdentId, version: usize, method: MethodRef) {
        self.0
            .insert((class, id), MethodCacheEntry { method, version });
    }

    fn get_entry(&self, class: Value, id: IdentId) -> Option<&MethodCacheEntry> {
        self.0.get(&(class, id))
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
    //class: Value,
    version: usize,
    //method: MethodRef,
    entries: [Option<(Value, MethodRef)>; INLINE_SIZE],
}

impl InlineCacheEntry {
    fn new() -> Self {
        InlineCacheEntry {
            version: 0,
            entries: [None; INLINE_SIZE],
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

    fn get_method(&mut self, id: u32, class: Value, version: usize) -> Option<MethodRef> {
        let entry = &mut self.table[id as usize];
        if entry.version != version {
            //eprintln!("version up");
            entry.version = version;
            entry.entries.iter_mut().for_each(|x| *x = None);
            return None;
        }
        for e in &entry.entries {
            if let Some((cache_class, method)) = e {
                if cache_class.id() == class.id() {
                    return Some(*method);
                }
            }
        }
        //eprintln!("not found");
        None
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
