use crate::vm::*;
use std::path::*;

pub fn init_file(globals: &mut Globals) -> Value {
    let id = globals.get_ident_id("File");
    let class = ClassRef::from(id, globals.object);
    let obj = Value::class(globals, class);
    globals.add_builtin_class_method(obj, "join", join);
    obj
}

// Class methods

fn join(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 2, 2)?;
    let mut path = PathBuf::from(match args[0].as_string() {
        Some(s) => s,
        None => return Err(vm.error_type("Arguments ust be String.")),
    });
    let arg = PathBuf::from(match args[1].as_string() {
        Some(s) => s,
        None => return Err(vm.error_type("Arguments ust be String.")),
    });

    for p in arg.iter() {
        if p == ".." {
            path.pop();
        } else {
            path.push(p);
        }
    }
    Ok(Value::string(path.to_string_lossy().to_string()))
}
