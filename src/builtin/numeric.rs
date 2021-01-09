use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let mut class = Module::class_under(globals.builtins.object);
    class.append_include(Module::new(globals.builtins.comparable), globals);
    globals.set_toplevel_constant("Numeric", class.get());
    class.get()
}
