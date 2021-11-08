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
    pub(crate) fn new(kind: RubyErrorKind) -> Self {
        Self { kind, info: vec![] }
    }

    pub(crate) fn new_with_info(kind: RubyErrorKind, source_info: SourceInfoRef, loc: Loc) -> Self {
        Self {
            kind,
            info: vec![(source_info, loc)],
        }
    }

    pub(crate) fn is_stop_iteration(&self) -> bool {
        matches!(
            &self.kind,
            RubyErrorKind::RuntimeErr {
                kind: RuntimeErrKind::StopIteration,
                ..
            }
        )
    }

    pub(crate) fn is_block_return(&self) -> bool {
        matches!(&self.kind, RubyErrorKind::BlockReturn)
    }

    pub(crate) fn is_exception(&self) -> bool {
        matches!(&self.kind, RubyErrorKind::Exception)
    }
}

impl RubyError {
    pub(crate) fn get_location(&self, pos: usize) -> String {
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

    pub fn show_err(self) {
        match Value::from_exception(self) {
            Some(ex) => match ex.if_exception() {
                Some(err) => eprintln!("{:?}", err),
                None => unreachable!(),
            },
            None => eprint!("None"),
        }
    }

    pub(crate) fn message(&self) -> String {
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
    fn new_runtime_err(kind: RuntimeErrKind, message: String) -> Self {
        let kind = RubyErrorKind::RuntimeErr { kind, message };
        RubyError::new(kind)
    }

    pub(crate) fn new_parse_err(err: ParseErrKind, source_info: SourceInfoRef, loc: Loc) -> Self {
        let kind = RubyErrorKind::ParseErr(err);
        RubyError::new_with_info(kind, source_info, loc)
    }

    pub(crate) fn conv_localjump_err(mut self) -> Self {
        self.kind = RubyErrorKind::RuntimeErr {
            kind: RuntimeErrKind::LocalJump,
            message: "Unexpected return.".to_string(),
        };
        self
    }
}

impl RubyError {
    pub(crate) fn runtime(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::Runtime, msg.into())
    }

    pub(crate) fn nomethod(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::NoMethod, msg.into())
    }

    pub(crate) fn undefined_op(
        method_name: impl Into<String>,
        rhs: Value,
        lhs: Value,
    ) -> RubyError {
        Self::nomethod(format!(
            "undefined method `{}' {} for {:?}:{}",
            method_name.into(),
            rhs.get_class_name(),
            lhs,
            lhs.get_class_name()
        ))
    }

    pub(crate) fn undefined_method(method: IdentId, receiver: Value) -> RubyError {
        Self::nomethod(format!(
            "undefined method `{:?}' for {:?}:{}",
            method,
            receiver,
            receiver.get_class().name()
        ))
    }

    pub(crate) fn undefined_method_for_class(method: IdentId, class: Module) -> RubyError {
        Self::nomethod(format!(
            "undefined method `{:?}' for {}",
            method,
            class.name()
        ))
    }

    pub(crate) fn internal(msg: impl Into<String>) -> RubyError {
        RubyError::new(RubyErrorKind::Internal(msg.into()))
    }

    pub(crate) fn none(msg: impl Into<String>) -> RubyError {
        RubyError::new(RubyErrorKind::None(msg.into()))
    }

    pub(crate) fn name(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::Name, msg.into())
    }

    pub(crate) fn uninitialized_constant(id: IdentId) -> RubyError {
        RubyError::name(format!("Uninitialized constant {:?}.", id))
    }

    pub(crate) fn typeerr(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::Type, msg.into())
    }

    pub(crate) fn wrong_type(kind: impl Into<String>, class: &str, val: Value) -> RubyError {
        RubyError::typeerr(format!(
            "{} must be an {}. (given:{})",
            kind.into(),
            class,
            val.get_class_name()
        ))
    }

    pub(crate) fn no_implicit_conv(other: Value, msg: impl Into<String>) -> RubyError {
        RubyError::typeerr(format!(
            "No implicit conversion of {:?} into {}.",
            other,
            msg.into()
        ))
    }

    pub(crate) fn cant_coerse(val: Value, class: &str) -> RubyError {
        RubyError::typeerr(format!("Can not coerce {:?} into {}.", val, class))
    }

    pub(crate) fn argument(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::Argument, msg.into())
    }

    pub(crate) fn argument_wrong(given: usize, expected: usize) -> RubyError {
        RubyError::argument(format!(
            "Wrong number of arguments. (given {}, expected {})",
            given, expected
        ))
    }

    pub(crate) fn argument_wrong_range(given: usize, min: usize, max: usize) -> RubyError {
        RubyError::argument(format!(
            "Wrong number of arguments. (given {}, expected {}..{})",
            given, min, max
        ))
    }

    pub(crate) fn regexp(err: fancy_regex::Error) -> RubyError {
        RubyError::new_runtime_err(
            RuntimeErrKind::Regexp,
            format!("Invalid string for a regular expression. {:?}", err),
        )
    }

    pub(crate) fn index(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::Index, msg.into())
    }

    pub(crate) fn fiber(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::Fiber, msg.into())
    }

    pub(crate) fn load(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::LoadError, msg.into())
    }

    pub(crate) fn range(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::Range, msg.into())
    }

    pub(crate) fn zero_div(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::ZeroDivision, msg.into())
    }

    pub(crate) fn math_domain(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::DomainError, msg.into())
    }
}

impl RubyError {
    pub(crate) fn method_return() -> RubyError {
        RubyError::new(RubyErrorKind::MethodReturn)
    }

    pub(crate) fn block_return() -> RubyError {
        RubyError::new(RubyErrorKind::BlockReturn)
    }

    pub(crate) fn value() -> RubyError {
        RubyError::new(RubyErrorKind::Exception)
    }

    pub(crate) fn stop_iteration(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::StopIteration, msg.into())
    }

    pub(crate) fn local_jump(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::LocalJump, msg.into())
    }
}

#[allow(unused_imports)]
mod tests {
    use crate::tests::*;

    #[test]
    fn errors() {
        let program = r#"
        errors = [
          ["a", NameError],
          ["break", SyntaxError],
          ["Integer('z')", ArgumentError],
          ["5 * :sym", TypeError],
          ["4 / 0", ZeroDivisionError],
          ["500.chr", RangeError],
        ]
        errors.each do | code, error|
          begin
            eval code
          rescue SyntaxError, StandardError => err
            assert error, err.class
          else
            raise
          end
        end
        "#;
        assert_script(program);
    }

    #[test]
    fn class_define_error() {
        let program = r#"
        assert_error {
            class Foo < 3
            end
        }
        assert_error {
            class Foo < Object
            end
            class Foo < Array
            end
        }
        assert_error {
            class Foo < Object
            end
            module Foo
            end
        }
        "#;
        assert_script(program);
    }
}
