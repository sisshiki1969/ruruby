use crate::*;
use fancy_regex::Regex;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Globals {
    // Global info
    pub const_values: ConstantValues,
    global_var: ValueTable,
    const_cache: ConstCache,
    pub case_dispatch: CaseDispatchMap,
    pub case_dispatch2: CaseDispatchMap2,

    main_fiber: Option<VMRef>,
    pub instant: std::time::Instant,
    pub const_version: u32,
    pub main_object: Value,
    pub regexp_cache: FxHashMap<String, Rc<Regex>>,
    source_files: Vec<PathBuf>,
    #[cfg(feature = "perf")]
    pub perf: Perf,
    pub startup_flag: bool,
    /// register for error handling
    pub val: Value,
    pub fiber_result: VMResult,
}

pub type GlobalsRef = Ref<Globals>;

impl GC for Globals {
    fn mark(&self, alloc: &mut Allocator) {
        self.const_values.mark(alloc);
        self.main_object.mark(alloc);
        self.global_var.values().for_each(|v| v.mark(alloc));
        for t in &self.case_dispatch.table {
            t.keys().for_each(|k| k.mark(alloc));
        }
        if let Some(vm) = self.main_fiber {
            vm.mark(alloc);
        }
        self.val.mark(alloc);
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
        let object = BuiltinClass::object();
        let main_object = Value::ordinary_object(object);
        main_object.set_var_by_str("/name", Value::string("main"));
        let mut globals = Globals {
            const_values: ConstantValues::new(),
            global_var: FxHashMap::default(),
            const_cache: ConstCache::new(),
            main_fiber: None,
            instant: std::time::Instant::now(),
            const_version: 0,
            main_object,
            case_dispatch: CaseDispatchMap::new(),
            case_dispatch2: CaseDispatchMap2::new(),
            regexp_cache: FxHashMap::default(),
            source_files: vec![],
            #[cfg(feature = "perf")]
            perf: Perf::new(),
            startup_flag: false,
            val: Value::nil(),
            fiber_result: Ok(Value::nil()),
        };

        BuiltinClass::initialize();

        io::init(&mut globals);
        file::init();

        let mut env_map = HashInfo::new(FxIndexMap::default());
        std::env::vars()
            .for_each(|(var, val)| env_map.insert(Value::string(var), Value::string(val)));
        #[cfg(windows)]
        if let None = env_map.get(&Value::string("HOME")) {
            let home_drive = env_map.get(&Value::string("HOMEDRIVE"));
            let home_path = env_map.get(&Value::string("HOMEPATH"));
            let user_profile = env_map.get(&Value::string("USERPROFILE"));
            let home = if home_drive.is_some() && home_drive.is_some() {
                home_drive.unwrap().as_string().unwrap().to_string()
                    + home_path.unwrap().as_string().unwrap()
            } else if let Some(up) = user_profile {
                up.as_string().unwrap().to_string()
            } else {
                "".to_string()
            };
            env_map.insert(
                Value::string("HOME"),
                Value::string(home.replace('\\', "/")),
            );
        };

        let env = Value::hash_from(env_map);
        globals.set_toplevel_constant("ENV", env);
        globals.set_global_var_by_str("$/", Value::string("\n"));
        globals
    }

    pub fn gc(&self) {
        ALLOC.with(|m| m.borrow_mut().gc(self));
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

    #[cfg(feature = "gc-debug")]
    pub fn print_mark(&self) {
        ALLOC.with(|m| m.borrow_mut().print_mark());
    }

    #[cfg(feature = "perf-method")]
    pub fn clear_const_cache(&mut self) {
        self.const_cache.clear();
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
        #[cfg(feature = "perf-method")]
        {
            self.const_cache.total += 1;
        }
        match &self.const_cache.get_entry(slot) {
            ConstCacheEntry {
                version,
                val: Some(val),
            } if *version == const_version => Some(*val),
            _ => {
                #[cfg(feature = "perf-method")]
                {
                    self.const_cache.missed += 1;
                }
                None
            }
        }
    }

    /// Bind `object` to the constant `name` of the root object.
    pub fn set_toplevel_constant(&mut self, name: &str, object: impl Into<Value>) {
        BuiltinClass::object().set_const_by_str(name, object.into());
        self.const_version += 1;
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
}

#[cfg(feature = "perf-method")]
impl Globals {
    pub fn print_constant_cache_stats(&self) {
        self.const_cache.print_stats()
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
        self.table[id].shallow_dup()
    }

    #[cfg(not(tarpaulin_include))]
    #[cfg(features = "emit-iseq")]
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
///  Inline constant cache
///
///  This module supports inline constant cache which is embedded in the instruction sequence directly.
///
#[derive(Debug, Clone)]
struct ConstCache {
    table: Vec<ConstCacheEntry>,
    id: u32,
    #[cfg(feature = "perf-method")]
    total: usize,
    #[cfg(feature = "perf-method")]
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
            #[cfg(feature = "perf-method")]
            total: 0,
            #[cfg(feature = "perf-method")]
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

#[cfg(feature = "perf-method")]
impl ConstCache {
    fn clear(&mut self) {
        self.missed = 0;
        self.total = 0;
    }

    fn print_stats(&self) {
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
    table: Vec<FxHashMap<HashKey, ISeqDisp>>,
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

    pub fn get_entry(&self, id: u32) -> &FxHashMap<HashKey, ISeqDisp> {
        &self.table[id as usize]
    }

    pub fn get_mut_entry(&mut self, id: u32) -> &mut FxHashMap<HashKey, ISeqDisp> {
        &mut self.table[id as usize]
    }
}

///
/// Case dispatch map-2.
///
/// This module supports optimization for case-when syntax when all of the when-conditions were integer literals.
///
#[derive(Debug, Clone)]
pub struct CaseDispatchMap2 {
    table: Vec<(i64, i64, Vec<ISeqDisp>)>, //(min, max, map)
}

impl CaseDispatchMap2 {
    fn new() -> Self {
        Self { table: vec![] }
    }

    pub fn new_entry(&mut self) -> u32 {
        let len = self.table.len();
        self.table.push((0, 0, vec![]));
        len as u32
    }

    pub fn get_entry(&self, id: u32) -> &(i64, i64, Vec<ISeqDisp>) {
        &self.table[id as usize]
    }

    pub fn get_mut_entry(&mut self, id: u32) -> &mut (i64, i64, Vec<ISeqDisp>) {
        &mut self.table[id as usize]
    }
}
