///
/// Comparable module
///
use crate::*;

pub fn init(_globals: &mut Globals) -> Value {
    let mut comparable = ClassRef::from_str("Comparable", None);
    comparable.add_builtin_instance_method("puts", puts);
    let comparable = Value::module(comparable);
    return comparable;
}

fn puts(_vm: &mut VM, _self_val: Value, _args: &Args) -> VMResult {
    Ok(Value::nil())
}
