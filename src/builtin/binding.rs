use crate::*;

pub fn init() -> Value {
    let class = Module::class_under_object();
    BuiltinClass::set_toplevel_constant("Binding", class);
    class.add_builtin_class_method("new", binding_new);
    class.add_builtin_method_by_str("receiver", receiver);
    class.add_builtin_method_by_str("local_variables", local_variables);
    class.into()
}

// Class methods

fn binding_new(_vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    Err(RubyError::undefined_method(IdentId::NEW, self_val))
}

// Instance methods

fn receiver(_vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    let ctx = self_val.as_binding();
    Ok(ctx.self_value)
}

fn local_variables(_vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    let ctx = self_val.as_binding();
    let mut vec = vec![];
    ctx.enumerate_local_vars(&mut vec);
    let ary = vec.into_iter().map(|id| Value::symbol(id)).collect();
    Ok(Value::array_from(ary))
}
