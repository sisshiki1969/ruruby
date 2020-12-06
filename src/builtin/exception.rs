use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let mut exception = Value::class_under(globals.builtins.object);
    exception.add_builtin_class_method("new", exception_new);
    exception.add_builtin_class_method("exception", exception_new);
    exception.add_builtin_method_by_str("inspect", inspect);
    let standard_error = Value::class_under(exception);
    exception.add_builtin_method_by_str("inspect", standard_inspect);
    globals.set_toplevel_constant("StandardError", standard_error);
    let runtime_error = Value::class_under(standard_error);
    globals.set_toplevel_constant("RuntimeError", runtime_error);
    exception
}

// Class methods

fn exception_new(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    if args.len() == 0 {
        Ok(Value::exception(self_val, RubyError::argument("")))
    } else {
        let mut arg = args[0];
        let err = arg.expect_string("1st arg")?;
        Ok(Value::exception(self_val, RubyError::argument(err)))
    }
}

// Instance methods

fn inspect(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let val = self_val;
    let err = match val.if_exception() {
        Some(err) => err,
        _ => unreachable!("Not a Exception."),
    };
    Ok(Value::string(format!("#<Exception {:?} >", err)))
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
