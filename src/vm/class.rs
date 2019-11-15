use crate::util::*;
use crate::vm::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassRef(*mut ClassInfo);

impl std::hash::Hash for ClassRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl ClassRef {
    pub fn new(id: IdentId) -> Self {
        let info = ClassInfo {
            id,
            instance_var: HashMap::new(),
            instance_method: HashMap::new(),
            class_method: HashMap::new(),
            subclass: HashMap::new(),
        };
        let boxed = Box::into_raw(Box::new(info));
        ClassRef(boxed)
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

impl std::ops::Deref for ClassRef {
    type Target = ClassInfo;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl std::ops::DerefMut for ClassRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0 }
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
