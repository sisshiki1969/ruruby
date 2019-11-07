use super::codegen::{ISeq, MethodInfo, MethodTable};
use super::value::Value;
use crate::parser::LvarCollector;
use crate::util::*;
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

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub id: IdentId,
    pub name: String,
    pub iseq: ISeq,
    pub lvar: LvarCollector,
    pub instance_var: HashMap<IdentId, Value>,
    pub instance_method: MethodTable,
    pub class_method: MethodTable,
    pub subclass: HashMap<IdentId, ClassRef>,
}

impl ClassInfo {
    pub fn new(id: IdentId, name: String, iseq: ISeq, lvar: LvarCollector) -> Self {
        ClassInfo {
            id,
            name,
            iseq,
            lvar,
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

    pub fn add_instance_method(&mut self, id: IdentId, info: MethodInfo) -> Option<MethodInfo> {
        self.instance_method.insert(id, info)
    }
}

impl From<u32> for ClassRef {
    fn from(num: u32) -> Self {
        ClassRef(num as usize)
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
    pub fn new_classref(&mut self) -> ClassRef {
        let new_class = ClassRef(self.class_id);
        self.class_id += 1;
        new_class
    }

    pub fn add_class(&mut self, id: IdentId, name: String, lvar: LvarCollector) -> ClassRef {
        let classref = ClassRef(self.class_id);
        self.class_id += 1;

        let info = ClassInfo::new(id, name, vec![], lvar);
        self.table.insert(classref, info);
        classref
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
