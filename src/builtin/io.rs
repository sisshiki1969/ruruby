use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let mut io_class = ClassInfo::from(globals.builtins.object);
    io_class.add_builtin_method_by_str("<<", output);
    io_class.add_builtin_method_by_str("isatty", isatty);
    io_class.add_builtin_method_by_str("tty?", isatty);
    io_class.add_builtin_method_by_str("flush", flush);

    let io_obj = Value::class(io_class);
    let stdout = Value::ordinary_object(io_obj);
    globals.builtins.object.set_var_by_str("STDOUT", stdout);
    globals.set_global_var_by_str("$>", stdout);
    globals.set_global_var_by_str("$stdout", stdout);

    io_obj
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

fn flush(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    io::stdout().flush().unwrap();
    Ok(self_val)
}
