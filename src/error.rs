use crate::util::{Annot, Loc};

#[derive(Debug, Clone, PartialEq)]
pub enum RubyErrorKind {
    ParseErr(ParseErrKind),
    RuntimeErr(RuntimeErrKind),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseErrKind {
    UnexpectedEOF,
    UnexpectedToken,
    SyntaxError(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeErrKind {
    Unimplemented(String),
    Internal(String),
    Name(String),
    NoMethod(String),
    Argument(String),
    Type(String),
}

pub type RubyError = Annot<RubyErrorKind>;

impl RubyError {
    pub fn new_runtime_err(err: RuntimeErrKind, loc: Loc) -> Self {
        let kind = RubyErrorKind::RuntimeErr(err);
        Annot::new(kind, loc)
    }
    pub fn new_parse_err(err: ParseErrKind, loc: Loc) -> Self {
        let kind = RubyErrorKind::ParseErr(err);
        Annot::new(kind, loc)
    }
}
