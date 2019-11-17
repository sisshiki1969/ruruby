use crate::vm::*;

#[derive(Debug, Clone, PartialEq)]
pub struct RangeInfo {
    pub start: PackedValue,
    pub end: PackedValue,
    pub exclude: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeRef(*mut RangeInfo);

impl std::hash::Hash for RangeRef {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl RangeRef {
    pub fn new(start: PackedValue, end: PackedValue, exclude: bool) -> Self {
        let info = RangeInfo {
            start,
            end,
            exclude,
        };
        let boxed = Box::into_raw(Box::new(info));
        RangeRef(boxed)
    }
}

impl std::ops::Deref for RangeRef {
    type Target = RangeInfo;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl std::ops::DerefMut for RangeRef {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0 }
    }
}
