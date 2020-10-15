use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ClassInfo {
    pub name: Option<IdentId>,
    pub method_table: MethodTable,
    pub superclass: Value,
    include: Vec<Value>,
    pub is_singleton: bool,
}

impl ClassInfo {
    pub fn new(name: impl Into<Option<IdentId>>, superclass: Value) -> Self {
        ClassInfo {
            name: name.into(),
            method_table: FxHashMap::default(),
            superclass,
            include: vec![],
            is_singleton: false,
        }
    }

    pub fn id(&self) -> u64 {
        self as *const Self as u64
    }

    pub fn new_singleton(name: impl Into<Option<IdentId>>, superclass: Value) -> Self {
        ClassInfo {
            name: name.into(),
            method_table: FxHashMap::default(),
            superclass,
            include: vec![],
            is_singleton: true,
        }
    }

    pub fn singleton_from(
        id: impl Into<Option<IdentId>>,
        superclass: impl Into<Option<Value>>,
    ) -> Self {
        let superclass = match superclass.into() {
            Some(superclass) => superclass,
            None => Value::nil(),
        };
        ClassInfo::new_singleton(id, superclass)
    }

    pub fn add_builtin_method(&mut self, id: IdentId, func: BuiltinFunc) {
        let info = MethodInfo::BuiltinFunc { name: id, func };
        let methodref = MethodRef::new(info);
        self.method_table.insert(id, methodref);
    }

    pub fn add_builtin_method_by_str(&mut self, name: &str, func: BuiltinFunc) {
        let name = IdentId::get_id(name);
        self.add_builtin_method(name, func);
    }

    pub fn superclass(&self) -> Option<&ClassInfo> {
        if self.superclass.is_nil() {
            None
        } else {
            Some(self.superclass.as_class())
        }
    }

    pub fn mut_superclass(&mut self) -> Option<&mut ClassInfo> {
        if self.superclass.is_nil() {
            None
        } else {
            Some(self.superclass.as_mut_class())
        }
    }

    pub fn add_method(
        &mut self,
        globals: &mut Globals,
        id: IdentId,
        info: MethodRef,
    ) -> Option<MethodRef> {
        globals.class_version += 1;
        self.method_table.insert(id, info)
    }

    pub fn name(&self) -> String {
        IdentId::get_ident_name(self.name)
    }

    /// Include `module` in `self` class.
    /// This method increments `class_version`.
    pub fn include_append(&mut self, globals: &mut Globals, module: Value) {
        self.include.push(module);
        globals.class_version += 1;
    }

    /// Get reference of included modules in `self` class.
    pub fn include(&self) -> &Vec<Value> {
        &self.include
    }

    pub fn from(id: impl Into<Option<IdentId>>, superclass: impl Into<Option<Value>>) -> Self {
        let superclass = match superclass.into() {
            Some(superclass) => superclass,
            None => Value::nil(),
        };
        ClassInfo::new(id, superclass)
    }

    pub fn from_str(name: &str, superclass: impl Into<Option<Value>>) -> Self {
        let superclass = match superclass.into() {
            Some(superclass) => superclass,
            None => Value::nil(),
        };
        ClassInfo::new(IdentId::get_id(name), superclass)
    }
}

impl GC for ClassInfo {
    fn mark(&self, alloc: &mut Allocator) {
        self.superclass.mark(alloc);
        self.include.iter().for_each(|v| v.mark(alloc));
    }
}
