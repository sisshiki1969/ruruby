use crate::*;

pub fn init_method(globals: &mut Globals) -> Value {
    let proc_id = IdentId::get_ident_id("Method");
    let class = ClassRef::from(proc_id, globals.builtins.object);
    globals.add_builtin_instance_method(class, "call", method_call);
    Value::class(globals, class)
}

pub fn method_call(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let method = match self_val.as_method() {
        Some(method) => method,
        None => return Err(vm.error_unimplemented("Expected Method object.")),
    };
    let res = vm.eval_send(method.method, method.receiver, args)?;
    Ok(res)
}
