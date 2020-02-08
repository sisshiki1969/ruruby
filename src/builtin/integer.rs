use crate::vm::*;

pub fn init_integer(globals: &mut Globals) -> PackedValue {
    let id = globals.get_ident_id("Integer");
    let class = ClassRef::from(id, globals.object);
    globals.add_builtin_instance_method(class, "times", integer_times);
    globals.add_builtin_instance_method(class, "chr", integer_chr);
    globals.add_builtin_instance_method(class, "to_f", integer_tof);
    PackedValue::class(globals, class)
}

// Class methods

// Instance methods

fn integer_times(
    vm: &mut VM,
    receiver: PackedValue,
    _args: &VecArray,
    block: Option<MethodRef>,
) -> VMResult {
    let num = receiver.as_fixnum().unwrap();
    if num < 1 {
        return Ok(PackedValue::nil());
    };
    match block {
        None => return Ok(PackedValue::nil()),
        Some(method) => {
            let context = vm.context();
            let self_value = context.self_value;
            let info = vm.globals.get_method_info(method);
            let iseq = info.as_iseq(&vm)?;
            for i in 0..num {
                let arg = VecArray::new1(PackedValue::fixnum(i));
                vm.vm_run(self_value, iseq, Some(context), &arg, None, None)?;
                vm.stack_pop();
            }
        }
    }
    Ok(receiver)
}

/// Built-in function "chr".
fn integer_chr(
    _vm: &mut VM,
    receiver: PackedValue,
    _args: &VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let num = receiver.as_fixnum().unwrap();
    Ok(Value::Char(num as u8).pack())
}

fn integer_tof(
    _vm: &mut VM,
    receiver: PackedValue,
    _args: &VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let num = receiver.as_fixnum().unwrap();
    Ok(PackedValue::flonum(num as f64))
}
