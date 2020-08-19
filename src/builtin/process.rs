use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let id = IdentId::get_id("Process");
    let class = ClassRef::from(id, BuiltinClass::object());
    let mut obj = Value::class(class);
    globals.add_builtin_class_method(obj, "clock_gettime", clock_gettime);
    let id = IdentId::get_id("CLOCK_MONOTONIC");
    obj.set_var(id, Value::fixnum(0));
    obj
}

// Class methods

fn clock_gettime(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(self_val, args.len(), 1)?;
    let duration = vm.globals.instant.elapsed();
    Ok(Value::flonum(duration.as_secs_f64()))
}
