use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ClassInfo {
    pub upper: Value,
    flags: ClassFlags,
    ext: ClassRef,
}

/// flags
/// 0000 0000
///        ||
///        |+-- 1 = singleton
///        +--- 1 = included module
#[derive(Debug, Clone, PartialEq)]
struct ClassFlags(u8);

const SINGLETON: u8 = 1 << 0;
const INCLUDED: u8 = 1 << 1;

impl ClassFlags {
    fn new(is_singleton: bool) -> Self {
        ClassFlags(if is_singleton { SINGLETON } else { 0 })
    }

    fn is_singleton(&self) -> bool {
        self.0 & SINGLETON != 0
    }

    fn is_included(&self) -> bool {
        self.0 & INCLUDED != 0
    }

    fn set_include(&mut self) {
        self.0 |= INCLUDED;
    }
}

impl GC for ClassInfo {
    fn mark(&self, alloc: &mut Allocator) {
        self.upper.mark(alloc);
        self.ext.const_table.values().for_each(|v| v.mark(alloc));
        self.ext.origin.mark(alloc);
    }
}

impl ClassInfo {
    fn new(superclass: impl Into<Option<Value>>, info: ClassExt, is_singleton: bool) -> Self {
        let superclass = match superclass.into() {
            Some(superclass) => superclass,
            None => Value::nil(),
        };
        ClassInfo {
            upper: superclass,
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

    /// Get superclass of `self`.
    ///
    /// If `self` has no superclass, return nil.
    pub fn superclass(&self) -> Value {
        let mut upper = self.upper;
        loop {
            if upper.is_nil() {
                return upper;
            }
            let cinfo = upper.as_module();
            if !cinfo.is_included() {
                return upper;
            };
            upper = cinfo.upper;
        }
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

    pub fn is_singleton(&self) -> bool {
        self.flags.is_singleton()
    }

    pub fn is_included(&self) -> bool {
        self.flags.is_included()
    }

    pub fn set_include(&mut self, origin: Value) {
        assert!(!origin.as_module().is_included());
        self.flags.set_include();
        self.ext.origin = origin;
    }

    pub fn append_include(&mut self, mut module: Value, globals: &mut Globals) {
        let superclass = self.upper;
        let mut imodule = module.dup();
        self.upper = imodule;
        imodule.as_mut_module().set_include(module);
        loop {
            module = match module.upper() {
                Some(module) => module,
                None => break,
            };
            let mut prev = imodule;
            imodule = module.dup();
            prev.as_mut_module().upper = imodule;
            let origin = if module.as_module().is_included() {
                module.as_module().origin()
            } else {
                module
            };
            imodule.as_mut_module().set_include(origin);
        }
        imodule.as_mut_module().upper = superclass;
        globals.class_version += 1;
    }

    pub fn origin(&self) -> Value {
        self.ext.origin
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

    /// Set a constant (`self`::`id`) to `val`.
    ///
    /// If `val` is a module or class, set the name of the class/module to the name of the constant.
    /// If the constant was already initialized, output warning.
    pub fn set_const(&mut self, id: IdentId, mut val: Value) {
        match val.if_mut_mod_class() {
            Some(cinfo) => {
                if cinfo.name().is_none() {
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

        if self.ext.const_table.insert(id, val).is_some() {
            eprintln!("warning: already initialized constant {:?}", id);
        }
    }

    pub fn set_const_by_str(&mut self, name: &str, val: Value) {
        let id = IdentId::get_id(name);
        self.set_const(id, val)
    }

    pub fn get_const(&self, id: IdentId) -> Option<Value> {
        self.ext.const_table.get(&id).cloned()
    }

    pub fn get_const_by_str(&self, name: &str) -> Option<Value> {
        let id = IdentId::get_id(name);
        self.get_const(id)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ClassExt {
    name: Option<IdentId>,
    method_table: MethodTable,
    const_table: ValueTable,
    /// This slot holds original module Value for include modules.
    origin: Value,
}

type ClassRef = Ref<ClassExt>;

impl ClassExt {
    fn new() -> Self {
        ClassExt {
            name: None,
            method_table: FxHashMap::default(),
            const_table: FxHashMap::default(),
            origin: Value::nil(),
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
}
