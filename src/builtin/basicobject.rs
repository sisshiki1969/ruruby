use crate::*;

pub fn init() {
    let mut class = BuiltinClass::object().superclass().unwrap();
    BuiltinClass::set_toplevel_constant("BasicObject", class);
    class.add_builtin_method(IdentId::_ALIAS_METHOD, alias_method);
    class.add_builtin_method(IdentId::_METHOD_MISSING, method_missing);
    class.add_builtin_method_by_str("__id__", basicobject_id);
}

/// An alias statement is compiled to method call for this func.
/// TODO: Currently, aliasing of global vars does not work.
fn alias_method(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(2)?;
    let new = vm[0].as_symbol().unwrap();
    let org = vm[1].as_symbol().unwrap();
    let is_new_gvar = IdentId::starts_with(new, "$");
    let is_org_gvar = IdentId::starts_with(org, "$");
    match (is_new_gvar, is_org_gvar) {
        (true, true) => {}
        (false, false) => {
            // TODO: Is it right?
            let mut class = self_val.get_class_if_object();
            let method = class.get_method_or_nomethod(org)?;
            class.add_method(new, method);
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

fn method_missing(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_min(1)?;
    let method_id = match vm[0].as_symbol() {
        Some(id) => id,
        None => {
            return Err(RubyError::argument(format!(
                "1st arg for method_missing must be symbol. {:?}",
                vm[0]
            )))
        }
    };
    if self_val.id() == vm.context().self_value.id() {
        Err(RubyError::name(format!(
            "Undefined local variable or method `{:?}' for {:?}",
            method_id, self_val
        )))
    } else {
        Err(RubyError::undefined_method(method_id, self_val))
    }
}

fn basicobject_id(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    Ok(Value::integer(self_val.id() as i64))
}

#[cfg(test)]
mod test {
    use crate::tests::*;

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

    #[test]
    fn alias2() {
        let program = r#"
        class AliasObject
          def value
            77
          end
        end
        @obj = AliasObject.new
        @meta = class << @obj; self; end
        assert @meta, @obj.singleton_class
        assert 77, @obj.value
        @meta.class_eval do
          alias __value value
        end
        assert 77, @obj.__value
        assert 77, AliasObject.new.value
        assert_error { AliasObject.new.__value }
        "#;
        assert_script(program);
    }

    #[test]
    fn method_missing() {
        let program = r#"
        assert_error {4.a}
        "#;
        assert_script(program);
    }

    #[test]
    fn basicobject_id() {
        let program = r#"
        assert 11, 5.__id__
        assert true, 5.__id__ == 5.__id__
        assert false, "ruby".__id__ == "ruby".__id__
        "#;
        assert_script(program);
    }
}
