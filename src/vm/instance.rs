use super::class::ClassRef;
use crate::vm::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct InstanceInfo {
    pub classref: ClassRef,
    pub class_name: String,
    pub instance_var: ValueTable,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstanceRef(usize);

impl std::hash::Hash for InstanceRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Into<u32> for InstanceRef {
    fn into(self) -> u32 {
        self.0 as u32
    }
}

impl From<u32> for InstanceRef {
    fn from(x: u32) -> Self {
        InstanceRef(x as usize)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GlobalInstanceTable {
    table: Vec<InstanceInfo>,
    instance_id: usize,
}

impl GlobalInstanceTable {
    pub fn new() -> Self {
        GlobalInstanceTable {
            table: vec![],
            instance_id: 0,
        }
    }

    pub fn new_instance(&mut self, classref: ClassRef, class_name: String) -> InstanceRef {
        let info = InstanceInfo::new(classref, class_name);
        let new_instance = InstanceRef(self.instance_id);
        self.instance_id += 1;
        self.table.push(info);
        new_instance
    }

    pub fn get(&self, instance_ref: InstanceRef) -> &InstanceInfo {
        &self.table[instance_ref.0]
    }

    pub fn get_mut(&mut self, instance_ref: InstanceRef) -> &mut InstanceInfo {
        &mut self.table[instance_ref.0]
    }
}
