use crate::*;

pub fn init() -> Value {
    let class = Module::class_under_object();
    class.add_builtin_class_method("new", false_new);
    class.add_builtin_class_method("allocate", false_allocate);
    class.add_builtin_method_by_str("&", and);
    class.add_builtin_method_by_str("|", or);
    class.add_builtin_method_by_str("^", xor);
    class.add_builtin_method_by_str("inspect", inspect);
    class.add_builtin_method_by_str("to_s", inspect);
    BuiltinClass::set_toplevel_constant("FalseClass", class);
    class.into()
}

// Class methods

fn false_new(_vm: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    Err(RubyError::undefined_method(IdentId::NEW, self_val))
}

fn false_allocate(_vm: &mut VM, _: Value, _args: &Args2) -> VMResult {
    Err(RubyError::typeerr("Allocator undefined for FalseClass"))
}

// Instance methods

fn and(vm: &mut VM, _: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    Ok(Value::false_val())
}

fn or(vm: &mut VM, _: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    Ok(Value::bool(vm[0].to_bool()))
}

fn xor(vm: &mut VM, _: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    Ok(Value::bool(vm[0].to_bool()))
}

fn inspect(vm: &mut VM, _: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    Ok(Value::string("false"))
}

#[cfg(test)]
mod tests {
    use crate::tests::*;
    #[test]
    fn falseclass() {
        let program = r#"
        assert false, false & true
        assert false, false & false
        assert false, false & nil
        assert false, false & 3

        assert true, false ^ true
        assert false, false ^ false
        assert false, false ^ nil
        assert true, false ^ 3

        assert true, false | true
        assert false, false | false
        assert false, false | nil
        assert true, false | 3

        assert false, false.send(:"&", true)
        assert false, false.send(:"&", false)
        assert false, false.send(:"&", nil)
        assert false, false.send(:"&", 3)

        assert true, false.send(:"^", true)
        assert false, false.send(:"^", false)
        assert false, false.send(:"^", nil)
        assert true, false.send(:"^", 3)

        assert true, false.send(:"|", true)
        assert false, false.send(:"|", false)
        assert false, false.send(:"|", nil)
        assert true, false.send(:"|", 3)

        assert "false", false.inspect
        assert "false", false.to_s
        assert FalseClass, false.class
    "#;
        assert_script(program);
    }
}
