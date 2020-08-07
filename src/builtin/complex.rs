use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let id = IdentId::get_id("Complex");
    let classref = ClassRef::from(id, globals.builtins.object);
    globals.add_builtin_instance_method(classref, "+", add);
    globals.add_builtin_instance_method(classref, "-", sub);
    globals.add_builtin_instance_method(classref, "*", mul);
    globals.add_builtin_instance_method(classref, "==", eq);
    globals.add_builtin_instance_method(classref, "abs2", abs2);
    let class = Value::class(globals, classref);
    globals.add_builtin_class_method(class, "rect", complex_rect);
    globals.add_builtin_class_method(class, "rectangular", complex_rect);
    class
}

// Class methods

fn complex_rect(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_range(args.len(), 1, 2)?;
    if !args[0].is_real() {
        return Err(vm.error_type("Not a real."));
    }
    let i = if args.len() == 1 {
        Value::fixnum(0)
    } else if args[1].is_real() {
        args[1]
    } else {
        return Err(vm.error_type("Not a real."));
    };
    Ok(Value::complex(&vm.globals, args[0], i))
}

// Instance methods

fn add(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let (r1, i1) = self_val.to_complex().unwrap();
    let (r2, i2) = match args[0].to_complex() {
        Some(t) => t,
        None => return Err(vm.error_type("Not a real.")),
    };
    Ok(Value::complex(
        &vm.globals,
        (r1 + r2).to_val(),
        (i1 + i2).to_val(),
    ))
}

fn sub(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let (r1, i1) = self_val.to_complex().unwrap();
    let (r2, i2) = match args[0].to_complex() {
        Some(t) => t,
        None => return Err(vm.error_type("Not a real.")),
    };
    Ok(Value::complex(
        &vm.globals,
        (r1 - r2).to_val(),
        (i1 - i2).to_val(),
    ))
}

fn mul(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let (r1, i1) = self_val.to_complex().unwrap();
    let (r2, i2) = match args[0].to_complex() {
        Some(t) => t,
        None => return Err(vm.error_type("Not a real.")),
    };
    let r = r1 * r2 - i1 * i2;
    let i = i1 * r2 + i2 * r1;
    Ok(Value::complex(&vm.globals, r.to_val(), i.to_val()))
}

fn eq(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let (r1, i1) = self_val.to_complex().unwrap();
    let (r2, i2) = match args[0].to_complex() {
        Some(t) => t,
        None => return Err(vm.error_type("Not a real.")),
    };
    let b = r1 == r2 && i1 == i2;
    Ok(Value::bool(b))
}

fn abs2(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let (r, i) = self_val.to_complex().unwrap();
    Ok((r * r + i * i).to_val())
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
        assert(17, Complex.rect(1, -4).abs2)
        assert(20.53, Complex.rect(1.7, -4.2).abs2)
        "#;
        assert_script(program);
    }
}
