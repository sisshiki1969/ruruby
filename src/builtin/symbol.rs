use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let mut symbol_class = ClassInfo::from(globals.builtins.object);
    symbol_class.add_builtin_method_by_str("to_sym", to_sym);

    let symbol_obj = Value::class(symbol_class);
    symbol_obj
}

fn to_sym(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(self_val)
}
