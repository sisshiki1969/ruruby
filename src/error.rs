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
                write!(f, "{:?} {}", kind, message)
            }
            RubyErrorKind::ParseErr(kind) => write!(f, "ParseErr: {:?}", kind),
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

#[derive(Debug, Clone, PartialEq)]
pub enum ParseErrKind {
    UnexpectedEOF,
    UnexpectedToken,
    SyntaxError(String),
    Name(String),
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

    pub fn new_with_info(kind: RubyErrorKind, source_info: SourceInfoRef, loc: Loc) -> Self {
        Self {
            kind,
            info: vec![(source_info, loc)],
        }
    }

    pub fn is_stop_iteration(&self) -> bool {
        match &self.kind {
            RubyErrorKind::RuntimeErr {
                kind: RuntimeErrKind::StopIteration,
                ..
            } => true,
            _ => false,
        }
    }

    pub fn is_block_return(&self) -> bool {
        match &self.kind {
            RubyErrorKind::BlockReturn => true,
            _ => false,
        }
    }

    pub fn is_exception(&self) -> bool {
        match &self.kind {
            RubyErrorKind::Exception => true,
            _ => false,
        }
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

    pub fn show_err(&self) {
        eprintln!("{}", self.message());
    }

    pub fn message(&self) -> String {
        match &self.kind {
            RubyErrorKind::ParseErr(e) => match e {
                ParseErrKind::UnexpectedEOF => "SyntaxError (Unexpected EOF)".to_string(),
                ParseErrKind::UnexpectedToken => "SyntaxError (Unexpected token)".to_string(),
                ParseErrKind::SyntaxError(n) => format!("SyntaxError ({})", n),
                ParseErrKind::Name(n) => format!("NameError ({})", n),
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

    pub fn to_exception_val(self) -> Option<Value> {
        let val = match &self.kind {
            RubyErrorKind::Exception => return None,
            RubyErrorKind::ParseErr(_) => {
                let err_class = BuiltinClass::get_toplevel_constant("SyntaxError").into_module();
                Value::exception(err_class, self)
            }
            RubyErrorKind::RuntimeErr { kind, .. } => match kind {
                RuntimeErrKind::Type => {
                    let err_class = BuiltinClass::get_toplevel_constant("TypeError").into_module();
                    Value::exception(err_class, self)
                }
                RuntimeErrKind::Argument => {
                    let err_class =
                        BuiltinClass::get_toplevel_constant("ArgumentError").into_module();
                    Value::exception(err_class, self)
                }
                RuntimeErrKind::NoMethod => {
                    let err_class =
                        BuiltinClass::get_toplevel_constant("NoMethodError").into_module();
                    Value::exception(err_class, self)
                }
                RuntimeErrKind::Runtime => {
                    let err_class =
                        BuiltinClass::get_toplevel_constant("RuntimeError").into_module();
                    Value::exception(err_class, self)
                }
                RuntimeErrKind::LoadError => {
                    let err_class = BuiltinClass::get_toplevel_constant("LoadError").into_module();
                    Value::exception(err_class, self)
                }
                RuntimeErrKind::StopIteration => {
                    let err_class =
                        BuiltinClass::get_toplevel_constant("StopIteration").into_module();
                    Value::exception(err_class, self)
                }
                RuntimeErrKind::Name => {
                    let err_class = BuiltinClass::get_toplevel_constant("NameError").into_module();
                    Value::exception(err_class, self)
                }
                RuntimeErrKind::ZeroDivision => {
                    let err_class =
                        BuiltinClass::get_toplevel_constant("ZeroDivisionError").into_module();
                    Value::exception(err_class, self)
                }
                RuntimeErrKind::Range => {
                    let err_class = BuiltinClass::get_toplevel_constant("RangeError").into_module();
                    Value::exception(err_class, self)
                }
                RuntimeErrKind::Index => {
                    let err_class = BuiltinClass::get_toplevel_constant("IndexError").into_module();
                    Value::exception(err_class, self)
                }
                RuntimeErrKind::Regexp => {
                    let err_class =
                        BuiltinClass::get_toplevel_constant("RegexpError").into_module();
                    Value::exception(err_class, self)
                }
                RuntimeErrKind::Fiber => {
                    let err_class = BuiltinClass::get_toplevel_constant("FiberError").into_module();
                    Value::exception(err_class, self)
                }
                RuntimeErrKind::LocalJump => {
                    let err_class =
                        BuiltinClass::get_toplevel_constant("LocalJumpError").into_module();
                    Value::exception(err_class, self)
                }
                RuntimeErrKind::DomainError => {
                    let math = BuiltinClass::get_toplevel_constant("Math");
                    let err = math
                        .into_module()
                        .get_const_noautoload(IdentId::get_id("DomainError"))
                        .unwrap();
                    Value::exception(err.into_module(), self)
                }
            },
            RubyErrorKind::MethodReturn | RubyErrorKind::BlockReturn => {
                let err_class = BuiltinClass::get_toplevel_constant("LocalJumpError").into_module();
                Value::exception(err_class, self)
            }
            _ => {
                let standard = BuiltinClass::standard();
                Value::exception(standard, self)
            }
        };
        Some(val)
    }
}

impl RubyError {
    fn new_runtime_err(kind: RuntimeErrKind, message: String) -> Self {
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
            receiver.get_class().name()
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

    pub fn cant_coerse(val: Value, class: &str) -> RubyError {
        RubyError::typeerr(format!("Can not coerce {:?} into {}.", val, class))
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
