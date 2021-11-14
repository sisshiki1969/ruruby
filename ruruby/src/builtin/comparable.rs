///
/// Comparable module
///
use crate::*;

pub(crate) fn init(globals: &mut Globals) -> Module {
    let class = Module::module();
    globals.set_toplevel_constant("Comparable", class);
    class.add_builtin_method_by_str(globals, "==", eq);
    class.add_builtin_method_by_str(globals, "<=", le);
    class.add_builtin_method_by_str(globals, "<", lt);
    class.add_builtin_method_by_str(globals, ">=", ge);
    class.add_builtin_method_by_str(globals, ">", gt);
    class
}

fn eq(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let res = vm.eval_send(IdentId::_CMP, self_val, &args.into(vm))?;
    let b = match res.as_fixnum() {
        Some(cmp) => match cmp {
            0 => true,
            _ => false,
        },
        None => false,
    };
    Ok(Value::bool(b))
}

fn le(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let res = vm.eval_send(IdentId::_CMP, self_val, &args.into(vm))?;
    let b = match res.as_fixnum() {
        Some(cmp) => match cmp {
            i if i <= 0 => true,
            _ => false,
        },
        None => false,
    };
    Ok(Value::bool(b))
}

fn lt(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let res = vm.eval_send(IdentId::_CMP, self_val, &args.into(vm))?;
    let b = match res.as_fixnum() {
        Some(cmp) => match cmp {
            i if i < 0 => true,
            _ => false,
        },
        None => unreachable!(),
    };
    Ok(Value::bool(b))
}

fn ge(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let res = vm.eval_send(IdentId::_CMP, self_val, &args.into(vm))?;
    let b = match res.as_fixnum() {
        Some(cmp) => match cmp {
            i if i >= 0 => true,
            _ => false,
        },
        None => false,
    };
    Ok(Value::bool(b))
}

fn gt(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let res = vm.eval_send(IdentId::_CMP, self_val, &args.into(vm))?;
    let b = match res.as_fixnum() {
        Some(cmp) => match cmp {
            i if i > 0 => true,
            _ => false,
        },
        None => unreachable!(),
    };
    Ok(Value::bool(b))
}

#[cfg(test)]
mod test {
    use crate::tests::*;

    #[test]
    fn comparable() {
        let program = r#"
    class Foo
        attr_accessor :x
        include Comparable
        def initialize(x)
            @x = x
        end
        def <=>(other)
            self.x<=>other.x
        end
    end

    assert (-1), Foo.new(1) <=> Foo.new(2)
    assert 0, Foo.new(2) <=> Foo.new(2)
    assert 1, Foo.new(2) <=> Foo.new(1)

    assert false, Foo.new(1) == Foo.new(2)
    #assert true, Foo.new(2) == Foo.new(2)
    assert false, Foo.new(2) == Foo.new(1)

    assert true, Foo.new(1) < Foo.new(2)
    assert false, Foo.new(2) < Foo.new(2)
    assert false, Foo.new(2) < Foo.new(1)
    
    assert true, Foo.new(1) <= Foo.new(2)
    assert true, Foo.new(2) <= Foo.new(2)
    assert false, Foo.new(2) <= Foo.new(1)
    
    assert false, Foo.new(1) > Foo.new(2)
    assert false, Foo.new(2) > Foo.new(2)
    assert true, Foo.new(2) > Foo.new(1)
    
    assert false, Foo.new(1) >= Foo.new(2)
    assert true, Foo.new(2) >= Foo.new(2)
    assert true, Foo.new(2) >= Foo.new(1)
    "#;
        assert_script(program);
    }
}
