use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let class = Module::class_under(globals.builtins.object);
    class.add_builtin_method_by_str("&", and);
    class.add_builtin_method_by_str("|", or);
    class.add_builtin_method_by_str("^", xor);
    class.add_builtin_method_by_str("=~", match_);
    class.add_builtin_method_by_str("nil?", nil_);
    class.add_builtin_method_by_str("to_a", toa);
    class.add_builtin_method_by_str("to_c", toc);
    class.add_builtin_method_by_str("to_f", tof);
    class.add_builtin_method_by_str("to_h", toh);
    class.add_builtin_method_by_str("to_i", toi);
    class.add_builtin_method_by_str("to_s", tos);
    globals.set_toplevel_constant("NilClass", class);
    class.into()
}

// Class methods

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

fn match_(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    Ok(Value::nil())
}

fn nil_(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(Value::true_val())
}

fn toa(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(Value::array_from(vec![]))
}

fn toc(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let zero = Value::integer(0);
    Ok(Value::complex(zero, zero))
}

fn tof(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(Value::float(0.0))
}

fn toh(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(Value::hash_from_map(FxHashMap::default()))
}

fn toi(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(Value::integer(0))
}

fn tos(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(Value::string(""))
}

#[cfg(test)]
mod tests {
    use crate::test::*;
    #[test]
    fn nilclass() {
        let program = r#"
        assert(false, nil & true) 
        assert(false, nil & false) 
        assert(false, nil & 3) 
        assert(false, nil & nil) 

        assert(false, nil.send(:"&", true))
        assert(false, nil.send(:"&", false)) 
        assert(false, nil.send(:"&", 3))
        assert(false, nil.send(:"&", nil)) 

        assert(true, nil | true) 
        assert(false, nil | false) 
        assert(true, nil | 3) 
        assert(false, nil | nil)

        assert(true, nil.send(:"|", true)) 
        assert(false, nil.send(:"|", false)) 
        assert(true, nil.send(:"|", 3))
        assert(false, nil.send(:"|", nil))

        assert(true, nil ^ true) 
        assert(false, nil ^ false) 
        assert(true, nil ^ 3) 
        assert(false, nil ^ nil) 

        assert(true, nil.send(:"^", true) 
        assert(false, nil.send(:"^", false) 
        assert(true, nil.send(:"^", 3) 
        assert(false, nil.send(:"^", nil) 

        assert(true, nil.nil?)

        assert(nil, nil =~ /a/)
        assert(nil, nil =~ /.?*/)

        assert([], nil.to_a)
        assert(0+0i, nil.to_c)
        assert(0.0, nil.to_f)
        assert({}, nil.to_h)
        assert(0, nil.to_i)
    "#;
        assert_script(program);
    }
}
