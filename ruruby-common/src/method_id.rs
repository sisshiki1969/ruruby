#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct MethodId(std::num::NonZeroU32);

impl std::default::Default for MethodId {
    fn default() -> Self {
        Self::new(1)
    }
}

impl MethodId {
    pub fn new(id: u32) -> Self {
        Self(std::num::NonZeroU32::new(id).unwrap())
    }

    pub const fn new_unchecked(id: u32) -> Self {
        Self(unsafe { std::num::NonZeroU32::new_unchecked(id) })
    }

    pub fn as_usize(&self) -> usize {
        self.0.get() as usize
    }
}

impl Into<u32> for MethodId {
    fn into(self) -> u32 {
        self.0.get()
    }
}

impl From<u32> for MethodId {
    fn from(id: u32) -> Self {
        Self::new(id)
    }
}
