use crate::*;

///
/// Wrapper struct for Module/Class object.
///
/// This type automatically dereferences ClassInfo.
/// Use into(self) to get inner Value.  
///
#[derive(Clone, Copy)]
pub struct Module(Value);

impl std::fmt::Debug for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl std::cmp::PartialEq for Module {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl std::cmp::Eq for Module {}

impl std::ops::Deref for Module {
    type Target = ClassInfo;
    fn deref(&self) -> &Self::Target {
        self.0.as_class()
    }
}

impl std::ops::DerefMut for Module {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut_class()
    }
}

impl std::hash::Hash for Module {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id().hash(state);
    }
}

impl Into<Value> for Module {
    fn into(self) -> Value {
        self.0
    }
}

impl GC for Module {
    fn mark(&self, alloc: &mut Allocator) {
        self.get().mark(alloc);
    }
}

impl Module {
    /// Construct new Module from `val`.
    ///
    /// ### Panics
    /// panics if `val` is neither Class nor Module.
    pub fn new(mut val: Value) -> Self {
        val.as_mut_class();
        Module(val)
    }

    /// Construct new Module from `val` without checking whether it is Class/Module.
    pub fn new_unchecked(val: Value) -> Self {
        Module(val)
    }

    /// Construct new dummy Module.
    pub fn default() -> Self {
        Module(Value::nil())
    }

    /// Get inner `Value`.
    fn get(self) -> Value {
        self.0
    }

    /// Get id(u64).
    pub fn id(self) -> u64 {
        self.0.id()
    }

    /// Duplicate `self`.
    /// This fn creates a new RValue.
    pub fn dup(&self) -> Self {
        Module(self.get().dup())
    }

    /// Get a class of `self`.
    pub fn class(&self) -> Module {
        self.get().rvalue().class()
    }

    /// Set `class` as a class of `self`.
    pub fn set_class(self, class: Module) {
        self.get().set_class(class)
    }

    /// Get a real module of `self`.
    /// If `self` is an included module, return its origin.
    pub fn real_module(&self) -> Module {
        if self.is_included() {
            self.origin().unwrap()
        } else {
            *self
        }
    }

    pub fn generate_included(&self) -> Module {
        let origin = self.real_module();
        self.dup().set_include(origin)
    }

    pub fn set_include(mut self, origin: Module) -> Module {
        self.0.as_mut_class().set_include(origin);
        self
    }

    /// Check whether `target_module` exists in the ancestors of `self`.
    pub fn include_module(&self, target_module: Module) -> bool {
        let mut module = *self;
        loop {
            let true_module = module.real_module();
            if true_module.id() == target_module.id() {
                return true;
            };
            match module.upper() {
                Some(upper) => module = upper,
                None => break,
            }
        }
        false
    }

    pub fn get_singleton_class(self) -> Module {
        self.get().get_singleton_class().unwrap()
    }

    /// Get method for a receiver which class is `self` and `method` (IdentId).
    pub fn get_method(self, method: IdentId) -> Option<MethodId> {
        let mut class = self;
        let mut singleton_flag = self.is_singleton();
        loop {
            match class.get_instance_method(method) {
                Some(method) => {
                    return Some(method);
                }
                None => match class.upper() {
                    Some(superclass) => class = superclass,
                    None => {
                        if singleton_flag {
                            singleton_flag = false;
                            class = self.class();
                        } else {
                            return None;
                        }
                    }
                },
            };
        }
    }

    /// Find method `id` from method tables of `self` class and all of its superclasses including their included modules.
    /// Return None if no method found.
    pub fn get_instance_method(&self, id: IdentId) -> Option<MethodId> {
        self.method_table().get(&id).cloned()
    }

    /// Add BuiltinFunc `func` named `name` to the singleton class of `self`.
    pub fn add_builtin_class_method(self, name: &str, func: BuiltinFunc) {
        self.get_singleton_class()
            .add_builtin_method_by_str(name, func);
    }

    /// Add an instance method `func` named `name` to `self`.
    pub fn add_builtin_method_by_str(mut self, name: &str, func: BuiltinFunc) {
        let name = IdentId::get_id(name);
        self.add_builtin_method(name, func);
    }

    /// Add a module function `func` named `name` to `self`.
    pub fn add_builtin_module_func(self, name: &str, func: BuiltinFunc) {
        self.add_builtin_method_by_str(name, func);
        self.get_singleton_class()
            .add_builtin_method_by_str(name, func);
    }
}

impl Module {
    pub fn set_var(self, id: IdentId, val: Value) -> Option<Value> {
        self.get().set_var(id, val)
    }

    pub fn set_var_by_str(self, name: &str, val: Value) {
        self.get().set_var_by_str(name, val)
    }

    pub fn get_var(&self, id: IdentId) -> Option<Value> {
        self.get().get_var(id)
    }

    pub fn set_var_if_exists(&self, id: IdentId, val: Value) -> bool {
        self.get().set_var_if_exists(id, val)
    }
}

impl Module {
    pub fn new_class(cinfo: ClassInfo) -> Module {
        assert!(!cinfo.is_module());
        let obj = RValue::new_class(cinfo).pack();
        obj.get_singleton_class().unwrap();
        obj.into_module()
    }

    pub fn bootstrap_class(cinfo: ClassInfo) -> Module {
        Module::new(RValue::new_bootstrap(cinfo).pack())
    }

    pub fn class_under(superclass: impl Into<Option<Module>>) -> Module {
        Module::new_class(ClassInfo::class_from(superclass))
    }

    pub fn class_under_object() -> Module {
        Module::new_class(ClassInfo::class_from(BuiltinClass::object()))
    }

    pub fn singleton_class_from(
        superclass: impl Into<Option<Module>>,
        target: impl Into<Value>,
    ) -> Module {
        Module::new(RValue::new_class(ClassInfo::singleton_from(superclass, target)).pack())
    }

    pub fn module() -> Module {
        Module::new(RValue::new_module(ClassInfo::module_from(None)).pack())
    }
}

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

    fn class_id(&self) -> u64 {
        self as *const ClassInfo as u64
    }

    pub fn class_from(superclass: impl Into<Option<Module>>) -> Self {
        Self::new(false, superclass, ClassExt::new())
    }

    pub fn module_from(superclass: impl Into<Option<Module>>) -> Self {
        Self::new(true, superclass, ClassExt::new())
    }

    pub fn singleton_from(superclass: impl Into<Option<Module>>, target: impl Into<Value>) -> Self {
        let target = target.into();
        Self::new(false, superclass, ClassExt::new_singleton(target))
    }

    /// Get an upper module/class of `self`.
    ///
    /// If `self` has no upper module/class, return None.
    pub fn upper(&self) -> Option<Module> {
        let mut upper = self.upper;
        loop {
            match upper {
                None => return None,
                Some(m) => {
                    if !m.has_prepend() {
                        return Some(m);
                    }
                    upper = m.upper;
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
                    if !m.is_included() {
                        return Some(m);
                    };
                    upper = m.upper;
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

    fn set_include(&mut self, origin: Module) {
        #[cfg(debug_assertions)]
        assert!(!origin.is_included());
        self.flags.set_include();
        self.ext.origin = Some(origin);
    }

    pub fn append_include(&mut self, module: Module) {
        self.append_include_without_increment_version(module);
        MethodRepo::inc_class_version();
    }

    pub fn append_include_without_increment_version(&mut self, mut module: Module) {
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
            prev.upper = Some(imodule);
        }
        imodule.upper = superclass;
    }

    pub fn append_prepend(&mut self, base: Module, module: Module) {
        let mut module = module;
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
            prev.upper = Some(imodule);
        }
        if !self.has_prepend() {
            let mut dummy = base.dup();
            dummy.upper = superclass;
            dummy.set_include(base);
            imodule.upper = Some(dummy);
            self.set_prepend();
        } else {
            imodule.upper = superclass;
        }
        MethodRepo::inc_class_version();
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
        let methodref = MethodRepo::add(info);
        self.ext.method_table.insert(id, methodref);
    }

    pub fn add_builtin_method_by_str(&mut self, name: &str, func: BuiltinFunc) {
        let name = IdentId::get_id(name);
        self.add_builtin_method(name, func);
    }

    pub fn add_method(&mut self, id: IdentId, info: MethodId) -> Option<MethodId> {
        self.ext.add_method(id, info)
    }

    /// Set a constant (`self`::`id`) to `val`.
    ///
    /// If `val` is a module or class, set the name of the class/module to the name of the constant.
    /// If the constant was already initialized, output warning.
    pub fn set_const(&mut self, id: IdentId, val: Value) {
        if let Some(mut module) = val.if_mod_class() {
            if module.op_name().is_none() {
                if self.class_id() == BuiltinClass::object().class_id() {
                    module.set_name(IdentId::get_name(id));
                } else {
                    match &self.ext.name {
                        Some(parent_name) => {
                            let name = format!("{}::{:?}", parent_name, id);
                            module.set_name(name);
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

    fn add_method(&mut self, id: IdentId, info: MethodId) -> Option<MethodId> {
        MethodRepo::inc_class_version();
        self.method_table.insert(id, info)
    }
}
