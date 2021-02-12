use crate::*;

pub fn init() -> Module {
    let mut class = Module::class_under_object();
    BuiltinClass::set_toplevel_constant("Numeric", class);
    class.append_include_without_increment_version(BuiltinClass::comparable());
    class
}
