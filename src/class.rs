use crate::util::{MethodInfo, MethodTable};
use crate::node::Node;
use crate::util::*;
use crate::value::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClassRef(usize);

impl std::hash::Hash for ClassRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub id: IdentId,
    pub name: String,
    pub body: Box<Node>,
    pub instance_var: HashMap<IdentId, Value>,
    pub instance_method: MethodTable,
    pub class_method: MethodTable,
    pub subclass: HashMap<IdentId, ClassRef>,
}

impl ClassInfo {
    pub fn new(id: IdentId, name: String, body: Node) -> Self {
        ClassInfo {
            id,
            name,
            body: Box::new(body),
            instance_var: HashMap::new(),
            instance_method: HashMap::new(),
            class_method: HashMap::new(),
            subclass: HashMap::new(),
        }
    }

    pub fn get_class_method(&self, id: IdentId) -> Option<&MethodInfo> {
        self.class_method.get(&id)
    }

    pub fn add_class_method(&mut self, id: IdentId, info: MethodInfo) -> Option<MethodInfo> {
        self.class_method.insert(id, info)
    }

    pub fn get_instance_method(&self, id: IdentId) -> Option<&MethodInfo> {
        self.instance_method.get(&id)
    }
}

#[derive(Debug, Clone)]
pub struct GlobalClassTable {
    table: HashMap<ClassRef, ClassInfo>,
    class_id: usize,
}

impl GlobalClassTable {
    pub fn new() -> Self {
        GlobalClassTable {
            table: HashMap::new(),
            class_id: 0,
        }
    }

    pub fn new_class(&mut self, id: IdentId, name: String, body: Node) -> ClassRef {
        let info = ClassInfo::new(id, name, body);
        let new_class = ClassRef(self.class_id);
        self.class_id += 1;
        self.table.insert(new_class, info);
        new_class
    }

    pub fn get(&self, class_ref: ClassRef) -> &ClassInfo {
        self.table
            .get(&class_ref)
            .unwrap_or_else(|| panic!("GlobalClassTable#get(): ClassRef is not valid."))
    }

    pub fn get_mut(&mut self, class_ref: ClassRef) -> &mut ClassInfo {
        self.table
            .get_mut(&class_ref)
            .unwrap_or_else(|| panic!("GlobalClassTable#get_mut(): ClassRef is not valid."))
    }
}
