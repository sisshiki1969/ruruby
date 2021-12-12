#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FnId(std::num::NonZeroU32);

impl std::default::Default for FnId {
    #[inline(always)]
    fn default() -> Self {
        Self::new(1)
    }
}

impl FnId {
    #[inline(always)]
    pub fn new(id: u32) -> Self {
        Self(std::num::NonZeroU32::new(id).unwrap())
    }

    #[inline(always)]
    pub const fn new_unchecked(id: u32) -> Self {
        Self(unsafe { std::num::NonZeroU32::new_unchecked(id) })
    }

    #[inline(always)]
    pub fn as_usize(&self) -> usize {
        self.0.get() as usize
    }
}

impl From<FnId> for u32 {
    #[inline(always)]
    fn from(id: FnId) -> u32 {
        id.0.get()
    }
}

impl From<u32> for FnId {
    #[inline(always)]
    fn from(id: u32) -> Self {
        Self::new(id)
    }
}
