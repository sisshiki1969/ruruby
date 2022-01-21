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
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl std::cmp::Eq for Module {}

impl std::ops::Deref for Module {
    type Target = ClassInfo;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.0.as_class()
    }
}

impl std::ops::DerefMut for Module {
    #[inline(always)]
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
    #[inline(always)]
    fn into(self) -> Value {
        self.0
    }
}

impl GC<RValue> for Module {
    fn mark(&self, alloc: &mut Allocator<RValue>) {
        self.get().mark(alloc);
    }
}

impl Module {
    /// Construct new Module from `val`.
    ///
    /// ### Panics
    /// panics if `val` is neither Class nor Module.
    #[inline(always)]
    pub(crate) fn new(mut val: Value) -> Self {
        val.as_mut_class();
        Module(val)
    }

    /// Construct new Module from `val` without checking whether it is Class/Module.
    #[inline(always)]
    pub(crate) fn new_unchecked(val: Value) -> Self {
        Module(val)
    }

    /// Construct new dummy Module.
    #[inline(always)]
    pub(crate) fn default() -> Self {
        Module(Value::nil())
    }

    /// Get inner `Value`.
    #[inline(always)]
    fn get(self) -> Value {
        self.0
    }

    /// Get id(u64).
    #[inline(always)]
    pub(crate) fn id(self) -> u64 {
        self.0.id()
    }

    /// Duplicate `self`.
    /// This fn creates a new RValue.
    pub(crate) fn dup(&self) -> Self {
        Module(self.get().shallow_dup())
    }

    #[inline(always)]
    /// Get a class of `self`.
    pub(crate) fn class(&self) -> Module {
        self.get().rvalue().class()
    }

    /// Set `class` as a class of `self`.
    pub(crate) fn set_class(self, class: Module) {
        self.get().set_class(class)
    }

    /// Get a real module of `self`.
    /// If `self` is an included module, return its origin.
    pub(crate) fn real_module(&self) -> Module {
        if self.is_included() {
            self.origin().unwrap()
        } else {
            *self
        }
    }

    pub(crate) fn generate_included(&self) -> Module {
        let origin = self.real_module();
        self.dup().set_include(origin)
    }

    pub(crate) fn set_include(mut self, origin: Module) -> Module {
        self.0.as_mut_class().set_include(origin);
        self
    }

    /// Check whether `target_module` exists in the ancestors of `self`.
    pub(crate) fn include_module(&self, target_module: Module) -> bool {
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

    /// Get singleton class of `self`.
    pub(crate) fn get_singleton_class(self) -> Module {
        self.get().get_singleton_class().unwrap()
    }

    /// Get MethodId from `method_id` for `self`.
    ///
    /// If the method was not found, return NoMethodError.
    pub(crate) fn get_method_or_nomethod(
        self,
        globals: &mut Globals,
        method_id: IdentId,
    ) -> Result<FnId, RubyError> {
        match globals.methods.find_method(self, method_id) {
            Some(m) => Ok(m),
            None => Err(VMError::undefined_method_for_class(method_id, self)),
        }
    }
}

pub struct DefinedMethod {
    fid: FnId,
    owner: Module,
}

impl DefinedMethod {
    fn new(fid: FnId, owner: Module) -> Self {
        Self { fid, owner }
    }

    pub fn fid(&self) -> FnId {
        self.fid
    }

    pub fn owner(&self) -> Module {
        self.owner
    }
}

impl Module {
    /// Get method for a receiver which class is `self` and `method` (IdentId) without using method cache.
    /// Returns `FnId` and its owner `Module`.
    pub(crate) fn search_method(&self, method: IdentId) -> Option<DefinedMethod> {
        let mut class = *self;
        let mut singleton_flag = self.is_singleton();
        loop {
            match class.get_instance_method(method) {
                Some(method) => {
                    return Some(DefinedMethod::new(method, class));
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

    pub(crate) fn search_method_no_inherit(&self, method: IdentId) -> Option<FnId> {
        self.get_instance_method(method)
    }

    /// Find method `id` from method tables of `self` class and all of its superclasses including their included modules.
    /// Return None if no method found.
    fn get_instance_method(&self, method: IdentId) -> Option<FnId> {
        self.ext.method_table.get(&method).cloned()
    }

    /// Add BuiltinFunc `func` named `name` to the singleton class of `self`.
    pub(crate) fn add_builtin_class_method(
        self,
        globals: &mut Globals,
        name: &str,
        func: BuiltinFunc,
    ) {
        self.get_singleton_class()
            .add_builtin_method_by_str(globals, name, func);
    }

    /// Add an instance method `func` named `name` to `self`.
    pub(crate) fn add_builtin_method_by_str(
        mut self,
        globals: &mut Globals,
        name: &str,
        func: BuiltinFunc,
    ) {
        let name = IdentId::get_id(name);
        self.add_builtin_method(globals, name, func);
    }

    /// Add a module function `func` named `name` to `self`.
    pub(crate) fn add_builtin_module_func(
        self,
        globals: &mut Globals,
        name: &str,
        func: BuiltinFunc,
    ) {
        self.add_builtin_method_by_str(globals, name, func);
        self.get_singleton_class()
            .add_builtin_method_by_str(globals, name, func);
    }
}

impl Module {
    pub(crate) fn set_var(self, id: IdentId, val: Value) -> Option<Value> {
        self.get().set_var(id, val)
    }

    pub(crate) fn set_var_by_str(self, name: &str, val: Value) {
        self.get().set_var_by_str(name, val)
    }

    pub(crate) fn get_var(&self, id: IdentId) -> Option<Value> {
        self.get().get_var(id)
    }

    pub(crate) fn set_var_if_exists(&self, id: IdentId, val: Value) -> bool {
        self.get().set_var_if_exists(id, val)
    }
}

impl Module {
    fn new_class(cinfo: ClassInfo) -> Module {
        assert!(!cinfo.is_module());
        let obj = RValue::new_class(cinfo).pack();
        obj.get_singleton_class().unwrap();
        obj.into_module()
    }

    pub(crate) fn bootstrap_class(superclass: impl Into<Option<Module>>) -> Module {
        let cinfo = ClassInfo::class_from(superclass);
        Module::new(RValue::new_bootstrap_class(cinfo).pack())
    }

    pub(crate) fn class_under(superclass: impl Into<Option<Module>>) -> Module {
        Module::new_class(ClassInfo::class_from(superclass))
    }

    pub(crate) fn class_under_object() -> Module {
        Module::new_class(ClassInfo::class_from(BuiltinClass::object()))
    }

    pub(crate) fn singleton_class_from(
        superclass: impl Into<Option<Module>>,
        target: impl Into<Value>,
    ) -> Module {
        Module::new(RValue::new_class(ClassInfo::singleton_from(superclass, target)).pack())
    }

    pub(crate) fn module() -> Module {
        Module::new(RValue::new_module(ClassInfo::module_from(None)).pack())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassInfo {
    upper: Option<Module>,
    flags: ClassFlags,
    ext: ClassRef,
}

impl Drop for ClassInfo {
    fn drop(&mut self) {
        //self.ext.free()
    }
}

impl GC<RValue> for ClassInfo {
    fn mark(&self, alloc: &mut Allocator<RValue>) {
        if let Some(upper) = &self.upper {
            upper.mark(alloc);
        }
        self.ext.const_table.values().for_each(|v| v.mark(alloc));
        if let Some(module) = &self.ext.origin {
            module.mark(alloc)
        };
    }
}

// Constructors
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

    fn class_from(superclass: impl Into<Option<Module>>) -> Self {
        Self::new(false, superclass, ClassExt::new())
    }

    fn module_from(superclass: impl Into<Option<Module>>) -> Self {
        Self::new(true, superclass, ClassExt::new())
    }

    pub(crate) fn singleton_from(
        superclass: impl Into<Option<Module>>,
        target: impl Into<Value>,
    ) -> Self {
        let target = target.into();
        Self::new(false, superclass, ClassExt::new_singleton(target))
    }
}

impl ClassInfo {
    /// Get an upper module/class of `self`.
    ///
    /// If `self` has no upper module/class, return None.
    pub(crate) fn upper(&self) -> Option<Module> {
        let mut m = self.upper?;
        loop {
            if !m.has_prepend() {
                return Some(m);
            }
            m = m.upper?;
        }
    }

    /// Get superclass of `self`.
    ///
    /// If `self` has no superclass, return nil.
    pub(crate) fn superclass(&self) -> Option<Module> {
        let mut m = self.upper?;
        loop {
            if !m.is_included() {
                return Some(m);
            };
            m = m.upper?;
        }
    }

    fn default_name(&self) -> String {
        if self.is_module() {
            format!("#<Module:0x{:016x}>", self.id())
        } else {
            format!("#<Class:0x{:016x}>", self.id())
        }
    }

    pub(crate) fn name(&self) -> String {
        match self.op_name() {
            Some(name) => name,
            None => self.default_name(),
        }
    }

    pub(crate) fn op_name(&self) -> Option<String> {
        let mut ext = self.ext;
        match &ext.name {
            Some(name) => Some(name.to_owned()),
            None => {
                if let Some(target) = ext.singleton_for {
                    let mut no_def_flag = false;
                    let s = format!(
                        "#<Class:{}>",
                        if let Some(c) = target.if_mod_class() {
                            match c.op_name() {
                                Some(name) => name,
                                None => {
                                    no_def_flag = true;
                                    self.default_name()
                                }
                            }
                        } else if let Some(o) = target.as_rvalue() {
                            o.to_s()
                        } else {
                            unreachable!()
                        }
                    );
                    if !no_def_flag {
                        ext.name = Some(s.clone());
                    };
                    Some(s)
                } else {
                    None
                }
            }
        }
    }

    pub(crate) fn inspect(&self) -> String {
        self.name()
    }

    pub(crate) fn set_name(&mut self, name: impl Into<String>) {
        self.ext.name = Some(name.into());
    }

    pub(crate) fn is_singleton(&self) -> bool {
        self.ext.singleton_for.is_some()
    }

    pub(crate) fn is_module(&self) -> bool {
        self.flags.is_module()
    }

    pub(crate) fn is_included(&self) -> bool {
        self.flags.is_included()
    }

    fn has_prepend(&self) -> bool {
        self.flags.has_prepend()
    }

    fn set_prepend(&mut self) {
        self.flags.set_prepend()
    }

    fn set_include(&mut self, origin: Module) {
        debug_assert!(!origin.is_included());
        self.flags.set_include();
        self.ext.origin = Some(origin);
    }

    pub(crate) fn append_include(&mut self, globals: &mut Globals, module: Module) {
        self.append_include_without_increment_version(module);
        globals.methods.inc_class_version();
    }

    pub(crate) fn append_include_without_increment_version(&mut self, mut module: Module) {
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

    pub(crate) fn append_prepend(
        &mut self,
        globals: &mut Globals,
        base: Module,
        mut module: Module,
    ) {
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
        globals.methods.inc_class_version();
    }

    #[inline(always)]
    pub(crate) fn origin(&self) -> Option<Module> {
        self.ext.origin
    }

    #[inline(always)]
    pub(crate) fn method_names(&self) -> indexmap::map::Keys<'_, IdentId, FnId> {
        self.ext.method_table.keys()
    }

    #[inline(always)]
    pub(crate) fn const_table(&self) -> &ConstTable {
        &self.ext.const_table
    }

    #[inline(always)]
    pub(crate) fn id(&self) -> u64 {
        self.ext.id()
    }

    pub(crate) fn add_builtin_method(
        &mut self,
        globals: &mut Globals,
        name: IdentId,
        func: BuiltinFunc,
    ) {
        let info = MethodInfo::BuiltinFunc {
            name,
            func,
            class: IdentId::get_id_from_string(self.name()),
        };
        let mmethod_id = globals.methods.add(info);
        self.add_method(globals, name, mmethod_id);
    }

    pub(crate) fn add_method(
        &mut self,
        globals: &mut Globals,
        name: IdentId,
        method_id: FnId,
    ) -> Option<FnId> {
        self.ext.add_method(globals, name, method_id)
    }

    /// Set a constant (`self`::`id`) to `val`.
    ///
    /// If `val` is a module or class object, set the name of `val` to the name of the constant.
    /// If the constant was already initialized, output warning.
    pub(crate) fn set_const(&mut self, id: IdentId, val: Value) {
        if let Some(mut module) = val.if_mod_class() {
            if module.op_name().is_none() {
                if self.class_id() == BuiltinClass::object().class_id() {
                    module.set_name(id.get_name());
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

        if let Some(ConstEntry::Value(_)) = self.ext.insert_const(id, val) {
            eprintln!("warning: already initialized constant {:?}", id);
        }
    }

    pub(crate) fn set_autoload(&mut self, id: IdentId, file_name: String) {
        self.ext.insert_const_autoload(id, file_name);
    }

    pub(crate) fn set_const_by_str(&mut self, name: &str, val: Value) {
        let id = IdentId::get_id(name);
        self.set_const(id, val)
    }

    pub(crate) fn get_mut_const(&mut self, id: IdentId) -> Option<&mut ConstEntry> {
        self.ext.get_mut_const(id)
    }

    pub(crate) fn get_const_noautoload(&mut self, id: IdentId) -> Option<Value> {
        match self.ext.get_mut_const(id) {
            Some(ConstEntry::Value(v)) => Some(*v),
            _ => None,
        }
    }

    pub(crate) fn enumerate_const(&self) -> std::collections::hash_map::Keys<IdentId, ConstEntry> {
        self.ext.enumerate_const()
    }
}

/// ClassFlags:
/// 0000 0 0 0 0
///        | | |
///        | | +-- 0 = class, 1 = module
///        | +---- 1 = included module
///        +------ 1 = module which has prepend
///
#[derive(Debug, Clone, PartialEq)]
struct ClassFlags(u8);

const IS_MODULE: u8 = 1 << 0;
const INCLUDED: u8 = 1 << 1;
const HAS_PREPEND: u8 = 1 << 2;

impl ClassFlags {
    fn new(is_module: bool) -> Self {
        ClassFlags(if is_module { IS_MODULE } else { 0 })
    }

    #[inline(always)]
    fn is_module(&self) -> bool {
        self.0 & IS_MODULE != 0
    }

    #[inline(always)]
    fn is_included(&self) -> bool {
        self.0 & INCLUDED != 0
    }

    #[inline(always)]
    fn has_prepend(&self) -> bool {
        self.0 & HAS_PREPEND != 0
    }

    #[inline(always)]
    fn set_include(&mut self) {
        self.0 |= INCLUDED;
    }

    #[inline(always)]
    fn set_prepend(&mut self) {
        self.0 |= HAS_PREPEND;
    }
}

#[derive(Debug, Clone)]
struct ClassExt {
    name: Option<String>,
    method_table: MethodTable,
    const_table: ConstTable,
    singleton_for: Option<Value>,
    /// This slot holds original module Value for include modules.
    origin: Option<Module>,
}

#[derive(Debug, Clone)]
pub enum ConstEntry {
    Autoload(String),
    Value(Value),
}

type ConstTable = FxHashMap<IdentId, ConstEntry>;

impl ConstEntry {
    pub(crate) fn mark(&self, alloc: &mut Allocator<RValue>) {
        match self {
            ConstEntry::Value(v) => v.mark(alloc),
            _ => {}
        }
    }
}

type ClassRef = Ref<ClassExt>;

impl ClassExt {
    fn new() -> Self {
        ClassExt {
            name: None,
            method_table: FxIndexMap::default(),
            const_table: FxHashMap::default(),
            singleton_for: None,
            origin: None,
        }
    }

    fn new_singleton(target: Value) -> Self {
        ClassExt {
            name: None,
            method_table: FxIndexMap::default(),
            const_table: FxHashMap::default(),
            singleton_for: Some(target),
            origin: None,
        }
    }

    fn add_method(&mut self, globals: &mut Globals, id: IdentId, info: FnId) -> Option<FnId> {
        globals.methods.inc_class_version();
        self.method_table.insert(id, info)
    }

    fn insert_const(&mut self, id: IdentId, val: Value) -> Option<ConstEntry> {
        self.const_table.insert(id, ConstEntry::Value(val))
    }

    fn insert_const_autoload(&mut self, id: IdentId, file_name: String) {
        let entry = self.const_table.get_mut(&id);
        match entry {
            Some(entry) => match entry {
                ConstEntry::Value(_) => {}
                ConstEntry::Autoload(_) => *entry = ConstEntry::Autoload(file_name),
            },
            None => {
                self.const_table.insert(id, ConstEntry::Autoload(file_name));
            }
        };
    }

    #[inline(always)]
    fn get_mut_const(&mut self, id: IdentId) -> Option<&mut ConstEntry> {
        self.const_table.get_mut(&id)
    }

    fn enumerate_const(&self) -> std::collections::hash_map::Keys<IdentId, ConstEntry> {
        self.const_table.keys()
    }
}
