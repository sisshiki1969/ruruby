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
    globals.add_builtin_instance_method(class, "merge", merge);
    PackedValue::class(globals, class)
}

macro_rules! as_hash {
    ($arg:expr, $vm:ident) => {
        $arg.as_hash()
            .ok_or($vm.error_type("Receiver must be a hash."))?;
    };
}

fn hash_clear(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let mut hash = as_hash!(args.self_value, vm);
    hash.map.clear();
    Ok(args.self_value)
}

fn hash_clone(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let hash = as_hash!(args.self_value, vm);
    Ok(PackedValue::hash(&vm.globals, hash.dup()))
}

fn hash_compact(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let mut hash = as_hash!(args.self_value, vm).dup();
    hash.map.retain(|_, &mut v| v != PackedValue::nil());
    Ok(PackedValue::hash(&vm.globals, hash))
}

fn hash_delete(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let mut hash = as_hash!(args.self_value, vm);
    let res = match hash.map.remove(&args[0]) {
        Some(v) => v,
        None => PackedValue::nil(),
    };
    Ok(res)
}

fn hash_empty(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let hash = as_hash!(args.self_value, vm);
    Ok(PackedValue::bool(hash.map.len() == 0))
}

fn hash_select(vm: &mut VM, args: &Args, block: Option<MethodRef>) -> VMResult {
    let hash = as_hash!(args.self_value, vm);
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
    let hash = as_hash!(args.self_value, vm);
    Ok(PackedValue::bool(hash.map.contains_key(&args[0])))
}

fn hash_has_value(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let hash = as_hash!(args.self_value, vm);
    let res = hash.map.values().find(|&&x| x == args[0]).is_some();
    Ok(PackedValue::bool(res))
}

fn hash_length(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let hash = as_hash!(args.self_value, vm);
    let len = hash.map.len();
    Ok(PackedValue::fixnum(len as i64))
}

fn hash_keys(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let hash = as_hash!(args.self_value, vm);
    let mut vec = vec![];
    for key in hash.map.keys() {
        vec.push(key.clone());
    }
    Ok(PackedValue::array_from(&vm.globals, vec))
}

fn hash_values(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let hash = as_hash!(args.self_value, vm);
    let mut vec = vec![];
    for val in hash.map.values() {
        vec.push(val.clone());
    }
    Ok(PackedValue::array_from(&vm.globals, vec))
}

fn each_value(vm: &mut VM, args: &Args, block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let hash = as_hash!(args.self_value, vm);
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
    let hash = as_hash!(args.self_value, vm);
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

fn merge(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let mut new = as_hash!(args.self_value, vm).dup();
    for i in 0..args.len() {
        let other = as_hash!(args[i], vm);
        for (k, v) in other.map.iter() {
            new.map.insert(*k, *v);
        }
    }
    Ok(PackedValue::hash(&vm.globals, new))
}

#[cfg(test)]
#[allow(unused_imports, dead_code)]
mod test {
    use crate::test::*;

    #[test]
    fn hash1() {
        let program = r#"
    h = {true => "true", false => "false", nil => "nil", 100 => "100", 7.7 => "7.7", "ruby" => "string", :ruby => "symbol"}
    assert(h[true], "true")
    assert(h[false], "false")
    assert(h[nil], "nil")
    assert(h[100], "100")
    assert(h[7.7], "7.7")
    assert(h["ruby"], "string")
    assert(h[:ruby], "symbol")
    "#;
        let expected = Value::Nil;
        eval_script(program, expected);
    }

    #[test]
    fn hash2() {
        let program = r#"
    h = {true: "true", false: "false", nil: "nil", 100 => "100", 7.7 => "7.7", ruby: "string"}
    assert(h[:true], "true")
    assert(h[:false], "false")
    assert(h[:nil], "nil")
    assert(h[100], "100")
    assert(h[7.7], "7.7")
    assert(h[:ruby], "string")
    "#;
        let expected = Value::Nil;
        eval_script(program, expected);
    }

    #[test]
    fn hash3() {
        let program = r#"
    h1 = {a: "symbol", c:nil, d:nil}
    assert(h1.has_key?(:a), true)
    assert(h1.has_key?(:b), false)
    assert(h1.has_value?("symbol"), true)
    assert(h1.has_value?(500), false)
    assert(h1.length, 3)
    assert(h1.size, 3)
    #assert(h1.keys, [:a, :d, :c])
    #assert(h1.values, ["symbol", nil, nil])
    h2 = h1.clone()
    h2[:b] = 100
    assert(h2[:b], 100)
    assert(h1[:b], nil)
    h3 = h2.compact
    assert(h3.delete(:a), "symbol")
    assert(h3.empty?, false)
    assert(h3.delete(:b), 100)
    assert(h3.delete(:c), nil)
    assert(h3.empty?, true)
    h2.clear()
    assert(h2.empty?, true)
    "#;
        let expected = Value::Nil;
        eval_script(program, expected);
    }

    #[test]
    fn hash_merge() {
        let program = r#"
        h1 = { "a" => 100, "b" => 200 }
        h2 = { "b" => 246, "c" => 300 }
        h3 = { "b" => 357, "d" => 400 }
        assert({"a"=>100, "b"=>200}, h1.merge)
        assert({"a"=>100, "b"=>246, "c"=>300}, h1.merge(h2)) 
        assert({"a"=>100, "b"=>357, "c"=>300, "d"=>400}, h1.merge(h2, h3)) 
        assert({"a"=>100, "b"=>200}, h1)
    "#;
        let expected = Value::Nil;
        eval_script(program, expected);
    }
}
