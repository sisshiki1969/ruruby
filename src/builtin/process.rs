use crate::*;

pub fn init(_globals: &mut Globals) -> Value {
    let id = IdentId::get_id("Process");
    let class = ClassRef::from(id, BuiltinClass::object());
    let mut class_val = Value::class(class);
    class_val.add_builtin_class_method("clock_gettime", clock_gettime);
    class_val.set_var_by_str("CLOCK_MONOTONIC", Value::integer(0));
    class_val
}

// Class methods

fn clock_gettime(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let duration = vm.globals.instant.elapsed();
    Ok(Value::float(duration.as_secs_f64()))
}
