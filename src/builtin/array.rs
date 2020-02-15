use crate::error::RubyError;
use crate::vm::*;

#[derive(Debug, Clone)]
pub struct ArrayInfo {
    pub elements: Vec<Value>,
}

impl ArrayInfo {
    pub fn new(elements: Vec<Value>) -> Self {
        ArrayInfo { elements }
    }
}

pub type ArrayRef = Ref<ArrayInfo>;

impl ArrayRef {
    pub fn from(elements: Vec<Value>) -> Self {
        ArrayRef::new(ArrayInfo::new(elements))
    }
}

pub fn init_array(globals: &mut Globals) -> Value {
    let array_id = globals.get_ident_id("Array");
    let class = ClassRef::from(array_id, globals.object);
    let obj = Value::class(globals, class);
    globals.add_builtin_instance_method(class, "[]=", array_set_elem);
    globals.add_builtin_instance_method(class, "push", array_push);
    globals.add_builtin_instance_method(class, "<<", array_push);
    globals.add_builtin_instance_method(class, "pop", array_pop);
    globals.add_builtin_instance_method(class, "shift", array_shift);
    globals.add_builtin_instance_method(class, "unshift", array_unshift);
    globals.add_builtin_instance_method(class, "length", array_length);
    globals.add_builtin_instance_method(class, "size", array_length);
    globals.add_builtin_instance_method(class, "empty?", array_empty);
    globals.add_builtin_instance_method(class, "*", array_mul);
    globals.add_builtin_instance_method(class, "+", array_add);
    globals.add_builtin_instance_method(class, "-", array_sub);
    globals.add_builtin_instance_method(class, "map", array_map);
    globals.add_builtin_instance_method(class, "flat_map", array_flat_map);
    globals.add_builtin_instance_method(class, "each", array_each);
    globals.add_builtin_instance_method(class, "include?", array_include);
    globals.add_builtin_instance_method(class, "reverse", array_reverse);
    globals.add_builtin_instance_method(class, "reverse!", array_reverse_);
    globals.add_builtin_instance_method(class, "transpose", array_transpose);
    globals.add_builtin_instance_method(class, "min", array_min);
    globals.add_builtin_instance_method(class, "fill", array_fill);
    globals.add_builtin_instance_method(class, "clear", array_clear);
    globals.add_builtin_instance_method(class, "uniq!", array_uniq_);
    globals.add_builtin_instance_method(class, "slice!", array_slice_);
    globals.add_builtin_class_method(obj, "new", array_new);
    obj
}

macro_rules! self_array {
    ($args:ident, $vm:ident) => {
        $args
            .self_value
            .as_array()
            .ok_or($vm.error_nomethod("Receiver must be an array."))?;
    };
}

// Class methods

fn array_new(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 2)?;
    let array_vec = match args.len() {
        0 => vec![],
        1 => match args[0].unpack() {
            RValue::FixNum(num) if num >= 0 => vec![Value::nil(); num as usize],
            RValue::Object(oref) => match oref.kind {
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
    let array = Value::array_from(&vm.globals, array_vec);
    Ok(array)
}

// Instance methods

pub fn array_set_elem(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 2, 3)?;
    let mut aref = self_array!(args, vm);
    let val = if args.len() == 3 { args[2] } else { args[1] };
    let index = args[0].expect_fixnum(&vm, "Index")?;
    if args.len() == 2 {
        if index >= aref.elements.len() as i64 {
            let padding = index as usize - aref.elements.len();
            aref.elements.append(&mut vec![Value::nil(); padding]);
            aref.elements.push(val);
        } else {
            let index = vm.get_array_index(index, aref.elements.len())?;
            aref.elements[index] = val;
        }
    } else {
        let index = vm.get_array_index(index, aref.elements.len())?;
        let len = args[1].expect_fixnum(&vm, "Length")?;
        if len < 0 {
            return Err(vm.error_index(format!("Negative length. {}", len)));
        } else {
            let len = len as usize;
            let end = std::cmp::min(aref.elements.len(), index + len);
            aref.elements.drain(index..end);
            aref.elements.insert(index, val);
        }
    };
    Ok(val)
}

fn array_push(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let mut aref = self_array!(args, vm);
    for arg in args.get_slice(0, args.len()) {
        aref.elements.push(*arg);
    }
    Ok(args.self_value)
}

fn array_pop(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let mut aref = self_array!(args, vm);
    let res = aref.elements.pop().unwrap_or_default();
    Ok(res)
}

fn array_shift(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let mut aref = self_array!(args, vm);
    let new = aref.elements.split_off(1);
    let res = aref.elements[0];
    aref.elements = new;
    Ok(res)
}

fn array_unshift(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    //vm.check_args_num(args.len(), 0, 0)?;
    if args.len() == 0 {
        return Ok(args.self_value);
    }
    let mut new = args.get_slice(0, args.len()).to_owned();
    let mut aref = self_array!(args, vm);
    new.append(&mut aref.elements);
    aref.elements = new;
    Ok(args.self_value)
}

fn array_length(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let aref = self_array!(args, vm);
    let res = Value::fixnum(aref.elements.len() as i64);
    Ok(res)
}

fn array_empty(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let aref = self_array!(args, vm);
    let res = Value::bool(aref.elements.is_empty());
    Ok(res)
}

fn array_mul(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let aref = self_array!(args, vm);
    let v = match args[0].unpack() {
        RValue::FixNum(num) => match num {
            i if i < 0 => return Err(vm.error_argument("Negative argument.")),
            0 => vec![],
            1 => aref.elements.clone(),
            _ => {
                let len = aref.elements.len();
                let src = &aref.elements[0..len];
                let mut v = vec![Value::nil(); len * num as usize];
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
    let res = Value::array_from(&vm.globals, v);
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
    Ok(Value::array_from(&vm.globals, lhs))
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
    Ok(Value::array_from(&vm.globals, v))
}

fn array_map(vm: &mut VM, args: &Args, block: Option<MethodRef>) -> VMResult {
    let aref = self_array!(args, vm);
    let iseq = match block {
        Some(method) => vm.globals.get_method_info(method).as_iseq(&vm)?,
        None => return Err(vm.error_argument("Currently, needs block.")),
    };
    let mut res = vec![];
    let context = vm.context();
    let param_num = iseq.req_params;
    let mut arg = Args::new(param_num);
    arg.self_value = context.self_value;
    for elem in &aref.elements {
        if param_num == 0 {
        } else if param_num == 1 {
            arg[0] = *elem;
        } else {
            match elem.as_array() {
                Some(ary) => {
                    for i in 0..param_num {
                        arg[i] = ary.elements[i];
                    }
                }
                None => arg[0] = *elem,
            }
        }
        vm.vm_run(iseq, Some(context), &arg, None, None)?;
        res.push(vm.stack_pop());
    }
    let res = Value::array_from(&vm.globals, res);
    Ok(res)
}

fn array_flat_map(vm: &mut VM, args: &Args, block: Option<MethodRef>) -> VMResult {
    let aref = self_array!(args, vm);
    let iseq = match block {
        Some(method) => vm.globals.get_method_info(method).as_iseq(&vm)?,
        None => return Err(vm.error_argument("Currently, needs block.")),
    };
    let mut res = vec![];
    let context = vm.context();
    let param_num = iseq.req_params;
    let mut arg = Args::new(param_num);
    arg.self_value = context.self_value;
    for elem in &aref.elements {
        if param_num == 0 {
        } else if param_num == 1 {
            arg[0] = *elem;
        } else {
            match elem.as_array() {
                Some(ary) => {
                    for i in 0..param_num {
                        arg[i] = ary.elements[i];
                    }
                }
                None => arg[0] = *elem,
            }
        }

        vm.vm_run(iseq, Some(context), &arg, None, None)?;
        let ary = vm.stack_pop();
        match ary.as_array() {
            Some(mut ary) => {
                res.append(&mut ary.elements);
            }
            None => res.push(ary),
        }
    }
    let res = Value::array_from(&vm.globals, res);
    Ok(res)
}

fn array_each(vm: &mut VM, args: &Args, block: Option<MethodRef>) -> VMResult {
    let aref = self_array!(args, vm);
    let iseq = match block {
        Some(method) => vm.globals.get_method_info(method).as_iseq(&vm)?,
        None => return Err(vm.error_argument("Currently, needs block.")),
    };
    let context = vm.context();
    let mut arg = Args::new1(context.self_value, None, Value::nil());
    for i in &aref.elements {
        arg[0] = *i;
        vm.vm_run(iseq, Some(context), &arg, None, None)?;
        vm.stack_pop();
    }
    Ok(args.self_value)
}

fn array_include(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let target = args[0];
    let aref = self_array!(args, vm);
    let res = aref.elements.iter().any(|x| x.clone().equal(target));
    Ok(Value::bool(res))
}

fn array_reverse(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let aref = self_array!(args, vm);
    let mut res = aref.elements.clone();
    res.reverse();
    Ok(Value::array_from(&vm.globals, res))
}

fn array_reverse_(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let mut aref = self_array!(args, vm);
    aref.elements.reverse();
    Ok(args.self_value)
}

fn array_transpose(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let aref = self_array!(args, vm);
    if aref.elements.len() == 0 {
        return Ok(Value::array_from(&vm.globals, vec![]));
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
        let ary = Value::array_from(&vm.globals, temp);
        trans.push(ary);
    }
    //aref.elements.reverse();
    let res = Value::array_from(&vm.globals, trans);
    Ok(res)
}

fn array_min(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    fn to_float(vm: &VM, val: Value) -> Result<f64, RubyError> {
        if val.is_packed_fixnum() {
            Ok(val.as_packed_fixnum() as f64)
        } else if val.is_packed_num() {
            Ok(val.as_packed_flonum())
        } else {
            Err(vm.error_type("Currently, each element must be Numeric."))
        }
    }

    let aref = self_array!(args, vm);
    if aref.elements.len() == 0 {
        return Ok(Value::nil());
    }
    let mut min_obj = aref.elements[0];
    let mut min = to_float(vm, min_obj)?;
    for elem in &aref.elements {
        let elem_f = to_float(vm, *elem)?;
        if elem_f < min {
            min_obj = *elem;
            min = elem_f;
        }
    }

    return Ok(min_obj);
}

fn array_fill(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let mut aref = self_array!(args, vm);
    for elem in &mut aref.elements {
        *elem = args[0];
    }
    Ok(args.self_value)
}

fn array_clear(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let mut aref = self_array!(args, vm);
    aref.elements.clear();
    Ok(args.self_value)
}

fn array_uniq_(vm: &mut VM, args: &Args, block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let mut aref = self_array!(args, vm);
    let mut set = std::collections::HashSet::new();
    match block {
        None => {
            aref.elements.retain(|x| set.insert(*x));
            Ok(args.self_value)
        }
        Some(block) => {
            let context = vm.context();
            aref.elements.retain(|x| {
                let block_args = Args::new1(context.self_value, None, *x);
                vm.eval_send(block, &block_args, None, None).unwrap();
                let res = vm.stack_pop();
                set.insert(res)
            });
            Ok(args.self_value)
        }
    }
}

fn array_slice_(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 2, 2)?;
    let start = args[0].expect_fixnum(vm, "Currently, first arg must be Integer.")?;
    if start < 0 {
        return Err(vm.error_argument("First arg must be positive value."));
    };
    let len = args[1].expect_fixnum(vm, "Currently, second arg must be Integer")?;
    if len < 0 {
        return Err(vm.error_argument("Second arg must be positive value."));
    };
    let start = start as usize;
    let len = len as usize;
    let mut aref = self_array!(args, vm);
    let new = aref.elements.drain(start..start + len).collect();
    Ok(Value::array_from(&vm.globals, new))
}
