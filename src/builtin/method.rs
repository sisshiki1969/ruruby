use crate::*;

pub fn init(_globals: &mut Globals) -> Value {
    let proc_id = IdentId::get_id("Method");
    let mut class = ClassRef::from(proc_id, BuiltinClass::object());
    class.add_builtin_instance_method("call", method_call);
    Value::class(class)
}

pub fn method_call(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let method = match self_val.as_method() {
        Some(method) => method,
        None => return Err(vm.error_unimplemented("Expected Method object.")),
    };
    let res = vm.eval_send(method.method, method.receiver, args)?;
    Ok(res)
}
