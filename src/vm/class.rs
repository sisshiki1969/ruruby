use crate::util::*;
use crate::vm::Context;
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

    pub fn add_class_method(&mut self, id: IdentId, info: MethodRef) -> Option<MethodRef> {
        self.class_method.insert(id, info)
    }

    pub fn get_instance_method(&self, id: IdentId) -> Option<&MethodRef> {
        self.instance_method.get(&id)
    }

    pub fn add_instance_method(&mut self, id: IdentId, info: MethodRef) -> Option<MethodRef> {
        self.instance_method.insert(id, info)
    }
}

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub id: IdentId,
    pub instance_var: ValueTable,
    pub instance_method: MethodTable,
    pub class_method: MethodTable,
    pub superclass: Option<ClassRef>,
    pub subclass: HashMap<IdentId, ClassRef>,
}

impl ClassInfo {
    pub fn new(id: IdentId, superclass: Option<ClassRef>) -> Self {
        ClassInfo {
            id,
            instance_var: HashMap::new(),
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

pub fn class_superclass(vm: &mut VM, receiver: PackedValue, _args: Vec<PackedValue>) -> VMResult {
    match receiver.unpack() {
        Value::Class(cref) => match cref.superclass {
            Some(superclass) => Ok(PackedValue::class(superclass)),
            None => Ok(PackedValue::nil()),
        },
        _ => Err(vm.error_nomethod("Illegal argument.")),
    }
}

/// Built-in function "new".
pub fn class_new(vm: &mut VM, receiver: PackedValue, args: Vec<PackedValue>) -> VMResult {
    match receiver.as_class() {
        Some(class_ref) => {
            let instance = InstanceRef::from(class_ref);
            let new_instance = PackedValue::instance(instance);
            if let Some(methodref) = class_ref.get_instance_method(IdentId::INITIALIZE) {
                let info = vm.globals.get_method_info(*methodref);
                let iseq = match info {
                    MethodInfo::RubyFunc { iseq } => iseq,
                    _ => panic!(),
                };
                let mut context = Context::new(new_instance, *iseq, CallMode::FromNative);
                let arg_len = args.len();
                for (i, id) in iseq.params.clone().iter().enumerate() {
                    context.lvar_scope[id.as_usize()] = if i < arg_len {
                        args[i]
                    } else {
                        PackedValue::nil()
                    };
                }
                vm.context_stack.last_mut().unwrap().pc = vm.pc;
                vm.vm_run(context)?;
                vm.exec_stack.pop().unwrap();
            };
            Ok(new_instance)
        }
        None => Err(vm.error_unimplemented(format!("Receiver must be a class! {:?}", receiver))),
    }
}
/// Built-in function "attr_accessor".
pub fn class_attr(vm: &mut VM, receiver: PackedValue, args: Vec<PackedValue>) -> VMResult {
    match receiver.as_class() {
        Some(classref) => {
            for arg in args {
                if arg.is_packed_symbol() {
                    let id = arg.as_packed_symbol();
                    let info = MethodInfo::AttrReader { id };
                    let methodref = vm.globals.add_method(info);
                    classref.clone().add_instance_method(id, methodref);

                    let assign_name = vm.globals.get_ident_name(id).clone() + "=";
                    let assign_id = vm.globals.get_ident_id(assign_name);
                    let info = MethodInfo::AttrWriter { id };
                    let methodref = vm.globals.add_method(info);
                    classref.clone().add_instance_method(assign_id, methodref);
                } else {
                    return Err(vm.error_name("Each of args for attr_accessor must be a symbol."));
                }
            }
        }
        None => unreachable!("Illegal attr_accesor in non-class object."),
    }
    Ok(PackedValue::nil())
}
