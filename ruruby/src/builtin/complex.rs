use crate::*;

pub(crate) fn init(globals: &mut Globals) -> Value {
    let class = Module::class_under(BuiltinClass::numeric());
    BuiltinClass::set_toplevel_constant("Complex", class);
    class.add_builtin_method_by_str(globals, "+", add);
    class.add_builtin_method_by_str(globals, "-", sub);
    class.add_builtin_method_by_str(globals, "*", mul);
    class.add_builtin_method_by_str(globals, "/", div);
    class.add_builtin_method_by_str(globals, "==", eq);
    class.add_builtin_method_by_str(globals, "abs2", abs2);
    class.add_builtin_method_by_str(globals, "abs", abs);
    class.add_builtin_method_by_str(globals, "rect", rect);
    //let mut class = Value::class(classref);
    class.add_builtin_class_method(globals, "rect", complex_rect);
    class.add_builtin_class_method(globals, "rectangular", complex_rect);
    class.into()
}

// Class methods

fn complex_rect(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    vm.check_args_range(1, 2)?;
    if !vm[0].is_real() {
        return Err(RubyError::typeerr("Not a real."));
    }
    let i = if args.len() == 1 {
        Value::integer(0)
    } else if vm[1].is_real() {
        vm[1]
    } else {
        return Err(RubyError::typeerr("Not a real."));
    };
    Ok(Value::complex(vm[0], i))
}

// Instance methods

fn add(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let (r1, i1) = self_val.to_complex().unwrap();
    let (r2, i2) = match vm[0].to_complex() {
        Some(t) => t,
        None => return Err(RubyError::typeerr("Not a real.")),
    };
    Ok(Value::complex((r1 + r2).into_val(), (i1 + i2).into_val()))
}

fn sub(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let (r1, i1) = self_val.to_complex().unwrap();
    let (r2, i2) = match vm[0].to_complex() {
        Some(t) => t,
        None => return Err(RubyError::typeerr("Not a real.")),
    };
    Ok(Value::complex((r1 - r2).into_val(), (i1 - i2).into_val()))
}

fn mul(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let (r1, i1) = self_val.to_complex().unwrap();
    let (r2, i2) = match vm[0].to_complex() {
        Some(t) => t,
        None => return Err(RubyError::typeerr("Not a real.")),
    };
    let r = r1.clone() * r2.clone() - i1.clone() * i2.clone();
    let i = i1 * r2 + i2 * r1;
    Ok(Value::complex(r.into_val(), i.into_val()))
}

fn div(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let (r1, i1) = self_val.to_complex().unwrap();
    let (r2, i2) = match vm[0].to_complex() {
        Some(t) => t,
        None => return Err(RubyError::typeerr("Not a real.")),
    };
    let abs2 = r2.clone().exp2() + i2.clone().exp2();
    let r = (r2.clone() * r1.clone() + i2.clone() * i1.clone()) / abs2.clone();
    let i = (r2 * i1 - r1 * i2) / abs2;
    Ok(Value::complex(r.into_val(), i.into_val()))
}

fn eq(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let (r1, i1) = self_val.to_complex().unwrap();
    let (r2, i2) = match vm[0].to_complex() {
        Some(t) => t,
        None => return Err(RubyError::typeerr("Not a real.")),
    };
    let b = r1 == r2 && i1 == i2;
    Ok(Value::bool(b))
}

fn abs2(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let (r, i) = self_val.to_complex().unwrap();
    Ok((r.exp2() + i.exp2()).into_val())
}

fn abs(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let (r, i) = self_val.to_complex().unwrap();
    Ok((r.exp2() + i.exp2()).sqrt().into_val())
}

fn rect(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let (r, i) = self_val.to_complex().unwrap();
    Ok(Value::array_from(vec![r.into_val(), i.into_val()]))
}

#[cfg(test)]
mod tests {
    use crate::tests::*;

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
