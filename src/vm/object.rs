use super::array::ArrayRef;
use super::class::ClassRef;
use super::hash::HashRef;
use super::procobj::ProcRef;
use super::range::RangeRef;
use super::regexp::RegexpRef;
use crate::vm::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectInfo {
    pub class: PackedValue,
    pub singleton: Option<PackedValue>,
    pub instance_var: ValueTable,
    pub kind: ObjKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjKind {
    Ordinary,
    Class(ClassRef),
    Module(ClassRef),
    Array(ArrayRef),
    SplatArray(ArrayRef), // internal use only.
    Hash(HashRef),
    Range(RangeRef),
    Proc(ProcRef),
    Regexp(RegexpRef),
    Method(MethodObjRef),
}

impl ObjectInfo {
    pub fn new(class: PackedValue) -> Self {
        ObjectInfo {
            class,
            instance_var: HashMap::new(),
            kind: ObjKind::Ordinary,
            singleton: None,
        }
    }

    pub fn new_class(globals: &Globals, classref: ClassRef) -> Self {
        ObjectInfo {
            class: globals.class,
            instance_var: HashMap::new(),
            kind: ObjKind::Class(classref),
            singleton: None,
        }
    }

    pub fn new_module(globals: &Globals, classref: ClassRef) -> Self {
        ObjectInfo {
            class: globals.module,
            instance_var: HashMap::new(),
            kind: ObjKind::Module(classref),
            singleton: None,
        }
    }

    pub fn new_array(globals: &Globals, arrayref: ArrayRef) -> Self {
        ObjectInfo {
            class: globals.array,
            instance_var: HashMap::new(),
            kind: ObjKind::Array(arrayref),
            singleton: None,
        }
    }

    pub fn new_splat(globals: &Globals, arrayref: ArrayRef) -> Self {
        ObjectInfo {
            class: globals.array,
            instance_var: HashMap::new(),
            kind: ObjKind::SplatArray(arrayref),
            singleton: None,
        }
    }

    pub fn new_hash(globals: &Globals, hashref: HashRef) -> Self {
        ObjectInfo {
            class: globals.hash,
            instance_var: HashMap::new(),
            kind: ObjKind::Hash(hashref),
            singleton: None,
        }
    }

    pub fn new_regexp(globals: &Globals, regexpref: RegexpRef) -> Self {
        ObjectInfo {
            class: globals.regexp,
            instance_var: HashMap::new(),
            kind: ObjKind::Regexp(regexpref),
            singleton: None,
        }
    }

    pub fn new_range(globals: &Globals, rangeref: RangeRef) -> Self {
        ObjectInfo {
            class: globals.range,
            instance_var: HashMap::new(),
            kind: ObjKind::Range(rangeref),
            singleton: None,
        }
    }

    pub fn new_proc(globals: &Globals, procref: ProcRef) -> Self {
        ObjectInfo {
            class: globals.procobj,
            instance_var: HashMap::new(),
            kind: ObjKind::Proc(procref),
            singleton: None,
        }
    }

    pub fn new_method(globals: &Globals, methodref: MethodObjRef) -> Self {
        ObjectInfo {
            class: globals.method,
            instance_var: HashMap::new(),
            kind: ObjKind::Method(methodref),
            singleton: None,
        }
    }
}

pub type ObjectRef = Ref<ObjectInfo>;

impl ObjectRef {
    pub fn from(class: PackedValue) -> Self {
        ObjectRef::new(ObjectInfo::new(class))
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

    pub fn new_splat(globals: &Globals, arrayref: ArrayRef) -> Self {
        ObjectRef::new(ObjectInfo::new_splat(globals, arrayref))
    }

    pub fn new_hash(globals: &Globals, hashref: HashRef) -> Self {
        ObjectRef::new(ObjectInfo::new_hash(globals, hashref))
    }

    pub fn new_regexp(globals: &Globals, regexpref: RegexpRef) -> Self {
        ObjectRef::new(ObjectInfo::new_regexp(globals, regexpref))
    }

    pub fn new_range(globals: &Globals, rangeref: RangeRef) -> Self {
        ObjectRef::new(ObjectInfo::new_range(globals, rangeref))
    }

    pub fn new_proc(globals: &Globals, context: ContextRef) -> Self {
        ObjectRef::new(ObjectInfo::new_proc(globals, ProcRef::from(context)))
    }

    pub fn new_method(
        globals: &Globals,
        name: IdentId,
        receiver: PackedValue,
        method: MethodRef,
    ) -> Self {
        ObjectRef::new(ObjectInfo::new_method(
            globals,
            MethodObjRef::from(name, receiver, method),
        ))
    }

    pub fn class(&self) -> ClassRef {
        self.class.as_class().unwrap()
    }

    pub fn get_instance_method(&self, id: IdentId) -> Option<MethodRef> {
        self.class().instance_method.get(&id).cloned()
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
    let class = receiver.get_class(&vm.globals);
    Ok(PackedValue::class(&mut vm.globals, class))
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
    match receiver.unpack() {
        Value::Object(mut obj) => match obj.singleton {
            Some(class) => Ok(class),
            None => {
                let mut singleton_class = ClassRef::from_no_superclass(None);
                singleton_class.is_singleton = true;
                let singleton_obj = PackedValue::class(&vm.globals, singleton_class);
                obj.singleton = Some(singleton_obj);
                Ok(singleton_obj)
            }
        },
        _ => Err(vm.error_type("Can not define singleton.")),
    }
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
