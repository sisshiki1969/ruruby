use crate::*;

pub(crate) fn init(globals: &mut Globals) -> Value {
    let exception = Module::class_under_object();
    BuiltinClass::set_toplevel_constant("Exception", exception);
    exception.add_builtin_class_method(globals, "new", exception_new);
    exception.add_builtin_class_method(globals, "exception", exception_new);
    exception.add_builtin_class_method(globals, "allocate", exception_allocate);

    exception.add_builtin_method_by_str(globals,"inspect", inspect);
    exception.add_builtin_method_by_str(globals,"to_s", tos);
    builtin::module::set_attr_accessor(
        globals,
        exception,
        &Args::new2(
            Value::symbol_from_str("message"),
            Value::symbol_from_str("backtrace"),
        ),
    )
    .unwrap();
    // StandardError.
    let standard_error = Module::class_under(exception);
    BUILTINS.with(|m| m.borrow_mut().standard = standard_error.into());
    BuiltinClass::set_toplevel_constant("StandardError", standard_error);

    let err = Module::class_under(standard_error);
    BuiltinClass::set_toplevel_constant("ArgumentError", err);

    let err = Module::class_under(standard_error);
    BuiltinClass::set_toplevel_constant("IndexError", err);

    let err = Module::class_under(standard_error);
    BuiltinClass::set_toplevel_constant("RegexpError", err);

    let err = Module::class_under(standard_error);
    BuiltinClass::set_toplevel_constant("TypeError", err);

    let err = Module::class_under(standard_error);
    BuiltinClass::set_toplevel_constant("FiberError", err);

    let name_error = Module::class_under(standard_error);
    BuiltinClass::set_toplevel_constant("NameError", name_error);
    let err = Module::class_under(name_error);
    BuiltinClass::set_toplevel_constant("NoMethodError", err);

    let err = Module::class_under(standard_error);
    BuiltinClass::set_toplevel_constant("ZeroDivisionError", err);

    let err = Module::class_under(standard_error);
    BuiltinClass::set_toplevel_constant("StopIteration", err);

    let err = Module::class_under(standard_error);
    BuiltinClass::set_toplevel_constant("LocalJumpError", err);

    // RuntimeError
    let runtime_error = Module::class_under(standard_error);
    BuiltinClass::set_toplevel_constant("RuntimeError", runtime_error);
    let frozen_error = Module::class_under(runtime_error);
    BuiltinClass::set_toplevel_constant("FrozenError", frozen_error);

    let script_error = Module::class_under(exception);
    BuiltinClass::set_toplevel_constant("ScriptError", script_error);
    // Subclasses of ScriptError.
    let err = Module::class_under(script_error);
    BuiltinClass::set_toplevel_constant("SyntaxError", err);

    exception.into()
}

// Class methods

fn exception_new(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_range(0, 1)?;
    let self_val = self_val.into_module();
    let new_instance = if args.len() == 0 {
        let class_name = self_val.name();
        Value::exception(self_val, RubyError::none(class_name))
    } else {
        let mut arg = vm[0];
        let err = arg.expect_string("1st arg")?;
        Value::exception(self_val, RubyError::none(err))
    };
    // Call initialize method if it exists.
    if let Some(method) = vm
        .globals
        .methods
        .find_method(self_val, IdentId::INITIALIZE)
    {
        vm.eval_method(method, new_instance, &args.into(vm))?;
    };
    Ok(new_instance)
}

fn exception_allocate(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let self_val = self_val.into_module();
    let new_instance = Value::exception(self_val, RubyError::none(""));
    Ok(new_instance)
}

// Instance methods

fn inspect(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let val = self_val;
    let err = match val.if_exception() {
        Some(err) => err,
        _ => unreachable!("Not a Exception."),
    };
    Ok(Value::string(format!(
        "#<{}: {}>",
        val.get_class_name(),
        err.message()
    )))
}

fn tos(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let val = self_val;
    let err = match val.if_exception() {
        Some(err) => err,
        _ => unreachable!("Not a Exception."),
    };
    Ok(Value::string(err.message()))
}

#[cfg(test)]
mod tests {
    use crate::tests::*;

    #[test]
    fn exception() {
        let program = r##"
        assert Exception, StandardError.superclass
        assert StandardError, RuntimeError.superclass
        assert StandardError, ArgumentError.superclass
        assert StandardError, NameError.superclass
        assert StandardError, TypeError.superclass
        assert StandardError, Math::DomainError.superclass
        assert RuntimeError, FrozenError.superclass
        assert NameError, NoMethodError.superclass

        assert "#<Exception: Exception>", Exception.new.inspect
        assert "#<Exception: foo>", Exception.new("foo").inspect
        assert "Exception", Exception.new.to_s
        assert "foo", Exception.new("foo").to_s

        assert "#<StandardError: StandardError>", StandardError.new.inspect
        assert "#<StandardError: foo>", StandardError.new("foo").inspect
        assert "StandardError", StandardError.new.to_s
        assert "foo", StandardError.new("foo").to_s
        assert Exception.singleton_class, StandardError.singleton_class.superclass

        assert "#<NoMethodError: NoMethodError>", NoMethodError.new.inspect
        assert "#<NoMethodError: foo>", NoMethodError.new("foo").inspect
        assert "NoMethodError", NoMethodError.new.to_s
        assert "foo", NoMethodError.new("foo").to_s
        assert StandardError.singleton_class, NameError.singleton_class.superclass

        assert StandardError.singleton_class, TypeError.singleton_class.superclass
        "##;
        assert_script(program);
    }
}
