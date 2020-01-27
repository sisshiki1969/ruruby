use crate::util::*;
use crate::vm::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: Option<IdentId>,
    pub instance_method: MethodTable,
    pub constants: ValueTable,
    pub superclass: PackedValue,
    pub is_singleton: bool,
}

impl ClassInfo {
    pub fn new(name: impl Into<Option<IdentId>>, superclass: PackedValue) -> Self {
        ClassInfo {
            name: name.into(),
            instance_method: HashMap::new(),
            //class_method: HashMap::new(),
            constants: HashMap::new(),
            superclass,
            is_singleton: false,
        }
    }
}

pub type ClassRef = Ref<ClassInfo>;

impl ClassRef {
    pub fn from_no_superclass(id: impl Into<Option<IdentId>>) -> Self {
        ClassRef::new(ClassInfo::new(id, PackedValue::nil()))
    }

    pub fn from(id: impl Into<Option<IdentId>>, superclass: PackedValue) -> Self {
        ClassRef::new(ClassInfo::new(id, superclass))
    }

    pub fn get_instance_method(&self, id: IdentId) -> Option<&MethodRef> {
        self.instance_method.get(&id)
    }

    pub fn superclass(&self) -> Option<ClassRef> {
        if self.superclass.is_nil() { None } else {
            Some(self.superclass.as_class().unwrap())
        }
    }
}

pub fn init_class(globals: &mut Globals) {
    let class = globals.class_class;
    globals.add_builtin_instance_method(class, "new", class_new);
    globals.add_builtin_instance_method(class, "superclass", superclass);
    globals.add_builtin_class_method(globals.class, "new", class_class_new);
}

/// Built-in function "new".
fn class_class_new(
    vm: &mut VM,
    _receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let id = vm.globals.get_ident_id("nil");
    let classref = ClassRef::from(id, vm.globals.object);
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
    let instance = ObjectRef::from(receiver);
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
    Ok(class.superclass)
}
