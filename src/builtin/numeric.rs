use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let class = ClassInfo::from(globals.builtins.object);
    let class_obj = Value::class(class);
    globals.set_toplevel_constant("Numeric", class_obj);
    class_obj
}
