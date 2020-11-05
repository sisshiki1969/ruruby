use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let exception = Value::class_from(globals.builtins.object);
    let standard_error = Value::class_from(exception);
    globals.set_constant("StandardError", standard_error);
    let runtime_error = Value::class_from(standard_error);
    globals.set_constant("RuntimeError", runtime_error);
    exception
}
