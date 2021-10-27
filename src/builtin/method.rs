use crate::*;

pub(crate) fn init(globals: &mut Globals) -> Value {
    let class = Module::class_under_object();
    BuiltinClass::set_toplevel_constant("Method", class);
    class.add_builtin_method_by_str(globals, "call", call);
    class.add_builtin_method_by_str(globals, "[]", call);
    class.add_builtin_method_by_str(globals, "unbind", unbind);
    class.add_builtin_method_by_str(globals, "owner", owner);
    class.into()
}

pub(crate) fn call(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    let method = self_val.as_method().unwrap();
    let res = vm.eval_method(method.method, method.receiver.unwrap(), &args.into(vm))?;
    Ok(res)
}

pub(crate) fn unbind(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let method = self_val.as_method().unwrap();
    let res = Value::unbound_method(method.name, method.method, method.owner);
    Ok(res)
}

pub(crate) fn owner(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let method = self_val.as_method().unwrap();
    let res = method.owner.into();
    Ok(res)
}

#[cfg(test)]
mod tests {
    use crate::tests::*;

    #[test]
    fn method() {
        let program = r#"
        class Foo
          def foo(); "foo"; end
          def bar(); "bar"; end
          def baz(); "baz"; end
        end
        
       obj = Foo.new
        
       # 任意のキーとメソッドの関係をハッシュに保持しておく
        methods = {1 => obj.method(:foo),
                   2 => obj.method(:bar),
                   3 => obj.method(:baz)}
        
       # キーを使って関連するメソッドを呼び出す
        assert "foo", methods[1].call       # => "foo"
        assert "bar", methods[2].call       # => "bar"
        assert "baz", methods[3].call       # => "baz"

    "#;
        assert_script(program);
    }

    #[test]
    fn method2() {
        let program = r#"
        class Foo
          def foo(arg)
            "foo called with arg #{arg}"
          end
        end
      
        m = Foo.new.method(:foo) # => #<Method: Foo#foo>
        assert "foo called with arg 1", m[1]  
        assert "foo called with arg 2", m.call(2) 
    "#;
        assert_script(program);
    }

    #[test]
    fn method3() {
        let program = r#"
    m1 = 4.method(:inspect)
    m2 = 4.method(:inspect)
    h = {m1=>100}
    assert "4", m1.call
    assert "4", m2.call
    assert 100, h[m1]
    assert 100, h[m2]
    "#;
        assert_script(program);
    }

    #[test]
    fn method_unbind() {
        let program = r#"
        class Foo
          def foo
            "foo"
          end
        end
        m = Foo.new.method(:foo)
        assert Foo, m.owner
        unbound = m.unbind
        assert UnboundMethod, unbound.class
        assert :foo, unbound.name
        assert Foo, unbound.owner
    "#;
        assert_script(program);
    }
}
