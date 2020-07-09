use crate::*;

pub fn init_error(globals: &mut Globals) -> Value {
    let id = IdentId::get_id("RuntimeError");
    let class = ClassRef::from(id, globals.builtins.object);
    Value::class(globals, class)
}
