use super::class::ClassRef;
use super::value::Value;
use crate::util::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct InstanceInfo {
    pub classref: ClassRef,
    pub class_name: String,
    pub instance_var: HashMap<IdentId, Value>,
}

impl InstanceInfo {
    pub fn new(classref: ClassRef, class_name: String) -> Self {
        InstanceInfo {
            classref,
            class_name,
            instance_var: HashMap::new(),
        }
    }

    pub fn get_classref(&self) -> ClassRef {
        self.classref
    }

    pub fn get_instance_var(&self, id: IdentId) -> Option<&Value> {
        self.instance_var.get(&id)
    }

    pub fn get_mut_instance_var(&mut self, id: IdentId) -> Option<&mut Value> {
        self.instance_var.get_mut(&id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceRef(usize);

impl std::hash::Hash for InstanceRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalInstanceTable {
    table: HashMap<InstanceRef, InstanceInfo>,
    instance_id: usize,
}

impl GlobalInstanceTable {
    pub fn new() -> Self {
        GlobalInstanceTable {
            table: HashMap::new(),
            instance_id: 0,
        }
    }

    pub fn new_instance(&mut self, classref: ClassRef, class_name: String) -> InstanceRef {
        let info = InstanceInfo::new(classref, class_name);
        let new_instance = InstanceRef(self.instance_id);
        self.instance_id += 1;
        self.table.insert(new_instance, info);
        new_instance
    }

    pub fn get(&self, instance_ref: InstanceRef) -> &InstanceInfo {
        self.table
            .get(&instance_ref)
            .unwrap_or_else(|| panic!("GlobalInstanceTable#get(): InstanceRef is not valid."))
    }

    pub fn get_mut(&mut self, instance_ref: InstanceRef) -> &mut InstanceInfo {
        self.table
            .get_mut(&instance_ref)
            .unwrap_or_else(|| panic!("GlobalInstanceTable#get_mut(): InstanceRef is not valid."))
    }
}
