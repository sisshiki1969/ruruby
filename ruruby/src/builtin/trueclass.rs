use crate::*;

pub(crate) fn init(globals: &mut Globals) -> Value {
    let class = Module::class_under_object();
    class.add_builtin_class_method(globals, "new", true_new);
    class.add_builtin_class_method(globals, "allocate", true_allocate);
    class.add_builtin_method_by_str(globals, "&", and);
    class.add_builtin_method_by_str(globals, "|", or);
    class.add_builtin_method_by_str(globals, "^", xor);
    class.add_builtin_method_by_str(globals, "inspect", inspect);
    class.add_builtin_method_by_str(globals, "to_s", inspect);
    globals.set_toplevel_constant("TrueClass", class);
    class.into()
}

// Class methods

fn true_new(_vm: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    Err(VMError::undefined_method(IdentId::NEW, self_val))
}

fn true_allocate(_vm: &mut VM, _: Value, _args: &Args2) -> VMResult {
    Err(RubyError::typeerr("Allocator undefined for TrueClass"))
}

// Instance methods

fn and(vm: &mut VM, _: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    Ok(Value::bool(vm[0].to_bool()))
}

fn or(vm: &mut VM, _: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    Ok(Value::true_val())
}

fn xor(vm: &mut VM, _: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    Ok(Value::bool(!vm[0].to_bool()))
}

fn inspect(vm: &mut VM, _: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    Ok(Value::string("true"))
}

#[cfg(test)]
mod tests {
    use crate::tests::*;
    #[test]
    fn trueclass() {
        let program = r#"
        assert true, true & true
        assert false, true & false
        assert false, true & nil
        assert true, true & 3

        assert false, true ^ true
        assert true, true ^ false
        assert true, true ^ nil
        assert false, true ^ 3

        assert true, true | true
        assert true, true | false
        assert true, true | nil
        assert true, true | 3

        assert true, true.send(:"&", true)
        assert false, true.send(:"&", false)
        assert false, true.send(:"&", nil)
        assert true, true.send(:"&", 3)

        assert false, true.send(:"^", true)
        assert true, true.send(:"^", false)
        assert true, true.send(:"^", nil)
        assert false, true.send(:"^", 3)

        assert true, true.send(:"|", true)
        assert true, true.send(:"|", false)
        assert true, true.send(:"|", nil)
        assert true, true.send(:"|", 3)

        assert "true", true.inspect
        assert "true", true.to_s
        assert TrueClass, true.class
    "#;
        assert_script(program);
    }
}
