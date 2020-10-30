use crate::*;

pub fn init(globals: &mut Globals) {
    let mut basic_object = globals.builtins.object.superclass().unwrap();
    let basic_class = basic_object.as_mut_class();
    basic_class.add_builtin_method(IdentId::_ALIAS_METHOD, alias_method);
}

fn alias_method(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 2)?;
    let old = args[1].as_symbol().unwrap();
    let new = args[0].as_symbol().unwrap();
    let mut class = vm.class();
    let method = vm.get_method(class, old)?;
    let class_info = class.as_mut_class();
    class_info.add_method(&mut vm.globals, new, method);
    Ok(Value::nil())
}

#[cfg(test)]
mod test {
    use crate::test::*;

    #[test]
    fn alias() {
        let program = r#"
        def foo
          42
        end
        alias bar foo
        alias :boo foo
        alias bee :foo
        alias :bzz :foo
        assert 42, foo
        assert 42, bar
        assert 42, boo
        assert 42, bee
        assert 42, bzz
        "#;
        assert_script(program);
    }
}
