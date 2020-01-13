use crate::util::*;
use crate::vm::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: IdentId,
    pub instance_method: MethodTable,
    pub class_method: MethodTable,
    pub constants: ValueTable,
    pub superclass: Option<ClassRef>,
}

impl ClassInfo {
    pub fn new(name: IdentId, superclass: Option<ClassRef>) -> Self {
        ClassInfo {
            name,
            instance_method: HashMap::new(),
            class_method: HashMap::new(),
            constants: HashMap::new(),
            superclass,
        }
    }
}

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

pub fn init_class(globals: &mut Globals) -> ClassRef {
    let class_id = globals.get_ident_id("Class");
    let class = ClassRef::from(class_id, globals.module_class);
    globals.add_builtin_instance_method(class, "new", class_new);
    globals.add_builtin_instance_method(class, "superclass", superclass);
    globals.add_builtin_class_method(class, "new", class_class_new);
    class
}

/// Built-in function "new".
fn class_class_new(
    vm: &mut VM,
    _receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let id = vm.globals.get_ident_id("nil");
    let classref = ClassRef::from(id, vm.globals.object_class);
    let val = PackedValue::class(&mut vm.globals, classref);

    Ok(val)
}

/// Built-in function "new".
fn class_new(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let class = vm.val_as_class(receiver)?;
    let instance = ObjectRef::from(class);
    let new_instance = PackedValue::object(instance);
    // call initialize method.
    if let Some(methodref) = class.get_instance_method(IdentId::INITIALIZE) {
        let iseq = vm.globals.get_method_info(*methodref).as_iseq(&vm)?;
        vm.vm_run(new_instance, iseq, None, args, None, None)?;
        vm.exec_stack.pop().unwrap();
    };
    Ok(new_instance)
}

fn superclass(
    vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let class = vm.val_as_class(receiver)?;
    match class.superclass {
        Some(superclass) => Ok(PackedValue::class(&mut vm.globals, superclass)),
        None => Ok(PackedValue::nil()),
    }
}
