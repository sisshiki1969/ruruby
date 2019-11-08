use crate::vm::*;

#[derive(Debug, Clone)]
pub struct Globals {
    // Global info
    pub ident_table: IdentifierTable,
    class_table: GlobalClassTable,
    instance_table: GlobalInstanceTable,
    method_table: GlobalMethodTable,
    toplevel_method: MethodTable,
}

impl Globals {
    pub fn new(ident_table: Option<IdentifierTable>) -> Self {
        Globals {
            ident_table: match ident_table {
                Some(table) => table,
                None => IdentifierTable::new(),
            },
            class_table: GlobalClassTable::new(),
            instance_table: GlobalInstanceTable::new(),
            method_table: GlobalMethodTable::new(),
            toplevel_method: MethodTable::new(),
        }
    }
    pub fn add_builtin_method(&mut self, name: impl Into<String>, func: BuiltinFunc) {
        let name = name.into();
        let id = self.get_ident_id(&name.clone());
        let info = MethodInfo::BuiltinFunc { name, func };
        self.add_toplevel_method(id, info);
    }

    pub fn get_ident_name(&mut self, id: IdentId) -> &String {
        self.ident_table.get_name(id)
    }

    pub fn get_ident_id(&mut self, name: &String) -> IdentId {
        self.ident_table.get_ident_id(name)
    }

    pub fn add_toplevel_method(&mut self, id: IdentId, info: MethodInfo) {
        self.toplevel_method.insert(id, info);
    }

    pub fn get_toplevel_method(&self, id: IdentId) -> Option<&MethodInfo> {
        self.toplevel_method.get(&id)
    }

    pub fn new_classref(&mut self) -> ClassRef {
        self.class_table.new_classref()
    }

    pub fn add_class(&mut self, id: IdentId) -> ClassRef {
        let name = self.get_ident_name(id).clone();
        self.class_table.add_class(id, name)
    }

    pub fn get_class_info(&self, class: ClassRef) -> &ClassInfo {
        self.class_table.get(class)
    }

    pub fn get_mut_class_info(&mut self, class: ClassRef) -> &mut ClassInfo {
        self.class_table.get_mut(class)
    }

    pub fn get_instance_info(&self, instance: InstanceRef) -> &InstanceInfo {
        self.instance_table.get(instance)
    }

    pub fn get_mut_instance_info(&mut self, instance: InstanceRef) -> &mut InstanceInfo {
        self.instance_table.get_mut(instance)
    }

    pub fn new_instance(&mut self, class_id: ClassRef) -> InstanceRef {
        let class_info = self.class_table.get(class_id);
        let class_name = class_info.name.clone();
        self.instance_table.new_instance(class_id, class_name)
    }

    pub fn add_method(&mut self, info: MethodInfo) -> MethodRef {
        self.method_table.add_method(info)
    }

    pub fn get_method_info(&self, method: MethodRef) -> &MethodInfo {
        self.method_table.get_method(method)
    }
}
