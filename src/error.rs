use crate::util::{Loc, SourceInfoRef};

#[derive(Debug, Clone, PartialEq)]
pub struct RubyError {
    pub kind: RubyErrorKind,
    source_info: SourceInfoRef,
    level: usize,
    loc: Loc,
}

impl RubyError {
    pub fn new(kind: RubyErrorKind, source_info: SourceInfoRef, level: usize, loc: Loc) -> Self {
        RubyError {
            kind,
            source_info,
            level,
            loc,
        }
    }

    pub fn loc(&self) -> Loc {
        self.loc
    }

    pub fn level(&self) -> usize {
        self.level
    }

    pub fn set_level(&mut self, level: usize) {
        self.level = level;
    }

    pub fn show_file_name(&self) {
        self.source_info.show_file_name()
    }

    pub fn show_loc(&self) {
        self.source_info.show_loc(&self.loc);
    }

    pub fn show_err(&self) {
        match &self.kind {
            RubyErrorKind::ParseErr(e) => {
                eprintln!("parse error: {:?}", e);
            }
            RubyErrorKind::RuntimeErr(e) => match e {
                RuntimeErrKind::Name(n) => eprintln!("NoNameError ({})", n),
                RuntimeErrKind::NoMethod(n) => eprintln!("NoMethodError ({})", n),
                RuntimeErrKind::Type(n) => eprintln!("TypeError ({})", n),
                RuntimeErrKind::Unimplemented(n) => eprintln!("UnimplementedError ({})", n),
                RuntimeErrKind::Internal(n) => eprintln!("InternalError ({})", n),
                RuntimeErrKind::Argument(n) => eprintln!("ArgumentError ({})", n),
                RuntimeErrKind::Index(n) => eprintln!("IndexError ({})", n),
            },
        }
    }
}

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
    LoadError(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeErrKind {
    Unimplemented(String),
    Internal(String),
    Name(String),
    NoMethod(String),
    Argument(String),
    Index(String),
    Type(String),
}

impl RubyError {
    pub fn new_runtime_err(err: RuntimeErrKind, source_info: SourceInfoRef, loc: Loc) -> Self {
        let kind = RubyErrorKind::RuntimeErr(err);
        RubyError::new(kind, source_info, 0, loc)
    }
    pub fn new_parse_err(
        err: ParseErrKind,
        source_info: SourceInfoRef,
        level: usize,
        loc: Loc,
    ) -> Self {
        let kind = RubyErrorKind::ParseErr(err);
        RubyError::new(kind, source_info, level, loc)
    }
}
