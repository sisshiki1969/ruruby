use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ClassInfo {
    pub superclass: Value,
    flags: ClassFlags,
    ext: ClassRef,
}

/// flags
/// 0000 0000
///         +-- 1 = singleton
#[derive(Debug, Clone, PartialEq)]
struct ClassFlags(u8);

const SINGLETON: u8 = 1;

impl ClassFlags {
    fn new(is_singleton: bool) -> Self {
        ClassFlags(if is_singleton { SINGLETON } else { 0 })
    }

    fn is_singleton(&self) -> bool {
        self.0 & SINGLETON != 0
    }
}

impl ClassInfo {
    fn new(superclass: impl Into<Option<Value>>, info: ClassExt, is_singleton: bool) -> Self {
        let superclass = match superclass.into() {
            Some(superclass) => superclass,
            None => Value::nil(),
        };
        ClassInfo {
            superclass,
            flags: ClassFlags::new(is_singleton),
            ext: ClassRef::new(info),
        }
    }

    pub fn from(superclass: impl Into<Option<Value>>) -> Self {
        Self::new(superclass, ClassExt::new(), false)
    }

    pub fn singleton_from(superclass: impl Into<Option<Value>>) -> Self {
        Self::new(superclass, ClassExt::new(), true)
    }

    pub fn name(&self) -> Option<IdentId> {
        self.ext.name
    }

    pub fn set_name(&mut self, name: impl Into<Option<IdentId>>) {
        self.ext.name = name.into();
    }

    pub fn name_str(&self) -> String {
        IdentId::get_ident_name(self.ext.name)
    }

    pub fn mut_super_classinfo(&mut self) -> Option<&mut ClassInfo> {
        if self.superclass.is_nil() {
            None
        } else {
            Some(self.superclass.as_mut_class())
        }
    }

    pub fn is_singleton(&self) -> bool {
        self.flags.is_singleton()
    }

    pub fn method_table(&self) -> &MethodTable {
        &self.ext.method_table
    }

    pub fn const_table(&self) -> &ValueTable {
        &self.ext.const_table
    }

    pub fn id(&self) -> u64 {
        self.ext.id()
    }

    pub fn add_builtin_method(&mut self, id: IdentId, func: BuiltinFunc) {
        let info = MethodInfo::BuiltinFunc { name: id, func };
        let methodref = MethodRef::new(info);
        self.ext.method_table.insert(id, methodref);
    }

    pub fn add_builtin_method_by_str(&mut self, name: &str, func: BuiltinFunc) {
        let name = IdentId::get_id(name);
        self.add_builtin_method(name, func);
    }

    pub fn add_method(
        &mut self,
        globals: &mut Globals,
        id: IdentId,
        info: MethodRef,
    ) -> Option<MethodRef> {
        self.ext.add_method(globals, id, info)
    }

    /// Set a constant (`parent`::`id`) to `val`.
    ///
    /// If `val` is a module or class, set the name of the class/module to the name of the constant.
    /// If the constant was already initialized, output warning.
    pub fn set_const(
        &mut self,
        //mut parent: Value,
        id: IdentId,
        mut val: Value,
    ) {
        match val.if_mut_module() {
            Some(cinfo) => {
                if cinfo.name() == None {
                    cinfo.set_name(if self == BuiltinClass::object().as_module() {
                        Some(id)
                    } else {
                        match self.name() {
                            Some(parent_name) => {
                                let name = IdentId::get_id(&format!("{:?}::{:?}", parent_name, id));
                                Some(name)
                            }
                            None => None,
                        }
                    });
                }
            }
            None => {}
        }

        if self.ext.set_const(id, val).is_some() {
            eprintln!("warning: already initialized constant {:?}", id);
        }
    }

    pub fn set_const_by_str(&mut self, name: &str, val: Value) {
        let id = IdentId::get_id(name);
        self.set_const(id, val)
    }

    pub fn get_const(&self, id: IdentId) -> Option<Value> {
        self.ext.get_const(id)
    }

    pub fn get_const_by_str(&self, name: &str) -> Option<Value> {
        let id = IdentId::get_id(name);
        self.ext.get_const(id)
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
    const_table: ValueTable,
    include: Vec<Value>,
}

type ClassRef = Ref<ClassExt>;

impl ClassExt {
    fn new() -> Self {
        ClassExt {
            name: None,
            method_table: FxHashMap::default(),
            const_table: FxHashMap::default(),
            include: vec![],
        }
    }

    fn add_method(
        &mut self,
        globals: &mut Globals,
        id: IdentId,
        info: MethodRef,
    ) -> Option<MethodRef> {
        globals.class_version += 1;
        self.method_table.insert(id, info)
    }

    fn set_const(&mut self, id: IdentId, val: Value) -> Option<Value> {
        self.const_table.insert(id, val)
    }

    fn get_const(&self, id: IdentId) -> Option<Value> {
        self.const_table.get(&id).cloned()
    }
}

impl GC for ClassInfo {
    fn mark(&self, alloc: &mut Allocator) {
        self.superclass.mark(alloc);
        self.ext.const_table.values().for_each(|v| v.mark(alloc));
        self.ext.include.iter().for_each(|v| v.mark(alloc));
    }
}
