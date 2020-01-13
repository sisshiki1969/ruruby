use crate::vm::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct HashInfo {
    pub map: HashMap<PackedValue, PackedValue>,
}

impl HashInfo {
    pub fn new(map: HashMap<PackedValue, PackedValue>) -> Self {
        HashInfo { map }
    }
}

pub type HashRef = Ref<HashInfo>;

impl HashRef {
    pub fn from(map: HashMap<PackedValue, PackedValue>) -> Self {
        HashRef::new(HashInfo::new(map))
    }
}

pub fn init_hash(globals: &mut Globals) -> ClassRef {
    let id = globals.get_ident_id("Hash");
    let class = ClassRef::from(id, globals.object_class);
    globals.add_builtin_instance_method(class, "clear", hash_clear);
    globals.add_builtin_instance_method(class, "clone", hash_clone);
    globals.add_builtin_instance_method(class, "dup", hash_clone);
    globals.add_builtin_instance_method(class, "compact", hash_compact);
    globals.add_builtin_instance_method(class, "delete", hash_delete);
    globals.add_builtin_instance_method(class, "empty?", hash_empty);
    globals.add_builtin_instance_method(class, "select", hash_select);
    globals.add_builtin_instance_method(class, "has_key?", hash_has_key);
    globals.add_builtin_instance_method(class, "has_value?", hash_has_value);
    globals.add_builtin_instance_method(class, "keys", hash_keys);
    globals.add_builtin_instance_method(class, "length", hash_length);
    globals.add_builtin_instance_method(class, "size", hash_length);
    globals.add_builtin_instance_method(class, "values", hash_values);
    //globals.add_builtin_class_method(class, "new", range_new);
    class
}

fn hash_clear(
    _vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let mut hash = receiver.as_hash().unwrap();
    hash.map.clear();
    Ok(receiver)
}

fn hash_clone(
    vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let hash = receiver.as_hash().unwrap();
    Ok(PackedValue::hash(&vm.globals, hash.dup()))
}

fn hash_compact(
    vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let mut hash = receiver.as_hash().unwrap().dup();
    hash.map.retain(|_, &mut v| v != PackedValue::nil());
    Ok(PackedValue::hash(&vm.globals, hash))
}

fn hash_delete(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let mut hash = receiver.as_hash().unwrap();
    let res = match hash.map.remove(&args[0]) {
        Some(v) => v,
        None => PackedValue::nil(),
    };
    Ok(res)
}

fn hash_empty(
    _vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let hash = receiver.as_hash().unwrap();
    Ok(PackedValue::bool(hash.map.len() == 0))
}

fn hash_select(
    vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    block: Option<MethodRef>,
) -> VMResult {
    let hash = receiver.as_hash().unwrap();
    let iseq = match block {
        Some(method) => vm.globals.get_method_info(method).as_iseq(&vm)?,
        None => return Err(vm.error_argument("Currently, needs block.")),
    };
    let mut res = HashMap::new();
    let context = vm.context();
    for (k, v) in hash.map.iter() {
        vm.vm_run(
            context.self_value,
            iseq,
            Some(context),
            VecArray::new2(k.clone(), v.clone()),
            None,
            None,
        )?;
        let b = vm.exec_stack.pop().unwrap();
        if vm.val_to_bool(b) {
            res.insert(k.clone(), v.clone());
        };
    }
    Ok(PackedValue::hash(&vm.globals, HashRef::from(res)))
}

fn hash_has_key(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let hash = receiver.as_hash().unwrap();
    Ok(PackedValue::bool(hash.map.contains_key(&args[0])))
}

fn hash_has_value(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let hash = receiver.as_hash().unwrap();
    let res = hash.map.values().any(|&x| x == args[0]);
    Ok(PackedValue::bool(res))
}

fn hash_length(
    _vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let hash = receiver.as_hash().unwrap();
    let len = hash.map.len();
    Ok(PackedValue::fixnum(len as i64))
}

fn hash_keys(
    vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let hash = receiver.as_hash().unwrap();
    let mut vec = vec![];
    for key in hash.map.keys() {
        vec.push(key.clone());
    }
    Ok(PackedValue::array(&vm.globals, ArrayRef::from(vec)))
}

fn hash_values(
    vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let hash = receiver.as_hash().unwrap();
    let mut vec = vec![];
    for val in hash.map.values() {
        vec.push(val.clone());
    }
    Ok(PackedValue::array(&vm.globals, ArrayRef::from(vec)))
}
