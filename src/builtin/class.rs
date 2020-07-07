use crate::*;

pub fn init(globals: &mut Globals) {
    let class = globals.class_class;
    globals.add_builtin_instance_method(class, "new", new);
    globals.add_builtin_instance_method(class, "superclass", superclass);
    globals.add_builtin_instance_method(class, "inspect", inspect);
    globals.add_builtin_class_method(globals.builtins.class, "new", class_new);
}

// Class methods

/// Create new class.
/// If a block is given, eval it in the context of newly created class.
/// args[0]: super class.
fn class_new(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_range(args.len(), 0, 1)?;
    let superclass = if args.len() == 0 {
        vm.globals.builtins.object
    } else {
        args[0]
    };
    let val = Value::class_from(&mut vm.globals, None, superclass);

    match args.block {
        Some(method) => {
            vm.class_push(val);
            let arg = Args::new1(val);
            vm.eval_method(method, val, Some(vm.context()), &arg)?;
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
fn superclass(vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    let class = vm.expect_class(self_val, "Receiver")?;
    Ok(class.superclass)
}

fn inspect(vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    let cref = vm.expect_class(self_val, "Receiver")?;
    let s = match cref.name {
        Some(id) => format! {"{}", IdentId::get_name(id)},
        None => format! {"#<Class:0x{:x}>", cref.id()},
    };
    Ok(Value::string(&vm.globals, s))
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
}
