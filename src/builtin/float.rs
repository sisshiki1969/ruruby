use crate::*;

pub fn init() -> Value {
    let mut class = Module::class_under(BuiltinClass::numeric());
    BUILTINS.with(|m| m.borrow_mut().float = class.into());
    BuiltinClass::set_toplevel_constant("Float", class);
    class.add_builtin_method_by_str("nan?", nan);
    class.add_builtin_method_by_str("%", rem);
    class.add_builtin_method_by_str("div", quotient);
    class.add_builtin_method_by_str("**", exp);
    class.add_builtin_method_by_str("<=>", cmp);
    class.add_builtin_method_by_str("floor", floor);
    class.add_builtin_method_by_str("to_i", toi);
    class.set_const_by_str("DIG", Value::integer(std::f64::DIGITS as i64));
    class.set_const_by_str("INFINITY", Value::float(std::f64::INFINITY));
    class.set_const_by_str("EPSILON", Value::float(std::f64::EPSILON));
    class.set_const_by_str("RADIX", Value::integer(std::f64::RADIX as i64));
    class.set_const_by_str("NAN", Value::float(std::f64::NAN));
    class.set_const_by_str("MIN", Value::float(std::f64::MIN_POSITIVE));
    class.set_const_by_str("MIN_EXP", Value::integer(std::f64::MIN_EXP as i64));
    class.set_const_by_str("MIN_10_EXP", Value::integer(std::f64::MIN_10_EXP as i64));
    class.set_const_by_str("MAX", Value::float(std::f64::MAX));
    class.set_const_by_str("MAX_EXP", Value::integer(std::f64::MAX_EXP as i64));
    class.set_const_by_str("MAX_10_EXP", Value::integer(std::f64::MAX_10_EXP as i64));
    class.into()
}

// Class methods

// Instance methods
fn nan(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let f = self_val.as_float().unwrap();
    Ok(Value::bool(f.is_nan()))
}

fn rem(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let f = self_val.as_float().unwrap();
    arith::rem_float(f, args[0])
}

fn quotient(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    match args[0].to_real() {
        Some(rhs) => {
            if rhs.is_zero() {
                return Err(RubyError::zero_div("Divided by zero."));
            }
            Ok(lhs.quotient(rhs).to_val())
        }
        None => Err(RubyError::undefined_op("div", args[0], self_val)),
    }
}

fn exp(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let f = self_val.as_float().unwrap();
    arith::exp_float(f, args[0])
}

fn cmp(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.as_float().unwrap();
    let res = arith::cmp_float(lhs, args[0]);
    Ok(Value::from_ord(res))
}

fn floor(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let lhs = self_val.as_float().unwrap();
    Ok(Value::integer(lhs.floor() as i64))
}

fn toi(_: &mut VM, self_val: Value, _: &Args) -> VMResult {
    //args.check_args_num( 1, 1)?;
    let num = self_val.as_float().unwrap().trunc() as i64;
    Ok(Value::integer(num))
}

#[cfg(test)]
mod tests {
    use crate::tests::*;
    #[test]
    fn float() {
        let program = r#"
        assert "34.5", 34.5.to_s
        assert "34.5", 34.5.inspect
        assert "34.0", 34.000.to_s
        "#;
        assert_script(program);
    }

    #[test]
    fn cmp() {
        let program = "
        assert(1, 1.3<=>1) 
        assert(-1, 1.3<=>5)
        assert(0, 1.3<=>1.3)
        assert(nil, 1.3<=>:foo)
        assert(1, 1.3.floor)
        assert(-2, (-1.3).floor)

        assert(1, 1.3.send(:<=>, 1) 
        assert(-1, 1.3.send(:<=>, 5)
        assert(0, 1.3.send(:<=>, 1.3)
        assert(nil, 1.3.send(:<=>, :foo)
    ";
        assert_script(program);
    }

    #[test]
    fn float_ops() {
        let program = "
        assert(5.0, 3.0.send(:+,2)) 
        assert(5.0, 3.0.send(:+,2.0)) 
        assert(-1.0, (-3.0).send(:+,2.0)) 
        assert(-1.0, (-3.0).send(:+,2)) 

        assert(1.0, 3.0.send(:-,2)) 
        assert(1.0, 3.0.send(:-,2.0)) 
        assert(-5.0, (-3.0).send(:-,2.0)) 
        assert(-5.0, (-3.0).send(:-,2)) 

        assert(6.0, 3.0.send(:*,2)) 
        assert(6.0, 3.0.send(:*,2.0)) 
        assert(-6.0, (-3.0).send(:*,2.0)) 
        assert(-6.0, (-3.0).send(:*,2)) 

        assert(1, 3.0.div(2.0)) 
        assert(-2, (-3.0).div(2.0)) 
        assert(-2, (-3.0).div(2)) 

        assert('NaN', (0/0.0).to_s)
        assert(true, (0/0.0).nan?)

        assert 8.0+4.0i, 5.0+(3+4.0i)
        assert 2.0-4.0i, 5.0-(3+4.0i)
        assert 15.0+20.0i, 5.0*(3+4.0i)
        assert 0.6-0.8i, 5.0/(3+4i)
    ";
        assert_script(program);
    }
}
