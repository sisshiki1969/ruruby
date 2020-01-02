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
