use crate::*;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContextKind {
    Method(Option<IdentId>),
    Class(IdentId),
    Block,
    Eval,
}

impl ContextKind {
    pub fn is_method(&self) -> bool {
        matches!(self, Self::Method(_))
    }
}

#[derive(Clone, PartialEq)]
pub struct ExceptionEntry {
    pub ty: ExceptionType,
    /// start position in ISeq.
    pub start: ISeqPos,
    /// end position in ISeq.
    pub end: ISeqPos,
    pub dest: ISeqPos,
}

/// Type of each exception.
#[derive(Debug, Clone, PartialEq)]
pub enum ExceptionType {
    /// When raised, exec stack is cleared.
    Rescue,
    /// When raised, exec stack does not change.
    Continue,
}

impl fmt::Debug for ExceptionEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!(
            "ExceptionEntry {:?} ({:?}, {:?}) => {:?}",
            self.ty, self.start, self.end, self.dest,
        ))
    }
}

impl ExceptionEntry {
    pub fn new_rescue(start: ISeqPos, end: ISeqPos, dest: ISeqPos) -> Self {
        Self {
            ty: ExceptionType::Rescue,
            start,
            end,
            dest,
        }
    }

    pub fn new_continue(start: ISeqPos, end: ISeqPos, dest: ISeqPos) -> Self {
        Self {
            ty: ExceptionType::Continue,
            start,
            end,
            dest,
        }
    }

    pub fn include(&self, pc: ISeqPos) -> bool {
        self.start < pc && pc <= self.end
    }
}
