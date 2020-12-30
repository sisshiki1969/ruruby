use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let mut class = ClassInfo::from(globals.builtins.object);
    class.set_const_by_str("CLOCK_MONOTONIC", Value::integer(0));
    let mut class_val = Value::class(class);
    globals.set_toplevel_constant("Process", class_val);
    class_val.add_builtin_class_method("clock_gettime", clock_gettime);
    class_val.add_builtin_class_method("pid", pid);
    class_val
}

// Class methods

fn clock_gettime(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let duration = vm.globals.instant.elapsed();
    Ok(Value::float(duration.as_secs_f64()))
}

fn pid(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(Value::integer(std::process::id as i64))
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn process() {
        let program = r#"
        Process.pid
        Process.clock_gettime(0)
        Process::CLOCK_MONOTONIC
        "#;
        assert_script(program);
    }
}
