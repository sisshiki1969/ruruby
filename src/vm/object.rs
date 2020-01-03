use super::array::ArrayRef;
use super::class::ClassRef;
use super::hash::HashRef;
use super::procobj::ProcRef;
use super::range::RangeRef;
use crate::vm::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectInfo {
    pub classref: ClassRef,
    pub instance_var: ValueTable,
    pub kind: ObjKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjKind {
    Ordinary,
    Class(ClassRef),
    Module(ClassRef),
    Array(ArrayRef),
    Hash(HashRef),
    Range(RangeRef),
    Proc(ProcRef),
}

impl ObjectInfo {
    pub fn new(classref: ClassRef) -> Self {
        ObjectInfo {
            classref,
            instance_var: HashMap::new(),
            kind: ObjKind::Ordinary,
        }
    }

    pub fn new_class(globals: &Globals, classref: ClassRef) -> Self {
        ObjectInfo {
            classref: globals.class_class,
            instance_var: HashMap::new(),
            kind: ObjKind::Class(classref),
        }
    }

    pub fn new_module(globals: &Globals, classref: ClassRef) -> Self {
        ObjectInfo {
            classref: globals.module_class,
            instance_var: HashMap::new(),
            kind: ObjKind::Module(classref),
        }
    }

    pub fn new_array(globals: &Globals, arrayref: ArrayRef) -> Self {
        ObjectInfo {
            classref: globals.array_class,
            instance_var: HashMap::new(),
            kind: ObjKind::Array(arrayref),
        }
    }

    pub fn new_hash(globals: &Globals, hashref: HashRef) -> Self {
        ObjectInfo {
            classref: globals.hash_class,
            instance_var: HashMap::new(),
            kind: ObjKind::Hash(hashref),
        }
    }

    pub fn new_range(globals: &Globals, rangeref: RangeRef) -> Self {
        ObjectInfo {
            classref: globals.range_class,
            instance_var: HashMap::new(),
            kind: ObjKind::Range(rangeref),
        }
    }

    pub fn new_proc(globals: &Globals, procref: ProcRef) -> Self {
        ObjectInfo {
            classref: globals.proc_class,
            instance_var: HashMap::new(),
            kind: ObjKind::Proc(procref),
        }
    }
}

pub type ObjectRef = Ref<ObjectInfo>;

impl ObjectRef {
    pub fn from(classref: ClassRef) -> Self {
        ObjectRef::new(ObjectInfo::new(classref))
    }

    pub fn new_class(globals: &Globals, classref: ClassRef) -> Self {
        ObjectRef::new(ObjectInfo::new_class(globals, classref))
    }

    pub fn new_module(globals: &Globals, classref: ClassRef) -> Self {
        ObjectRef::new(ObjectInfo::new_module(globals, classref))
    }

    pub fn new_array(globals: &Globals, arrayref: ArrayRef) -> Self {
        ObjectRef::new(ObjectInfo::new_array(globals, arrayref))
    }

    pub fn new_hash(globals: &Globals, hashref: HashRef) -> Self {
        ObjectRef::new(ObjectInfo::new_hash(globals, hashref))
    }

    pub fn new_range(globals: &Globals, rangeref: RangeRef) -> Self {
        ObjectRef::new(ObjectInfo::new_range(globals, rangeref))
    }

    pub fn new_proc(globals: &Globals, context: ContextRef) -> Self {
        ObjectRef::new(ObjectInfo::new_proc(globals, ProcRef::from(context)))
    }

    pub fn get_instance_method(&self, id: IdentId) -> Option<&MethodRef> {
        self.classref.instance_method.get(&id)
    }
}

pub fn init_object(globals: &mut Globals) {
    let object = globals.object_class;
    globals.add_builtin_instance_method(object, "class", object_class);
}

fn object_class(
    vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let class = receiver.get_class(&vm.globals);
    Ok(PackedValue::class(&mut vm.globals, class))
}
