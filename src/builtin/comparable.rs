///
/// Comparable module
///
use crate::*;

pub fn init(_globals: &mut Globals) -> Value {
    let mut comparable = Value::module();
    comparable.add_builtin_method_by_str("==", eq);
    comparable.add_builtin_method_by_str("<=", le);
    comparable.add_builtin_method_by_str("<", lt);
    comparable.add_builtin_method_by_str(">=", ge);
    comparable.add_builtin_method_by_str(">", gt);
    comparable
}

fn eq(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let res = vm.send_args(IdentId::_CMP, self_val, args)?;
    let b = match res.as_integer() {
        Some(cmp) => match cmp {
            0 => true,
            _ => false,
        },
        None => false,
    };
    Ok(Value::bool(b))
}

fn le(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let res = vm.send_args(IdentId::_CMP, self_val, args)?;
    let b = match res.as_integer() {
        Some(cmp) => match cmp {
            i if i <= 0 => true,
            _ => false,
        },
        None => false,
    };
    Ok(Value::bool(b))
}

fn lt(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let res = vm.send_args(IdentId::_CMP, self_val, args)?;
    let b = match res.as_integer() {
        Some(cmp) => match cmp {
            i if i < 0 => true,
            _ => false,
        },
        None => unreachable!(),
    };
    Ok(Value::bool(b))
}

fn ge(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let res = vm.send_args(IdentId::_CMP, self_val, args)?;
    let b = match res.as_integer() {
        Some(cmp) => match cmp {
            i if i >= 0 => true,
            _ => false,
        },
        None => false,
    };
    Ok(Value::bool(b))
}

fn gt(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let res = vm.send_args(IdentId::_CMP, self_val, args)?;
    let b = match res.as_integer() {
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
    use crate::test::*;

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
