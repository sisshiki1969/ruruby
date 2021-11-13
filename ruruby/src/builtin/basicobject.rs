use crate::*;

pub(crate) fn init(globals: &mut Globals) {
    let mut class = BuiltinClass::object().superclass().unwrap();
    BuiltinClass::set_toplevel_constant("BasicObject", class);
    class.add_builtin_method(globals, IdentId::_ALIAS_METHOD, alias_method);
    class.add_builtin_method(globals, IdentId::_METHOD_MISSING, method_missing);
    class.add_builtin_method_by_str(globals, "__id__", basicobject_id);
    class.add_builtin_method_by_str(globals, "instance_exec", instance_exec);
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
            let method = class.get_method_or_nomethod(&mut vm.globals, org)?;
            class.add_method(&mut vm.globals, new, method);
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
    if self_val.id() == vm.self_value().id() {
        Err(RubyError::name(format!(
            "Undefined local variable or method `{:?}' for {:?}",
            method_id, self_val
        )))
    } else {
        Err(VMError::undefined_method(method_id, self_val))
    }
}

fn basicobject_id(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    Ok(Value::integer(self_val.id() as i64))
}

/// instance_exec(*args) {|*vars| ... } -> object
/// https://docs.ruby-lang.org/ja/latest/method/BasicObject/i/instance_exec.html
fn instance_exec(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    let block = args.expect_block()?;
    let res = vm.eval_block_self(block, self_val, &args.into(vm));
    res
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
    fn bo_method_missing() {
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

    #[test]
    fn instance_eval() {
        let program = r#"
        class KlassWithSecret
          def initialize
            @secret = 99
          end
        end
        k = KlassWithSecret.new
        # 以下で x には 5 が渡される
        assert 104, k.instance_exec(5) {|x| @secret + x }   #=> 104
    "#;
        assert_script(program);
    }
}
