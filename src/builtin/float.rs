use crate::*;

pub fn init_float(globals: &mut Globals) -> Value {
    let id = globals.get_ident_id("Float");
    let class = ClassRef::from(id, globals.builtins.object);
    globals.add_builtin_instance_method(class, "<=>", cmp);
    globals.add_builtin_instance_method(class, "floor", floor);
    Value::class(globals, class)
}

// Class methods

// Instance methods

fn cmp(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    //use std::cmp::Ordering;
    vm.check_args_num(args.len(), 1)?;
    let lhs = vm.expect_flonum(self_val, "Receiver")?;
    let res = match args[0].unpack() {
        RV::Integer(rhs) => lhs.partial_cmp(&(rhs as f64)),
        RV::Float(rhs) => lhs.partial_cmp(&rhs),
        _ => return Ok(Value::nil()),
    };
    match res {
        Some(ord) => Ok(Value::fixnum(ord as i64)),
        None => Ok(Value::nil()),
    }
}

fn floor(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    match self_val.unpack() {
        RV::Float(f) => {
            let i = f.floor() as i64;
            Ok(Value::fixnum(i))
        }
        _ => Err(vm.error_type("Receiver must be a Float.")),
    }
}
