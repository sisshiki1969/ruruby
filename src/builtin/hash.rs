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

pub fn init_hash(globals: &mut Globals) -> PackedValue {
    let id = globals.get_ident_id("Hash");
    let class = ClassRef::from(id, globals.object);
    globals.add_builtin_instance_method(class, "clear", hash_clear);
    globals.add_builtin_instance_method(class, "clone", hash_clone);
    globals.add_builtin_instance_method(class, "dup", hash_clone);
    globals.add_builtin_instance_method(class, "compact", hash_compact);
    globals.add_builtin_instance_method(class, "delete", hash_delete);
    globals.add_builtin_instance_method(class, "empty?", hash_empty);
    globals.add_builtin_instance_method(class, "select", hash_select);
    globals.add_builtin_instance_method(class, "has_key?", hash_has_key);
    globals.add_builtin_instance_method(class, "key?", hash_has_key);
    globals.add_builtin_instance_method(class, "include?", hash_has_key);
    globals.add_builtin_instance_method(class, "member?", hash_has_key);
    globals.add_builtin_instance_method(class, "has_value?", hash_has_value);
    globals.add_builtin_instance_method(class, "keys", hash_keys);
    globals.add_builtin_instance_method(class, "length", hash_length);
    globals.add_builtin_instance_method(class, "size", hash_length);
    globals.add_builtin_instance_method(class, "values", hash_values);
    globals.add_builtin_instance_method(class, "each_value", each_value);
    globals.add_builtin_instance_method(class, "each", each);
    PackedValue::class(globals, class)
}

fn hash_clear(_vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let mut hash = args.self_value.as_hash().unwrap();
    hash.map.clear();
    Ok(args.self_value)
}

fn hash_clone(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let hash = args.self_value.as_hash().unwrap();
    Ok(PackedValue::hash(&vm.globals, hash.dup()))
}

fn hash_compact(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let mut hash = args.self_value.as_hash().unwrap().dup();
    hash.map.retain(|_, &mut v| v != PackedValue::nil());
    Ok(PackedValue::hash(&vm.globals, hash))
}

fn hash_delete(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let mut hash = args.self_value.as_hash().unwrap();
    let res = match hash.map.remove(&args[0]) {
        Some(v) => v,
        None => PackedValue::nil(),
    };
    Ok(res)
}

fn hash_empty(_vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let hash = args.self_value.as_hash().unwrap();
    Ok(PackedValue::bool(hash.map.len() == 0))
}

fn hash_select(vm: &mut VM, args: &Args, block: Option<MethodRef>) -> VMResult {
    let hash = args.self_value.as_hash().unwrap();
    let iseq = match block {
        Some(method) => vm.globals.get_method_info(method).as_iseq(&vm)?,
        None => return Err(vm.error_argument("Currently, needs block.")),
    };
    let mut res = HashMap::new();
    let context = vm.context();
    let mut arg = Args::new2(
        args.self_value,
        None,
        PackedValue::nil(),
        PackedValue::nil(),
    );
    for (k, v) in hash.map.iter() {
        arg[0] = *k;
        arg[1] = *v;
        vm.vm_run(iseq, Some(context), &arg, None, None)?;
        let b = vm.stack_pop();
        if vm.val_to_bool(b) {
            res.insert(k.clone(), v.clone());
        };
    }
    Ok(PackedValue::hash(&vm.globals, HashRef::from(res)))
}

fn hash_has_key(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let hash = args.self_value.as_hash().unwrap();
    Ok(PackedValue::bool(hash.map.contains_key(&args[0])))
}

fn hash_has_value(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let hash = args.self_value.as_hash().unwrap();
    let res = hash.map.values().find(|&&x| x == args[0]).is_some();
    Ok(PackedValue::bool(res))
}

fn hash_length(_vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let hash = args.self_value.as_hash().unwrap();
    let len = hash.map.len();
    Ok(PackedValue::fixnum(len as i64))
}

fn hash_keys(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let hash = args.self_value.as_hash().unwrap();
    let mut vec = vec![];
    for key in hash.map.keys() {
        vec.push(key.clone());
    }
    Ok(PackedValue::array_from(&vm.globals, vec))
}

fn hash_values(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let hash = args.self_value.as_hash().unwrap();
    let mut vec = vec![];
    for val in hash.map.values() {
        vec.push(val.clone());
    }
    Ok(PackedValue::array_from(&vm.globals, vec))
}

fn each_value(vm: &mut VM, args: &Args, block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let hash = args.self_value.as_hash().unwrap();
    let iseq = match block {
        Some(method) => vm.globals.get_method_info(method).as_iseq(&vm)?,
        None => return Err(vm.error_argument("Currently, needs block.")),
    };
    let context = vm.context();
    let mut arg = Args::new1(context.self_value, None, PackedValue::nil());
    for (_, v) in &hash.map {
        arg[0] = *v;
        vm.vm_run(iseq, Some(context), &arg, None, None)?;
        vm.stack_pop();
    }
    Ok(args.self_value)
}

fn each(vm: &mut VM, args: &Args, block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let hash = args.self_value.as_hash().unwrap();
    let iseq = match block {
        Some(method) => vm.globals.get_method_info(method).as_iseq(&vm)?,
        None => return Err(vm.error_argument("Currently, needs block.")),
    };
    let context = vm.context();
    let mut arg = Args::new2(
        context.self_value,
        None,
        PackedValue::nil(),
        PackedValue::nil(),
    );
    for (k, v) in &hash.map {
        arg[0] = *k;
        arg[1] = *v;
        vm.vm_run(iseq, Some(context), &arg, None, None)?;
        vm.stack_pop();
    }
    Ok(args.self_value)
}
