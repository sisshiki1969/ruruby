use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let id = IdentId::get_id("Complex");
    let classref = ClassRef::from(id, globals.builtins.object);
    globals.add_builtin_instance_method(classref, "+", add);
    globals.add_builtin_instance_method(classref, "-", sub);
    globals.add_builtin_instance_method(classref, "*", mul);
    globals.add_builtin_instance_method(classref, "==", eq);
    let class = Value::class(globals, classref);
    globals.add_builtin_class_method(class, "rect", complex_rect);
    class
}

// Class methods

fn complex_rect(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 2)?;
    if !args[0].is_real() {
        return Err(vm.error_type("Not a real."));
    }
    if !args[1].is_real() {
        return Err(vm.error_type("Not a real."));
    }
    Ok(Value::complex(&vm.globals, args[0], args[1]))
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
    eprintln!("eq");
    let b = r1 == r2 && i1 == i2;
    Ok(Value::bool(b))
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn complex1() {
        let program = r#"
        assert_error { Complex.rect(:a, 3)}
        assert_error { Complex.rect(3, :a)}
        assert(Complex.rect(3, 6.0), Complex.rect(1, 4.5) + Complex.rect(2, 1.5))
        assert(Complex.rect(3, 6.0), Complex.rect(5, 7.5) - Complex.rect(2, 1.5))
        assert(Complex.rect(3, 19), Complex.rect(5, 7) * Complex.rect(2, 1))
        "#;
        assert_script(program);
    }
}
