use crate::*;
use fancy_regex::Regex;
use std::path::{Path, PathBuf};
use std::rc::Rc;
mod constants;
mod method;
use constants::*;
pub use method::*;
#[cfg(feature = "perf-method")]
mod method_perf;

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
    pub methods: MethodRepo,
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
            methods: MethodRepo::new(),
        };

        BuiltinClass::initialize(&mut globals);

        io::init(&mut globals);
        file::init(&mut globals);

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

    pub(crate) fn gc(&self) {
        ALLOC.with(|m| m.borrow_mut().gc(self));
    }

    pub(crate) fn add_source_file(&mut self, file_path: &Path) -> Option<usize> {
        if self.source_files.contains(&file_path.to_path_buf()) {
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
    pub(crate) fn new() -> Self {
        Self { table: vec![] }
    }

    pub(crate) fn insert(&mut self, val: Value) -> usize {
        let id = self.table.len();
        self.table.push(val);
        id
    }

    pub(crate) fn get(&self, id: usize) -> Value {
        self.table[id].shallow_dup()
    }

    #[cfg(not(tarpaulin_include))]
    #[cfg(features = "emit-iseq")]
    pub(crate) fn dump(&self) {
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

    pub(crate) fn new_entry(&mut self) -> u32 {
        self.id += 1;
        self.table.push(FxHashMap::default());
        self.id - 1
    }

    pub(crate) fn get_entry(&self, id: u32) -> &FxHashMap<HashKey, ISeqDisp> {
        &self.table[id as usize]
    }

    pub(crate) fn get_mut_entry(&mut self, id: u32) -> &mut FxHashMap<HashKey, ISeqDisp> {
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

    pub(crate) fn new_entry(&mut self) -> u32 {
        let len = self.table.len();
        self.table.push((0, 0, vec![]));
        len as u32
    }

    pub(crate) fn get_entry(&self, id: u32) -> &(i64, i64, Vec<ISeqDisp>) {
        &self.table[id as usize]
    }

    pub(crate) fn get_mut_entry(&mut self, id: u32) -> &mut (i64, i64, Vec<ISeqDisp>) {
        &mut self.table[id as usize]
    }
}
