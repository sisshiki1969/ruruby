use crate::vm::*;

pub fn init_module(globals: &mut Globals) -> ClassRef {
    let id = globals.get_ident_id("Module");
    let class = ClassRef::from(id, globals.object_class);
    globals.add_builtin_instance_method(class, "constants", constants);
    globals.add_builtin_instance_method(class, "attr_accessor", attr_accessor);
    globals.add_builtin_instance_method(class, "attr", attr_reader);
    globals.add_builtin_instance_method(class, "attr_reader", attr_reader);
    globals.add_builtin_instance_method(class, "attr_writer", attr_writer);
    class
}

fn constants(
    vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let class = vm.val_as_class(receiver)?;
    let v: Vec<PackedValue> = class
        .constants
        .keys()
        .map(|k| PackedValue::symbol(k.clone()))
        .collect();
    Ok(PackedValue::array(&vm.globals, ArrayRef::from(v)))
}

fn attr_accessor(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let class = vm.val_as_class(receiver)?;
    for arg in args.iter() {
        if arg.is_packed_symbol() {
            let id = arg.as_packed_symbol();
            define_reader(vm, class, id);
            define_writer(vm, class, id);
        } else {
            return Err(vm.error_name("Each of args for attr_accessor must be a symbol."));
        }
    }
    Ok(PackedValue::nil())
}

fn attr_reader(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let class = vm.val_as_class(receiver)?;
    for arg in args.iter() {
        if arg.is_packed_symbol() {
            let id = arg.as_packed_symbol();
            define_reader(vm, class, id);
        } else {
            return Err(vm.error_name("Each of args for attr_accessor must be a symbol."));
        }
    }
    Ok(PackedValue::nil())
}

fn attr_writer(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let class = vm.val_as_class(receiver)?;
    for arg in args.iter() {
        if arg.is_packed_symbol() {
            let id = arg.as_packed_symbol();
            define_writer(vm, class, id);
        } else {
            return Err(vm.error_name("Each of args for attr_accessor must be a symbol."));
        }
    }
    Ok(PackedValue::nil())
}

fn define_reader(vm: &mut VM, class: ClassRef, id: IdentId) {
    let info = MethodInfo::AttrReader { id };
    let methodref = vm.globals.add_method(info);
    vm.add_instance_method(class, id, methodref);
}

fn define_writer(vm: &mut VM, class: ClassRef, id: IdentId) {
    let assign_name = vm.globals.get_ident_name(id).clone() + "=";
    let assign_id = vm.globals.get_ident_id(assign_name);
    let info = MethodInfo::AttrWriter { id };
    let methodref = vm.globals.add_method(info);
    vm.add_instance_method(class, assign_id, methodref);
}
