use crate::vm::*;

#[derive(Debug, Clone)]
pub struct ArrayInfo {
    pub elements: Vec<PackedValue>,
}

impl ArrayInfo {
    pub fn new(elements: Vec<PackedValue>) -> Self {
        ArrayInfo { elements }
    }
}

pub type ArrayRef = Ref<ArrayInfo>;

impl ArrayRef {
    pub fn from(elements: Vec<PackedValue>) -> Self {
        ArrayRef::new(ArrayInfo::new(elements))
    }
}

pub fn init_array(globals: &mut Globals) -> PackedValue {
    let array_id = globals.get_ident_id("Array");
    let class = ClassRef::from(array_id, globals.object);
    let obj = PackedValue::class(globals, class);
    globals.add_builtin_instance_method(class, "push", array_push);
    globals.add_builtin_instance_method(class, "<<", array_push);
    globals.add_builtin_instance_method(class, "pop", array_pop);
    globals.add_builtin_instance_method(class, "shift", array_shift);
    globals.add_builtin_instance_method(class, "length", array_length);
    globals.add_builtin_instance_method(class, "size", array_length);
    globals.add_builtin_instance_method(class, "empty?", array_empty);
    globals.add_builtin_instance_method(class, "*", array_mul);
    globals.add_builtin_instance_method(class, "+", array_add);
    globals.add_builtin_instance_method(class, "-", array_sub);
    globals.add_builtin_instance_method(class, "map", array_map);
    globals.add_builtin_instance_method(class, "each", array_each);
    globals.add_builtin_instance_method(class, "include?", array_include);
    globals.add_builtin_instance_method(class, "reverse", array_reverse);
    globals.add_builtin_instance_method(class, "reverse!", array_reverse_);
    globals.add_builtin_instance_method(class, "transpose", array_transpose);
    globals.add_builtin_class_method(obj, "new", array_new);
    obj
}

// Class methods

fn array_new(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 2)?;
    let array_vec = match args.len() {
        0 => vec![],
        1 => match args[0].unpack() {
            Value::FixNum(num) if num >= 0 => vec![PackedValue::nil(); num as usize],
            Value::Object(oref) => match oref.kind {
                ObjKind::Array(aref) => aref.elements.clone(),
                _ => return Err(vm.error_nomethod("Invalid arguments")),
            },
            _ => return Err(vm.error_nomethod("Invalid arguments")),
        },
        2 => {
            let arg_num = args[0]
                .as_fixnum()
                .ok_or(vm.error_nomethod("Invalid arguments"))?;
            vec![args[1]; arg_num as usize]
        }
        _ => unreachable!(),
    };
    let array = PackedValue::array_from(&vm.globals, array_vec);
    Ok(array)
}

// Instance methods

fn array_push(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let mut aref = args
        .self_value
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    for i in 0..args.len() {
        aref.elements.push(args[i]);
    }
    Ok(args.self_value)
}

fn array_pop(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let mut aref = args
        .self_value
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    let res = aref.elements.pop().unwrap_or_default();
    Ok(res)
}

fn array_shift(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let mut aref = args
        .self_value
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    let new = aref.elements.split_off(1);
    let res = aref.elements.clone();
    aref.elements = new;
    Ok(res[0])
}

fn array_length(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let aref = args
        .self_value
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    let res = PackedValue::fixnum(aref.elements.len() as i64);
    Ok(res)
}

fn array_empty(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let aref = args
        .self_value
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    let res = PackedValue::bool(aref.elements.is_empty());
    Ok(res)
}

fn array_mul(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let aref = args
        .self_value
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    let v = match args[0].unpack() {
        Value::FixNum(num) => match num {
            i if i < 0 => return Err(vm.error_argument("Negative argument.")),
            0 => vec![],
            1 => aref.elements.clone(),
            _ => {
                let len = aref.elements.len();
                let src = &aref.elements[0..len];
                let mut v = vec![PackedValue::nil(); len * num as usize];
                //println!("dest:{:?} src:{:?}", aref.elements, src);
                let mut i = 0;
                for _ in 0..num {
                    v[i..i + len].copy_from_slice(src);
                    i += len;
                }
                v
            }
        },
        _ => return Err(vm.error_nomethod(" ")),
    };
    let res = PackedValue::array_from(&vm.globals, v);
    Ok(res)
}

fn array_add(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let mut lhs = args
        .self_value
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?
        .elements
        .clone();
    let mut rhs = args[0]
        .as_array()
        .ok_or(vm.error_nomethod("An arg must be an array."))?
        .elements
        .clone();
    lhs.append(&mut rhs);
    Ok(PackedValue::array_from(&vm.globals, lhs))
}

fn array_sub(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let lhs_v = &args
        .self_value
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?
        .elements;
    let rhs_v = &args[0]
        .as_array()
        .ok_or(vm.error_nomethod("An arg must be an array."))?
        .elements;
    let mut v = vec![];
    for lhs in lhs_v {
        let mut flag = true;
        for rhs in rhs_v {
            if lhs.equal(*rhs) {
                flag = false;
                break;
            }
        }
        if flag {
            v.push(*lhs)
        }
    }
    Ok(PackedValue::array_from(&vm.globals, v))
}

fn array_map(vm: &mut VM, args: &Args, block: Option<MethodRef>) -> VMResult {
    let aref = args
        .self_value
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    let iseq = match block {
        Some(method) => vm.globals.get_method_info(method).as_iseq(&vm)?,
        None => return Err(vm.error_argument("Currently, needs block.")),
    };
    let mut res = vec![];
    let context = vm.context();
    let mut arg = Args::new1(context.self_value, None, PackedValue::nil());
    for i in &aref.elements {
        arg[0] = *i;
        vm.vm_run(iseq, Some(context), &arg, None, None)?;
        res.push(vm.stack_pop());
    }
    let res = PackedValue::array_from(&vm.globals, res);
    Ok(res)
}

fn array_each(vm: &mut VM, args: &Args, block: Option<MethodRef>) -> VMResult {
    let aref = args
        .self_value
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    let iseq = match block {
        Some(method) => vm.globals.get_method_info(method).as_iseq(&vm)?,
        None => return Err(vm.error_argument("Currently, needs block.")),
    };
    let context = vm.context();
    let mut arg = Args::new1(context.self_value, None, PackedValue::nil());
    for i in &aref.elements {
        arg[0] = *i;
        vm.vm_run(iseq, Some(context), &arg, None, None)?;
        vm.stack_pop();
    }
    Ok(args.self_value)
}

fn array_include(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let target = args[0];
    let aref = args
        .self_value
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    let res = aref.elements.iter().any(|x| x.clone().equal(target));
    Ok(PackedValue::bool(res))
}

fn array_reverse(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let aref = args
        .self_value
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    let mut res = aref.elements.clone();
    res.reverse();
    Ok(PackedValue::array_from(&vm.globals, res))
}

fn array_reverse_(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let mut aref = args
        .self_value
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    aref.elements.reverse();
    Ok(args.self_value)
}

fn array_transpose(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let aref = args
        .self_value
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    if aref.elements.len() == 0 {
        return Ok(PackedValue::array_from(&vm.globals, vec![]));
    }
    let mut vec = vec![];
    for elem in &aref.elements {
        let ary = elem
            .as_array()
            .ok_or(vm.error_nomethod("Each element of receiver must be an array."))?
            .elements
            .clone();
        vec.push(ary);
    }
    let len = vec[0].len();
    let mut trans = vec![];
    for i in 0..len {
        let mut temp = vec![];
        for v in &vec {
            if v.len() != len {
                return Err(vm.error_index("Element size differs."));
            }
            temp.push(v[i]);
        }
        let ary = PackedValue::array_from(&vm.globals, temp);
        trans.push(ary);
    }
    //aref.elements.reverse();
    let res = PackedValue::array_from(&vm.globals, trans);
    Ok(res)
}
