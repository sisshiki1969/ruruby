use crate::*;
//use chrono;

pub fn init_time(globals: &mut Globals) -> Value {
    let time_id = IdentId::get_id("Time");
    let class = ClassRef::from(time_id, globals.builtins.object);
    let class_obj = Value::class(globals, class);
    //globals.add_builtin_instance_method(class, "now", now);
    globals.add_builtin_class_method(class_obj, "now", time_now);
    class_obj
}

fn time_now(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(self_val, args.len(), 0)?;
    let new_obj = Value::ordinary_object(self_val);
    Ok(new_obj)
}
