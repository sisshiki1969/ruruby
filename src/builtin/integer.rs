use crate::*;

pub fn init() -> Value {
    let class = Module::class_under(BuiltinClass::numeric());
    BuiltinClass::set_toplevel_constant("Integer", class);
    class.add_builtin_method_by_str("to_s", inspect);
    class.add_builtin_method_by_str("inspect", inspect);
    class.add_builtin_method_by_str("+@", plus);
    class.add_builtin_method_by_str("-@", minus);
    class.add_builtin_method_by_str("+", add);
    class.add_builtin_method_by_str("-", sub);
    class.add_builtin_method_by_str("*", mul);
    class.add_builtin_method_by_str("div", quotient);
    class.add_builtin_method_by_str("==", eq);
    class.add_builtin_method_by_str("===", eq);
    class.add_builtin_method_by_str("!=", neq);
    class.add_builtin_method_by_str(">=", ge);
    class.add_builtin_method_by_str(">", gt);
    class.add_builtin_method_by_str("<=", le);
    class.add_builtin_method_by_str("<", lt);
    class.add_builtin_method_by_str("<=>", cmp);
    class.add_builtin_method_by_str("[]", index);
    class.add_builtin_method_by_str(">>", shr);
    class.add_builtin_method_by_str("<<", shl);

    class.add_builtin_method_by_str("times", times);
    class.add_builtin_method_by_str("upto", upto);
    class.add_builtin_method_by_str("step", step);
    class.add_builtin_method_by_str("chr", chr);
    class.add_builtin_method_by_str("to_f", tof);
    class.add_builtin_method_by_str("to_i", toi);
    class.add_builtin_method_by_str("to_int", toi);
    class.add_builtin_method_by_str("floor", floor);
    class.add_builtin_method_by_str("abs", abs);
    class.add_builtin_method_by_str("even?", even);
    class.add_builtin_method_by_str("odd?", odd);
    class.add_builtin_method_by_str("size", size);
    class.add_builtin_method_by_str("next", next);
    class.add_builtin_method_by_str("succ", next);
    class.into()
}

// Class methods

// Instance methods
fn inspect(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let i = self_val.as_integer().unwrap();
    Ok(Value::string(i.to_string()))
}

fn plus(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(self_val)
}

fn minus(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let rec = self_val.as_integer().unwrap();
    Ok(Value::integer(-rec))
}

fn add(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    match args[0].to_real() {
        Some(rhs) => Ok((lhs + rhs).to_val()),
        None => match args[0].to_complex() {
            Some((r, i)) => {
                let r = lhs + r;
                let i = i;
                Ok(Value::complex(r.to_val(), i.to_val()))
            }
            None => Err(RubyError::undefined_op("+", args[0], self_val)),
        },
    }
}

fn sub(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    match args[0].to_real() {
        Some(rhs) => Ok((lhs - rhs).to_val()),
        None => match args[0].to_complex() {
            Some((r, i)) => {
                let r = lhs - r;
                let i = -i;
                Ok(Value::complex(r.to_val(), i.to_val()))
            }
            None => Err(RubyError::undefined_op("-", args[0], self_val)),
        },
    }
}

fn mul(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    match args[0].to_real() {
        Some(rhs) => Ok((lhs * rhs).to_val()),
        None => match args[0].to_complex() {
            Some((r, i)) => {
                let r = lhs * r;
                let i = lhs * i;
                Ok(Value::complex(r.to_val(), i.to_val()))
            }
            None => Err(RubyError::undefined_op("-", args[0], self_val)),
        },
    }
}

fn quotient(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    match args[0].to_real() {
        Some(rhs) => {
            if rhs.is_zero() {
                return Err(RubyError::zero_div("Divided by zero."));
            }
            Ok((lhs.quo(rhs)).to_val())
        }
        None => Err(RubyError::undefined_op("div", args[0], self_val)),
    }
}

fn eq(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.as_integer().unwrap();
    match args[0].unpack() {
        RV::Integer(rhs) => Ok(Value::bool(lhs == rhs)),
        RV::Float(rhs) => Ok(Value::bool(lhs as f64 == rhs)),
        _ => Ok(Value::bool(false)),
    }
}

fn neq(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.as_integer().unwrap();
    match args[0].unpack() {
        RV::Integer(rhs) => Ok(Value::bool(lhs != rhs)),
        RV::Float(rhs) => Ok(Value::bool(lhs as f64 != rhs)),
        _ => Ok(Value::bool(true)),
    }
}

macro_rules! define_cmp {
    ($self_val:ident, $args:ident, $op:ident) => {
        $args.check_args_num(1)?;
        let lhs = $self_val.expect_integer("Receiver")?;
        match $args[0].unpack() {
            RV::Integer(rhs) => return Ok(Value::bool(lhs.$op(&rhs))),
            RV::Float(rhs) => return Ok(Value::bool((lhs as f64).$op(&rhs))),
            _ => {
                return Err(RubyError::argument(format!(
                    "Comparison of Integer with {} failed.",
                    $args[0].get_class_name()
                )))
            }
        }
    };
}

fn ge(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    define_cmp!(self_val, args, ge);
}

fn gt(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    define_cmp!(self_val, args, gt);
}

fn le(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    define_cmp!(self_val, args, le);
}

fn lt(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    define_cmp!(self_val, args, lt);
}

fn cmp(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    //use std::cmp::Ordering;
    args.check_args_num(1)?;
    let lhs = self_val.as_integer().unwrap();
    let res = match args[0].unpack() {
        RV::Integer(rhs) => lhs.partial_cmp(&rhs),
        RV::Float(rhs) => (lhs as f64).partial_cmp(&rhs),
        _ => return Ok(Value::nil()),
    };
    match res {
        Some(ord) => Ok(Value::integer(ord as i64)),
        None => Ok(Value::nil()),
    }
}

fn index(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let i = self_val.as_integer().unwrap();
    let index = args[0].expect_integer("Index")?;
    let val = if index < 0 || 63 < index {
        0
    } else {
        (i >> index) & 1
    };
    Ok(Value::integer(val))
}

fn shr(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.as_integer().unwrap();
    let rhs = args[0];
    match rhs.as_integer() {
        Some(rhs) => Ok(Value::integer(lhs >> rhs)),
        None => Err(RubyError::no_implicit_conv(rhs, "Integer")),
    }
}

fn shl(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.as_integer().unwrap();
    let rhs = args[0];
    match rhs.as_integer() {
        Some(rhs) => Ok(Value::integer(lhs << rhs)),
        None => Err(RubyError::no_implicit_conv(rhs, "Integer")),
    }
}

fn times(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let block = match &args.block {
        Block::None => {
            let id = IdentId::get_id("times");
            let val = vm.create_enumerator(id, self_val, args.clone())?;
            return Ok(val);
        }
        method => method,
    };
    let num = self_val.as_integer().unwrap();
    if num < 1 {
        return Ok(self_val);
    };
    let mut args = Args::new1(Value::uninitialized());
    for i in 0..num {
        args[0] = Value::integer(i);
        vm.eval_block(block, &args)?;
    }
    Ok(self_val)
}

fn upto(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let block = match &args.block {
        Block::None => {
            let id = IdentId::get_id("upto");
            let val = vm.create_enumerator(id, self_val, args.clone())?;
            return Ok(val);
        }
        method => method,
    };
    let num = self_val.as_integer().unwrap();
    let max = args[0].expect_integer("Arg")?;
    if num <= max {
        let mut args = Args::new1(Value::uninitialized());
        for i in num..max + 1 {
            args[0] = Value::integer(i);
            vm.eval_block(block, &args)?;
        }
        //let iter = (num..max + 1).map(|i| Value::integer(i));
        //vm.eval_block_iter1(block, iter, false)?;
    }
    Ok(self_val)
}

fn step(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 2)?;
    let method = match &args.block {
        Block::None => {
            let id = IdentId::get_id("step");
            let val = vm.create_enumerator(id, self_val, args.clone())?;
            return Ok(val);
        }
        method => method,
    };
    let start = self_val.as_integer().unwrap();
    let limit = args[0].expect_integer("Limit")?;
    let step = if args.len() == 2 {
        let step = args[1].expect_integer("Step")?;
        if step == 0 {
            return Err(RubyError::argument("Step can not be 0."));
        }
        step
    } else {
        1
    };

    let mut arg = Args::new(1);
    let mut i = start;
    loop {
        if step > 0 && i > limit || step < 0 && limit > i {
            break;
        }
        arg[0] = Value::integer(i);
        vm.eval_block(method, &arg)?;
        i += step;
    }

    Ok(self_val)
}

/// Built-in function "chr".
fn chr(_: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let num = self_val.as_integer().unwrap();
    if 0 > num || num > 255 {
        return Err(RubyError::range(format!("{} Out of char range.", num)));
    };
    Ok(Value::bytes(vec![num as u8]))
}

fn floor(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    Ok(self_val)
}

fn abs(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let num = self_val.as_integer().unwrap();
    Ok(Value::integer(num.abs()))
}

fn tof(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let num = self_val.as_integer().unwrap();
    Ok(Value::float(num as f64))
}

fn toi(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(self_val)
}

fn even(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let num = self_val.as_integer().unwrap();
    Ok(Value::bool(num % 2 == 0))
}

fn odd(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let num = self_val.as_integer().unwrap();
    Ok(Value::bool(num % 2 != 0))
}

fn size(_: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(Value::integer(8))
}

fn next(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let i = self_val.as_integer().unwrap();
    Ok(Value::integer(i + 1))
}

#[cfg(test)]
mod tests {
    use crate::test::*;
    #[test]
    fn integer1() {
        let program = r#"
        assert("777", 777.inspect)
        assert("777", 777.to_s)
        assert(4.0, 4.to_f)
        assert(-4.0, -4.to_f)
        assert(4, 4.floor)
        assert(-4, -4.floor)
        assert(true, 8.even?)
        assert(false, 9.even?)
        assert(8, 1.size)
        "#;
        assert_script(program);
    }

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

            assert(0, 3.send(:"<=>", 3))
            assert(1, 5.send(:"<=>", 3))
            assert(-1, 3.send(:"<=>", 5))
            assert(0, 3.send(:"<=>", 3.0))
            assert(1, 5.send(:"<=>", 3.9))
            assert(-1, 3.send(:"<=>", 5.8))
            assert(nil, 3.send(:"<=>", "three"))

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
    fn integer_shift() {
        let program = r#"
        assert 8785458905193172896, (0x7f3d870a761a99f4 << 3) & 0x7fffffffffffffff
        assert 1146079111924634430, 0x7f3d870a761a99f4 >> 3
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

    #[test]
    fn integer_upto() {
        let program = r#"
        res = []
        assert 5, 5.upto(8) {|x| res << x * x}
        assert [25, 36, 49, 64], res
        res = []
        assert 5, 5.upto(4) {|x| res << x * x}
        assert [], res
        enum = 5.upto(8)
        assert [10, 12, 14, 16], enum.map{|x| x * 2}
        "#;
        assert_script(program);
    }

    #[test]
    fn integer_step() {
        let program = r#"
        res = 0
        4.step(20){|x| res += x}
        assert(204, res)
        res = 0
        4.step(20, 3){|x| res += x}
        assert(69, res)
        "#;
        assert_script(program);
    }

    #[test]
    fn integer_quotient() {
        let program = r#"
        assert(1, 3.div(2))
        assert(1, 3.div(2.0))
        assert(-2, (-3).div(2))
        assert(-2, (-3).div(2.0))
        "#;
        assert_script(program);
    }
}
