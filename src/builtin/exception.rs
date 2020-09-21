use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let exception = Value::class_from_str("Exception", globals.builtins.object);
    let standard_error = Value::class_from_str("StandardError", exception);
    BuiltinClass::set_class("StandardError", standard_error);
    let runtime_error = Value::class_from_str("RuntimeError", standard_error);
    BuiltinClass::set_class("RuntimeError", runtime_error);
    exception
}
