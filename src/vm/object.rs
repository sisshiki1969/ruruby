use super::class::ClassRef;
use crate::vm::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct InstanceInfo {
    pub classref: ClassRef,
    pub instance_var: ValueTable,
}

impl InstanceInfo {
    pub fn new(classref: ClassRef) -> Self {
        InstanceInfo {
            classref,
            instance_var: HashMap::new(),
        }
    }
}

pub type InstanceRef = Ref<InstanceInfo>;

impl InstanceRef {
    pub fn from(classref: ClassRef) -> Self {
        InstanceRef::new(InstanceInfo::new(classref))
    }

    pub fn get_instance_method(&self, id: IdentId) -> Option<&MethodRef> {
        self.classref.instance_method.get(&id)
    }
}

pub fn init_object(globals: &mut Globals) {
    let object = globals.object_class;
    globals.add_builtin_instance_method(object, "class", object_class);
}

pub fn object_class(vm: &mut VM, receiver: PackedValue, _args: Vec<PackedValue>) -> VMResult {
    let val = match receiver.unpack() {
        Value::Class(_cref) => PackedValue::class(vm.globals.class_class),
        Value::Instance(iref) => PackedValue::class(iref.classref),
        Value::Array(_) => PackedValue::class(vm.globals.array_class),
        _ => PackedValue::class(vm.globals.object_class),
    };
    Ok(val)
}
