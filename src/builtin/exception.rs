use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let mut exception = Value::class_under(globals.builtins.object);
    exception.add_builtin_class_method("new", exception_new);
    exception.add_builtin_class_method("exception", exception_new);
    exception.add_builtin_method_by_str("inspect", inspect);
    builtin::module::set_attr_accessor(
        globals,
        exception,
        &Args::new2(
            Value::symbol_from_str("message"),
            Value::symbol_from_str("backtrace"),
        ),
    )
    .unwrap();
    let standard_error = Value::class_under(exception);
    exception.add_builtin_method_by_str("inspect", standard_inspect);
    globals.set_toplevel_constant("StandardError", standard_error);
    let runtime_error = Value::class_under(standard_error);
    globals.set_toplevel_constant("RuntimeError", runtime_error);
    exception
}

// Class methods

fn exception_new(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let new_instance = if args.len() == 0 {
        Value::exception(self_val, RubyError::argument(""))
    } else {
        let mut arg = args[0];
        let err = arg.expect_string("1st arg")?;
        Value::exception(self_val, RubyError::argument(err))
    };
    // Call initialize method if it exists.
    if let Some(method) = vm.globals.find_method(self_val, IdentId::INITIALIZE) {
        vm.eval_send(method, new_instance, args)?;
    };
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
    Ok(Value::string(format!("#<Exception {:?} >", err.message())))
}

fn standard_inspect(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let val = self_val;
    let err = match val.if_exception() {
        Some(err) => err,
        _ => unreachable!("Not a Exception."),
    };
    Ok(Value::string(format!("#<StandardError: {:?} >", err.kind)))
}
