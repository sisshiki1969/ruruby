use crate::vm::*;

pub fn init_module(globals: &mut Globals) {
    let class = globals.module_class;
    globals.add_builtin_instance_method(class, "constants", constants);
    globals.add_builtin_instance_method(class, "instance_methods", instance_methods);
    globals.add_builtin_instance_method(class, "attr_accessor", attr_accessor);
    globals.add_builtin_instance_method(class, "attr", attr_reader);
    globals.add_builtin_instance_method(class, "attr_reader", attr_reader);
    globals.add_builtin_instance_method(class, "attr_writer", attr_writer);
    globals.add_builtin_instance_method(class, "module_function", module_function);
    globals.add_builtin_instance_method(class, "singleton_class?", singleton_class);
}

fn constants(
    vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let v: Vec<PackedValue> = receiver
        .as_object()
        .unwrap()
        .var_table
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
                .method_table
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
                            .method_table
                            .keys()
                            .map(|k| PackedValue::symbol(*k))
                            .collect(),
                    )
                    .cloned()
                    .collect();
                match class.superclass() {
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
    for arg in args.iter() {
        if arg.is_packed_symbol() {
            let id = arg.as_packed_symbol();
            define_reader(vm, receiver, id);
            define_writer(vm, receiver, id);
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
    for arg in args.iter() {
        if arg.is_packed_symbol() {
            let id = arg.as_packed_symbol();
            define_reader(vm, receiver, id);
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
    for arg in args.iter() {
        if arg.is_packed_symbol() {
            let id = arg.as_packed_symbol();
            define_writer(vm, receiver, id);
        } else {
            return Err(vm.error_name("Each of args for attr_accessor must be a symbol."));
        }
    }
    Ok(PackedValue::nil())
}

fn define_reader(vm: &mut VM, class: PackedValue, id: IdentId) {
    let instance_var_id = get_instance_var(vm, id);
    let info = MethodInfo::AttrReader {
        id: instance_var_id,
    };
    let methodref = vm.globals.add_method(info);
    vm.add_instance_method(class, id, methodref);
}

fn define_writer(vm: &mut VM, class: PackedValue, id: IdentId) {
    let assign_id = vm.globals.ident_table.add_postfix(id, "=");
    let instance_var_id = get_instance_var(vm, id);
    let info = MethodInfo::AttrWriter {
        id: instance_var_id,
    };
    let methodref = vm.globals.add_method(info);
    vm.add_instance_method(class, assign_id, methodref);
}

fn get_instance_var(vm: &mut VM, id: IdentId) -> IdentId {
    vm.globals
        .get_ident_id(format!("@{}", vm.globals.get_ident_name(id)))
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
