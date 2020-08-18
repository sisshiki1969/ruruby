use crate::*;

pub fn init_io(globals: &mut Globals) -> Value {
    let io_id = IdentId::get_id("IO");
    let class = ClassRef::from(io_id, globals.builtins.object);
    let obj = Value::class(globals, class);
    globals.add_builtin_instance_method(class, "<<", output);
    globals.add_builtin_instance_method(class, "isatty", isatty);
    globals.add_builtin_instance_method(class, "tty?", isatty);
    let stdout = Value::ordinary_object(obj);
    let stdout_id = IdentId::get_id("STDOUT");
    globals.builtins.object.set_var(stdout_id, stdout);
    let id = IdentId::get_id("$>");
    globals.global_var.insert(id, stdout);

    obj
}

use std::io::{self, Write};

fn output(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(self_val, args.len(), 1)?;
    let s = match args[0].as_string() {
        Some(s) => s.clone(),
        None => vm.val_to_s(args[0]),
    };
    print!("{}", s);
    io::stdout().flush().unwrap();
    Ok(self_val)
}

fn isatty(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(self_val, args.len(), 0)?;
    Ok(Value::true_val())
}
