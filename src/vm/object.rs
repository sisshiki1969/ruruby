use super::array::ArrayRef;
use super::class::ClassRef;
use super::hash::HashRef;
use super::procobj::ProcRef;
use super::regexp::RegexpRef;
use crate::vm::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectInfo {
    pub class: PackedValue,
    pub singleton: Option<PackedValue>,
    pub var_table: ValueTable,
    pub kind: ObjKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjKind {
    Ordinary,
    Class(ClassRef),
    Module(ClassRef),
    Range(RangeInfo),
    Array(ArrayRef),
    SplatArray(ArrayRef), // internal use only.
    Hash(HashRef),
    Proc(ProcRef),
    Regexp(RegexpRef),
    Method(MethodObjRef),
}

impl ObjectInfo {
    pub fn as_ref(&self) -> ObjectRef {
        Ref(unsafe {
            core::ptr::NonNull::new_unchecked(self as *const ObjectInfo as *mut ObjectInfo)
        })
    }

    pub fn new_ordinary(class: PackedValue) -> Self {
        ObjectInfo {
            class,
            var_table: HashMap::new(),
            kind: ObjKind::Ordinary,
            singleton: None,
        }
    }

    pub fn new_class(globals: &Globals, classref: ClassRef) -> Self {
        ObjectInfo {
            class: globals.class,
            var_table: HashMap::new(),
            kind: ObjKind::Class(classref),
            singleton: None,
        }
    }

    pub fn new_module(globals: &Globals, classref: ClassRef) -> Self {
        ObjectInfo {
            class: globals.module,
            var_table: HashMap::new(),
            kind: ObjKind::Module(classref),
            singleton: None,
        }
    }

    pub fn new_array(globals: &Globals, arrayref: ArrayRef) -> Self {
        ObjectInfo {
            class: globals.array,
            var_table: HashMap::new(),
            kind: ObjKind::Array(arrayref),
            singleton: None,
        }
    }

    pub fn new_splat(globals: &Globals, arrayref: ArrayRef) -> Self {
        ObjectInfo {
            class: globals.array,
            var_table: HashMap::new(),
            kind: ObjKind::SplatArray(arrayref),
            singleton: None,
        }
    }

    pub fn new_hash(globals: &Globals, hashref: HashRef) -> Self {
        ObjectInfo {
            class: globals.hash,
            var_table: HashMap::new(),
            kind: ObjKind::Hash(hashref),
            singleton: None,
        }
    }

    pub fn new_regexp(globals: &Globals, regexpref: RegexpRef) -> Self {
        ObjectInfo {
            class: globals.regexp,
            var_table: HashMap::new(),
            kind: ObjKind::Regexp(regexpref),
            singleton: None,
        }
    }

    pub fn new_range(globals: &Globals, info: RangeInfo) -> Self {
        ObjectInfo {
            class: globals.range,
            var_table: HashMap::new(),
            kind: ObjKind::Range(info),
            singleton: None,
        }
    }

    pub fn new_proc(globals: &Globals, procref: ProcRef) -> Self {
        ObjectInfo {
            class: globals.procobj,
            var_table: HashMap::new(),
            kind: ObjKind::Proc(procref),
            singleton: None,
        }
    }

    pub fn new_method(globals: &Globals, methodref: MethodObjRef) -> Self {
        ObjectInfo {
            class: globals.method,
            var_table: HashMap::new(),
            kind: ObjKind::Method(methodref),
            singleton: None,
        }
    }
}

pub type ObjectRef = Ref<ObjectInfo>;

impl ObjectRef {
    pub fn class(&self) -> ClassRef {
        self.class.as_class().unwrap()
    }

    pub fn get_instance_method(&self, id: IdentId) -> Option<MethodRef> {
        self.class().method_table.get(&id).cloned()
    }
}

pub fn init_object(globals: &mut Globals) {
    let object = globals.object_class;
    globals.add_builtin_instance_method(object, "class", class);
    globals.add_builtin_instance_method(object, "object_id", object_id);
    globals.add_builtin_instance_method(object, "singleton_class", singleton_class);
    globals.add_builtin_instance_method(object, "inspect", inspect);
}

fn class(
    vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let class = receiver.get_class_object(&vm.globals);
    Ok(class)
}

fn object_id(
    _vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let id = receiver.id();
    Ok(PackedValue::fixnum(id as i64))
}

fn singleton_class(
    vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    vm.get_singleton_class(receiver)
}

fn inspect(
    vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let inspect = vm.val_pp(receiver);
    Ok(PackedValue::string(inspect))
}
