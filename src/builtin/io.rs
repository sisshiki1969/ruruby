use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let io_id = IdentId::get_id("IO");
    let mut class = ClassRef::from(io_id, BuiltinClass::object());
    let obj = Value::class(class);
    class.add_builtin_instance_method("<<", output);
    class.add_builtin_instance_method("isatty", isatty);
    class.add_builtin_instance_method("tty?", isatty);
    let stdout = Value::ordinary_object(obj);
    let stdout_id = IdentId::get_id("STDOUT");
    BuiltinClass::object().set_var(stdout_id, stdout);
    let id = IdentId::get_id("$>");
    globals.global_var.insert(id, stdout);

    obj
}

use std::io::{self, Write};

fn output(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(self_val, args.len(), 1)?;
    match args[0].as_string() {
        Some(s) => print!("{}", s),
        None => {
            let s = vm.val_to_s(args[0]);
            print!("{}", s)
        }
    };
    io::stdout().flush().unwrap();
    Ok(self_val)
}

fn isatty(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(self_val, args.len(), 0)?;
    Ok(Value::true_val())
}
