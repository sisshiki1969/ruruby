use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct ClassInfo {
    upper: Option<Module>,
    flags: ClassFlags,
    ext: ClassRef,
}

impl GC for ClassInfo {
    fn mark(&self, alloc: &mut Allocator) {
        if let Some(upper) = &self.upper {
            upper.mark(alloc);
        }
        self.ext.const_table.values().for_each(|v| v.mark(alloc));
        if let Some(module) = &self.ext.origin {
            module.mark(alloc)
        };
    }
}

impl ClassInfo {
    fn new(is_module: bool, superclass: impl Into<Option<Module>>, info: ClassExt) -> Self {
        ClassInfo {
            upper: match superclass.into() {
                Some(c) => Some(c),
                None => None,
            },
            flags: ClassFlags::new(is_module),
            ext: ClassRef::new(info),
        }
    }

    pub fn class_from(superclass: impl Into<Option<Module>>) -> Self {
        Self::new(false, superclass, ClassExt::new())
    }

    pub fn module_from(superclass: impl Into<Option<Module>>) -> Self {
        Self::new(true, superclass, ClassExt::new())
    }

    pub fn singleton_from(superclass: impl Into<Option<Module>>, target: Value) -> Self {
        Self::new(false, superclass, ClassExt::new_singleton(target))
    }

    pub fn upper(&self) -> Option<Module> {
        let mut upper = self.upper;
        loop {
            match upper {
                None => return None,
                Some(m) => {
                    let cinfo = m.as_module();
                    if !cinfo.has_prepend() {
                        return Some(m);
                    }
                    upper = cinfo.upper;
                }
            }
        }
    }

    /// Get superclass of `self`.
    ///
    /// If `self` has no superclass, return nil.
    pub fn superclass(&self) -> Option<Module> {
        let mut upper = self.upper;
        loop {
            match upper {
                None => return None,
                Some(m) => {
                    let cinfo = m.as_module();
                    if !cinfo.is_included() {
                        return Some(m);
                    };
                    upper = cinfo.upper;
                }
            }
        }
    }

    fn default_name(&self) -> String {
        if self.is_module() {
            format!("#<Module:0x{:016x}>", self.id())
        } else {
            format!("#<Class:0x{:016x}>", self.id())
        }
    }

    pub fn name(&self) -> String {
        match self.op_name() {
            Some(name) => name,
            None => self.default_name(),
        }
    }

    pub fn op_name(&self) -> Option<String> {
        let mut ext = self.ext;
        match &ext.name {
            Some(name) => Some(name.to_owned()),
            None => {
                if let Some(target) = ext.singleton_for {
                    let s = format!(
                        "#<Class:{}>",
                        if let Some(c) = target.if_mod_class() {
                            match c.op_name() {
                                Some(name) => {
                                    ext.name = Some(name.clone());
                                    name
                                }
                                None => self.default_name(),
                            }
                        } else if let Some(o) = target.as_rvalue() {
                            let name = o.to_s();
                            ext.name = Some(name.clone());
                            name
                        } else {
                            unreachable!()
                        }
                    );
                    Some(s)
                } else {
                    None
                }
            }
        }
    }

    pub fn inspect(&self) -> String {
        self.name()
    }

    pub fn set_name(&mut self, name: impl Into<String>) {
        self.ext.name = Some(name.into());
    }

    pub fn is_singleton(&self) -> bool {
        self.ext.singleton_for.is_some()
    }

    pub fn singleton_for(&self) -> Option<Value> {
        self.ext.singleton_for
    }

    pub fn is_module(&self) -> bool {
        self.flags.is_module()
    }

    pub fn is_included(&self) -> bool {
        self.flags.is_included()
    }

    fn has_prepend(&self) -> bool {
        self.flags.has_prepend()
    }

    fn set_prepend(&mut self) {
        self.flags.set_prepend()
    }

    pub fn set_include(&mut self, origin: Module) {
        #[cfg(debug_assertions)]
        assert!(!origin.as_module().is_included());
        self.flags.set_include();
        self.ext.origin = Some(origin);
    }

    pub fn append_include(&mut self, mut module: Module, globals: &mut Globals) {
        let superclass = self.upper;
        let mut imodule = module.generate_included();
        self.upper = Some(imodule);
        loop {
            module = match module.upper() {
                Some(module) => module,
                None => break,
            };
            let mut prev = imodule;
            imodule = module.generate_included();
            prev.as_mut_module().upper = Some(imodule);
        }
        imodule.as_mut_module().upper = superclass;
        globals.class_version += 1;
    }

    pub fn append_prepend(&mut self, base: Value, module: Value, globals: &mut Globals) {
        let mut module = Module::new(module);
        let base = Module::new(base);
        let superclass = self.upper;
        let mut imodule = module.generate_included();
        self.upper = Some(imodule);
        loop {
            module = match module.upper() {
                Some(module) => module,
                None => break,
            };
            let mut prev = imodule;
            imodule = module.generate_included();
            prev.as_mut_module().upper = Some(imodule);
        }
        if !self.has_prepend() {
            let mut dummy = base.dup();
            let mut dinfo = dummy.as_mut_module();
            dinfo.upper = superclass;
            dinfo.set_include(base);
            imodule.as_mut_module().upper = Some(dummy);
            self.set_prepend();
        } else {
            imodule.as_mut_module().upper = superclass;
        }
        globals.class_version += 1;
    }

    pub fn origin(&self) -> Option<Module> {
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
        if let Some(cinfo) = val.if_mut_mod_class() {
            if cinfo.ext.name.is_none() {
                if self == BuiltinClass::object().as_module() {
                    cinfo.set_name(IdentId::get_name(id));
                } else {
                    match &self.ext.name {
                        Some(parent_name) => {
                            let name = format!("{}::{:?}", parent_name, id);
                            cinfo.set_name(name);
                        }
                        None => {}
                    }
                };
            }
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Module(Value);

impl std::ops::Deref for Module {
    type Target = Value;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Module {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Module {
    pub fn new(val: Value) -> Self {
        Module(val)
    }

    pub fn dup(&self) -> Self {
        Module((**self).dup())
    }

    pub fn generate_included(&self) -> Module {
        let origin = if self.as_module().is_included() {
            self.as_module().origin().unwrap()
        } else {
            *self
        };
        let mut imodule = self.dup();
        imodule.as_mut_module().set_include(origin);
        imodule
    }

    /// Get superclass of `self`.
    ///
    /// If `self` was a module/class which has no superclass or `self` was not a module/class, return None.
    pub fn superclass(&self) -> Option<Module> {
        match self.if_mod_class() {
            Some(cinfo) => cinfo.superclass(),
            None => None,
        }
    }

    /// Examine whether `self` is a singleton class.
    /// Panic if `self` is not a class object.
    pub fn is_singleton(&self) -> bool {
        self.as_module().is_singleton()
    }
}

/// ClassFlags:
/// 0000 0000
///       |||
///       ||+-- 0 = class, 1 = module
///       |+--- 1 = included module
///       +---- 1 = module which has prepend
#[derive(Debug, Clone, PartialEq)]
struct ClassFlags(u8);

const IS_MODULE: u8 = 1 << 0;
const INCLUDED: u8 = 1 << 1;
const HAS_PREPEND: u8 = 1 << 2;

impl ClassFlags {
    fn new(is_module: bool) -> Self {
        ClassFlags(if is_module { IS_MODULE } else { 0 })
    }

    fn is_module(&self) -> bool {
        self.0 & IS_MODULE != 0
    }

    fn is_included(&self) -> bool {
        self.0 & INCLUDED != 0
    }

    fn has_prepend(&self) -> bool {
        self.0 & HAS_PREPEND != 0
    }

    fn set_include(&mut self) {
        self.0 |= INCLUDED;
    }

    fn set_prepend(&mut self) {
        self.0 |= HAS_PREPEND;
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ClassExt {
    name: Option<String>,
    method_table: MethodTable,
    const_table: ValueTable,
    singleton_for: Option<Value>,
    /// This slot holds original module Value for include modules.
    origin: Option<Module>,
}

type ClassRef = Ref<ClassExt>;

impl ClassExt {
    fn new() -> Self {
        ClassExt {
            name: None,
            method_table: FxHashMap::default(),
            const_table: FxHashMap::default(),
            singleton_for: None,
            origin: None,
        }
    }

    fn new_singleton(target: Value) -> Self {
        ClassExt {
            name: None,
            method_table: FxHashMap::default(),
            const_table: FxHashMap::default(),
            singleton_for: Some(target),
            origin: None,
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
