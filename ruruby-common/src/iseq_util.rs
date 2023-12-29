use std::ops::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, PartialOrd)]
pub struct ISeqPos(pub usize);

impl From<ISeqPos> for usize {
    #[inline(always)]
    fn from(pos: ISeqPos) -> usize {
        pos.0
    }
}

impl Add<ISeqDisp> for ISeqPos {
    type Output = Self;
    #[inline(always)]
    fn add(self, other: ISeqDisp) -> Self {
        Self(((self.0) as i64 + other.0 as i64) as usize)
    }
}

impl AddAssign<ISeqDisp> for ISeqPos {
    #[inline(always)]
    fn add_assign(&mut self, other: ISeqDisp) {
        *self = *self + other
    }
}

impl Add<usize> for ISeqPos {
    type Output = Self;
    #[inline(always)]
    fn add(self, other: usize) -> Self {
        Self(((self.0) as i64 + other as i64) as usize)
    }
}

impl AddAssign<usize> for ISeqPos {
    #[inline(always)]
    fn add_assign(&mut self, other: usize) {
        *self = *self + other
    }
}

impl Sub<usize> for ISeqPos {
    type Output = Self;
    #[inline(always)]
    fn sub(self, other: usize) -> Self {
        Self(((self.0) as i64 - other as i64) as usize)
    }
}

impl SubAssign<usize> for ISeqPos {
    #[inline(always)]
    fn sub_assign(&mut self, other: usize) {
        *self = *self - other
    }
}

impl Sub<ISeqPos> for ISeqPos {
    type Output = ISeqDisp;
    #[inline(always)]
    fn sub(self, other: ISeqPos) -> Self::Output {
        ISeqDisp((other.0 as i64 - self.0 as i64) as i32)
    }
}

impl ISeqPos {
    #[inline(always)]
    pub fn from(pos: usize) -> Self {
        ISeqPos(pos)
    }

    #[inline(always)]
    pub fn into_usize(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ISeqDisp(i32);

impl ISeqDisp {
    #[inline(always)]
    pub fn from_i32(disp: i32) -> Self {
        Self(disp)
    }

    #[inline(always)]
    pub fn to_i32(self) -> i32 {
        self.0
    }
}
