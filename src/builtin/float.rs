use crate::*;

pub fn init(_globals: &mut Globals) -> Value {
    let id = IdentId::get_id("Float");
    let mut class = ClassRef::from(id, BuiltinClass::object());
    class.add_builtin_instance_method("+", add);
    class.add_builtin_instance_method("-", sub);
    class.add_builtin_instance_method("*", mul);
    class.add_builtin_instance_method("div", quotient);
    class.add_builtin_instance_method("<=>", cmp);
    class.add_builtin_instance_method("floor", floor);
    Value::class(class)
}

// Class methods

// Instance methods

fn add(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let lhs = self_val.to_real().unwrap();
    match args[0].to_real() {
        Some(rhs) => Ok((lhs + rhs).to_val()),
        None => match args[0].to_complex() {
            Some((r, i)) => {
                let r = lhs + r;
                let i = i;
                Ok(Value::complex(r.to_val(), i.to_val()))
            }
            None => Err(vm.error_undefined_op("+", args[0], self_val)),
        },
    }
}

fn sub(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let lhs = self_val.to_real().unwrap();
    match args[0].to_real() {
        Some(rhs) => Ok((lhs - rhs).to_val()),
        None => match args[0].to_complex() {
            Some((r, i)) => {
                let r = lhs - r;
                let i = -i;
                Ok(Value::complex(r.to_val(), i.to_val()))
            }
            None => Err(vm.error_undefined_op("-", args[0], self_val)),
        },
    }
}

fn mul(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let lhs = self_val.to_real().unwrap();
    match args[0].to_real() {
        Some(rhs) => Ok((lhs * rhs).to_val()),
        None => match args[0].to_complex() {
            Some((r, i)) => {
                let r = lhs * r;
                let i = lhs * i;
                Ok(Value::complex(r.to_val(), i.to_val()))
            }
            None => Err(vm.error_undefined_op("-", args[0], self_val)),
        },
    }
}

fn quotient(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let lhs = self_val.to_real().unwrap();
    match args[0].to_real() {
        Some(rhs) => Ok((lhs.quo(rhs)).to_val()),
        None => Err(vm.error_undefined_op("div", args[0], self_val)),
    }
}

fn cmp(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    //use std::cmp::Ordering;
    vm.check_args_num(args.len(), 1)?;
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

fn floor(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let lhs = self_val.as_float().unwrap();
    Ok(Value::integer(lhs.floor() as i64))
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn float() {
        let program = "
        assert(1, 1.3<=>1) 
        assert(-1, 1.3<=>5)
        assert(0, 1.3<=>1.3)
        assert(nil, 1.3<=>:foo)
        assert(1, 1.3.floor)
        assert(-2, (-1.3).floor)
    ";
        assert_script(program);
    }

    #[test]
    fn float_quotient() {
        let program = "
        assert(1, 3.0.div(2)) 
        assert(1, 3.0.div(2.0)) 
        assert(-2, (-3.0).div(2.0)) 
        assert(-2, (-3.0).div(2)) 
    ";
        assert_script(program);
    }
}
