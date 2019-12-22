use crate::util::*;
use crate::vm::*;
use std::collections::HashMap;

pub type ClassRef = Ref<ClassInfo>;

impl ClassRef {
    pub fn from_no_superclass(id: IdentId) -> Self {
        ClassRef::new(ClassInfo::new(id, None))
    }

    pub fn from(id: IdentId, superclass: ClassRef) -> Self {
        ClassRef::new(ClassInfo::new(id, Some(superclass)))
    }

    pub fn get_class_method(&self, id: IdentId) -> Option<&MethodRef> {
        self.class_method.get(&id)
    }

    pub fn get_instance_method(&self, id: IdentId) -> Option<&MethodRef> {
        self.instance_method.get(&id)
    }
}

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub id: IdentId,
    pub instance_method: MethodTable,
    pub class_method: MethodTable,
    pub superclass: Option<ClassRef>,
    pub subclass: HashMap<IdentId, ClassRef>,
}

impl ClassInfo {
    pub fn new(id: IdentId, superclass: Option<ClassRef>) -> Self {
        ClassInfo {
            id,
            instance_method: HashMap::new(),
            class_method: HashMap::new(),
            superclass,
            subclass: HashMap::new(),
        }
    }
}

pub fn init_class(globals: &mut Globals) -> ClassRef {
    let class_id = globals.get_ident_id("Class");
    let class = ClassRef::from(class_id, globals.object_class);
    globals.add_builtin_instance_method(class, "superclass", class_superclass);
    globals.add_builtin_instance_method(class, "new", class_new);
    globals.add_builtin_instance_method(class, "attr_accessor", class_attr);
    class
}

// Class methods

fn class_superclass(
    vm: &mut VM,
    receiver: PackedValue,
    _args: Vec<PackedValue>,
    _block: Option<ContextRef>,
) -> VMResult {
    match receiver.as_class() {
        Some(cref) => match cref.superclass {
            Some(superclass) => Ok(PackedValue::class(&mut vm.globals, superclass)),
            None => Ok(PackedValue::nil()),
        },
        None => Err(vm.error_nomethod("Illegal argument.")),
    }
}

/// Built-in function "new".
fn class_new(
    vm: &mut VM,
    receiver: PackedValue,
    args: Vec<PackedValue>,
    _block: Option<ContextRef>,
) -> VMResult {
    match receiver.as_class() {
        Some(class_ref) => {
            let instance = ObjectRef::from(class_ref);
            let new_instance = PackedValue::object(instance);
            // call initialize method.
            if let Some(methodref) = class_ref.get_instance_method(IdentId::INITIALIZE) {
                let iseq = vm.globals.get_method_info(*methodref).as_iseq(&vm)?;
                vm.vm_run(new_instance, iseq, None, args, None)?;
                vm.exec_stack.pop().unwrap();
            };
            Ok(new_instance)
        }
        None => Err(vm.error_unimplemented(format!("Receiver must be a class! {:?}", receiver))),
    }
}
/// Built-in function "attr_accessor".
fn class_attr(
    vm: &mut VM,
    receiver: PackedValue,
    args: Vec<PackedValue>,
    _block: Option<ContextRef>,
) -> VMResult {
    match receiver.as_class() {
        Some(classref) => {
            for arg in args {
                if arg.is_packed_symbol() {
                    let id = arg.as_packed_symbol();
                    let info = MethodInfo::AttrReader { id };
                    let methodref = vm.globals.add_method(info);
                    vm.add_instance_method(classref, id, methodref);

                    let assign_name = vm.globals.get_ident_name(id).clone() + "=";
                    let assign_id = vm.globals.get_ident_id(assign_name);
                    let info = MethodInfo::AttrWriter { id };
                    let methodref = vm.globals.add_method(info);
                    vm.add_instance_method(classref, assign_id, methodref);
                } else {
                    return Err(vm.error_name("Each of args for attr_accessor must be a symbol."));
                }
            }
        }
        None => unreachable!("Illegal attr_accesor in non-class object."),
    }
    Ok(PackedValue::nil())
}
