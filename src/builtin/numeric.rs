use crate::*;

pub fn init(_globals: &mut Globals) -> Value {
    let class = ClassInfo::from_str("Numeric", BuiltinClass::object());
    Value::class(class)
}
