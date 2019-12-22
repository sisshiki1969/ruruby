use crate::vm::*;

pub fn init_integer(globals: &mut Globals) -> ClassRef {
    let id = globals.get_ident_id("Integer");
    let class = ClassRef::from(id, globals.object_class);
    globals.add_builtin_instance_method(class, "times", integer_times);
    globals.add_builtin_instance_method(class, "chr", integer_chr);
    //globals.add_builtin_class_method(class, "new", integer_new);
    class
}

// Class methods

// Instance methods

fn integer_times(
    vm: &mut VM,
    receiver: PackedValue,
    _args: Vec<PackedValue>,
    block: Option<ContextRef>,
) -> VMResult {
    let num = receiver.as_fixnum().unwrap();
    if num < 1 {
        return Ok(PackedValue::nil());
    };
    match block {
        None => return Ok(PackedValue::nil()),
        Some(context) => {
            for i in 0..num {
                vm.vm_run(
                    context.self_value,
                    context.iseq_ref,
                    context.outer,
                    vec![PackedValue::fixnum(i)],
                    None,
                )?;
                vm.exec_stack.pop().unwrap();
            }
        }
    }
    Ok(receiver)
}

/// Built-in function "chr".
fn integer_chr(
    _vm: &mut VM,
    receiver: PackedValue,
    _args: Vec<PackedValue>,
    _block: Option<ContextRef>,
) -> VMResult {
    let num = receiver.as_fixnum().unwrap();
    Ok(Value::Char(num as u8).pack())
}
