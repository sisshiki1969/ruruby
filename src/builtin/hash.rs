use crate::*;
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::Deref;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HashInfo {
    Map(HashMap<HashKey, Value>),
    IdentMap(HashMap<IdentKey, Value>),
}

#[derive(Debug, Clone, Copy)]
pub struct HashKey(pub Value);

impl Deref for HashKey {
    type Target = Value;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Hash for HashKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self.as_rvalue() {
            None => self.0.hash(state),
            Some(lhs) => match &lhs.kind {
                ObjKind::Integer(lhs) => lhs.hash(state),
                ObjKind::Float(lhs) => (*lhs as u64).hash(state),
                ObjKind::String(lhs) => lhs.hash(state),
                ObjKind::Array(lhs) => lhs.elements.hash(state),
                ObjKind::Range(lhs) => lhs.hash(state),
                ObjKind::Hash(lhs) => {
                    for (key, val) in lhs.iter() {
                        key.hash(state);
                        val.hash(state);
                    }
                }
                ObjKind::Method(lhs) => lhs.inner().hash(state),
                _ => self.0.hash(state),
            },
        }
    }
}

impl PartialEq for HashKey {
    // Object#eql?()
    // This type of equality is used for comparison for keys of Hash.
    fn eq(&self, other: &Self) -> bool {
        match (self.as_rvalue(), other.as_rvalue()) {
            (None, None) => self.0 == other.0,
            (Some(lhs), Some(rhs)) => match (&lhs.kind, &rhs.kind) {
                (ObjKind::Integer(lhs), ObjKind::Integer(rhs)) => *lhs == *rhs,
                (ObjKind::Float(lhs), ObjKind::Float(rhs)) => *lhs == *rhs,
                (ObjKind::String(lhs), ObjKind::String(rhs)) => *lhs == *rhs,
                (ObjKind::Array(lhs), ObjKind::Array(rhs)) => lhs.elements == rhs.elements,
                (ObjKind::Range(lhs), ObjKind::Range(rhs)) => *lhs == *rhs,
                (ObjKind::Hash(lhs), ObjKind::Hash(rhs)) => lhs.inner() == rhs.inner(),
                (ObjKind::Method(lhs), ObjKind::Method(rhs)) => *lhs.inner() == *rhs.inner(),
                _ => lhs.kind == rhs.kind,
            },
            _ => false,
        }
    }
}

impl Eq for HashKey {}

#[derive(Debug, Clone, Copy)]
pub struct IdentKey(pub Value);

impl Deref for IdentKey {
    type Target = Value;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Hash for IdentKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (*self.0).hash(state);
    }
}

impl PartialEq for IdentKey {
    // Object#eql?()
    // This type of equality is used for comparison for keys of Hash.
    fn eq(&self, other: &Self) -> bool {
        *self.0 == *other.0
    }
}
impl Eq for IdentKey {}

use std::collections::hash_map;

pub enum IntoIter {
    Map(hash_map::IntoIter<HashKey, Value>),
    IdentMap(hash_map::IntoIter<IdentKey, Value>),
}

impl IntoIter {
    fn new(hash: HashInfo) -> IntoIter {
        match hash {
            HashInfo::Map(map) => IntoIter::Map(map.into_iter()),
            HashInfo::IdentMap(map) => IntoIter::IdentMap(map.into_iter()),
        }
    }
}

impl Iterator for IntoIter {
    type Item = (Value, Value);
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            IntoIter::Map(map) => match map.next() {
                Some((k, v)) => Some((k.0, v)),
                None => None,
            },
            IntoIter::IdentMap(map) => match map.next() {
                Some((k, v)) => Some((k.0, v)),
                None => None,
            },
        }
    }
}

macro_rules! define_iter {
    ($trait:ident) => {
        pub enum $trait<'a> {
            Map(hash_map::$trait<'a, HashKey, Value>),
            IdentMap(hash_map::$trait<'a, IdentKey, Value>),
        }
    };
}

define_iter!(Iter);
define_iter!(IterMut);

macro_rules! define_iter_new {
    ($ty1: ident, $ty2: ty, $method: ident) => {
        impl<'a> $ty1<'a> {
            fn new(hash: $ty2) -> $ty1 {
                match hash {
                    HashInfo::Map(map) => $ty1::Map(map.$method()),
                    HashInfo::IdentMap(map) => $ty1::IdentMap(map.$method()),
                }
            }
        }
    };
}

define_iter_new!(Iter, &HashInfo, iter);
define_iter_new!(IterMut, &mut HashInfo, iter_mut);

macro_rules! define_iterator {
    ($ty2:ident) => {
        impl<'a> Iterator for $ty2<'a> {
            type Item = (Value, Value);
            fn next(&mut self) -> Option<Self::Item> {
                match self {
                    $ty2::Map(map) => match map.next() {
                        Some((k, v)) => Some((k.0, *v)),
                        None => None,
                    },
                    $ty2::IdentMap(map) => match map.next() {
                        Some((k, v)) => Some((k.0, *v)),
                        None => None,
                    },
                }
            }
        }
    };
}

define_iterator!(Iter);
define_iterator!(IterMut);

macro_rules! define_into_iterator {
    ($ty1:ty, $ty2:ident) => {
        impl<'a> IntoIterator for $ty1 {
            type Item = (Value, Value);
            type IntoIter = $ty2<'a>;
            fn into_iter(self) -> $ty2<'a> {
                $ty2::new(self)
            }
        }
    };
}

define_into_iterator!(&'a HashInfo, Iter);
define_into_iterator!(&'a mut HashInfo, IterMut);

impl IntoIterator for HashInfo {
    type Item = (Value, Value);
    type IntoIter = IntoIter;

    fn into_iter(self) -> IntoIter {
        IntoIter::new(self)
    }
}

impl HashInfo {
    pub fn new(map: HashMap<HashKey, Value>) -> Self {
        HashInfo::Map(map)
    }

    pub fn iter(&self) -> Iter {
        Iter::new(self)
    }

    pub fn iter_mut(&mut self) -> IterMut {
        IterMut::new(self)
    }

    pub fn get(&self, v: &Value) -> Option<&Value> {
        match self {
            HashInfo::Map(map) => map.get(&HashKey(*v)),
            HashInfo::IdentMap(map) => map.get(&IdentKey(*v)),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            HashInfo::Map(map) => map.len(),
            HashInfo::IdentMap(map) => map.len(),
        }
    }

    pub fn clear(&mut self) {
        match self {
            HashInfo::Map(map) => map.clear(),
            HashInfo::IdentMap(map) => map.clear(),
        }
    }

    pub fn insert(&mut self, k: Value, v: Value) {
        match self {
            HashInfo::Map(map) => map.insert(HashKey(k), v),
            HashInfo::IdentMap(map) => map.insert(IdentKey(k), v),
        };
    }

    pub fn remove(&mut self, k: Value) -> Option<Value> {
        match self {
            HashInfo::Map(map) => map.remove(&HashKey(k)),
            HashInfo::IdentMap(map) => map.remove(&IdentKey(k)),
        }
    }

    pub fn contains_key(&self, k: Value) -> bool {
        match self {
            HashInfo::Map(map) => map.contains_key(&HashKey(k)),
            HashInfo::IdentMap(map) => map.contains_key(&IdentKey(k)),
        }
    }

    pub fn keys(&self) -> Vec<Value> {
        match self {
            HashInfo::Map(map) => map.keys().map(|x| x.0).collect(),
            HashInfo::IdentMap(map) => map.keys().map(|x| x.0).collect(),
        }
    }

    pub fn values(&self) -> Vec<Value> {
        match self {
            HashInfo::Map(map) => map.values().cloned().collect(),
            HashInfo::IdentMap(map) => map.values().cloned().collect(),
        }
    }
}

pub type HashRef = Ref<HashInfo>;

impl HashRef {
    pub fn from(map: HashMap<HashKey, Value>) -> Self {
        HashRef::new(HashInfo::new(map))
    }
}

pub fn init_hash(globals: &mut Globals) -> Value {
    let id = globals.get_ident_id("Hash");
    let class = ClassRef::from(id, globals.builtins.object);
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
    globals.add_builtin_instance_method(class, "sort", sort);
    Value::class(globals, class)
}

macro_rules! as_hash {
    ($arg:expr, $vm:ident) => {
        $arg.as_hash()
            .ok_or($vm.error_type("Receiver must be a hash."))?;
    };
}

fn hash_clear(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let mut hash = as_hash!(self_val, vm);
    hash.clear();
    Ok(self_val)
}

fn hash_clone(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let hash = as_hash!(self_val, vm);
    Ok(Value::hash(&vm.globals, hash.dup()))
}

fn hash_compact(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let hash = as_hash!(self_val, vm).dup();
    match hash.inner_mut() {
        HashInfo::Map(map) => map.retain(|_, &mut v| v != Value::nil()),
        HashInfo::IdentMap(map) => map.retain(|_, &mut v| v != Value::nil()),
    }
    Ok(Value::hash(&vm.globals, hash))
}

fn hash_delete(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let mut hash = as_hash!(self_val, vm);
    let res = match hash.remove(args[0]) {
        Some(v) => v,
        None => Value::nil(),
    };
    Ok(res)
}

fn hash_empty(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let hash = as_hash!(self_val, vm);
    Ok(Value::bool(hash.len() == 0))
}

fn hash_select(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let hash = as_hash!(self_val, vm);
    let method = vm.expect_block(args.block)?;
    let mut res = HashMap::new();
    let mut arg = Args::new2(Value::nil(), Value::nil());
    for (k, v) in hash.iter() {
        arg[0] = k;
        arg[1] = v;
        let b = vm.eval_block(method, &arg)?;
        if vm.val_to_bool(b) {
            res.insert(HashKey(k), v);
        };
    }

    Ok(Value::hash(&vm.globals, HashRef::from(res)))
}

fn hash_has_key(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let hash = as_hash!(self_val, vm);
    let res = hash.contains_key(args[0]);
    Ok(Value::bool(res))
}

fn hash_has_value(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let hash = as_hash!(self_val, vm);
    let res = hash.iter().find(|(_, v)| *v == args[0]).is_some();
    Ok(Value::bool(res))
}

fn hash_length(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let hash = as_hash!(self_val, vm);
    let len = hash.len();
    Ok(Value::fixnum(len as i64))
}

fn hash_keys(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let hash = as_hash!(self_val, vm);
    Ok(Value::array_from(&vm.globals, hash.keys()))
}

fn hash_values(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let hash = as_hash!(self_val, vm);
    Ok(Value::array_from(&vm.globals, hash.values()))
}

fn each_value(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let hash = as_hash!(self_val, vm);
    let method = vm.expect_block(args.block)?;
    let mut arg = Args::new1(Value::nil());
    for (_, v) in hash.iter() {
        arg[0] = v;
        vm.eval_block(method, &arg)?;
    }

    Ok(self_val)
}

fn each_key(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let hash = as_hash!(self_val, vm);
    let method = vm.expect_block(args.block)?;
    let mut arg = Args::new1(Value::nil());

    for (k, _) in hash.iter() {
        arg[0] = k;
        vm.eval_block(method, &arg)?;
    }

    Ok(self_val)
}

fn each(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let hash = as_hash!(self_val, vm);
    let method = vm.expect_block(args.block)?;
    let mut arg = Args::new2(Value::nil(), Value::nil());

    for (k, v) in hash.iter() {
        arg[0] = k;
        arg[1] = v;
        vm.eval_block(method, &arg)?;
    }

    Ok(self_val)
}

fn merge(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let mut new = as_hash!(self_val, vm).dup();
    for arg in args.iter() {
        let other = as_hash!(arg, vm);
        for (k, v) in other.iter() {
            new.insert(k, v);
        }
    }

    Ok(Value::hash(&vm.globals, new))
}

fn fetch(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 2)?;
    let key = args[0];
    let default = if args.len() == 2 {
        args[1]
    } else {
        Value::nil()
    };
    let hash = as_hash!(self_val, vm);
    let val = match hash.get(&key) {
        Some(val) => val.clone(),
        None => default,
    };

    Ok(val)
}

fn compare_by_identity(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let hash = as_hash!(self_val, vm);
    let inner = hash.inner_mut();
    match inner {
        HashInfo::Map(map) => {
            let new_map = map.into_iter().map(|(k, v)| (IdentKey(k.0), *v)).collect();
            *inner = HashInfo::IdentMap(new_map);
        }
        HashInfo::IdentMap(_) => {}
    };
    Ok(self_val)
}

fn sort(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let hash = as_hash!(self_val, vm);
    let mut vec = vec![];
    for (k, v) in hash.iter() {
        let ary = vec![k, v];
        vec.push(Value::array_from(&vm.globals, ary));
    }
    let aref = ArrayRef::from(vec);
    vm.sort_array(aref)?;
    Ok(Value::array(&vm.globals, aref))
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
        assert_script(program);
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
        assert_script(program);
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
        assert_script(program);
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
        assert_script(program);
    }

    #[test]
    fn hash_compare_by_identity() {
        let program = r#"
        a = "a"
        h1 = {}
        h1[a] = 100
        assert 100, h1["a"]
        assert 100, h1[a]
        h2 = {}
        h2.compare_by_identity
        h2[a] = 100
        assert nil, h2["a"]
        assert 100, h2[a]
    "#;
        assert_script(program);
    }
}
