use crate::*;

pub fn init_math(globals: &mut Globals) -> Value {
    let id = globals.get_ident_id("Math");
    let class = ClassRef::from(id, globals.builtins.object);
    let obj = Value::class(globals, class);
    globals.add_builtin_class_method(obj, "sqrt", sqrt);
    globals.add_builtin_class_method(obj, "cos", cos);
    globals.add_builtin_class_method(obj, "sin", sin);
    obj
}

// Class methods

// Instance methods

fn sqrt(vm: &mut VM, args: &Args) -> VMResult {
    let arg = args[0];
    let num = if arg.is_packed_num() {
        if arg.is_packed_fixnum() {
            arg.as_packed_fixnum() as f64
        } else {
            arg.as_packed_flonum()
        }
    } else {
        return Err(vm.error_type("Must be a number."));
    };
    let res = Value::flonum(num.sqrt());
    Ok(res)
}

fn cos(vm: &mut VM, args: &Args) -> VMResult {
    let arg = args[0];
    let num = if arg.is_packed_num() {
        if arg.is_packed_fixnum() {
            arg.as_packed_fixnum() as f64
        } else {
            arg.as_packed_flonum()
        }
    } else {
        return Err(vm.error_type("Must be a number."));
    };
    let res = Value::flonum(num.cos());
    Ok(res)
}

fn sin(vm: &mut VM, args: &Args) -> VMResult {
    let arg = args[0];
    let num = if arg.is_packed_num() {
        if arg.is_packed_fixnum() {
            arg.as_packed_fixnum() as f64
        } else {
            arg.as_packed_flonum()
        }
    } else {
        return Err(vm.error_type("Must be a number."));
    };
    let res = Value::flonum(num.sin());
    Ok(res)
}
