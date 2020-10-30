use crate::*;

pub fn init(globals: &mut Globals) {
    let mut class_obj = globals.builtins.class;
    let class = class_obj.as_mut_class();
    class.add_builtin_method_by_str("new", new);
    class.add_builtin_method_by_str("superclass", superclass);
    class.add_builtin_method_by_str("inspect", inspect);
    class.add_builtin_method_by_str("name", name);
    class_obj.add_builtin_class_method("new", class_new);
}

// Class methods

/// Create new class.
/// If a block is given, eval it in the context of newly created class.
/// args[0]: super class.
fn class_new(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_range(args.len(), 0, 1)?;
    let superclass = if args.len() == 0 {
        BuiltinClass::object()
    } else {
        args[0]
    };
    let val = Value::class_from(superclass);

    match &args.block {
        Some(block) => {
            vm.class_push(val);
            let arg = Args::new1(val);
            vm.eval_block_self(block, val, &arg)?;
            vm.class_pop();
        }
        None => {}
    };
    Ok(val)
}

/// Create new instance of `self`.
pub fn new(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let new_instance = Value::ordinary_object(self_val);
    // Call initialize method if it exists.
    if let Some(method) = self_val.get_instance_method(IdentId::INITIALIZE) {
        vm.eval_send(method, new_instance, args)?;
    };
    Ok(new_instance)
}

/// Get super class of `self`.
fn superclass(vm: &mut VM, mut self_val: Value, _args: &Args) -> VMResult {
    let class = self_val.expect_class(vm, "Receiver")?;
    Ok(class.superclass)
}

fn inspect(vm: &mut VM, mut self_val: Value, _args: &Args) -> VMResult {
    let cref = self_val.expect_class(vm, "Receiver")?;
    let s = match cref.name {
        Some(id) => format! {"{:?}", id},
        None => format! {"#<Class:0x{:x}>", cref.id()},
    };
    Ok(Value::string(s))
}

fn name(_vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    let cref = self_val.as_class();
    let val = match cref.name {
        Some(id) => Value::string(format! {"{:?}", id}),
        None => Value::nil(),
    };
    Ok(val)
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn class_new() {
        let program = r#"
        A = Class.new{
            attr_accessor :a
            def initialize
                @a = 100
            end
        }
        assert(100, A.new.a)
        assert("A", A.inspect)
        "#;
        assert_script(program);
    }

    #[test]
    fn class_name() {
        let program = r#"
        assert("Integer", Integer.name)
        class A
        end
        assert("A", A.name)
        a = Class.new()
        assert(nil, a.name)
        B = a
        assert("B", a.name)
        "#;
        assert_script(program);
    }
}
