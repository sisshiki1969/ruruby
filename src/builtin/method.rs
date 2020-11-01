use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let mut class = ClassInfo::from(globals.builtins.object);
    class.add_builtin_method_by_str("call", method_call);
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
