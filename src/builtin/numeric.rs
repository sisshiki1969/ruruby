use crate::*;

pub fn init(builtins: &BuiltinClass) -> Module {
    let mut class = Module::class_under(builtins.object);
    BuiltinClass::set_toplevel_constant("Numeric", class);
    class.append_include_without_increment_version(builtins.comparable);
    class
}
