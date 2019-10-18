use crate::class::*;
use crate::util::*;
use crate::value::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct InstanceInfo {
    pub class_id: ClassRef,
    pub class_name: String,
    pub instance_var: HashMap<IdentId, Value>,
}

impl InstanceInfo {
    pub fn new(class_id: ClassRef, class_name: String) -> Self {
        InstanceInfo {
            class_id,
            class_name,
            instance_var: HashMap::new(),
        }
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

    pub fn new_instance(&mut self, class_id: ClassRef, class_name: String) -> InstanceRef {
        let info = InstanceInfo::new(class_id, class_name);
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
