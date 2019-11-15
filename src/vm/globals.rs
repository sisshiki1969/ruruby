use crate::vm::*;

#[derive(Debug, Clone)]
pub struct Globals {
    // Global info
    pub ident_table: IdentifierTable,
    method_table: GlobalMethodTable,
    toplevel_method: MethodTable,
}

impl Globals {
    pub fn new(ident_table: Option<IdentifierTable>) -> Self {
        let mut globals = Globals {
            ident_table: match ident_table {
                Some(table) => table,
                None => IdentifierTable::new(),
            },
            method_table: GlobalMethodTable::new(),
            toplevel_method: MethodTable::new(),
        };
        globals.get_ident_id("initialize");
        globals
    }
    pub fn add_builtin_method(&mut self, name: impl Into<String>, func: BuiltinFunc) {
        let name = name.into();
        let id = self.get_ident_id(&name.clone());
        let info = MethodInfo::BuiltinFunc { name, func };
        let methodref = self.add_method(info);
        self.add_toplevel_method(id, methodref);
    }

    pub fn get_ident_name(&mut self, id: IdentId) -> &String {
        self.ident_table.get_name(id)
    }

    pub fn get_ident_id(&mut self, name: impl Into<String>) -> IdentId {
        self.ident_table.get_ident_id(&name.into())
    }

    pub fn add_toplevel_method(&mut self, id: IdentId, info: MethodRef) {
        self.toplevel_method.insert(id, info);
    }

    pub fn get_toplevel_method(&self, id: IdentId) -> Option<&MethodRef> {
        self.toplevel_method.get(&id)
    }

    pub fn add_class(&mut self, id: IdentId) -> ClassRef {
        let name = self.get_ident_name(id).clone();
        ClassRef::new(id, name)
    }

    pub fn add_method(&mut self, info: MethodInfo) -> MethodRef {
        self.method_table.add_method(info)
    }

    pub fn get_method_info(&self, method: MethodRef) -> &MethodInfo {
        self.method_table.get_method(method)
    }

    pub fn get_mut_method_info(&mut self, method: MethodRef) -> &mut MethodInfo {
        self.method_table.get_mut_method(method)
    }
}
