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
    RuntimeErr {
        kind: RuntimeErrKind,
        message: String,
    },
    MethodReturn(Value),
    BlockReturn(Value),
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
    Unimplemented,
    Internal,
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
}

impl RubyError {
    pub fn new(kind: RubyErrorKind, source_info: SourceInfoRef, level: usize, loc: Loc) -> Self {
        RubyError {
            kind,
            info: vec![(source_info, loc)],
            level,
        }
    }

    pub fn is_block_return(&self) -> bool {
        match &self.kind {
            RubyErrorKind::BlockReturn(_) => true,
            _ => false,
        }
    }

    pub fn is_method_return(&self) -> bool {
        match &self.kind {
            RubyErrorKind::MethodReturn(_) => true,
            _ => false,
        }
    }

    pub fn is_stop_iteration(&self) -> bool {
        match &self.kind {
            RubyErrorKind::RuntimeErr { kind, .. } => match kind {
                RuntimeErrKind::StopIteration => true,
                _ => false,
            },
            _ => false,
        }
    }
}

impl RubyError {
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
        self.info[pos].0.show_file_name();
    }

    pub fn show_loc(&self, pos: usize) {
        self.info[pos].0.show_loc(&self.info[pos].1);
    }

    pub fn show_err(&self) {
        match &self.kind {
            RubyErrorKind::ParseErr(e) => match e {
                ParseErrKind::UnexpectedEOF => eprintln!("Unexpected EOF"),
                ParseErrKind::UnexpectedToken => eprintln!("Unexpected token"),
                ParseErrKind::SyntaxError(n) => eprintln!("SyntaxError: {}", n),
                ParseErrKind::LoadError(n) => eprintln!("LoadError: {}", n),
            },
            RubyErrorKind::RuntimeErr { kind, message } => {
                match kind {
                    RuntimeErrKind::Name => eprint!("NoNameError"),
                    RuntimeErrKind::NoMethod => eprint!("NoMethodError"),
                    RuntimeErrKind::Type => eprint!("TypeError"),
                    RuntimeErrKind::Unimplemented => eprint!("UnimplementedError"),
                    RuntimeErrKind::Internal => eprint!("InternalError"),
                    RuntimeErrKind::Argument => eprint!("ArgumentError"),
                    RuntimeErrKind::Index => eprint!("IndexError"),
                    RuntimeErrKind::Regexp => eprint!("RegexpError"),
                    RuntimeErrKind::Fiber => eprint!("FiberError"),
                    RuntimeErrKind::LocalJump => eprint!("LocalJumpError"),
                    RuntimeErrKind::StopIteration => eprint!("StopIteration"),
                    RuntimeErrKind::Runtime => eprint!("RuntimeError"),
                };
                eprintln!("({})", message);
            }
            RubyErrorKind::MethodReturn(_) => {
                eprintln!("LocalJumpError");
            }
            RubyErrorKind::BlockReturn(_) => {
                eprintln!("LocalJumpError");
            }
        }
    }
}

impl RubyError {
    pub fn new_runtime_err(
        kind: RuntimeErrKind,
        message: String,
        source_info: SourceInfoRef,
        loc: Loc,
    ) -> Self {
        let kind = RubyErrorKind::RuntimeErr { kind, message };
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

    pub fn new_method_return(val: Value, source_info: SourceInfoRef, loc: Loc) -> Self {
        RubyError::new(RubyErrorKind::MethodReturn(val), source_info, 0, loc)
    }

    pub fn new_block_return(val: Value, source_info: SourceInfoRef, loc: Loc) -> Self {
        RubyError::new(RubyErrorKind::BlockReturn(val), source_info, 0, loc)
    }

    pub fn conv_localjump_err(mut self) -> Self {
        self.kind = RubyErrorKind::RuntimeErr {
            kind: RuntimeErrKind::LocalJump,
            message: "Unexpected return.".to_string(),
        };
        self
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
