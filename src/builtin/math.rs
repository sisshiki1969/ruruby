use crate::*;

pub fn init(_globals: &mut Globals) -> Value {
    let id = IdentId::get_id("Math");
    let class = ClassRef::from(id, BuiltinClass::object());
    let mut class_val = Value::class(class);
    class_val.add_builtin_class_method("sqrt", sqrt);
    class_val.add_builtin_class_method("cos", cos);
    class_val.add_builtin_class_method("sin", sin);
    class_val
}

// Class methods

// Instance methods

fn sqrt(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let arg = args[0];
    let num = if arg.is_packed_num() {
        if arg.is_packed_fixnum() {
            arg.as_packed_fixnum() as f64
        } else {
            arg.as_packed_flonum()
        }
    } else {
        return Err(vm.error_type("Must be a number."));
    };
    let res = Value::float(num.sqrt());
    Ok(res)
}

fn cos(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let arg = args[0];
    let num = if arg.is_packed_num() {
        if arg.is_packed_fixnum() {
            arg.as_packed_fixnum() as f64
        } else {
            arg.as_packed_flonum()
        }
    } else {
        return Err(vm.error_type("Must be a number."));
    };
    let res = Value::float(num.cos());
    Ok(res)
}

fn sin(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let arg = args[0];
    let num = if arg.is_packed_num() {
        if arg.is_packed_fixnum() {
            arg.as_packed_fixnum() as f64
        } else {
            arg.as_packed_flonum()
        }
    } else {
        return Err(vm.error_type("Must be a number."));
    };
    let res = Value::float(num.sin());
    Ok(res)
}

#[cfg(test)]
mod test {
    use crate::test::*;

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
