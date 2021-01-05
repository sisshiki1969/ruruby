use crate::*;

pub fn init(globals: &mut Globals) {
    let mut class = globals.builtins.class;
    class.add_builtin_class_method("new", class_new);

    class.add_builtin_method_by_str("new", new);
    class.add_builtin_method_by_str("allocate", allocate);
    class.add_builtin_method_by_str("superclass", superclass);
    class.add_builtin_method_by_str("inspect", inspect);
}

// Class methods

/// Create new class.
/// If a block is given, eval it in the context of newly created class.
/// args[0]: super class.
fn class_new(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let superclass = if args.len() == 0 {
        BuiltinClass::object()
    } else {
        args[0]
    };
    let val = Value::class_under(superclass);

    match &args.block {
        Block::None => {}
        _ => {
            vm.class_push(val);
            let arg = Args::new1(val);
            let res = vm.eval_block_self(&args.block, val, &arg);
            vm.class_pop();
            res?;
        }
    };
    Ok(val)
}

/// Create new instance of `self`.
pub fn new(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let new_instance = Value::ordinary_object(self_val);
    // Call initialize method if it exists.
    if let Some(method) = vm.globals.find_method(self_val, IdentId::INITIALIZE) {
        vm.eval_send(method, new_instance, args)?;
    };
    Ok(new_instance)
}

/// Create new instance of `self` without initialization.
fn allocate(_vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let new_instance = Value::ordinary_object(self_val);
    Ok(new_instance)
}

/// Get super class of `self`.
fn superclass(vm: &mut VM, mut self_val: Value, _args: &Args) -> VMResult {
    self_val.expect_class(vm, "Receiver")?;
    let superclass = match self_val.superclass() {
        Some(superclass) => superclass,
        None => Value::nil(),
    };
    Ok(superclass)
}

fn inspect(_: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    let cref = self_val.if_mod_class().unwrap();
    Ok(Value::string(cref.inspect()))
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
        assert(0, Class.new.inspect =~ /#<Class:0x.{16}>/)
        "#;
        assert_script(program);
    }
}
