use crate::*;

pub fn init(globals: &mut Globals) -> Module {
    let mut class = Module::class_under(globals.builtins.object);
    class.append_include(globals.builtins.comparable, globals);
    globals.set_toplevel_constant("Numeric", class);
    class
}
