use crate::*;

#[derive(Clone, PartialEq)]
pub struct RubyError(Box<ErrorInfo>);

impl std::ops::Deref for RubyError {
    type Target = ErrorInfo;
    fn deref(&self) -> &ErrorInfo {
        &self.0
    }
}

impl std::ops::DerefMut for RubyError {
    fn deref_mut(&mut self) -> &mut ErrorInfo {
        &mut self.0
    }
}

impl std::fmt::Debug for RubyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

#[derive(Clone, PartialEq)]
pub struct ErrorInfo {
    pub kind: RubyErrorKind,
    pub info: Vec<(SourceInfoRef, Loc)>,
    level: usize,
}

impl std::fmt::Debug for ErrorInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            RubyErrorKind::RuntimeErr { kind, message } => {
                write!(f, "{:?} {}", kind, message)
            }
            RubyErrorKind::ParseErr(kind) => write!(f, "ParseErr: {:?}", kind),
            RubyErrorKind::MethodReturn => write!(f, "MethodReturn"),
            RubyErrorKind::BlockReturn => write!(f, "BlockReturn"),
            RubyErrorKind::Value(val) => write!(f, "{:?}", val),
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
    Value(Value),
    MethodReturn,
    BlockReturn,
    Internal(String),
    None(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseErrKind {
    UnexpectedEOF,
    UnexpectedToken,
    SyntaxError(String),
    Name(String),
}

#[derive(Debug, Clone, PartialEq)]
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
}

impl RubyError {
    pub fn new(kind: RubyErrorKind, level: usize) -> Self {
        Self(Box::new(ErrorInfo {
            kind,
            info: vec![],
            level,
        }))
    }

    pub fn new_with_info(
        kind: RubyErrorKind,
        source_info: SourceInfoRef,
        level: usize,
        loc: Loc,
    ) -> Self {
        Self(Box::new(ErrorInfo {
            kind,
            info: vec![(source_info, loc)],
            level,
        }))
    }

    pub fn is_stop_iteration(&self) -> bool {
        match &self.0.kind {
            RubyErrorKind::RuntimeErr {
                kind: RuntimeErrKind::StopIteration,
                ..
            } => true,
            _ => false,
        }
    }
}

impl RubyError {
    #[cfg(not(tarpaulin_include))]
    pub fn level(&self) -> usize {
        self.0.level
    }

    #[cfg(not(tarpaulin_include))]
    pub fn set_level(&mut self, level: usize) {
        self.0.level = level;
    }

    pub fn get_location(&self, pos: usize) -> String {
        if let Some(info) = self.0.info.get(pos) {
            info.0.get_location(&self.0.info[pos].1)
        } else {
            "".to_string()
        }
    }

    pub fn show_loc(&self, pos: usize) {
        if let Some(info) = self.0.info.get(pos) {
            info.0.show_loc(&self.0.info[pos].1);
        }
    }

    pub fn show_all_loc(&self) {
        for i in 0..self.info.len() {
            eprint!("{}:", i);
            self.show_loc(i);
        }
    }

    pub fn show_err(&self) {
        eprintln!("{}", self.message());
    }

    pub fn message(&self) -> String {
        match &self.0.kind {
            RubyErrorKind::ParseErr(e) => match e {
                ParseErrKind::UnexpectedEOF => "Unexpected EOF".to_string(),
                ParseErrKind::UnexpectedToken => "Unexpected token".to_string(),
                ParseErrKind::SyntaxError(n) => format!("SyntaxError: {}", n),
                ParseErrKind::Name(n) => format!("NameError: {}", n),
            },
            RubyErrorKind::RuntimeErr { message, .. } => message.to_owned(),
            RubyErrorKind::MethodReturn => "LocalJumpError".to_string(),
            RubyErrorKind::BlockReturn => "LocalJumpError".to_string(),
            RubyErrorKind::Value(val) => val.if_exception().unwrap().message(),
            RubyErrorKind::None(msg) => msg.to_owned(),
            RubyErrorKind::Internal(msg) => {
                format!("InternalError\n{}", msg)
            }
        }
    }

    pub fn to_exception_val(&self) -> Value {
        match &self.kind {
            RubyErrorKind::Value(val) => *val,
            RubyErrorKind::RuntimeErr { kind, .. } => match &kind {
                RuntimeErrKind::Type => {
                    let err_class = BuiltinClass::get_toplevel_constant("TypeError")
                        .unwrap()
                        .into_module();
                    Value::exception(err_class, self.clone())
                }
                RuntimeErrKind::Argument => {
                    let err_class = BuiltinClass::get_toplevel_constant("ArgumentError")
                        .unwrap()
                        .into_module();
                    Value::exception(err_class, self.clone())
                }
                RuntimeErrKind::NoMethod => {
                    let err_class = BuiltinClass::get_toplevel_constant("NoMethodError")
                        .unwrap()
                        .into_module();
                    Value::exception(err_class, self.clone())
                }
                RuntimeErrKind::Runtime => {
                    let err_class = BuiltinClass::get_toplevel_constant("RuntimeError")
                        .unwrap()
                        .into_module();
                    Value::exception(err_class, self.clone())
                }
                RuntimeErrKind::LoadError => {
                    let err_class = BuiltinClass::get_toplevel_constant("LoadError")
                        .unwrap()
                        .into_module();
                    Value::exception(err_class, self.clone())
                }
                RuntimeErrKind::StopIteration => {
                    let err_class = BuiltinClass::get_toplevel_constant("StopIteration")
                        .unwrap()
                        .into_module();
                    Value::exception(err_class, self.clone())
                }
                RuntimeErrKind::Name => {
                    let err_class = BuiltinClass::get_toplevel_constant("NameError")
                        .unwrap()
                        .into_module();
                    Value::exception(err_class, self.clone())
                }
                _ => {
                    let standard = BuiltinClass::standard();
                    Value::exception(standard, self.clone())
                }
            },
            _ => {
                let standard = BuiltinClass::standard();
                Value::exception(standard, self.clone())
            }
        }
    }
}

impl RubyError {
    fn new_runtime_err(kind: RuntimeErrKind, message: String) -> Self {
        let kind = RubyErrorKind::RuntimeErr { kind, message };
        RubyError::new(kind, 0)
    }

    pub fn new_parse_err(
        err: ParseErrKind,
        source_info: SourceInfoRef,
        level: usize,
        loc: Loc,
    ) -> Self {
        let kind = RubyErrorKind::ParseErr(err);
        RubyError::new_with_info(kind, source_info, level, loc)
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

    pub fn undefined_op(method_name: impl Into<String>, rhs: Value, lhs: Value) -> RubyError {
        Self::nomethod(format!(
            "undefined method `{}' {} for {:?}:{}",
            method_name.into(),
            rhs.get_class_name(),
            lhs,
            lhs.get_class_name()
        ))
    }

    pub fn undefined_method(method: IdentId, receiver: Value) -> RubyError {
        Self::nomethod(format!(
            "undefined method `{:?}' for {:?}:{}",
            method,
            receiver,
            receiver.get_class_name()
        ))
    }

    pub fn undefined_method_for_class(method: IdentId, class: Module) -> RubyError {
        Self::nomethod(format!(
            "undefined method `{:?}' for {}",
            method,
            class.name()
        ))
    }

    pub fn internal(msg: impl Into<String>) -> RubyError {
        RubyError::new(RubyErrorKind::Internal(msg.into()), 0)
    }

    pub fn none(msg: impl Into<String>) -> RubyError {
        RubyError::new(RubyErrorKind::None(msg.into()), 0)
    }

    pub fn name(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::Name, msg.into())
    }

    pub fn typeerr(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::Type, msg.into())
    }

    pub fn wrong_type(kind: impl Into<String>, class: &str, val: Value) -> RubyError {
        RubyError::typeerr(format!(
            "{} must be an {}. (given:{})",
            kind.into(),
            class,
            val.get_class_name()
        ))
    }

    pub fn no_implicit_conv(other: Value, msg: impl Into<String>) -> RubyError {
        RubyError::typeerr(format!(
            "No implicit conversion of {:?} into {}.",
            other,
            msg.into()
        ))
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

    pub fn regexp(err: fancy_regex::Error) -> RubyError {
        RubyError::new_runtime_err(
            RuntimeErrKind::Regexp,
            format!("Invalid string for a regular expression. {:?}", err),
        )
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
}

impl RubyError {
    pub fn method_return() -> RubyError {
        RubyError::new(RubyErrorKind::MethodReturn, 0)
    }

    pub fn block_return() -> RubyError {
        RubyError::new(RubyErrorKind::BlockReturn, 0)
    }

    pub fn value(val: Value) -> RubyError {
        RubyError::new(RubyErrorKind::Value(val), 0)
    }

    pub fn stop_iteration(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::StopIteration, msg.into())
    }

    pub fn local_jump(msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::LocalJump, msg.into())
    }
}

#[allow(unused_imports)]
mod tests {
    use crate::test::*;

    #[test]
    fn errors() {
        let program = r#"
        assert_error { a }
        assert_error { break }
        assert_error { Integer("z") }
        assert_error { 5 * :sym }
        assert_error { 4 / 0 }
        assert_error { 500.chr }
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
