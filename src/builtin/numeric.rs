use num::Zero;

use crate::*;

pub fn init() -> Module {
    let mut class = Module::class_under_object();
    BuiltinClass::set_toplevel_constant("Numeric", class);
    class.append_include_without_increment_version(BuiltinClass::comparable());
    class.add_builtin_method_by_str("to_s", inspect);
    class.add_builtin_method_by_str("inspect", inspect);
    class.add_builtin_method_by_str("+", add);
    class.add_builtin_method_by_str("-", sub);
    class.add_builtin_method_by_str("*", mul);
    class.add_builtin_method_by_str("/", div);
    class.add_builtin_method_by_str("%", rem);
    class.add_builtin_method_by_str(">=", ge);
    class.add_builtin_method_by_str(">", gt);
    class.add_builtin_method_by_str("<=", le);
    class.add_builtin_method_by_str("<", lt);
    class
}

// Instance methods
fn inspect(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let s = match self_val.to_real() {
        Some(r) => match r {
            Real::Bignum(n) => n.to_string(),
            Real::Integer(i) => i.to_string(),
            Real::Float(f) => float_format(f),
        },
        None => match self_val.as_complex() {
            Some((r, i)) => {
                let (r, i) = (r.to_real().unwrap(), i.to_real().unwrap());
                if !i.is_negative() {
                    format!("({:?}+{:?}i)", r, i)
                } else {
                    format!("({:?}{:?}i)", r, i)
                }
            }
            None => unreachable!(),
        },
    };
    Ok(Value::string(s))
}

fn float_format(f: f64) -> String {
    let fabs = f.abs();
    if f.is_zero() {
        "0.0".to_string()
    } else if fabs < 0.001 || fabs >= 1000000000000000.0 {
        format!("{:.1e}", f)
    } else if f.fract().is_zero() {
        format!("{:.1}", f)
    } else {
        format!("{}", f)
    }
}

fn add(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    match args[0].to_real() {
        Some(rhs) => Ok((lhs + rhs).to_val()),
        None => match args[0].to_complex() {
            Some((r, i)) => {
                let r = lhs + r;
                let i = i;
                Ok(Value::complex(r.to_val(), i.to_val()))
            }
            None => Err(RubyError::undefined_op("+", args[0], self_val)),
        },
    }
}

fn sub(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    match args[0].to_real() {
        Some(rhs) => Ok((lhs - rhs).to_val()),
        None => match args[0].to_complex() {
            Some((r, i)) => {
                let r = lhs - r;
                let i = -i;
                Ok(Value::complex(r.to_val(), i.to_val()))
            }
            None => Err(RubyError::undefined_op("-", args[0], self_val)),
        },
    }
}

fn mul(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    match args[0].to_real() {
        Some(rhs) => Ok((lhs * rhs).to_val()),
        None => match args[0].to_complex() {
            Some((r, i)) => {
                let r = lhs.clone() * r;
                let i = lhs * i;
                Ok(Value::complex(r.to_val(), i.to_val()))
            }
            None => Err(RubyError::typeerr(format!(
                "{:?} can't be coerced into Integer.",
                args[0]
            ))),
        },
    }
}

fn div(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    match args[0].to_real() {
        Some(rhs) => Ok((lhs / rhs).to_val()),
        None => match args[0].to_complex() {
            Some((r, i)) => {
                let divider = r.clone().exp2() + i.clone().exp2();
                let r = (lhs.clone() * r).divide(divider.clone());
                let i = (-lhs * i).divide(divider);
                Ok(Value::complex(r.to_val(), i.to_val()))
            }
            None => Err(RubyError::typeerr(format!(
                "{:?} can't be coerced into Integer.",
                args[0]
            ))),
        },
    }
}

fn rem(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    match args[0].to_real() {
        Some(rhs) => Ok((lhs % rhs).to_val()),
        None => Err(RubyError::typeerr(format!(
            "{:?} can't be coerced into Integer.",
            args[0]
        ))),
    }
}

macro_rules! define_cmp {
    ($op:ident) => {
        fn $op(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
            args.check_args_num(1)?;
            let lhs = self_val.to_real().unwrap();
            match args[0].to_real() {
                Some(rhs) => return Ok(Value::bool(lhs.$op(&rhs))),
                _ => {
                    return Err(RubyError::argument(format!(
                        "Comparison of Integer with {} failed.",
                        args[0].get_class_name()
                    )))
                }
            }
        }
    };
}

define_cmp!(ge);
define_cmp!(gt);
define_cmp!(le);
define_cmp!(lt);
