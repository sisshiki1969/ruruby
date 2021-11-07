#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ISeqPos(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ISeqDisp(i32);

impl ISeqDisp {
    pub fn from_i32(disp: i32) -> Self {
        Self(disp)
    }

    pub fn to_i32(self) -> i32 {
        self.0
    }
}

impl std::fmt::Debug for ISeqPos {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("ISeqPos({})", self.0))
    }
}

impl From<ISeqPos> for usize {
    fn from(pos: ISeqPos) -> usize {
        pos.0
    }
}

impl std::ops::Add<ISeqDisp> for ISeqPos {
    type Output = Self;
    fn add(self, other: ISeqDisp) -> Self {
        Self(((self.0) as i64 + other.0 as i64) as usize)
    }
}

impl std::ops::AddAssign<ISeqDisp> for ISeqPos {
    fn add_assign(&mut self, other: ISeqDisp) {
        *self = *self + other
    }
}

impl std::ops::Add<usize> for ISeqPos {
    type Output = Self;
    fn add(self, other: usize) -> Self {
        Self(((self.0) as i64 + other as i64) as usize)
    }
}

impl std::ops::AddAssign<usize> for ISeqPos {
    fn add_assign(&mut self, other: usize) {
        *self = *self + other
    }
}

impl std::ops::Sub<usize> for ISeqPos {
    type Output = Self;
    fn sub(self, other: usize) -> Self {
        Self(((self.0) as i64 - other as i64) as usize)
    }
}

impl std::ops::SubAssign<usize> for ISeqPos {
    fn sub_assign(&mut self, other: usize) {
        *self = *self - other
    }
}

impl std::ops::Sub<ISeqPos> for ISeqPos {
    type Output = ISeqDisp;
    fn sub(self, other: ISeqPos) -> Self::Output {
        ISeqDisp((other.0 as i64 - self.0 as i64) as i32)
    }
}

impl ISeqPos {
    pub fn from(pos: usize) -> Self {
        ISeqPos(pos)
    }

    pub fn into_usize(self) -> usize {
        self.0
    }
}
