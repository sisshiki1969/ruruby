use crate::util::*;
use crate::vm::*;
use std::collections::HashMap;

pub type ClassRef = Ref<ClassInfo>;

impl ClassRef {
    pub fn from(id: IdentId) -> Self {
        ClassRef::new(ClassInfo::new(id))
    }

    pub fn get_class_method(&self, id: IdentId) -> Option<&MethodRef> {
        self.class_method.get(&id)
    }

    pub fn add_class_method(&mut self, id: IdentId, info: MethodRef) -> Option<MethodRef> {
        self.class_method.insert(id, info)
    }

    pub fn get_instance_method(&self, id: IdentId) -> Option<&MethodRef> {
        self.instance_method.get(&id)
    }

    pub fn add_instance_method(&mut self, id: IdentId, info: MethodRef) -> Option<MethodRef> {
        self.instance_method.insert(id, info)
    }
}

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub id: IdentId,
    pub instance_var: ValueTable,
    pub instance_method: MethodTable,
    pub class_method: MethodTable,
    pub subclass: HashMap<IdentId, ClassRef>,
}

impl ClassInfo {
    pub fn new(id: IdentId) -> Self {
        ClassInfo {
            id,
            instance_var: HashMap::new(),
            instance_method: HashMap::new(),
            class_method: HashMap::new(),
            subclass: HashMap::new(),
        }
    }
}
