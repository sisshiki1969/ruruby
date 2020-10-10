use crate::*;

pub fn init(_globals: &mut Globals) -> Value {
    let class = ClassRef::from_str("Numeric", BuiltinClass::object());
    Value::class(class)
}
