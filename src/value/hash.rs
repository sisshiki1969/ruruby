use crate::*;
use std::hash::Hash;
use std::ops::Deref;

#[derive(Debug, Clone)]
pub enum HashInfo {
    Map(FxIndexMap<HashKey, Value>),
    IdentMap(FxIndexMap<IdentKey, Value>),
}

impl PartialEq for HashInfo {
    // This type of equality is used for comparison for keys of Hash.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (HashInfo::Map(map1), HashInfo::Map(map2)) => map1 == map2,
            (HashInfo::IdentMap(map1), HashInfo::IdentMap(map2)) => {
                if map1.len() != map2.len() {
                    return false;
                };
                let mut m1 = FxHashMap::default();
                map1.iter().for_each(|(k, v)| {
                    m1.insert(HashKey(k.0), *v);
                });
                map2.iter()
                    .all(|(k, v)| m1.get(&HashKey(k.0)).map_or(false, |v2| *v == *v2))
                /*
                let mut m1 = FxHashMap::default();
                for (k, v) in map1 {
                    let a = m1.get_mut(&(k.0, *v));
                    match a {
                        Some(c) => *c += 1,
                        None => {
                            m1.insert((k.0, *v), 1usize);
                        }
                    };
                }
                let mut m2 = FxHashMap::default();
                for (k, v) in map2 {
                    let a = m2.get_mut(&(k.0, *v));
                    match a {
                        Some(c) => *c += 1,
                        None => {
                            m2.insert((k.0, *v), 1usize);
                        }
                    };
                }
                m1 == m2*/
            }
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq)]
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
                ObjKind::Invalid => panic!("Invalid rvalue. (maybe GC problem) {:?}", lhs),
                ObjKind::BigNum(lhs) => lhs.hash(state),
                ObjKind::Float(lhs) => lhs.to_bits().hash(state),
                ObjKind::String(lhs) => lhs.hash(state),
                ObjKind::Array(lhs) => lhs.elements.hash(state),
                ObjKind::Range(lhs) => lhs.hash(state),
                ObjKind::Hash(lhs) => {
                    for (key, val) in lhs.iter() {
                        key.hash(state);
                        val.hash(state);
                    }
                }
                ObjKind::Method(lhs) => (*lhs).hash(state),
                _ => self.0.hash(state),
            },
        }
    }
}

impl PartialEq for HashKey {
    // Object#eql?()
    // This type of equality is used for comparison for keys of Hash.
    fn eq(&self, other: &Self) -> bool {
        if self.0.id() == other.0.id() {
            return true;
        }
        match (self.as_rvalue(), other.as_rvalue()) {
            (None, None) => self.0.id() == other.0.id(),
            (Some(lhs), Some(rhs)) => lhs.eql(rhs),
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq)]
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
        self.0.id() == other.0.id()
    }
}

pub enum IntoIter {
    Map(indexmap::map::IntoIter<HashKey, Value>),
    IdentMap(indexmap::map::IntoIter<IdentKey, Value>),
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
            IntoIter::Map(map) => map.next().map(|(k, v)| (k.0, v)),
            IntoIter::IdentMap(map) => map.next().map(|(k, v)| (k.0, v)),
        }
    }
}

macro_rules! define_iter {
    ($trait:ident) => {
        pub enum $trait<'a> {
            Map(indexmap::map::$trait<'a, HashKey, Value>),
            IdentMap(indexmap::map::$trait<'a, IdentKey, Value>),
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
                    $ty2::Map(map) => map.next().map(|(k, v)| (k.0, *v)),
                    $ty2::IdentMap(map) => map.next().map(|(k, v)| (k.0, *v)),
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

impl GC for HashInfo {
    fn mark(&self, alloc: &mut Allocator) {
        for (k, v) in self.iter() {
            k.mark(alloc);
            v.mark(alloc);
        }
    }
}

impl HashInfo {
    pub(crate) fn new(map: FxIndexMap<HashKey, Value>) -> Self {
        HashInfo::Map(map)
    }

    pub(crate) fn iter(&self) -> Iter {
        Iter::new(self)
    }

    /*pub(crate) fn iter_mut(&mut self) -> IterMut {
        IterMut::new(self)
    }*/

    pub(crate) fn get(&self, v: &Value) -> Option<&Value> {
        match self {
            HashInfo::Map(map) => map.get(&HashKey(*v)),
            HashInfo::IdentMap(map) => map.get(&IdentKey(*v)),
        }
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            HashInfo::Map(map) => map.len(),
            HashInfo::IdentMap(map) => map.len(),
        }
    }

    pub(crate) fn clear(&mut self) {
        match self {
            HashInfo::Map(map) => map.clear(),
            HashInfo::IdentMap(map) => map.clear(),
        }
    }

    pub(crate) fn insert(&mut self, k: Value, v: Value) {
        match self {
            HashInfo::Map(map) => map.insert(HashKey(k), v),
            HashInfo::IdentMap(map) => map.insert(IdentKey(k), v),
        };
    }

    pub(crate) fn remove(&mut self, k: Value) -> Option<Value> {
        match self {
            HashInfo::Map(map) => map.remove(&HashKey(k)),
            HashInfo::IdentMap(map) => map.remove(&IdentKey(k)),
        }
    }

    pub(crate) fn contains_key(&self, k: Value) -> bool {
        match self {
            HashInfo::Map(map) => map.contains_key(&HashKey(k)),
            HashInfo::IdentMap(map) => map.contains_key(&IdentKey(k)),
        }
    }

    pub(crate) fn keys(&self) -> Vec<Value> {
        match self {
            HashInfo::Map(map) => map.keys().map(|x| x.0).collect(),
            HashInfo::IdentMap(map) => map.keys().map(|x| x.0).collect(),
        }
    }

    pub(crate) fn values(&self) -> Vec<Value> {
        match self {
            HashInfo::Map(map) => map.values().cloned().collect(),
            HashInfo::IdentMap(map) => map.values().cloned().collect(),
        }
    }

    pub(crate) fn to_s(&self, vm: &mut VM) -> Result<String, RubyError> {
        let s = match self.len() {
            0 => "{}".to_string(),
            _ => {
                let mut result = "".to_string();
                let mut first = true;
                for (k, v) in self.iter() {
                    let k_inspect = vm.val_inspect(k)?;
                    let v_inspect = vm.val_inspect(v)?;
                    result = if first {
                        format!("{}=>{}", k_inspect, v_inspect)
                    } else {
                        format!("{}, {}=>{}", result, k_inspect, v_inspect)
                    };
                    first = false;
                }
                format! {"{{{}}}", result}
            }
        };
        Ok(s)
    }
}
