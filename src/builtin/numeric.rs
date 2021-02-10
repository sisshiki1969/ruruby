use crate::*;

pub fn init(builtins: &mut BuiltinClass) -> Module {
    let mut class = Module::class_under(builtins.object);
    builtins.set_toplevel_constant("Numeric", class);
    class.append_include_without_increment_version(builtins.comparable);
    class
}
