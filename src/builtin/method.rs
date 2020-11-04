use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let mut class = ClassInfo::from(globals.builtins.object);
    class.add_builtin_method_by_str("call", method_call);
    Value::class(class)
}

pub fn method_call(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let method = match self_val.as_method() {
        Some(method) => method,
        None => return Err(vm.error_unimplemented("Expected Method object.")),
    };
    let res = vm.eval_send(method.method, method.receiver, args)?;
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
}
