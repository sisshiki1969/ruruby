use crate::vm::*;

pub fn init_module(globals: &mut Globals) -> ClassRef {
    let id = globals.get_ident_id("Module");
    let class = ClassRef::from(id, globals.object_class);
    globals.add_builtin_instance_method(class, "constants", constants);
    globals.add_builtin_instance_method(class, "instance_methods", instance_methods);
    globals.add_builtin_instance_method(class, "attr_accessor", attr_accessor);
    globals.add_builtin_instance_method(class, "attr", attr_reader);
    globals.add_builtin_instance_method(class, "attr_reader", attr_reader);
    globals.add_builtin_instance_method(class, "attr_writer", attr_writer);
    globals.add_builtin_instance_method(class, "module_function", module_function);
    globals.add_builtin_instance_method(class, "singleton_class?", singleton_class);
    class
}

fn constants(
    vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let class = vm.val_as_module(receiver)?;
    let v: Vec<PackedValue> = class
        .constants
        .keys()
        .map(|k| PackedValue::symbol(k.clone()))
        .collect();
    Ok(PackedValue::array_from(&vm.globals, v))
}

fn instance_methods(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let mut class = vm.val_as_module(receiver)?;
    vm.check_args_num(args.len(), 0, 1)?;
    let inherited_too = args.len() == 0 || vm.val_to_bool(args[0]);
    match inherited_too {
        false => {
            let v = class
                .instance_method
                .keys()
                .map(|k| PackedValue::symbol(k.clone()))
                .collect();
            Ok(PackedValue::array_from(&vm.globals, v))
        }
        true => {
            let mut v = std::collections::HashSet::new();
            loop {
                v = v
                    .union(
                        &class
                            .instance_method
                            .keys()
                            .map(|k| PackedValue::symbol(*k))
                            .collect(),
                    )
                    .cloned()
                    .collect();
                match class.superclass {
                    Some(superclass) => class = superclass,
                    None => break,
                };
            }
            Ok(PackedValue::array_from(
                &vm.globals,
                v.iter().cloned().collect(),
            ))
        }
    }
}

fn attr_accessor(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let class = vm.val_as_module(receiver)?;
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
    let class = vm.val_as_module(receiver)?;
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
    let class = vm.val_as_module(receiver)?;
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
    let assign_name = vm.globals.get_ident_name(id).to_string() + "=";
    let assign_id = vm.globals.get_ident_id(assign_name);
    let info = MethodInfo::AttrWriter { id };
    let methodref = vm.globals.add_method(info);
    vm.add_instance_method(class, assign_id, methodref);
}

fn module_function(
    _vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    Ok(receiver)
}

fn singleton_class(
    vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let class = vm.val_as_module(receiver)?;
    Ok(PackedValue::bool(class.is_singleton))
}
