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
