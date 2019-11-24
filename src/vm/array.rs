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
    globals.add_builtin_class_method(array_class, "new", array::array_new);
    array_class
}

// Class methods

pub fn array_new(vm: &mut VM, _receiver: PackedValue, args: Vec<PackedValue>) -> VMResult {
    let array_vec = match args.len() {
        0 => vec![],
        1 => match args[0].as_fixnum() {
            Some(num) => vec![PackedValue::nil(); num as usize],
            None => match args[0].as_array() {
                Some(aref) => aref.elements.clone(),
                None => return Err(vm.error_nomethod("Invalid arguments")),
            },
        },
        2 => {
            let arg_num = args[0]
                .as_fixnum()
                .ok_or(vm.error_nomethod("Invalid arguments"))?;
            vec![args[1]; arg_num as usize]
        }
        _ => return Err(vm.error_nomethod("Wrong number of arguments.")),
    };
    let array = PackedValue::array(ArrayRef::from(array_vec));
    Ok(array)
}

// Instance methods

pub fn array_push(vm: &mut VM, receiver: PackedValue, args: Vec<PackedValue>) -> VMResult {
    let mut aref = receiver
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    for arg in args {
        aref.elements.push(arg);
    }
    Ok(receiver)
}

pub fn array_pop(vm: &mut VM, receiver: PackedValue, _args: Vec<PackedValue>) -> VMResult {
    let mut aref = receiver
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    let res = aref.elements.pop().unwrap_or(PackedValue::nil());
    Ok(res)
}
