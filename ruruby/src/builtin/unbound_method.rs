use crate::*;

pub(crate) fn init(globals: &mut Globals) -> Value {
    let class = Module::class_under_object();
    globals.set_toplevel_constant("UnboundMethod", class);
    class.add_builtin_method_by_str(globals, "bind", bind);
    class.add_builtin_method_by_str(globals, "bind_call", bind_call);
    class.add_builtin_method_by_str(globals, "clone", clone);
    class.add_builtin_method_by_str(globals, "name", name);
    class.add_builtin_method_by_str(globals, "owner", owner);
    class.into()
}

pub(crate) fn bind(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(1)?;
    let method = self_val.as_unbound_method().unwrap();
    let res = Value::method(method.name, vm[0], method.method, method.owner);
    Ok(res)
}

pub(crate) fn bind_call(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_min(1)?;
    let method = self_val.as_unbound_method().unwrap();
    let (mut src, mut len) = vm.args_range();
    src += 1;
    len -= 1;
    let args = Args2::new(len);
    let res = vm.eval_method_range(method.method, vm[0], src, len, &args)?;
    Ok(res)
}

pub(crate) fn clone(_: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    let method = self_val.as_unbound_method().unwrap();
    let res = Value::unbound_method(method.name, method.method, method.owner);
    Ok(res)
}

pub(crate) fn name(_: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    let method = self_val.as_unbound_method().unwrap();
    let res = Value::symbol(method.name);
    Ok(res)
}

pub(crate) fn owner(_: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    let method = self_val.as_unbound_method().unwrap();
    let res = method.owner.into();
    Ok(res)
}

#[cfg(test)]
mod tests {
    use crate::tests::*;
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
