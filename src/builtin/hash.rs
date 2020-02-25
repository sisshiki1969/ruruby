use crate::vm::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum HashInfo {
    Map(HashMap<Value, Value>),
    IdentMap(HashMap<IdentValue, Value>),
}

impl HashInfo {
    pub fn new(map: HashMap<Value, Value>) -> Self {
        HashInfo::Map(map)
    }

    pub fn get(&self, v: &Value) -> Option<&Value> {
        match self {
            HashInfo::Map(map) => map.get(v),
            HashInfo::IdentMap(map) => map.get(&IdentValue(*v)),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            HashInfo::Map(map) => map.len(),
            HashInfo::IdentMap(map) => map.len(),
        }
    }

    pub fn insert(&mut self, k: Value, v: Value) {
        match self {
            HashInfo::Map(map) => map.insert(k, v),
            HashInfo::IdentMap(map) => map.insert(IdentValue(k), v),
        };
    }
}

pub type HashRef = Ref<HashInfo>;

impl HashRef {
    pub fn from(map: HashMap<Value, Value>) -> Self {
        HashRef::new(HashInfo::new(map))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct IdentValue(pub Value);

impl std::ops::Deref for IdentValue {
    type Target = Value;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::hash::Hash for IdentValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (*self.0).hash(state);
    }
}

impl PartialEq for IdentValue {
    // Object#eql?()
    // This type of equality is used for comparison for keys of Hash.
    // Regexp, Array, Hash must be implemented.
    fn eq(&self, other: &Self) -> bool {
        *self.0 == *other.0
    }
}
impl Eq for IdentValue {}

pub fn init_hash(globals: &mut Globals) -> Value {
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
    globals.add_builtin_instance_method(class, "each_key", each_key);
    globals.add_builtin_instance_method(class, "each", each);
    globals.add_builtin_instance_method(class, "merge", merge);
    globals.add_builtin_instance_method(class, "fetch", fetch);
    globals.add_builtin_instance_method(class, "compare_by_identity", compare_by_identity);
    Value::class(globals, class)
}

macro_rules! as_hash {
    ($arg:expr, $vm:ident) => {
        $arg.as_hash()
            .ok_or($vm.error_type("Receiver must be a hash."))?;
    };
}

fn hash_clear(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let hash = as_hash!(args.self_value, vm);
    match hash.inner_mut() {
        HashInfo::Map(map) => map.clear(),
        HashInfo::IdentMap(map) => map.clear(),
    }

    Ok(args.self_value)
}

fn hash_clone(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let hash = as_hash!(args.self_value, vm);
    Ok(Value::hash(&vm.globals, hash.dup()))
}

fn hash_compact(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let hash = as_hash!(args.self_value, vm).dup();
    match hash.inner_mut() {
        HashInfo::Map(map) => map.retain(|_, &mut v| v != Value::nil()),
        HashInfo::IdentMap(map) => map.retain(|_, &mut v| v != Value::nil()),
    }
    Ok(Value::hash(&vm.globals, hash))
}

fn hash_delete(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let hash = as_hash!(args.self_value, vm);
    let res = match hash.inner_mut() {
        HashInfo::Map(map) => match map.remove(&args[0]) {
            Some(v) => v,
            None => Value::nil(),
        },
        HashInfo::IdentMap(map) => match map.remove(&IdentValue(args[0])) {
            Some(v) => v,
            None => Value::nil(),
        },
    };
    Ok(res)
}

fn hash_empty(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let hash = as_hash!(args.self_value, vm);
    Ok(Value::bool(hash.len() == 0))
}

fn hash_select(vm: &mut VM, args: &Args, block: Option<MethodRef>) -> VMResult {
    let hash = as_hash!(args.self_value, vm);
    let iseq = match block {
        Some(method) => vm.globals.get_method_info(method).as_iseq(&vm)?,
        None => return Err(vm.error_argument("Currently, needs block.")),
    };
    let mut res = HashMap::new();
    let context = vm.context();
    let mut arg = Args::new2(args.self_value, None, Value::nil(), Value::nil());
    match hash.inner() {
        HashInfo::Map(map) => {
            for (k, v) in map {
                arg[0] = *k;
                arg[1] = *v;
                vm.vm_run(iseq, Some(context), &arg, None, None)?;
                let b = vm.stack_pop();
                if vm.val_to_bool(b) {
                    res.insert(k.clone(), v.clone());
                };
            }
        }
        HashInfo::IdentMap(map) => {
            for (k, v) in map.iter() {
                arg[0] = k.0;
                arg[1] = *v;
                vm.vm_run(iseq, Some(context), &arg, None, None)?;
                let b = vm.stack_pop();
                if vm.val_to_bool(b) {
                    res.insert(k.0, v.clone());
                };
            }
        }
    }

    Ok(Value::hash(&vm.globals, HashRef::from(res)))
}

fn hash_has_key(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let hash = as_hash!(args.self_value, vm);
    let res = match hash.inner() {
        HashInfo::Map(map) => map.contains_key(&args[0]),
        HashInfo::IdentMap(map) => map.contains_key(&IdentValue(args[0])),
    };
    Ok(Value::bool(res))
}

fn hash_has_value(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let hash = as_hash!(args.self_value, vm);
    let res = match hash.inner() {
        HashInfo::Map(map) => map.values().find(|&&x| x == args[0]).is_some(),
        HashInfo::IdentMap(map) => map.values().find(|&&x| x == args[0]).is_some(),
    };
    Ok(Value::bool(res))
}

fn hash_length(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let hash = as_hash!(args.self_value, vm);
    let len = hash.len();
    Ok(Value::fixnum(len as i64))
}

fn hash_keys(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let hash = as_hash!(args.self_value, vm);
    let mut vec = vec![];
    match hash.inner() {
        HashInfo::Map(map) => {
            for key in map.keys() {
                vec.push(key.clone());
            }
        }
        HashInfo::IdentMap(map) => {
            for key in map.keys() {
                vec.push(key.0.clone());
            }
        }
    };
    Ok(Value::array_from(&vm.globals, vec))
}

fn hash_values(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let hash = as_hash!(args.self_value, vm);
    let mut vec = vec![];
    match hash.inner() {
        HashInfo::Map(map) => {
            for val in map.values() {
                vec.push(val.clone());
            }
        }
        HashInfo::IdentMap(map) => {
            for val in map.values() {
                vec.push(val.clone());
            }
        }
    };

    Ok(Value::array_from(&vm.globals, vec))
}

fn each_value(vm: &mut VM, args: &Args, block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let hash = as_hash!(args.self_value, vm);
    let iseq = match block {
        Some(method) => vm.globals.get_method_info(method).as_iseq(&vm)?,
        None => return Err(vm.error_argument("Currently, needs block.")),
    };
    let context = vm.context();
    let mut arg = Args::new1(context.self_value, None, Value::nil());
    match hash.inner() {
        HashInfo::Map(map) => {
            for (_, v) in map {
                arg[0] = *v;
                vm.vm_run(iseq, Some(context), &arg, None, None)?;
                vm.stack_pop();
            }
        }
        HashInfo::IdentMap(map) => {
            for (_, v) in map {
                arg[0] = *v;
                vm.vm_run(iseq, Some(context), &arg, None, None)?;
                vm.stack_pop();
            }
        }
    };

    Ok(args.self_value)
}

fn each_key(vm: &mut VM, args: &Args, block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let hash = as_hash!(args.self_value, vm);
    let iseq = match block {
        Some(method) => vm.globals.get_method_info(method).as_iseq(&vm)?,
        None => return Err(vm.error_argument("Currently, needs block.")),
    };
    let context = vm.context();
    let mut arg = Args::new1(context.self_value, None, Value::nil());
    match hash.inner() {
        HashInfo::Map(map) => {
            for (k, _v) in map {
                arg[0] = *k;
                vm.vm_run(iseq, Some(context), &arg, None, None)?;
                vm.stack_pop();
            }
        }
        HashInfo::IdentMap(map) => {
            for (k, _v) in map {
                arg[0] = **k;
                vm.vm_run(iseq, Some(context), &arg, None, None)?;
                vm.stack_pop();
            }
        }
    };

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
    let mut arg = Args::new2(context.self_value, None, Value::nil(), Value::nil());
    match hash.inner() {
        HashInfo::Map(map) => {
            for (k, v) in map {
                arg[0] = *k;
                arg[1] = *v;
                vm.vm_run(iseq, Some(context), &arg, None, None)?;
                vm.stack_pop();
            }
        }
        HashInfo::IdentMap(map) => {
            for (k, v) in map {
                arg[0] = k.0;
                arg[1] = *v;
                vm.vm_run(iseq, Some(context), &arg, None, None)?;
                vm.stack_pop();
            }
        }
    };

    Ok(args.self_value)
}

fn merge(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let new = as_hash!(args.self_value, vm).dup();
    match new.inner_mut() {
        HashInfo::Map(new) => {
            for i in 0..args.len() {
                let other = as_hash!(args[i], vm);
                match other.inner() {
                    HashInfo::Map(other) => {
                        for (k, v) in other {
                            new.insert(*k, *v);
                        }
                    }
                    HashInfo::IdentMap(other) => {
                        for (k, v) in other {
                            new.insert(k.0, *v);
                        }
                    }
                }
            }
        }
        HashInfo::IdentMap(new) => {
            for i in 0..args.len() {
                let other = as_hash!(args[i], vm);
                match other.inner_mut() {
                    HashInfo::Map(other) => {
                        for (k, v) in other {
                            new.insert(IdentValue(*k), *v);
                        }
                    }
                    HashInfo::IdentMap(other) => {
                        for (k, v) in other {
                            new.insert(*k, *v);
                        }
                    }
                }
            }
        }
    };

    Ok(Value::hash(&vm.globals, new))
}

fn fetch(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 1, 2)?;
    let key = args[0];
    let default = if args.len() == 2 {
        args[1]
    } else {
        Value::nil()
    };
    let hash = as_hash!(args.self_value, vm);
    let val = match hash.get(&key) {
        Some(val) => val.clone(),
        None => default,
    };

    Ok(val)
}

fn compare_by_identity(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let hash = as_hash!(args.self_value, vm);
    let inner = hash.inner_mut();
    match inner {
        HashInfo::Map(map) => {
            let mut new_map = HashMap::new();
            for (k, v) in map {
                new_map.insert(IdentValue(*k), *v);
            }
            *inner = HashInfo::IdentMap(new_map);
        }
        HashInfo::IdentMap(_) => {}
    };
    Ok(args.self_value)
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
        let expected = RValue::Nil;
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
        let expected = RValue::Nil;
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
        let expected = RValue::Nil;
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
        let expected = RValue::Nil;
        eval_script(program, expected);
    }
}
