use crate::*;

pub fn init() -> Value {
    let class = Module::class_under(BuiltinClass::numeric());
    BuiltinClass::set_toplevel_constant("Float", class);
    class.add_builtin_method_by_str("to_s", inspect);
    class.add_builtin_method_by_str("inspect", inspect);
    class.add_builtin_method_by_str("+", add);
    class.add_builtin_method_by_str("-", sub);
    class.add_builtin_method_by_str("*", mul);
    class.add_builtin_method_by_str("div", quotient);
    class.add_builtin_method_by_str("<=>", cmp);
    class.add_builtin_method_by_str("floor", floor);
    class.add_builtin_method_by_str("to_i", toi);
    class.into()
}

// Class methods

// Instance methods
fn inspect(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let f = self_val.as_float().unwrap();
    let s = if f.fract() == 0.0 {
        format!("{:.1}", f)
    } else {
        f.to_string()
    };
    Ok(Value::string(s))
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

fn cmp(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    //use std::cmp::Ordering;
    args.check_args_num(1)?;
    let lhs = self_val.as_float().unwrap();
    let res = match args[0].unpack() {
        RV::Integer(rhs) => lhs.partial_cmp(&(rhs as f64)),
        RV::Float(rhs) => lhs.partial_cmp(&rhs),
        _ => return Ok(Value::nil()),
    };
    match res {
        Some(ord) => Ok(Value::integer(ord as i64)),
        None => Ok(Value::nil()),
    }
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
    use crate::test::*;
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
    ";
        assert_script(program);
    }
}
