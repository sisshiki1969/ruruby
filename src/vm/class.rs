use crate::util::*;
use crate::vm::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassRef(usize);

impl std::hash::Hash for ClassRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Into<u32> for ClassRef {
    fn into(self) -> u32 {
        self.0 as u32
    }
}

impl From<u32> for ClassRef {
    fn from(num: u32) -> Self {
        ClassRef(num as usize)
    }
}

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub id: IdentId,
    pub name: String,
    pub instance_var: ValueTable,
    pub instance_method: MethodTable,
    pub class_method: MethodTable,
    pub subclass: HashMap<IdentId, ClassRef>,
}

impl ClassInfo {
    pub fn new(id: IdentId, name: String) -> Self {
        ClassInfo {
            id,
            name,
            instance_var: HashMap::new(),
            instance_method: HashMap::new(),
            class_method: HashMap::new(),
            subclass: HashMap::new(),
        }
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
pub struct GlobalClassTable {
    table: Vec<ClassInfo>,
    class_id: usize,
}

impl GlobalClassTable {
    pub fn new() -> Self {
        GlobalClassTable {
            table: vec![],
            class_id: 0,
        }
    }

    pub fn add_class(&mut self, id: IdentId, name: String) -> ClassRef {
        let classref = ClassRef(self.class_id);
        self.class_id += 1;

        self.table.push(ClassInfo::new(id, name));
        classref
    }

    pub fn get(&self, class_ref: ClassRef) -> &ClassInfo {
        &self.table[class_ref.0]
    }

    pub fn get_mut(&mut self, class_ref: ClassRef) -> &mut ClassInfo {
        &mut self.table[class_ref.0]
    }
}
