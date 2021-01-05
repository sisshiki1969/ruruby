use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let mut class = ClassInfo::class_from(globals.builtins.object);
    class.add_builtin_method_by_str("&", and);
    class.add_builtin_method_by_str("|", or);
    class.add_builtin_method_by_str("^", xor);
    class.add_builtin_method_by_str("inspect", inspect);
    class.add_builtin_method_by_str("to_s", inspect);
    let class_obj = Value::class(class);
    globals.set_toplevel_constant("FalseClass", class_obj);
    class_obj
}

// Instance methods

fn and(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    Ok(Value::false_val())
}

fn or(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    Ok(Value::bool(args[0].to_bool()))
}

fn xor(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    Ok(Value::bool(args[0].to_bool()))
}

fn inspect(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(Value::string("false"))
}

#[cfg(test)]
mod tests {
    use crate::test::*;
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
