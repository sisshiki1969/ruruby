use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let id = IdentId::get_ident_id("Float");
    let class = ClassRef::from(id, globals.builtins.object);
    globals.add_builtin_instance_method(class, "<=>", cmp);
    globals.add_builtin_instance_method(class, "floor", floor);
    Value::class(globals, class)
}

// Class methods

// Instance methods

fn cmp(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    //use std::cmp::Ordering;
    vm.check_args_num(args.len(), 1)?;
    let lhs = self_val.as_flonum().unwrap();
    let res = match args[0].unpack() {
        RV::Integer(rhs) => lhs.partial_cmp(&(rhs as f64)),
        RV::Float(rhs) => lhs.partial_cmp(&rhs),
        _ => return Ok(Value::nil()),
    };
    match res {
        Some(ord) => Ok(Value::fixnum(ord as i64)),
        None => Ok(Value::nil()),
    }
}

fn floor(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let lhs = self_val.as_flonum().unwrap();
    Ok(Value::fixnum(lhs.floor() as i64))
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
}
