use crate::*;

pub fn init(_globals: &mut Globals) -> Value {
    let class = ClassInfo::from(BuiltinClass::object());
    Value::class(class)
}
