use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let class = ClassInfo::from(globals.builtins.object);
    Value::class(class)
}
