use crate::*;

pub fn init() -> Value {
    let class = Module::class_under_object();
    class.add_builtin_class_method("new", nil_new);
    class.add_builtin_class_method("allocate", nil_allocate);
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
    BuiltinClass::set_toplevel_constant("NilClass", class);
    class.into()
}

// Class methods

fn nil_new(_vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    Err(RubyError::undefined_method(IdentId::NEW, self_val))
}

fn nil_allocate(_vm: &mut VM, _: Value, _args: &Args) -> VMResult {
    Err(RubyError::typeerr("Allocator undefined for NilClass"))
}

// Instance methods

fn and(vm: &mut VM, _: Value, _: &Args) -> VMResult {
    vm.check_args_num(1)?;
    Ok(Value::false_val())
}

fn or(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(1)?;
    Ok(Value::bool(args[0].to_bool()))
}

fn xor(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(1)?;
    Ok(Value::bool(args[0].to_bool()))
}

fn match_(vm: &mut VM, _: Value, _: &Args) -> VMResult {
    vm.check_args_num(1)?;
    Ok(Value::nil())
}

fn nil_(vm: &mut VM, _: Value, _: &Args) -> VMResult {
    vm.check_args_num(0)?;
    Ok(Value::true_val())
}

fn toa(vm: &mut VM, _: Value, _: &Args) -> VMResult {
    vm.check_args_num(0)?;
    Ok(Value::array_empty())
}

fn toc(vm: &mut VM, _: Value, _: &Args) -> VMResult {
    vm.check_args_num(0)?;
    let zero = Value::integer(0);
    Ok(Value::complex(zero, zero))
}

fn tof(vm: &mut VM, _: Value, _: &Args) -> VMResult {
    vm.check_args_num(0)?;
    Ok(Value::float(0.0))
}

fn toh(vm: &mut VM, _: Value, _: &Args) -> VMResult {
    vm.check_args_num(0)?;
    Ok(Value::hash_from_map(FxIndexMap::default()))
}

fn toi(vm: &mut VM, _: Value, _: &Args) -> VMResult {
    vm.check_args_num(0)?;
    Ok(Value::integer(0))
}

fn tos(vm: &mut VM, _: Value, _: &Args) -> VMResult {
    vm.check_args_num(0)?;
    Ok(Value::string(""))
}

#[cfg(test)]
mod tests {
    use crate::tests::*;
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
