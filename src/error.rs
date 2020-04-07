use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct RubyError {
    pub kind: RubyErrorKind,
    pub info: Vec<(SourceInfoRef, Loc)>,
    level: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RubyErrorKind {
    ParseErr(ParseErrKind),
    RuntimeErr(RuntimeErrKind),
    MethodReturn(MethodRef),
    BlockReturn,
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
    Regexp(String),
    Fiber(String),
}

impl RubyError {
    pub fn new(kind: RubyErrorKind, source_info: SourceInfoRef, level: usize, loc: Loc) -> Self {
        RubyError {
            kind,
            info: vec![(source_info, loc)],
            level,
        }
    }

    pub fn loc(&self) -> Loc {
        self.info[0].1
    }

    pub fn level(&self) -> usize {
        self.level
    }

    pub fn set_level(&mut self, level: usize) {
        self.level = level;
    }

    pub fn show_file_name(&self, pos: usize) {
        self.info[pos].0.show_file_name()
    }

    pub fn show_loc(&self, pos: usize) {
        self.info[pos].0.show_loc(&self.info[pos].1);
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
                RuntimeErrKind::Regexp(n) => eprintln!("RegexpError ({})", n),
                RuntimeErrKind::Fiber(n) => eprintln!("FiberError ({})", n),
            },
            RubyErrorKind::MethodReturn(_) => {
                eprintln!("LocalJumpError");
            }
            RubyErrorKind::BlockReturn => {
                eprintln!("LocalJumpError");
            }
        }
    }
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

    pub fn new_method_return(method: MethodRef, source_info: SourceInfoRef, loc: Loc) -> Self {
        RubyError::new(RubyErrorKind::MethodReturn(method), source_info, 0, loc)
    }

    pub fn new_block_return(source_info: SourceInfoRef, loc: Loc) -> Self {
        RubyError::new(RubyErrorKind::BlockReturn, source_info, 0, loc)
    }
}
