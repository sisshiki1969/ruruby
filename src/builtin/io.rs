use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let io_id = IdentId::get_id("IO");
    let mut class = ClassInfo::from(io_id, BuiltinClass::object());
    class.add_builtin_method_by_str("<<", output);
    class.add_builtin_method_by_str("isatty", isatty);
    class.add_builtin_method_by_str("tty?", isatty);

    let obj = Value::class(class);
    let stdout = Value::ordinary_object(obj);
    BuiltinClass::object().set_var_by_str("STDOUT", stdout);
    let id = IdentId::get_id("$>");
    globals.set_global_var(id, stdout);

    obj
}

use std::io::{self, Write};

fn output(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    match args[0].as_string() {
        Some(s) => print!("{}", s),
        None => {
            let s = vm.val_to_s(args[0])?;
            print!("{}", s)
        }
    };
    io::stdout().flush().unwrap();
    Ok(self_val)
}

fn isatty(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    Ok(Value::true_val())
}
