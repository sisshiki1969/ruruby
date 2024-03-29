use crate::*;

pub(crate) fn init(globals: &mut Globals) -> Value {
    let io_class = Module::class_under_object();
    io_class.add_builtin_method_by_str(globals, "<<", output);
    io_class.add_builtin_method_by_str(globals, "isatty", isatty);
    io_class.add_builtin_method_by_str(globals, "tty?", isatty);
    io_class.add_builtin_method_by_str(globals, "flush", flush);
    globals.set_toplevel_constant("IO", io_class);
    let stdout = Value::ordinary_object(io_class);
    globals.set_toplevel_constant("STDOUT", stdout);
    globals.set_global_var_by_str("$>", stdout);
    globals.set_global_var_by_str("$stdout", stdout);

    io_class.into()
}

use std::io::{self, Write};

fn output(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(1)?;
    let arg0 = vm[0];
    match arg0.as_string() {
        Some(s) => print!("{}", s),
        None => {
            let s = arg0.val_to_s(vm)?;
            print!("{}", s)
        }
    };
    io::stdout().flush().unwrap();
    Ok(self_val)
}

fn isatty(_: &mut VM, _: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    Ok(Value::true_val())
}

fn flush(_: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    io::stdout().flush().unwrap();
    Ok(self_val)
}
