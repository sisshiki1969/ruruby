use crate::vm::*;

#[derive(Debug, Clone)]
pub struct Globals {
    // Global info
    pub ident_table: IdentifierTable,
    class_table: GlobalClassTable,
    instance_table: GlobalInstanceTable,
    method_table: MethodTable,
}

impl Globals {
    pub fn new(ident_table: Option<IdentifierTable>) -> Self {
        Globals {
            ident_table: match ident_table {
                Some(table) => table,
                None => IdentifierTable::new(),
            },
            class_table: GlobalClassTable::new(),
            method_table: MethodTable::new(),
            instance_table: GlobalInstanceTable::new(),
        }
    }

    pub fn get_ident_name(&mut self, id: IdentId) -> &String {
        self.ident_table.get_name(id)
    }

    pub fn get_ident_id(&mut self, name: &String) -> IdentId {
        self.ident_table.get_ident_id(name)
    }

    pub fn add_method(&mut self, id: IdentId, info: MethodInfo) {
        self.method_table.insert(id, info);
    }

    pub fn get_method(&mut self, id: IdentId) -> Option<&MethodInfo> {
        self.method_table.get(&id)
    }

    pub fn add_class(
        &mut self,
        id: IdentId,
        name: String,
        iseq: ISeq,
        lvar: LvarCollector,
    ) -> ClassRef {
        self.class_table.new_class(id, name, iseq, lvar)
    }
    pub fn get_class_info(&self, class: ClassRef) -> &ClassInfo {
        self.class_table.get(class)
    }
    pub fn get_mut_class_info(&mut self, class: ClassRef) -> &mut ClassInfo {
        self.class_table.get_mut(class)
    }

    pub fn new_instance(&mut self, class_id: ClassRef) -> InstanceRef {
        let class_info = self.class_table.get(class_id);
        let class_name = class_info.name.clone();
        self.instance_table.new_instance(class_id, class_name)
    }
}
