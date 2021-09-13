use indexmap::IndexSet;

use crate::*;

pub fn init() -> Value {
    let class = Module::class_under_object();
    BuiltinClass::set_toplevel_constant("Binding", class);
    class.add_builtin_class_method("new", binding_new);
    class.add_builtin_method_by_str("eval", eval);
    class.add_builtin_method_by_str("receiver", receiver);
    class.add_builtin_method_by_str("local_variables", local_variables);
    class.add_builtin_method_by_str("local_variable_defined?", local_variable_defined);
    class.into()
}

// Class methods

fn binding_new(_vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    Err(RubyError::undefined_method(IdentId::NEW, self_val))
}

// Instance methods

fn eval(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 3)?;
    let ctx = self_val.as_binding();
    let mut arg0 = vm[0];
    let code = arg0.expect_string("1st arg")?.to_string();
    let path = if args.len() >= 2 {
        let mut arg1 = vm[1];
        arg1.expect_string("2nd arg")?.to_string()
    } else {
        vm.context()
            .iseq_ref
            .source_info
            .path
            .to_string_lossy()
            .to_string()
    };
    vm.eval_binding(path, code, ctx)
}

fn receiver(_vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let ctx = self_val.as_binding();
    Ok(ctx.self_value)
}

fn local_variables(_vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let ctx = self_val.as_binding();
    let mut vec = IndexSet::default();
    ctx.enumerate_local_vars(&mut vec);
    let ary = vec.into_iter().map(|id| Value::symbol(id)).collect();
    Ok(Value::array_from(ary))
}

fn local_variable_defined(_vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let ctx = self_val.as_binding();
    let mut vec = IndexSet::default();
    let var = args[0].expect_symbol_or_string("Arg")?;
    ctx.enumerate_local_vars(&mut vec);
    let b = vec.get(&var).is_some();
    Ok(Value::bool(b))
}

#[cfg(test)]
mod test {
    use crate::tests::*;

    #[test]
    fn binding1() {
        let program = r#"
        def f(x)
          a = 1
          b = binding
          a = 2
          b
        end

        b = f(3)
        assert [:x, :a, :b], b.local_variables
        assert 3, eval "x", b
        assert 2, eval "a", b
        assert true, b.local_variable_defined?(:x)
        assert true, b.local_variable_defined?(:a)
        assert false, b.local_variable_defined?(:z)
        assert b.eval("self"), b.receiver
        assert_error { Binding.new }
        "#;
        assert_script(program);
    }

    #[test]
    fn binding2() {
        let program = r#"
        def f(x)
          a = 1
          1.times do |x|
            return binding
          end
        end

        b = f(3)
        assert [:x, :a], b.local_variables
        assert 0, eval "x", b
        assert 1, eval "a", b
        "#;
        assert_script(program);
    }

    #[test]
    fn binding3() {
        let program = r#"
        def f(x)
          a = 1
          [binding, binding]
        end

        b1, b2 = f(3)
        assert [:x, :a], b1.local_variables
        assert [:x, :a], b2.local_variables
        eval "z = 7", b1
        assert [:z, :x, :a], b1.local_variables
        assert [:x, :a], b2.local_variables
        
        "#;
        assert_script(program);
    }
    #[test]
    fn eval() {
        let program = r#"
        def get_binding(str)
          binding
        end
        str = "hello"

        assert "hello Fred", eval("str + ' Fred'")
        assert "bye Fred", get_binding("bye").eval("str + ' Fred'")
        "#;
        assert_script(program);
    }
}
