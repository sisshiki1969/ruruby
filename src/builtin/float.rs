use crate::*;

pub fn init_float(globals: &mut Globals) -> Value {
    let id = globals.get_ident_id("Float");
    let class = ClassRef::from(id, globals.builtins.object);
    globals.add_builtin_instance_method(class, "floor", floor);
    Value::class(globals, class)
}

// Class methods

// Instance methods

fn floor(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    match args.self_value.unpack() {
        RV::FloatNum(f) => {
            let i = f.floor() as i64;
            Ok(Value::fixnum(i))
        }
        _ => Err(vm.error_type("Receiver must be a Float.")),
    }
}
