use crate::vm::*;

#[derive(Debug, Clone)]
pub struct ArrayInfo {
    pub elements: Vec<PackedValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArrayRef(*mut ArrayInfo);

impl std::hash::Hash for ArrayRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl ArrayRef {
    pub fn new(elements: Vec<PackedValue>) -> Self {
        let info = ArrayInfo { elements };
        let boxed = Box::into_raw(Box::new(info));
        ArrayRef(boxed)
    }
}

impl std::ops::Deref for ArrayRef {
    type Target = ArrayInfo;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl std::ops::DerefMut for ArrayRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0 }
    }
}
