use crate::vm::*;

pub fn init_string(globals: &mut Globals) -> ClassRef {
    let id = globals.get_ident_id("String");
    let class = ClassRef::from(id, globals.object_class);
    globals.add_builtin_instance_method(class, "start_with?", string_start_with);
    /*
    globals.add_builtin_class_method(class, "new", range_new);
    */
    class
}

fn string_start_with(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let string = receiver.as_string().unwrap();
    let arg = match args[0].as_string() {
        Some(arg) => arg,
        None => return Err(vm.error_argument("An arg must be a String.")),
    };
    let res = string.starts_with(&arg);
    Ok(PackedValue::bool(res))
}
