use crate::*;

pub fn init_struct(globals: &mut Globals) -> Value {
    let id = globals.get_ident_id("Struct");
    let class = ClassRef::from(id, globals.builtins.object);
    Value::class(globals, class)
}
