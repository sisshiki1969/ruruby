///
/// Wrapper of ID for local variables.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LvarId(usize);

impl LvarId {
    pub fn as_usize(&self) -> usize {
        self.0
    }

    pub fn as_u32(&self) -> u32 {
        self.0 as u32
    }
}

impl From<usize> for LvarId {
    fn from(id: usize) -> Self {
        LvarId(id)
    }
}

impl Into<usize> for LvarId {
    fn into(self) -> usize {
        self.0
    }
}

impl From<u32> for LvarId {
    fn from(id: u32) -> Self {
        LvarId(id as usize)
    }
}
