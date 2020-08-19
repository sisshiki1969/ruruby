use crate::*;

pub fn init() -> Value {
    let id = IdentId::get_id("RuntimeError");
    let class = ClassRef::from(id, BuiltinClass::object());
    Value::class(class)
}
