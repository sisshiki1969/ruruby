use crate::*;

pub(crate) fn init(globals: &mut Globals) -> Value {
    let class = Module::class_under_object();
    class.add_builtin_class_method(globals, "new", false_new);
    class.add_builtin_class_method(globals, "allocate", false_allocate);
    class.add_builtin_method_by_str(globals, "&", and);
    class.add_builtin_method_by_str(globals, "|", or);
    class.add_builtin_method_by_str(globals, "^", xor);
    class.add_builtin_method_by_str(globals, "inspect", inspect);
    class.add_builtin_method_by_str(globals, "to_s", inspect);
    globals.set_toplevel_constant("FalseClass", class);
    class.into()
}

// Class methods

fn false_new(_vm: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    Err(VMError::undefined_method(IdentId::NEW, self_val))
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
