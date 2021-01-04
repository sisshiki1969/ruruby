use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let mut class = ClassInfo::from(globals.builtins.object);
    class.append_include(globals.builtins.comparable, globals);
    let class_obj = Value::class(class);
    globals.set_toplevel_constant("Numeric", class_obj);
    class_obj
}
