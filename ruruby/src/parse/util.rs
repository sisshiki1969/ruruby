use crate::*;

#[derive(Debug, Clone)]
pub struct Annot<T> {
    pub kind: T,
    pub loc: Loc,
}

impl<T: PartialEq> std::cmp::PartialEq for Annot<T> {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.loc == other.loc
    }
}

impl<T> Annot<T> {
    pub(crate) fn new(kind: T, loc: Loc) -> Self {
        Annot { kind, loc }
    }

    pub(crate) fn loc(&self) -> Loc {
        self.loc
    }
}
