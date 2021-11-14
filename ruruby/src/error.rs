use crate::*;

pub struct VMError;

impl VMError {
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

    pub(crate) fn regexp(err: fancy_regex::Error) -> RubyError {
        RubyError::new_runtime_err(
            RuntimeErrKind::Regexp,
            format!("Invalid string for a regular expression. {:?}", err),
        )
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
