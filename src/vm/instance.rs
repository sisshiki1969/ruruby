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

