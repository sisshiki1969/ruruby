use crate::*;

pub fn init() -> Value {
    let class = Module::class_under_object();
    BuiltinClass::set_toplevel_constant("UnboundMethod", class);
    class.add_builtin_method_by_str("bind", bind);
    class.add_builtin_method_by_str("bind_call", bind_call);
    class.add_builtin_method_by_str("clone", clone);
    class.add_builtin_method_by_str("name", name);
    class.add_builtin_method_by_str("owner", owner);
    class.into()
}

pub fn bind(_vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let method = self_val.as_method().unwrap();
    let res = Value::method(method.name, args[0], method.method, method.owner);
    Ok(res)
}

pub fn bind_call(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_min(1)?;
    let method = self_val.as_method().unwrap();
    let mut new_args = Args::new(args.len() - 1);
    for i in 1..args.len() {
        new_args[i - 1] = args[i];
    }
    let res = vm.eval_method(method.method, args[0], &new_args)?;
    Ok(res)
}

pub fn clone(_vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let method = self_val.as_method().unwrap();
    let res = Value::unbound_method(method.name, method.method, method.owner);
    Ok(res)
}

pub fn name(_vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let method = self_val.as_method().unwrap();
    let res = Value::symbol(method.name);
    Ok(res)
}

pub fn owner(_vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let method = self_val.as_method().unwrap();
    let res = method.owner.into();
    Ok(res)
}

#[cfg(test)]
mod tests {
    use crate::test::*;
    #[test]
    fn unbound_method() {
        let program = r#"
            class A
              def foo(x)
                x.upcase
              end
            end
            um = A.new.method(:foo).unbind
            assert :foo, um.name
            assert :foo, um.clone.name
            assert "FOO", um.clone.bind(A.new).call("foo")
            assert "GOO", um.clone.bind_call(A.new, "goo")
            assert A, um.owner

            assert UnboundMethod, um.class
            assert Object, um.class.superclass
            m = um.bind(A.new)
            assert "HOO", m.call("hoo")
            assert A, m.owner
    "#;
        assert_script(program);
    }
}