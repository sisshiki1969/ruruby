use indexmap::IndexSet;

use crate::*;

pub(crate) fn init(globals: &mut Globals) -> Value {
    let class = Module::class_under_object();
    globals.set_toplevel_constant("Binding", class);
    class.add_builtin_class_method(globals, "new", binding_new);
    class.add_builtin_method_by_str(globals, "eval", eval);
    class.add_builtin_method_by_str(globals, "irb", irb);
    class.add_builtin_method_by_str(globals, "receiver", receiver);
    class.add_builtin_method_by_str(globals, "local_variables", local_variables);
    class.add_builtin_method_by_str(globals, "local_variable_defined?", local_variable_defined);
    class.into()
}

// Class methods

fn binding_new(_vm: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    Err(VMError::undefined_method(IdentId::NEW, self_val))
}

// Instance methods

fn eval(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_range(1, 3)?;
    let heap_ep = self_val.as_binding();
    let mut arg0 = vm[0];
    let code = arg0.expect_string("1st arg")?.to_string();
    let path = if args.len() >= 2 {
        let mut arg1 = vm[1];
        arg1.expect_string("2nd arg")?.to_string()
    } else {
        vm.caller_iseq()
            .source_info
            .path
            .to_string_lossy()
            .to_string()
    };
    vm.eval_binding(path, code, heap_ep)
}

fn irb(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    let heap_ep = self_val.as_binding();
    let cfp = vm.caller_cfp();
    let iseq = cfp.ep().iseq();
    let loc = iseq.get_loc(cfp.pc());
    eprint!("From:");
    iseq.source_info.show_loc(&loc);
    let res = vm.invoke_repl(heap_ep)?;
    Ok(res)
}

fn receiver(_: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    let ep = self_val.as_binding();
    Ok(ep.self_value())
}

fn local_variables(_: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    let ctx = self_val.as_binding();
    let mut vec = IndexSet::default();
    ctx.enumerate_local_vars(&mut vec);
    let ary = vec.into_iter().map(|id| Value::symbol(id)).collect();
    Ok(Value::array_from(ary))
}

fn local_variable_defined(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(1)?;
    let ctx = self_val.as_binding();
    let mut vec = IndexSet::default();
    let var = vm[0].expect_symbol_or_string("Arg")?;
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
