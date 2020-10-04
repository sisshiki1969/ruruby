use crate::*;

pub fn init(_globals: &mut Globals) -> Value {
    let id = IdentId::get_id("Process");
    let class = ClassRef::from(id, BuiltinClass::object());
    let mut class_val = Value::class(class);
    class_val.add_builtin_class_method("clock_gettime", clock_gettime);
    class_val.add_builtin_class_method("pid", pid);
    class_val.set_var_by_str("CLOCK_MONOTONIC", Value::integer(0));
    class_val
}

// Class methods

fn clock_gettime(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let duration = vm.globals.instant.elapsed();
    Ok(Value::float(duration.as_secs_f64()))
}

fn pid(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
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
