use crate::*;

pub fn init(builtins: &mut BuiltinClass) -> Value {
    let class = Module::class_under(builtins.object);
    class.add_builtin_method_by_str("&", and);
    class.add_builtin_method_by_str("|", or);
    class.add_builtin_method_by_str("^", xor);
    class.add_builtin_method_by_str("inspect", inspect);
    class.add_builtin_method_by_str("to_s", inspect);
    builtins.set_toplevel_constant("TrueClass", class);
    class.into()
}

// Instance methods

fn and(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    Ok(Value::bool(args[0].to_bool()))
}

fn or(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    Ok(Value::true_val())
}

fn xor(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    Ok(Value::bool(!args[0].to_bool()))
}

fn inspect(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(Value::string("true"))
}

#[cfg(test)]
mod tests {
    use crate::test::*;
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
