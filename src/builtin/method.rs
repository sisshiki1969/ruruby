use crate::*;

pub fn init() -> Value {
    let class = Module::class_under_object();
    BuiltinClass::set_toplevel_constant("Method", class);
    class.add_builtin_method_by_str("call", call);
    class.add_builtin_method_by_str("[]", call);
    class.into()
}

pub fn call(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let method = match self_val.as_method() {
        Some(method) => method,
        None => return Err(RubyError::internal("Expected Method object.")),
    };
    let res = vm.eval_method(method.method, method.receiver, args)?;
    Ok(res)
}

#[cfg(test)]
mod tests {
    use crate::test::*;

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
}
