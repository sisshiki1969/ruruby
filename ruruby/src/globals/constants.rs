use crate::*;

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
    pub(crate) fn set_const(
        &mut self,
        mut class_obj: Module,
        class_name: IdentId,
        val: impl Into<Value>,
    ) {
        let val = val.into();
        class_obj.set_const(class_name, val);
        self.const_version += 1;
    }

    /// Search inline constant cache for `slot`.
    ///
    /// Return None if not found.
    pub(crate) fn find_const_cache(&mut self, slot: u32) -> Option<Value> {
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

    /// Get object bound to the constant `name` of the root object.
    pub fn get_toplevel_constant(&self, class_name: &str) -> Value {
        let id = IdentId::get_id(class_name);
        match BuiltinClass::object().get_const_noautoload(id) {
            Some(val) => val,
            _ => unreachable!("{} is not defined in Object.", class_name),
        }
    }

    /// Bind `object` to the constant `name` of the root object.
    pub fn set_toplevel_constant(&mut self, name: &str, object: impl Into<Value>) {
        BuiltinClass::object().set_const_by_str(name, object.into());
        self.const_version += 1;
    }
}

impl Globals {
    pub(crate) fn add_const_cache_entry(&mut self) -> u32 {
        self.const_cache.add_entry()
    }

    pub(crate) fn set_const_cache(&mut self, id: u32, val: Value) {
        let version = self.const_version;
        self.const_cache.set(id, version, val)
    }
}

#[cfg(feature = "perf-method")]
impl Globals {
    pub fn print_constant_cache_stats(&self) {
        self.const_cache.print_stats()
    }

    pub(crate) fn clear_const_cache(&mut self) {
        self.const_cache.clear();
    }
}

///
///  Inline constant cache
///
///  This module supports inline constant cache which is embedded in the instruction sequence directly.
///
#[derive(Debug, Clone)]
pub(super) struct ConstCache {
    table: Vec<ConstCacheEntry>,
    id: u32,
    #[cfg(feature = "perf-method")]
    total: usize,
    #[cfg(feature = "perf-method")]
    missed: usize,
}

impl ConstCache {
    pub(super) fn new() -> Self {
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

#[derive(Debug, Clone)]
pub(super) struct ConstCacheEntry {
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
