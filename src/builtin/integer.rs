use crate::*;

pub fn init_integer(globals: &mut Globals) -> Value {
    let id = globals.get_ident_id("Integer");
    let class = ClassRef::from(id, globals.builtins.object);
    globals.add_builtin_instance_method(class, "times", times);
    globals.add_builtin_instance_method(class, "step", step);
    globals.add_builtin_instance_method(class, "chr", chr);
    globals.add_builtin_instance_method(class, "to_f", tof);
    globals.add_builtin_instance_method(class, "floor", floor);
    globals.add_builtin_instance_method(class, "even?", even);
    Value::class(globals, class)
}

// Class methods

// Instance methods

fn times(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let num = self_val.as_fixnum().unwrap();
    if num < 1 {
        return Ok(Value::nil());
    };
    match args.block {
        None => return Ok(Value::nil()),
        Some(method) => {
            let mut arg = Args::new1(None, Value::nil());
            for i in 0..num {
                arg[0] = Value::fixnum(i);
                vm.eval_block(method, &arg)?;
            }
        }
    }
    Ok(self_val)
}

fn step(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    //let method = vm.expect_block(args.block)?;
    let method = match args.block {
        Some(method) => method,
        None => {
            let id = vm.globals.get_ident_id("step");
            let val = Value::enumerator(&vm.globals, id, self_val, args.clone());
            return Ok(val);
        }
    };
    vm.check_args_num(args.len(), 1, 2)?;

    let start = self_val.as_fixnum().unwrap();
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

    if method == MethodRef::from(0) {
        let mut ary = vec![];
        let mut i = start;
        loop {
            if step > 0 && i > limit || step < 0 && limit > i {
                break;
            }
            ary.push(Value::fixnum(i));
            i += step;
        }
        let val = Value::array_from(&vm.globals, ary);
        return Ok(val);
    }

    let mut arg = Args::new1(None, Value::nil());
    let mut i = start;
    loop {
        if step > 0 && i > limit || step < 0 && limit > i {
            break;
        }
        arg[0] = Value::fixnum(i);
        vm.eval_block(method, &arg)?;
        i += step;
    }

    Ok(self_val)
}

/// Built-in function "chr".
fn chr(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let num = self_val.as_fixnum().unwrap() as u64 as u8;
    Ok(Value::bytes(&vm.globals, vec![num]))
}

fn floor(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    self_val.as_fixnum().unwrap();
    Ok(self_val)
}

fn tof(_vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let num = self_val.as_fixnum().unwrap();
    Ok(Value::flonum(num as f64))
}

fn even(_vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let num = self_val.as_fixnum().unwrap();
    Ok(Value::bool(num % 2 == 0))
}
