use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let exception = Value::class_from(globals.builtins.object);
    let standard_error = Value::class_from(exception);
    BuiltinClass::set_class("StandardError", standard_error);
    let runtime_error = Value::class_from(standard_error);
    BuiltinClass::set_class("RuntimeError", runtime_error);
    exception
}
