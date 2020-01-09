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

pub fn init_array(globals: &mut Globals) -> ClassRef {
    let array_id = globals.get_ident_id("Array");
    let array_class = ClassRef::from(array_id, globals.object_class);
    globals.add_builtin_instance_method(array_class, "push", array::array_push);
    globals.add_builtin_instance_method(array_class, "pop", array::array_pop);
    globals.add_builtin_instance_method(array_class, "length", array::array_length);
    globals.add_builtin_instance_method(array_class, "size", array::array_length);
    globals.add_builtin_instance_method(array_class, "*", array::array_mul);
    globals.add_builtin_instance_method(array_class, "map", array::array_map);
    globals.add_builtin_instance_method(array_class, "each", array::array_each);
    globals.add_builtin_instance_method(array_class, "include?", array::array_include);
    globals.add_builtin_class_method(array_class, "new", array::array_new);
    array_class
}

// Class methods

fn array_new(
    vm: &mut VM,
    _receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
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
    let array = PackedValue::array(&mut vm.globals, ArrayRef::from(array_vec));
    Ok(array)
}

// Instance methods

fn array_push(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let mut aref = receiver
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    for arg in args.iter() {
        aref.elements.push(arg.clone());
    }
    Ok(receiver)
}

fn array_pop(
    vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let mut aref = receiver
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    let res = aref.elements.pop().unwrap_or(PackedValue::nil());
    Ok(res)
}

fn array_length(
    vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let aref = receiver
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    let res = PackedValue::fixnum(aref.elements.len() as i64);
    Ok(res)
}

fn array_mul(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let aref = receiver
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
    let res = PackedValue::array(&vm.globals, ArrayRef::from(v));
    Ok(res)
}

fn array_map(
    vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    block: Option<MethodRef>,
) -> VMResult {
    let aref = receiver
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    let iseq = match block {
        Some(method) => vm.globals.get_method_info(method).as_iseq(&vm)?,
        None => return Err(vm.error_argument("Currently, needs block.")),
    };
    let mut res = vec![];
    let context = vm.context();
    for i in &aref.elements {
        vm.vm_run(
            context.self_value,
            iseq,
            Some(context),
            VecArray::new1(i.clone()),
            None,
        )?;
        res.push(vm.exec_stack.pop().unwrap());
    }
    let res = PackedValue::array(&vm.globals, ArrayRef::from(res));
    Ok(res)
}

fn array_each(
    vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    block: Option<MethodRef>,
) -> VMResult {
    let aref = receiver
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    let iseq = match block {
        Some(method) => vm.globals.get_method_info(method).as_iseq(&vm)?,
        None => return Err(vm.error_argument("Currently, needs block.")),
    };
    let context = vm.context();
    for i in &aref.elements {
        vm.vm_run(
            context.self_value,
            iseq,
            Some(context),
            VecArray::new1(i.clone()),
            None,
        )?;
        vm.exec_stack.pop().unwrap();
    }
    let res = receiver;
    Ok(res)
}

fn array_include(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let target = args[0];
    let aref = receiver
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    let res = aref
        .elements
        .iter()
        .any(|x| match vm.eval_eq(x.clone(), target) {
            Ok(res) => res,
            Err(_) => false,
        });
    Ok(PackedValue::bool(res))
}
