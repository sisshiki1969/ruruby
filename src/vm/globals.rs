use crate::vm::*;

#[derive(Debug, Clone)]
pub struct Globals {
    // Global info
    pub ident_table: IdentifierTable,
    method_table: GlobalMethodTable,
    toplevel_method: MethodTable,
    pub array_class: Option<ClassRef>,
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
            array_class: None,
        };
        globals.get_ident_id("initialize");
        globals
    }
    pub fn add_builtin_method(&mut self, name: impl Into<String>, func: BuiltinFunc) {
        let name = name.into();
        let id = self.get_ident_id(&name);
        let info = MethodInfo::BuiltinFunc { name, func };
        let methodref = self.add_method(info);
        self.add_toplevel_method(id, methodref);
    }

    pub fn get_ident_name(&self, id: IdentId) -> &String {
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

    pub fn add_method(&mut self, info: MethodInfo) -> MethodRef {
        self.method_table.add_method(info)
    }

    pub fn get_method_info(&self, method: MethodRef) -> &MethodInfo {
        self.method_table.get_method(method)
    }

    pub fn get_mut_method_info(&mut self, method: MethodRef) -> &mut MethodInfo {
        self.method_table.get_mut_method(method)
    }

    pub fn add_builtin_class_method(
        &mut self,
        classref: ClassRef,
        name: impl Into<String>,
        func: BuiltinFunc,
    ) {
        let name = name.into();
        let id = self.get_ident_id(&name);
        let info = MethodInfo::BuiltinFunc { name, func };
        let func_ref = self.add_method(info);
        classref.clone().add_class_method(id, func_ref);
    }

    pub fn add_builtin_instance_method(
        &mut self,
        classref: ClassRef,
        name: impl Into<String>,
        func: BuiltinFunc,
    ) {
        let name = name.into();
        let id = self.get_ident_id(&name);
        let info = MethodInfo::BuiltinFunc { name, func };
        let methodref = self.add_method(info);
        classref.clone().add_instance_method(id, methodref);
    }

    pub fn get_class_name(&self, val:PackedValue) -> String {
        match val.unpack() {
            Value::Nil => "NilClass".to_string(),
            Value::Bool(true) => "TrueClass".to_string(),
            Value::Bool(false) => "FalseClass".to_string(),
            Value::FixNum(_) => "Integer".to_string(),
            Value::FloatNum(_) => "Float".to_string(),
            Value::String(_) => "String".to_string(),
            Value::Symbol(_) => "Symbol".to_string(),
            Value::Array(_) => "Array".to_string(),
            Value::Range(_) => "Range".to_string(),
            Value::Class(_) => "Class".to_string(),
            Value::Instance(iref) => {
                self.get_ident_name(iref.classref.id).clone()
            }
            Value::Char(_) => "Char".to_string(),
        }
    }
}
