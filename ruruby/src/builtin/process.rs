use crate::*;

pub(crate) fn init(globals: &mut Globals) -> Value {
    let mut class = Module::class_under_object();
    class.set_const_by_str("CLOCK_MONOTONIC", Value::integer(0));
    globals.set_toplevel_constant("Process", class);
    class.add_builtin_class_method(globals, "clock_gettime", clock_gettime);
    class.add_builtin_class_method(globals, "pid", pid);
    class.into()
}

// Class methods

fn clock_gettime(vm: &mut VM, _: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let duration = vm.globals.instant.elapsed();
    Ok(Value::float(duration.as_secs_f64()))
}

fn pid(vm: &mut VM, _: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    Ok(Value::integer(std::process::id as i64))
}

#[cfg(test)]
mod tests {
    use crate::tests::*;

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
