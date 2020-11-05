use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ClassInfo {
    ext: ClassRef,
}

impl ClassInfo {
    fn new(info: ClassExt) -> Self {
        ClassInfo {
            ext: ClassRef::new(info),
        }
    }

    pub fn from(superclass: impl Into<Option<Value>>) -> Self {
        let superclass = match superclass.into() {
            Some(superclass) => superclass,
            None => Value::nil(),
        };
        Self::new(ClassExt::new(None, superclass))
    }

    pub fn singleton_from(
        id: impl Into<Option<IdentId>>,
        superclass: impl Into<Option<Value>>,
    ) -> Self {
        let superclass = match superclass.into() {
            Some(superclass) => superclass,
            None => Value::nil(),
        };
        Self::new(ClassExt::new_singleton(id, superclass))
    }

    pub fn name(&self) -> Option<IdentId> {
        self.ext.name
    }

    pub fn set_name(&mut self, name:impl Into<Option<IdentId>>)  {
        self.ext.name = name.into();
    }

    pub fn name_str(&self) -> String {
        IdentId::get_ident_name(self.ext.name)
    }

    pub fn superclass(&self) -> Value {
        self.ext.superclass
    }

    pub fn mut_superclass(&mut self) -> Option<&mut ClassInfo> {
        if self.ext.superclass.is_nil() {
            None
        } else {
            Some(self.ext.superclass.as_mut_class())
        }
    }

    pub fn is_singleton(&self) -> bool {
        self.ext.is_singleton
    }

    pub fn method_table(&self) -> &MethodTable {
        &self.ext.method_table
    }

    pub fn id(&self) -> u64 {
        self.ext.id()
    }

    pub fn add_builtin_method(&mut self, id:IdentId, func:BuiltinFunc)  {
        self.ext.add_builtin_method(id, func)
    }

    pub fn add_builtin_method_by_str(&mut self, name:&str, func:BuiltinFunc)  {
        self.ext.add_builtin_method_by_str(name, func)
    }

    pub fn add_method(&mut self, globals:&mut Globals, id:IdentId, info:MethodRef) -> Option<MethodRef> {
        self.ext.add_method(globals, id, info)
    }

    pub fn include(&self) -> &Vec<Value> {
        &self.ext.include
    }

    /// Include `module` in `self` class.
    /// This method increments `class_version`.
    pub fn include_append(&mut self, globals: &mut Globals, module: Value) {
        globals.class_version += 1;
        self.ext.include.push(module);
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ClassExt {
    name: Option<IdentId>,
    method_table: MethodTable,
    superclass: Value,
    include: Vec<Value>,
    is_singleton: bool,
}

type ClassRef = Ref<ClassExt>;

impl ClassExt {
    fn new(name: impl Into<Option<IdentId>>, superclass: Value) -> Self {
        ClassExt {
            name: name.into(),
            method_table: FxHashMap::default(),
            superclass,
            include: vec![],
            is_singleton: false,
        }
    }

    fn new_singleton(name: impl Into<Option<IdentId>>, superclass: Value) -> Self {
        ClassExt {
            name: name.into(),
            method_table: FxHashMap::default(),
            superclass,
            include: vec![],
            is_singleton: true,
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

    pub fn add_builtin_method(&mut self, id: IdentId, func: BuiltinFunc) {
        let info = MethodInfo::BuiltinFunc { name: id, func };
        let methodref = MethodRef::new(info);
        self.method_table.insert(id, methodref);
    }

    pub fn add_builtin_method_by_str(&mut self, name: &str, func: BuiltinFunc) {
        let name = IdentId::get_id(name);
        self.add_builtin_method(name, func);
    }
}

impl GC for ClassInfo {
    fn mark(&self, alloc: &mut Allocator) {
        self.ext.superclass.mark(alloc);
        self.ext.include.iter().for_each(|v| v.mark(alloc));
    }
}
