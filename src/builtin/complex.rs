use crate::*;

pub fn init() -> Value {
    let class = Module::class_under(BuiltinClass::numeric());
    BuiltinClass::set_toplevel_constant("Complex", class);
    class.add_builtin_method_by_str("+", add);
    class.add_builtin_method_by_str("-", sub);
    class.add_builtin_method_by_str("*", mul);
    class.add_builtin_method_by_str("/", div);
    class.add_builtin_method_by_str("==", eq);
    class.add_builtin_method_by_str("abs2", abs2);
    class.add_builtin_method_by_str("abs", abs);
    class.add_builtin_method_by_str("rect", rect);
    //let mut class = Value::class(classref);
    class.add_builtin_class_method("rect", complex_rect);
    class.add_builtin_class_method("rectangular", complex_rect);
    class.into()
}

// Class methods

fn complex_rect(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 2)?;
    if !args[0].is_real() {
        return Err(RubyError::typeerr("Not a real."));
    }
    let i = if args.len() == 1 {
        Value::integer(0)
    } else if args[1].is_real() {
        args[1]
    } else {
        return Err(RubyError::typeerr("Not a real."));
    };
    Ok(Value::complex(args[0], i))
}

// Instance methods

fn add(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let (r1, i1) = self_val.to_complex().unwrap();
    let (r2, i2) = match args[0].to_complex() {
        Some(t) => t,
        None => return Err(RubyError::typeerr("Not a real.")),
    };
    Ok(Value::complex((r1 + r2).to_val(), (i1 + i2).to_val()))
}

fn sub(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let (r1, i1) = self_val.to_complex().unwrap();
    let (r2, i2) = match args[0].to_complex() {
        Some(t) => t,
        None => return Err(RubyError::typeerr("Not a real.")),
    };
    Ok(Value::complex((r1 - r2).to_val(), (i1 - i2).to_val()))
}

fn mul(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let (r1, i1) = self_val.to_complex().unwrap();
    let (r2, i2) = match args[0].to_complex() {
        Some(t) => t,
        None => return Err(RubyError::typeerr("Not a real.")),
    };
    let r = r1 * r2 - i1 * i2;
    let i = i1 * r2 + i2 * r1;
    Ok(Value::complex(r.to_val(), i.to_val()))
}

fn div(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let (r1, i1) = self_val.to_complex().unwrap();
    let (r2, i2) = match args[0].to_complex() {
        Some(t) => t,
        None => return Err(RubyError::typeerr("Not a real.")),
    };
    let abs2 = r2 * r2 + i2 * i2;
    let r = (r2 * r1 + i2 * i1) / abs2;
    let i = (r2 * i1 - r1 * i2) / abs2;
    Ok(Value::complex(r.to_val(), i.to_val()))
}

fn eq(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let (r1, i1) = self_val.to_complex().unwrap();
    let (r2, i2) = match args[0].to_complex() {
        Some(t) => t,
        None => return Err(RubyError::typeerr("Not a real.")),
    };
    let b = r1 == r2 && i1 == i2;
    Ok(Value::bool(b))
}

fn abs2(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let (r, i) = self_val.to_complex().unwrap();
    Ok((r * r + i * i).to_val())
}

fn abs(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let (r, i) = self_val.to_complex().unwrap();
    Ok((r * r + i * i).sqrt().to_val())
}

fn rect(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let (r, i) = self_val.to_complex().unwrap();
    Ok(Value::array_from(vec![r.to_val(), i.to_val()]))
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn complex1() {
        let program = r#"
        assert_error { Complex.rect(:a, 3)}
        assert_error { Complex.rect(3, :a)}
        assert(Complex.rect(5.2), Complex.rect(5.2, 0))
        assert(Complex.rect(3, 6.0), Complex.rect(1, 4.5) + Complex.rect(2, 1.5))
        assert(Complex.rect(3, 6.0), Complex.rect(5, 7.5) - Complex.rect(2, 1.5))
        assert(Complex.rect(3, 19), Complex.rect(5, 7) * Complex.rect(2, 1))
        assert(1-4i, (14-5i)/(2+3i))
        assert(2.3000000000000007+4i, (-10.2+16.51i)/(2+3.7i))
        assert(true, 4+5i == 4+5i)
        assert(false, 4+5i == 4+7i)
        assert(false, 4+5i == :dee)
        assert(17, Complex.rect(1, -4).abs2)
        assert(20.53, Complex.rect(1.7, -4.2).abs2)
        assert([4,-3], (4-3i).rect)
        "#;
        assert_script(program);
    }

    #[test]
    fn complex_error() {
        let program = r#"
        assert_error { 4+3i+:ee }
        assert_error { 4+3i-:ee }
        assert_error { 4+3i*:ee }
        assert_error { 4+3i/:ee }
        "#;
        assert_script(program);
    }
    #[test]
    fn complex2() {
        let program = r#"
        assert(17, (1-4i).abs2)
        assert(20.53, (1.7-4.2i).abs2)
        assert(2.23606797749979, (1+2i).abs)
        assert(5.0, (3+4i).abs)
        assert(0.7071067811865476, (0.5+0.5i).abs)
        "#;
        assert_script(program);
    }
}
