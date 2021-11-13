mod node;
pub mod parser;
mod token;
use node::*;
use token::*;
pub mod codegen;
use ruruby_common::*;

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
    fn new(kind: T, loc: Loc) -> Self {
        Annot { kind, loc }
    }

    fn loc(&self) -> Loc {
        self.loc
    }
}
