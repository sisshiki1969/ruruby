use crate::*;

pub fn init() -> Module {
    let mut class = Module::class_under_object();
    BuiltinClass::set_toplevel_constant("Numeric", class);
    class.append_include_without_increment_version(BuiltinClass::comparable());
    class.add_builtin_method_by_str("to_s", inspect);
    class.add_builtin_method_by_str("inspect", inspect);
    class.add_builtin_method_by_str("+", add);
    class.add_builtin_method_by_str("-", sub);
    class.add_builtin_method_by_str("*", mul);
    class.add_builtin_method_by_str("/", div);
    class.add_builtin_method_by_str("==", eq);
    class.add_builtin_method_by_str("!=", ne);
    class.add_builtin_method_by_str(">=", ge);
    class.add_builtin_method_by_str(">", gt);
    class.add_builtin_method_by_str("<=", le);
    class.add_builtin_method_by_str("<", lt);
    class
}

// Instance methods
fn inspect(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    vm.check_args_num(0)?;
    let s = match self_val.to_real() {
        Some(r) => match r {
            Real::Bignum(n) => n.to_string(),
            Real::Integer(i) => i.to_string(),
            Real::Float(f) => format!("{:?}", f),
        },
        None => match self_val.as_complex() {
            Some((r, i)) => {
                let (r, i) = (r.to_real().unwrap(), i.to_real().unwrap());
                if !i.is_negative() {
                    format!("({:?}+{:?}i)", r, i)
                } else {
                    format!("({:?}{:?}i)", r, i)
                }
            }
            None => unreachable!(),
        },
    };
    Ok(Value::string(s))
}

fn add(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    vm.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    let arg0 = vm[0];
    match arg0.to_real() {
        Some(rhs) => Ok((lhs + rhs).to_val()),
        None => match arg0.to_complex() {
            Some((r, i)) => {
                let r = lhs + r;
                let i = i;
                Ok(Value::complex(r.to_val(), i.to_val()))
            }
            None => Err(RubyError::cant_coerse(arg0, "Integer")),
        },
    }
}

fn sub(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    vm.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    let arg0 = vm[0];
    match arg0.to_real() {
        Some(rhs) => Ok((lhs - rhs).to_val()),
        None => match arg0.to_complex() {
            Some((r, i)) => {
                let r = lhs - r;
                let i = -i;
                Ok(Value::complex(r.to_val(), i.to_val()))
            }
            None => Err(RubyError::cant_coerse(arg0, "Integer")),
        },
    }
}

fn mul(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    vm.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    let arg0 = vm[0];
    match arg0.to_real() {
        Some(rhs) => Ok((lhs * rhs).to_val()),
        None => match arg0.to_complex() {
            Some((r, i)) => {
                let r = lhs.clone() * r;
                let i = lhs * i;
                Ok(Value::complex(r.to_val(), i.to_val()))
            }
            None => Err(RubyError::cant_coerse(arg0, "Integer")),
        },
    }
}

fn div(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    vm.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    let arg0 = vm[0];
    match arg0.to_real() {
        Some(rhs) => {
            if rhs.is_zero() {
                return Err(RubyError::zero_div("Divided by zero."));
            }
            Ok((lhs / rhs).to_val())
        }
        None => match arg0.to_complex() {
            Some((r, i)) => {
                let divider = r.clone().exp2() + i.clone().exp2();
                if divider.is_zero() {
                    return Err(RubyError::zero_div("Divided by zero."));
                }
                let r = (lhs.clone() * r).divide(divider.clone());
                let i = (-lhs * i).divide(divider);
                Ok(Value::complex(r.to_val(), i.to_val()))
            }
            None => Err(RubyError::cant_coerse(arg0, "Integer")),
        },
    }
}

macro_rules! define_cmp {
    ($op:ident) => {
        fn $op(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
            vm.check_args_num(1)?;
            let arg0 = vm[0];
            let lhs = self_val.to_real().unwrap();
            match arg0.to_real() {
                Some(rhs) => return Ok(Value::bool(lhs.$op(&rhs))),
                _ => {
                    return Err(RubyError::argument(format!(
                        "Comparison of {} with {} failed.",
                        self_val.get_class_name(),
                        arg0.get_class_name()
                    )))
                }
            }
        }
    };
}

define_cmp!(ge);
define_cmp!(gt);
define_cmp!(le);
define_cmp!(lt);
define_cmp!(eq);
define_cmp!(ne);

#[cfg(test)]
mod tests {
    use crate::tests::*;

    #[test]
    fn inspect() {
        let program = r##"
        assert "(-23-7i)", (-23-7i).inspect
        assert "(23-7i)", (23-7i).inspect
        assert "0.0", 0.0000.inspect
        assert "0.0003", 0.0003.inspect
        assert "-0.0003", -0.0003.inspect
        "##;
        assert_script(program);
    }

    #[test]
    fn ops() {
        let program = r##"
        assert_error { 100.+ "" }
        assert_error { 0.01.+ "" }
        assert_error { 100.- "" }
        assert_error { 0.01.- "" }
        assert_error { 100.* "" }
        assert_error { 0.01.* "" }
        assert_error { 100./ "" }
        assert_error { 0.01./ "" }
        assert 1, 100 % 3
        assert -1, -100 % -3
        assert 2, -100 % 3
        assert -2, 100 % -3
        assert 1.0, 100.0 % 3.0
        assert 2.0, -100.0 % 3.0

        assert_error {4.0.== ""}
        assert_error {4.0.!= ""}
        assert_error {4.0.<= ""}
        assert_error {4.0.< ""}
        assert_error {4.0.>= ""}
        assert_error {4.0.> ""}
        "##;
        assert_script(program);
    }
}
