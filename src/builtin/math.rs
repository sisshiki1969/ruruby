use crate::*;

pub fn init() -> Value {
    let mut class = Module::class_under_object();
    BuiltinClass::set_toplevel_constant("Math", class);
    class.add_builtin_class_method("sqrt", sqrt);
    class.add_builtin_class_method("cos", cos);
    class.add_builtin_class_method("sin", sin);
    class.set_const_by_str("PI", Value::float(std::f64::consts::PI));
    let err = Module::class_under(BuiltinClass::standard());
    class.set_const_by_str("DomainError", err.into());
    class.into()
}

// Class methods

fn coerce_to_float(val: Value) -> Result<f64, RubyError> {
    if let Some(real) = val.to_real() {
        Ok(real.to_f64())
    } else {
        Err(RubyError::typeerr("Must be a number."))
    }
}

fn sqrt(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let num = coerce_to_float(args[0])?;
    let res = Value::float(num.sqrt());
    Ok(res)
}

fn cos(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let num = coerce_to_float(args[0])?;
    let res = Value::float(num.cos());
    Ok(res)
}

fn sin(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let num = coerce_to_float(args[0])?;
    let res = Value::float(num.sin());
    Ok(res)
}

#[cfg(test)]
mod test {
    use crate::tests::*;

    #[test]
    fn math() {
        let program = r#"
        assert(3, Math.sqrt(9))
        assert(65536, Math.sqrt(65536*65536))
        assert(7.7090855488832135, Math.sqrt(59.43))

        assert(-0.8011436155469337, Math.cos(2.5))
        #assert(0.5984721441039565, Math.sin(2.5))
        assert(0.5403023058681398, Math.cos(1))
        assert(0.8414709848078965, Math.sin(1))
        "#;
        assert_script(program);
    }
}
