use crate::*;

pub fn init(globals: &mut Globals) {
    let mut basic_object = globals.builtins.object.superclass().unwrap();
    let basic_class = basic_object.as_mut_class();
    basic_class.add_builtin_method(IdentId::_ALIAS_METHOD, alias_method);
}

/// An alias statement is compiled to method call for this func.
/// TODO: Currently, aliasing of global vars does not work.
fn alias_method(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(2)?;
    let new = args[0].as_symbol().unwrap();
    let org = args[1].as_symbol().unwrap();
    let is_new_gvar = IdentId::starts_with(new, "$");
    let is_org_gvar = IdentId::starts_with(org, "$");
    match (is_new_gvar, is_org_gvar) {
        (true, true) => {}
        (false, false) => {
            let mut class = vm.class();
            let method = vm.get_method(class, org)?;
            class
                .as_mut_class()
                .add_method(&mut vm.globals, new, method);
        }
        (true, false) => {
            return Err(RubyError::argument(
                "2nd arg of alias must be a global variable.",
            ))
        }
        (false, true) => {
            return Err(RubyError::argument(
                "2nd arg of alias must be a method name.",
            ))
        }
    }
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
