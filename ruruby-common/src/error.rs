use crate::*;

#[derive(Clone, PartialEq)]
pub struct RubyError {
    pub kind: RubyErrorKind,
    pub info: Vec<(SourceInfoRef, Loc)>,
}

impl std::fmt::Debug for RubyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            RubyErrorKind::RuntimeErr { kind, message } => {
                write!(f, "{:?}: ({})", kind, message)
            }
            RubyErrorKind::ParseErr(kind) => write!(f, "{:?}", kind),
            RubyErrorKind::MethodReturn => write!(f, "MethodReturn"),
            RubyErrorKind::BlockReturn => write!(f, "BlockReturn"),
            RubyErrorKind::Exception => write!(f, "Exception"),
            RubyErrorKind::Internal(msg) => write!(f, "InternalError {}", msg),
            RubyErrorKind::None(msg) => write!(f, "{}", msg),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RubyErrorKind {
    ParseErr(ParseErrKind),
    RuntimeErr {
        kind: RuntimeErrKind,
        message: String,
    },
    Exception,
    MethodReturn,
    BlockReturn,
    Internal(String),
    None(String),
}

#[derive(Clone, PartialEq)]
pub enum ParseErrKind {
    UnexpectedEOF,
    SyntaxError(String),
}

impl std::fmt::Debug for ParseErrKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedEOF => write!(f, "SyntaxError (Unexpected EOF.)"),
            Self::SyntaxError(msg) => write!(f, "SyntaxError ({})", msg),
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum RuntimeErrKind {
    Name,
    NoMethod,
    Argument,
    Index,
    Type,
    Regexp,
    Fiber,
    LocalJump,
    StopIteration,
    Runtime,
    LoadError,
    Range,
    ZeroDivision,
    DomainError,
}

impl std::fmt::Debug for RuntimeErrKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::Name => write!(f, "NameError"),
            Self::NoMethod => write!(f, "NoMethodError"),
            Self::Argument => write!(f, "ArgumentError"),
            Self::Index => write!(f, "IndexError"),
            Self::Type => write!(f, "TypeError"),
            Self::Regexp => write!(f, "RegexpError"),
            Self::Fiber => write!(f, "FiberError"),
            Self::LocalJump => write!(f, "LocalJumpError"),
            Self::StopIteration => write!(f, "StopIteration"),
            Self::Runtime => write!(f, "Runtime"),
            Self::LoadError => write!(f, "LoadError"),
            Self::Range => write!(f, "RangeError"),
            Self::ZeroDivision => write!(f, "ZeroDivisionError"),
            Self::DomainError => write!(f, "Math::DomainError"),
        }
    }
}

impl RubyError {
    pub fn new(kind: RubyErrorKind) -> Self {
        Self { kind, info: vec![] }
    }

    fn new_with_info(kind: RubyErrorKind, source_info: SourceInfoRef, loc: Loc) -> Self {
        Self {
            kind,
            info: vec![(source_info, loc)],
        }
    }

    pub fn is_stop_iteration(&self) -> bool {
        matches!(
            &self.kind,
            RubyErrorKind::RuntimeErr {
                kind: RuntimeErrKind::StopIteration,
                ..
            }
        )
    }

    pub fn is_block_return(&self) -> bool {
        matches!(&self.kind, RubyErrorKind::BlockReturn)
    }

    pub fn is_exception(&self) -> bool {
        matches!(&self.kind, RubyErrorKind::Exception)
    }
}

impl RubyError {
    pub fn get_location(&self, pos: usize) -> String {
        if let Some(info) = self.info.get(pos) {
            info.0.get_location(&self.info[pos].1)
        } else {
            "".to_string()
        }
    }

    pub fn show_loc(&self, pos: usize) {
        if let Some(info) = self.info.get(pos) {
            info.0.show_loc(&self.info[pos].1);
        }
    }

    pub fn show_all_loc(&self) {
        for i in 0..self.info.len() {
            eprint!("{}:", i);
            self.show_loc(i);
        }
    }

    pub fn message(&self) -> String {
        match &self.kind {
            RubyErrorKind::ParseErr(e) => match e {
                ParseErrKind::UnexpectedEOF => "SyntaxError (Unexpected EOF)".to_string(),
                ParseErrKind::SyntaxError(n) => format!("SyntaxError ({})", n),
            },
            RubyErrorKind::RuntimeErr { kind, message } => format!("{:?} ({})", kind, message),
            RubyErrorKind::MethodReturn => "LocalJumpError".to_string(),
            RubyErrorKind::BlockReturn => "LocalJumpError".to_string(),
            RubyErrorKind::Exception => "Exception".to_string(),
            RubyErrorKind::None(msg) => msg.to_owned(),
            RubyErrorKind::Internal(msg) => {
                format!("InternalError\n{}", msg)
            }
        }
    }
}

impl RubyError {
    pub fn new_runtime_err(kind: RuntimeErrKind, message: String) -> Self {
        let kind = RubyErrorKind::RuntimeErr { kind, message };
        RubyError::new(kind)
    }

    pub fn new_parse_err(err: ParseErrKind, source_info: SourceInfoRef, loc: Loc) -> Self {
        let kind = RubyErrorKind::ParseErr(err);
        RubyError::new_with_info(kind, source_info, loc)
    }

    pub fn conv_localjump_err(mut self) -> Self {
        self.kind = RubyErrorKind::RuntimeErr {
            kind: RuntimeErrKind::LocalJump,
            message: "Unexpected return.".to_string(),
        };
        self
    }
}

impl RubyError {
    pub fn runtime(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::Runtime, msg.into())
    }

    pub fn nomethod(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::NoMethod, msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> RubyError {
        RubyError::new(RubyErrorKind::Internal(msg.into()))
    }

    pub fn none(msg: impl Into<String>) -> RubyError {
        RubyError::new(RubyErrorKind::None(msg.into()))
    }

    pub fn name(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::Name, msg.into())
    }

    pub fn uninitialized_constant(id: IdentId) -> RubyError {
        RubyError::name(format!("Uninitialized constant {:?}.", id))
    }

    pub fn typeerr(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::Type, msg.into())
    }

    pub fn argument(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::Argument, msg.into())
    }

    pub fn argument_wrong(given: usize, expected: usize) -> RubyError {
        RubyError::argument(format!(
            "Wrong number of arguments. (given {}, expected {})",
            given, expected
        ))
    }

    pub fn argument_wrong_range(given: usize, min: usize, max: usize) -> RubyError {
        RubyError::argument(format!(
            "Wrong number of arguments. (given {}, expected {}..{})",
            given, min, max
        ))
    }

    pub fn index(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::Index, msg.into())
    }

    pub fn fiber(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::Fiber, msg.into())
    }

    pub fn load(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::LoadError, msg.into())
    }

    pub fn range(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::Range, msg.into())
    }

    pub fn zero_div(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::ZeroDivision, msg.into())
    }

    pub fn math_domain(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::DomainError, msg.into())
    }
}

impl RubyError {
    pub fn method_return() -> RubyError {
        RubyError::new(RubyErrorKind::MethodReturn)
    }

    pub fn block_return() -> RubyError {
        RubyError::new(RubyErrorKind::BlockReturn)
    }

    pub fn value() -> RubyError {
        RubyError::new(RubyErrorKind::Exception)
    }

    pub fn stop_iteration(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::StopIteration, msg.into())
    }

    pub fn local_jump(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::LocalJump, msg.into())
    }
}
