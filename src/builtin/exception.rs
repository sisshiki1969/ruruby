use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let class = Module::class_under(globals.builtins.object);
    class.add_builtin_class_method("new", exception_new);
    class.add_builtin_class_method("exception", exception_new);
    class.add_builtin_class_method("allocate", exception_allocate);

    class.add_builtin_method_by_str("inspect", inspect);
    class.add_builtin_method_by_str("to_s", tos);
    builtin::module::set_attr_accessor(
        globals,
        class,
        &Args::new2(
            Value::symbol_from_str("message"),
            Value::symbol_from_str("backtrace"),
        ),
    )
    .unwrap();
    let standard_error = Module::class_under(class);
    globals.set_toplevel_constant("StandardError", standard_error.get());
    // Subclasses of StandardError.
    let err = Module::class_under(standard_error);
    globals.set_toplevel_constant("ArgumentError", err.get());
    let err = Module::class_under(standard_error);
    globals.set_toplevel_constant("TypeError", err.get());
    let err = Module::class_under(standard_error);
    globals.set_toplevel_constant("NoMethodError", err.get());
    let runtime_error = Module::class_under(standard_error);
    globals.set_toplevel_constant("StopIteration", runtime_error.get());
    let runtime_error = Module::class_under(standard_error);
    globals.set_toplevel_constant("RuntimeError", runtime_error.get());
    let frozen_error = Module::class_under(runtime_error);
    globals.set_toplevel_constant("FrozenError", frozen_error.get());
    class.get()
}

// Class methods

fn exception_new(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let self_val = self_val.into_module();
    let new_instance = if args.len() == 0 {
        let class_name = self_val.name();
        Value::exception(self_val, RubyError::none(class_name))
    } else {
        let mut arg = args[0];
        let err = arg.expect_string("1st arg")?;
        Value::exception(self_val, RubyError::none(err))
    };
    // Call initialize method if it exists.
    if let Some(method) = vm.globals.find_method(self_val, IdentId::INITIALIZE) {
        vm.eval_send(method, new_instance, args)?;
    };
    Ok(new_instance)
}

fn exception_allocate(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let self_val = self_val.into_module();
    let new_instance = Value::exception(self_val, RubyError::none(""));
    Ok(new_instance)
}

// Instance methods

fn inspect(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
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

fn tos(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let val = self_val;
    let err = match val.if_exception() {
        Some(err) => err,
        _ => unreachable!("Not a Exception."),
    };
    Ok(Value::string(err.message()))
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn exception() {
        let program = r##"
        assert Exception, StandardError.superclass
        assert StandardError, RuntimeError.superclass
        assert StandardError, ArgumentError.superclass
        assert StandardError, NoMethodError.superclass
        assert StandardError, TypeError.superclass
        assert RuntimeError, FrozenError.superclass

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
        assert StandardError.singleton_class, NoMethodError.singleton_class.superclass

        assert StandardError.singleton_class, TypeError.singleton_class.superclass
        "##;
        assert_script(program);
    }
}
