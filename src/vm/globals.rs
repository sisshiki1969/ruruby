use crate::vm::*;

#[derive(Debug, Clone)]
pub struct Globals {
    // Global info
    pub ident_table: IdentifierTable,
    method_table: GlobalMethodTable,
    pub main_class: ClassRef,
    pub array_class: ClassRef,
    pub class_class: ClassRef,
    pub proc_class: ClassRef,
    pub object_class: ClassRef,
}

impl Globals {
    pub fn new(ident_table: Option<IdentifierTable>) -> Self {
        let mut ident_table = match ident_table {
            Some(table) => table,
            None => IdentifierTable::new(),
        };
        let object_id = ident_table.get_ident_id("Object");
        let object_class = ClassRef::from_no_superclass(object_id);
        let main_id = ident_table.get_ident_id("main");
        let main_class = ClassRef::from(main_id, object_class);
        let mut globals = Globals {
            ident_table,
            method_table: GlobalMethodTable::new(),
            main_class,
            array_class: object_class,
            class_class: object_class,
            proc_class: object_class,
            object_class,
        };
        object::init_object(&mut globals);
        globals.array_class = array::init_array(&mut globals);
        globals.class_class = class::init_class(&mut globals);
        globals.proc_class = proc::init_proc(&mut globals);
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
        self.object_class.add_instance_method(id, info);
    }

    pub fn get_toplevel_method(&self, id: IdentId) -> Option<&MethodRef> {
        self.object_class.get_instance_method(id)
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

    pub fn get_class_name(&self, val: PackedValue) -> String {
        match val.unpack() {
            Value::Nil => "NilClass".to_string(),
            Value::Bool(true) => "TrueClass".to_string(),
            Value::Bool(false) => "FalseClass".to_string(),
            Value::FixNum(_) => "Integer".to_string(),
            Value::FloatNum(_) => "Float".to_string(),
            Value::String(_) => "String".to_string(),
            Value::Symbol(_) => "Symbol".to_string(),
            Value::Char(_) => "Char".to_string(),
            Value::Object(oref) => match oref.kind {
                ObjKind::Array(_) => "Array".to_string(),
                ObjKind::Range(_) => "Range".to_string(),
                ObjKind::Class(_) => "Class".to_string(),
                ObjKind::Proc(_) => "Proc".to_string(),
                ObjKind::Ordinary => self.get_ident_name(oref.classref.id).clone(),
            },
        }
    }
}
