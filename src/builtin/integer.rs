use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let id = IdentId::get_ident_id("Integer");
    let class = ClassRef::from(id, globals.builtins.object);
    globals.add_builtin_instance_method(class, "==", eq);
    globals.add_builtin_instance_method(class, "!=", neq);
    globals.add_builtin_instance_method(class, ">=", ge);
    globals.add_builtin_instance_method(class, ">", gt);
    globals.add_builtin_instance_method(class, "<=", le);
    globals.add_builtin_instance_method(class, "<", lt);
    globals.add_builtin_instance_method(class, "<=>", cmp);

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

fn eq(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let lhs = vm.expect_integer(self_val, "Receiver")?;
    match args[0].unpack() {
        RV::Integer(rhs) => Ok(Value::bool(lhs == rhs)),
        RV::Float(rhs) => Ok(Value::bool(lhs as f64 == rhs)),
        _ => Ok(Value::bool(false)),
    }
}

fn neq(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let lhs = vm.expect_integer(self_val, "Receiver")?;
    match args[0].unpack() {
        RV::Integer(rhs) => Ok(Value::bool(lhs != rhs)),
        RV::Float(rhs) => Ok(Value::bool(lhs as f64 != rhs)),
        _ => Ok(Value::bool(true)),
    }
}

macro_rules! define_cmp {
    ($vm:ident, $self_val:ident, $args:ident, $op:ident) => {
        $vm.check_args_num($args.len(), 1)?;
        let lhs = $vm.expect_integer($self_val, "Receiver")?;
        match $args[0].unpack() {
            RV::Integer(rhs) => return Ok(Value::bool(lhs.$op(&rhs))),
            RV::Float(rhs) => return Ok(Value::bool((lhs as f64).$op(&rhs))),
            _ => {
                return Err($vm.error_argument(format!(
                    "Comparison of Integer with {} failed.",
                    $vm.globals.get_class_name($args[0])
                )))
            }
        }
    };
}

fn ge(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    define_cmp!(vm, self_val, args, ge);
}

fn gt(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    define_cmp!(vm, self_val, args, gt);
}

fn le(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    define_cmp!(vm, self_val, args, le);
}

fn lt(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    define_cmp!(vm, self_val, args, lt);
}

fn cmp(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    //use std::cmp::Ordering;
    vm.check_args_num(args.len(), 1)?;
    let lhs = vm.expect_integer(self_val, "Receiver")?;
    let res = match args[0].unpack() {
        RV::Integer(rhs) => lhs.partial_cmp(&rhs),
        RV::Float(rhs) => (lhs as f64).partial_cmp(&rhs),
        _ => return Ok(Value::nil()),
    };
    match res {
        Some(ord) => Ok(Value::fixnum(ord as i64)),
        None => Ok(Value::nil()),
    }
}

fn times(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let method = match args.block {
        Some(method) => method,
        None => {
            let id = IdentId::get_ident_id("times");
            let val = Value::enumerator(&vm.globals, id, self_val, args.clone());
            return Ok(val);
        }
    };
    let num = vm.expect_integer(self_val, "Receiver")?;
    if num < 1 {
        return Ok(self_val);
    };
    let mut arg = Args::new1(Value::nil());
    for i in 0..num {
        arg[0] = Value::fixnum(i);
        vm.eval_block(method, &arg)?;
    }
    Ok(self_val)
}

fn step(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_range(args.len(), 1, 2)?;
    let method = match args.block {
        Some(method) => method,
        None => {
            let id = IdentId::get_ident_id("step");
            let val = Value::enumerator(&vm.globals, id, self_val, args.clone());
            return Ok(val);
        }
    };
    let start = vm.expect_integer(self_val, "Start")?;
    let limit = vm.expect_integer(args[0], "Limit")?;
    let step = if args.len() == 2 {
        let step = vm.expect_integer(args[1], "Step")?;
        if step == 0 {
            return Err(vm.error_argument("Step can not be 0."));
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

    let mut arg = Args::new1(Value::nil());
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
    let num = self_val.as_fixnum().unwrap();
    if 0 > num || num > 255 {
        return Err(vm.error_unimplemented("Currently, receiver must be 0..255."));
    };
    Ok(Value::bytes(&vm.globals, vec![num as u8]))
}

fn floor(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
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

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn integer_cmp() {
        let program = r#"
            assert true, 4.send(:"==", 4)
            assert false, 4.send(:"==", 14)
            assert false, 4.send(:"==", "4")
            assert true, 4.send(:"==", 4.0)
            assert false, 4.send(:"==", 4.1)

            assert false, 4.send(:"!=", 4)
            assert true, 4.send(:"!=", 14)
            assert true, 4.send(:"!=", "4")
            assert false, 4.send(:"!=", 4.0)
            assert true, 4.send(:"!=", 4.1)

            assert true, 4.send(:">=", -1)
            assert true, 4.send(:">=", 4)
            assert false, 4.send(:">=", 14)
            #assert false, 4.send(:">=", "4")
            assert true, 4.send(:">=", 3.9)
            assert true, 4.send(:">=", 4.0)
            assert false, 4.send(:">=", 4.1)

            assert false, 4.send(:"<=", -1)
            assert true, 4.send(:"<=", 4)
            assert true, 4.send(:"<=", 14)
            #assert false, 4.send(:"<=", "4")
            assert false, 4.send(:"<=", 3.9)
            assert true, 4.send(:"<=", 4.0)
            assert true, 4.send(:"<=", 4.1)

            assert true, 4.send(:">", -1)
            assert false, 4.send(:">", 4)
            assert false, 4.send(:">", 14)
            #assert false, 4.send(:">", "4")
            assert true, 4.send(:">", 3.9)
            assert false, 4.send(:">", 4.0)
            assert false, 4.send(:">", 4.1)

            assert false, 4.send(:"<", -1)
            assert false, 4.send(:"<", 4)
            assert true, 4.send(:"<", 14)
            #assert false, 4.send(:"<", "4")
            assert false, 4.send(:"<", 3.9)
            assert false, 4.send(:"<", 4.0)
            assert true, 4.send(:"<", 4.1)

            assert(0, 3 <=> 3)
            assert(1, 5 <=> 3)
            assert(-1, 3 <=> 5)
            assert(0, 3 <=> 3.0)
            assert(1, 5 <=> 3.9)
            assert(-1, 3 <=> 5.8)
            assert(nil, 3 <=> "three")
        "#;
        assert_script(program);
    }

    #[test]
    fn integer_times() {
        let program = r#"
        res = []
        assert 5, 5.times {|x| res[x] = x * x}
        assert [0,1,4,9,16], res
        res = []
        assert 0, 0.times {|x| res[x] = x * x}
        assert [], res
        "#;
        assert_script(program);
    }
}
