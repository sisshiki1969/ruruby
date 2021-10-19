use crate::*;

pub fn init(globals: &mut Globals) {
    let class = BuiltinClass::class();
    BuiltinClass::set_toplevel_constant("Class", class);
    class.add_builtin_class_method(globals, "new", class_new);
    class.add_builtin_method_by_str(globals, "new", new);
    class.add_builtin_method_by_str(globals, "allocate", allocate);
    class.add_builtin_method_by_str(globals, "superclass", superclass);
    class.add_builtin_method_by_str(globals, "inspect", inspect);
    class.add_builtin_method_by_str(globals, "to_s", inspect);
}

// Class methods

/// Create new class.
/// If a block is given, eval it in the context of newly created class.
/// args[0]: super class.
fn class_new(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    vm.check_args_range(0, 1)?;
    let superclass = if args.len() == 0 {
        BuiltinClass::object()
    } else {
        vm[0].expect_class("1st arg")?
    };
    let module = Module::class_under(superclass);
    let val = module.into();
    match &args.block {
        None => {}
        Some(block) => {
            let arg = Args::new1(val);
            let res = vm.eval_block_self(block, val, &arg);
            res?;
        }
    };
    Ok(val)
}

/// Create new instance of `self`.
pub fn new(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    let self_val = self_val.into_module();
    let new_instance = Value::ordinary_object(self_val);
    // Call initialize method if it exists.
    if let Some(method) = vm
        .globals
        .methods
        .find_method(self_val, IdentId::INITIALIZE)
    {
        vm.eval_method(method, new_instance, &args.into(vm))?;
    };
    Ok(new_instance)
}

/// Create new instance of `self` without initialization.
fn allocate(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let self_val = self_val.into_module();
    let new_instance = Value::ordinary_object(self_val);
    Ok(new_instance)
}

/// Get super class of `self`.
fn superclass(_: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    let self_val = self_val.into_module();
    let superclass = match self_val.superclass() {
        Some(superclass) => superclass.into(),
        None => Value::nil(),
    };
    Ok(superclass)
}

fn inspect(_: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    // self_val may be a non-class object. ex) instance of BasicObject with singleton class.
    let s = match self_val.if_mod_class() {
        Some(m) => m.inspect(),
        None => format!("!{:?}", self_val),
    };
    Ok(Value::string(s))
}

#[cfg(test)]
mod tests {
    use crate::tests::*;

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
