use crate::vm::*;

pub fn init_integer(globals: &mut Globals) -> Value {
    let id = globals.get_ident_id("Integer");
    let class = ClassRef::from(id, globals.builtins.object);
    globals.add_builtin_instance_method(class, "times", integer_times);
    globals.add_builtin_instance_method(class, "step", integer_step);
    globals.add_builtin_instance_method(class, "chr", integer_chr);
    globals.add_builtin_instance_method(class, "to_f", integer_tof);
    globals.add_builtin_instance_method(class, "even?", integer_even);
    Value::class(globals, class)
}

// Class methods

// Instance methods

fn integer_times(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let num = args.self_value.as_fixnum().unwrap();
    if num < 1 {
        return Ok(Value::nil());
    };
    match args.block {
        None => return Ok(Value::nil()),
        Some(method) => {
            let context = vm.context();
            let self_value = context.self_value;
            let iseq = vm.get_iseq(method)?;
            let mut arg = Args::new1(self_value, None, Value::nil());
            for i in 0..num {
                arg[0] = Value::fixnum(i);
                vm.vm_run(iseq, Some(context), &arg, None)?;
                vm.stack_pop();
            }
        }
    }
    Ok(args.self_value)
}

fn integer_step(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 2)?;
    let start = args.self_value.as_fixnum().unwrap();
    let limit = args[0].as_fixnum().unwrap();
    let step = if args.len() == 2 {
        let step = args[1].as_fixnum().unwrap();
        if step == 0 {
            return Err(vm.error_argument("Step must not be 0."));
        }
        step
    } else {
        1
    };

    match args.block {
        None => return Err(vm.error_argument("Currently, needs block.")),
        Some(method) => {
            let context = vm.context();
            let self_value = context.self_value;
            let iseq = vm.get_iseq(method)?;
            let mut arg = Args::new1(self_value, None, Value::nil());
            let mut i = start;
            loop {
                if step > 0 && i > limit || step < 0 && limit > i {
                    break;
                }
                arg[0] = Value::fixnum(i);
                vm.vm_run(iseq, Some(context), &arg, None)?;
                vm.stack_pop();
                i += step;
            }
        }
    }
    Ok(args.self_value)
}

/// Built-in function "chr".
fn integer_chr(_vm: &mut VM, args: &Args) -> VMResult {
    let num = args.self_value.as_fixnum().unwrap();
    Ok(RValue::Char(num as u8).pack())
}

fn integer_tof(_vm: &mut VM, args: &Args) -> VMResult {
    let num = args.self_value.as_fixnum().unwrap();
    Ok(Value::flonum(num as f64))
}

fn integer_even(_vm: &mut VM, args: &Args) -> VMResult {
    let num = args.self_value.as_fixnum().unwrap();
    Ok(Value::bool(num % 2 == 0))
}
