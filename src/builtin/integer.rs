use std::ops::{BitAnd, BitOr};

use crate::*;
use num::{bigint::ToBigInt, BigInt, Signed, Zero};

pub fn init() -> Value {
    let class = Module::class_under(BuiltinClass::numeric());
    BUILTINS.with(|m| m.borrow_mut().integer = class.into());
    BuiltinClass::set_toplevel_constant("Integer", class);
    class.add_builtin_method_by_str("+@", plus);
    class.add_builtin_method_by_str("-@", minus);
    class.add_builtin_method_by_str("div", quotient);
    class.add_builtin_method_by_str("==", eq);
    class.add_builtin_method_by_str("===", eq);
    class.add_builtin_method_by_str("!=", neq);
    class.add_builtin_method_by_str("<=>", cmp);
    class.add_builtin_method_by_str("[]", index);
    class.add_builtin_method_by_str(">>", shr);
    class.add_builtin_method_by_str("<<", shl);
    class.add_builtin_method_by_str("&", band);
    class.add_builtin_method_by_str("|", bor);

    class.add_builtin_method_by_str("times", times);
    class.add_builtin_method_by_str("upto", upto);
    class.add_builtin_method_by_str("downto", downto);
    class.add_builtin_method_by_str("step", step);
    class.add_builtin_method_by_str("chr", chr);
    class.add_builtin_method_by_str("to_f", tof);
    class.add_builtin_method_by_str("to_i", toi);
    class.add_builtin_method_by_str("to_int", toi);
    class.add_builtin_method_by_str("floor", floor);
    class.add_builtin_method_by_str("abs", abs);
    class.add_builtin_method_by_str("even?", even);
    class.add_builtin_method_by_str("odd?", odd);
    class.add_builtin_method_by_str("size", size);
    class.add_builtin_method_by_str("next", next);
    class.add_builtin_method_by_str("succ", next);

    class.add_builtin_method_by_str("_fixnum?", fixnum);
    class.into()
}

// Class methods

// Instance methods
fn plus(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(self_val)
}

fn minus(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let rec = self_val.to_real().unwrap();
    Ok((-rec).to_val())
}

fn quotient(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    match args[0].to_real() {
        Some(rhs) => {
            if rhs.is_zero() {
                return Err(RubyError::zero_div("Divided by zero."));
            }
            Ok(lhs.quotient(rhs).to_val())
        }
        None => Err(RubyError::undefined_op("div", args[0], self_val)),
    }
}

fn eq(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    match args[0].to_real() {
        Some(rhs) => Ok(Value::bool(lhs == rhs)),
        _ => Ok(Value::bool(false)),
    }
}

fn neq(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    match args[0].to_real() {
        Some(rhs) => Ok(Value::bool(lhs != rhs)),
        _ => Ok(Value::bool(true)),
    }
}

fn cmp(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    //use std::cmp::Ordering;
    args.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    let res = match args[0].to_real() {
        Some(rhs) => lhs.partial_cmp(&rhs),
        _ => return Ok(Value::nil()),
    };
    match res {
        Some(ord) => Ok(Value::integer(ord as i64)),
        None => Ok(Value::nil()),
    }
}

/// self[nth] -> Integer
/// NOT SUPPORTED: self[nth, len] -> Integer
/// NOT SUPPORTED: self[range] -> Integer
///
/// https://docs.ruby-lang.org/ja/latest/method/Integer/i/=5b=5d.html
fn index(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    if let Some(i) = self_val.as_fixnum() {
        if args[0].as_bignum().is_some() {
            return Ok(Value::integer(0));
        }
        let index = args[0].coerce_to_fixnum("Index")?;
        let val = if index < 0 || 63 < index {
            0
        } else {
            (i >> index) & 1
        };
        Ok(Value::integer(val))
    } else if let Some(i) = self_val.as_bignum() {
        if args[0].as_bignum().is_some() {
            return Ok(Value::integer(0));
        }
        let index = args[0].coerce_to_fixnum("Index")?;
        let val = (i >> index) & BigInt::from(1);
        Ok(Value::bignum(val))
    } else {
        unreachable!()
    }
}

fn shr(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    if let Some(lhs) = self_val.as_fixnum() {
        let rhs = args[0];
        match rhs.as_fixnum() {
            Some(rhs) => {
                if rhs >= 0 {
                    Ok(fixnum_shr(lhs, rhs))
                } else {
                    Ok(fixnum_shl(lhs, -rhs))
                }
            }
            None => shr_bignum(rhs),
        }
    } else if let Some(lhs) = self_val.as_bignum() {
        let rhs = args[0];
        match rhs.as_fixnum() {
            Some(rhs) => {
                if rhs >= 0 {
                    Ok(Value::bignum(lhs >> rhs))
                } else {
                    Ok(Value::bignum(lhs << -rhs))
                }
            }
            None => shr_bignum(rhs),
        }
    } else {
        unreachable!()
    }
}

fn shr_bignum(rhs: Value) -> VMResult {
    match rhs.as_bignum() {
        Some(rhs) => {
            if !rhs.is_negative() {
                Ok(Value::integer(0))
            } else {
                Err(RubyError::runtime("Shift width too big"))
            }
        }
        None => Err(RubyError::no_implicit_conv(rhs, "Integer")),
    }
}

fn shl(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    if let Some(lhs) = self_val.as_fixnum() {
        let rhs = args[0];
        match rhs.as_fixnum() {
            Some(rhs) => {
                if rhs >= 0 {
                    Ok(fixnum_shl(lhs, rhs))
                } else {
                    Ok(fixnum_shr(lhs, -rhs))
                }
            }
            None => shl_bignum(rhs),
        }
    } else if let Some(lhs) = self_val.as_bignum() {
        let rhs = args[0];
        match rhs.as_fixnum() {
            Some(rhs) => {
                if rhs >= 0 {
                    Ok(Value::bignum(lhs << rhs))
                } else {
                    Ok(Value::bignum(lhs >> -rhs))
                }
            }
            None => shl_bignum(rhs),
        }
    } else {
        unreachable!()
    }
}

fn shl_bignum(rhs: Value) -> VMResult {
    match rhs.as_bignum() {
        Some(rhs) => {
            if rhs.is_negative() {
                Ok(Value::integer(0))
            } else {
                Err(RubyError::runtime("Shift width too big"))
            }
        }
        None => Err(RubyError::no_implicit_conv(rhs, "Integer")),
    }
}

/// rhs must be a non-negative value.
fn fixnum_shr(lhs: i64, rhs: i64) -> Value {
    if rhs < u32::MAX as i64 {
        match lhs.checked_shr(rhs as u32) {
            Some(i) => Value::integer(i),
            None => Value::integer(0),
        }
    } else {
        Value::bignum(lhs.to_bigint().unwrap() >> rhs)
    }
}

/// rhs must be a non-negative value.
fn fixnum_shl(lhs: i64, rhs: i64) -> Value {
    if rhs < u32::MAX as i64 {
        match lhs.checked_shl(rhs as u32) {
            Some(i) => Value::integer(i),
            None => {
                let n = lhs.to_bigint().unwrap() << rhs;
                Value::bignum(n)
            }
        }
    } else {
        Value::bignum(lhs.to_bigint().unwrap() << rhs)
    }
}

macro_rules! bit_ops {
    ($fname:ident, $op:ident) => {
        fn $fname(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
            args.check_args_num(1)?;
            if let Some(lhs) = self_val.as_fixnum() {
                let rhs = args[0];
                match rhs.as_fixnum() {
                    Some(rhs) => Ok(Value::integer(lhs.$op(rhs))),
                    None => match rhs.as_bignum() {
                        Some(rhs) => Ok(Value::bignum(lhs.to_bigint().unwrap().$op(rhs))),
                        None => Err(RubyError::no_implicit_conv(rhs, "Integer")),
                    },
                }
            } else if let Some(lhs) = self_val.as_bignum() {
                let rhs = match args[0].as_fixnum() {
                    Some(rhs) => rhs.to_bigint().unwrap(),
                    None => match args[0].as_bignum() {
                        Some(rhs) => rhs,
                        None => return Err(RubyError::no_implicit_conv(args[0], "Integer")),
                    },
                };
                Ok(Value::bignum(lhs.$op(rhs)))
            } else {
                unreachable!()
            }
        }
    };
}

bit_ops!(band, bitand);
bit_ops!(bor, bitor);

fn times(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let block = match &args.block {
        None => {
            let id = IdentId::get_id("times");
            let val = vm.create_enumerator(id, self_val, args.clone())?;
            return Ok(val);
        }
        Some(block) => block,
    };
    if let Some(num) = self_val.as_fixnum() {
        if num < 1 {
            return Ok(self_val);
        };
        let iter = (0..num).map(|i| Value::integer(i));
        vm.eval_block_each1(block, iter, self_val)
    } else if let Some(num) = self_val.as_bignum() {
        if !num.is_positive() {
            return Ok(self_val);
        };
        let iter = num::range(BigInt::zero(), num).map(|num| Value::bignum(num));
        vm.eval_block_each1(block, iter, self_val)
    } else {
        unreachable!()
    }
}

/// Integer#upto(min) { |n| .. } -> self
/// Integer#upto(min) -> Enumerator
/// https://docs.ruby-lang.org/ja/latest/method/Integer/i/upto.html
fn upto(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let block = match &args.block {
        None => {
            let id = IdentId::get_id("upto");
            let val = vm.create_enumerator(id, self_val, args.clone())?;
            return Ok(val);
        }
        Some(block) => block,
    };
    let num = self_val.as_fixnum().unwrap();
    let max = args[0].coerce_to_fixnum("Arg")?;
    if num <= max {
        let iter = (num..max + 1).map(|i| Value::integer(i));
        vm.eval_block_each1(block, iter, self_val)
    } else {
        Ok(self_val)
    }
}

/// Integer#downto(min) { |n| .. } -> self
/// Integer#downto(min) -> Enumerator
/// https://docs.ruby-lang.org/ja/latest/method/Integer/i/downto.html
fn downto(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let block = match &args.block {
        None => {
            let id = IdentId::get_id("downto");
            let val = vm.create_enumerator(id, self_val, args.clone())?;
            return Ok(val);
        }
        Some(block) => block,
    };
    let num = self_val.as_fixnum().unwrap();
    let min = args[0].coerce_to_fixnum("Arg")?;
    if num >= min {
        let iter = (min..num + 1).rev().map(|i| Value::integer(i));
        vm.eval_block_each1(block, iter, self_val)
    } else {
        Ok(self_val)
    }
}

struct Step {
    cur: i64,
    limit: i64,
    step: i64,
}

impl Iterator for Step {
    type Item = Value;
    fn next(&mut self) -> Option<Self::Item> {
        if self.step > 0 && self.cur > self.limit || self.step < 0 && self.limit > self.cur {
            None
        } else {
            let v = Value::integer(self.cur);
            self.cur += self.step;
            Some(v)
        }
    }
}

fn step(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 2)?;
    let block = match &args.block {
        None => {
            let id = IdentId::get_id("step");
            let val = vm.create_enumerator(id, self_val, args.clone())?;
            return Ok(val);
        }
        Some(block) => block,
    };
    let start = self_val.as_fixnum().unwrap();
    let limit = args[0].coerce_to_fixnum("Limit")?;
    let step = if args.len() == 2 {
        let step = args[1].coerce_to_fixnum("Step")?;
        if step == 0 {
            return Err(RubyError::argument("Step can not be 0."));
        }
        step
    } else {
        1
    };

    let iter = Step {
        cur: start,
        step,
        limit,
    };
    vm.eval_block_each1(block, iter, self_val)
}

/// Built-in function "chr".
fn chr(_: &mut VM, self_val: Value, _: &Args) -> VMResult {
    if let Some(num) = self_val.as_fixnum() {
        if 0 > num || num > 255 {
            return Err(RubyError::range(format!("{} Out of char range.", num)));
        };
        Ok(Value::bytes(vec![num as u8]))
    } else {
        return Err(RubyError::range(format!(
            "{:?} Out of char range.",
            self_val
        )));
    }
}

fn floor(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    Ok(self_val)
}

fn abs(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    if let Some(num) = self_val.as_fixnum() {
        Ok(Value::integer(num.abs()))
    } else if let Some(num) = self_val.as_bignum() {
        Ok(Value::bignum(num.abs()))
    } else {
        unreachable!()
    }
}

fn tof(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let num = self_val.to_real().unwrap();
    Ok(Value::float(num.to_f64()))
}

fn toi(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(self_val)
}

fn even(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let num = self_val.to_real().unwrap();
    Ok(Value::bool((num % Real::Integer(2)).is_zero()))
}

fn odd(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let num = self_val.to_real().unwrap();
    Ok(Value::bool(!(num % Real::Integer(2)).is_zero()))
}

fn size(_: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(Value::integer(8))
}

fn next(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    if let Some(i) = self_val.as_fixnum() {
        match i.checked_add(1) {
            Some(i) => Ok(Value::integer(i)),
            None => Ok(Value::bignum(BigInt::from(i) + 1)),
        }
    } else if let Some(n) = self_val.as_bignum() {
        Ok(Value::bignum(n + 1))
    } else {
        unreachable!()
    }
}

fn fixnum(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let b = self_val.as_fixnum().is_some();
    Ok(Value::bool(b))
}

#[cfg(test)]
mod tests {
    use crate::tests::*;
    #[test]
    fn integer1() {
        let program = r#"
        assert("777", 777.inspect)
        assert("777", 777.to_s)
        assert(4.0, 4.to_f)
        assert(-4.0, -4.to_f)
        assert(4, 4.floor)
        assert(-4, -4.floor)

        assert(true, 8.even?)
        assert(false, 8.odd?)
        assert(false, 9.even?)
        assert(true, 9.odd?)

        assert(true, 99999999999999999999999999999998.even?)
        assert(false, 99999999999999999999999999999998.odd?)
        assert(false, 99999999999999999999999999999999.even?)
        assert(true, 99999999999999999999999999999999.odd?)

        assert 6, 6.abs
        assert 6, -6.abs

        assert 8888888888855555555555555777777776, 8888888888855555555555555777777776.abs
        assert 8888888888855555555555555777777776, -8888888888855555555555555777777776.abs

        assert(8, 1.size)

        assert 19, 18.next
        assert 7777777777777777777777777777, 7777777777777777777777777776.next
        assert true, 0x3fff_ffff_ffff_ffff._fixnum?
        assert true, 0x3fff_ffff_ffff_ffff.next == 4611686018427387904
        assert false, 0x3fff_ffff_ffff_ffff.next._fixnum?
        "#;
        assert_script(program);
    }

    #[test]
    fn integer_ops() {
        let program = r#"
        assert 999999999999999999999999999 * -1, -999999999999999999999999999
        assert 999999999999999999999999999, +999999999999999999999999999

        assert 30, 25.+ 5
        assert 20, 25.- 5
        assert 125, 25.* 5
        assert 5, 25./ 5
        assert 30, 25.+5
        assert 20, 25.-5
        assert 125, 25.*5
        assert 5, 25./5

        assert 4000000000000000000000000, 2000000000000000000000000+2000000000000000000000000
        assert 2000000000000000000000002, 2000000000000000000000000+2
        assert 2000000000000000000000002, 2+2000000000000000000000000
        assert 1000000000000000000000000, 3000000000000000000000000-2000000000000000000000000
        assert 2999999999999999999999998, 3000000000000000000000000-2
        assert -2999999999999999999999998, 2-3000000000000000000000000
        assert 8000000000000000000000000, 4*2000000000000000000000000
        assert 8000000000000000000000000, 2000000000000000000000000*4
        assert 2000000000000000000000000, 8000000000000000000000000/4
        assert 3,6000000000000000000000000/2000000000000000000000000

        assert 8+4i, 5+(3+4i)
        assert 2-4i, 5-(3+4i)
        assert 15+20i, 5*(3+4i)
        assert 0.6-0.8i, 5/(3+4i)

        assert 475, +(475)
        assert 475, 475.+@
        assert -475, -(475)
        assert -475, 475.-@
        "#;
        assert_script(program);
    }

    #[test]
    fn integer_bitwise_ops() {
        let program = r#"
        assert 79, 75487.& 111
        assert 79, 75487 & 111
        assert 110, 9999999999999999999999999999999998.& 111
        assert 110, 9999999999999999999999999999999998 & 111
        assert 110, 111.& 9999999999999999999999999999999998
        assert 110, 111 & 9999999999999999999999999999999998
        assert 6665389879227453860303075412000, 6666666666666666666666666666666.& 77777777777777777777777777777777
        assert 6665389879227453860303075412000, 6666666666666666666666666666666 & 77777777777777777777777777777777

        assert 75519, 75487.| 111
        assert 75519, 75487 | 111
        assert 9999999999999555555599999999999999, 9999999999999555555599999999999888.| 111
        assert 9999999999999555555599999999999999, 9999999999999555555599999999999888 | 111
        assert 9999999999999555555599999999999999, 111.| 9999999999999555555599999999999888
        assert 9999999999999555555599999999999999, 111 | 9999999999999555555599999999999888
        assert 77779054565216990584141369032443, 6666666666666666666666666666666.| 77777777777777777777777777777777
        assert 77779054565216990584141369032443, 6666666666666666666666666666666 | 77777777777777777777777777777777
        "#;
        assert_script(program);
    }

    #[test]
    fn integer_overflow() {
        let program = r#"
        assert true, 0x3fff_ffff_ffff_ffff._fixnum?
        assert true, 0x3fff_ffff_ffff_ffff + 1 == 4611686018427387904
        assert false, (0x3fff_ffff_ffff_ffff + 1)._fixnum?
        assert true, 0x3fff_ffff_ffff_ffff * 5 == 23058430092136939515
        assert false, (0x3fff_ffff_ffff_ffff * 5)._fixnum?
        assert true, 0x3fff_ffff_ffff_ffff == 0x3fff_ffff_ffff_ffff * 5 / 5
        assert true, (0x3fff_ffff_ffff_ffff * 5 / 5)._fixnum?

        assert true, (-0x4000_0000_0000_0000)._fixnum?
        assert true, -0x4000_0000_0000_0000 - 1 == -4611686018427387905
        assert false, (-0x4000_0000_0000_0000 - 1)._fixnum?
        assert true, (-0x4000_0000_0000_0000 - 1 + 1)._fixnum?
        assert true, -0x4000_0000_0000_0000 == -0x4000_0000_0000_0000 - 1 + 1

        "#;
        assert_script(program);
    }

    #[test]
    fn integer_cmp() {
        let program = r#"
            assert true, 4.== 4
            assert false, 4.== 14
            assert false, 4.== "4"
            assert true, 4.== 4.0
            assert false, 4.== 4.1

            assert false, 4.!= 4
            assert true, 4.!= 14
            assert true, 4.!= "4"
            assert false, 4.!= 4.0
            assert true, 4.!= 4.1

            assert true, 4.>= -1
            assert true, 4.>= 4
            assert false, 4.>= 14
            assert true, 4.>= 3.9
            assert true, 4.>= 4.0
            assert false, 4.>= 4.1

            assert false, 4.<= -1
            assert true, 4.<= 4
            assert true, 4.<= 14
            assert false, 4.<= 3.9
            assert true, 4.<= 4.0
            assert true, 4.<= 4.1

            assert true, 4.> -1
            assert false, 4.> 4
            assert false, 4.> 14
            assert true, 4.> 3.9
            assert false, 4.> 4.0
            assert false, 4.> 4.1

            assert false, 4.< -1
            assert false, 4.< 4
            assert true, 4.< 14
            assert false, 4.< 3.9
            assert false, 4.< 4.0
            assert true, 4.< 4.1

            assert 0, 3.<=> 3
            assert 1, 5.<=> 3
            assert -1, 3.<=> 5
            assert 0, 3.<=> 3.0
            assert 1, 5.<=> 3.9
            assert -1, 3.<=> 5.8
            assert nil, 3.<=> "three"
            assert nil, Float::NAN.<=> Float::NAN

            assert 0, 3 <=> 3
            assert 1, 5 <=> 3
            assert -1, 3 <=> 5
            assert 0, 3 <=> 3.0
            assert 1, 5 <=> 3.9
            assert -1, 3 <=> 5.8
            assert nil, 3 <=> "three"
        "#;
        assert_script(program);
    }
    #[test]
    fn integer_shift() {
        let program = r#"
        assert 4, 19>>2
        assert 4, 19<<-2
        assert 2383614482228453421613056, 2019<<70
        assert 2383614482228453421613056, 2019>>-70
        assert 76, 19<<2
        assert 76, 19>>-2
        assert 2019, 2383614482228453421613056>>70
        assert 2019, 2383614482228453421613056<<-70

        assert 4, 19.>>2
        assert 4, 19.<<(-2)
        assert 2383614482228453421613056, 2019.<<70
        assert 2383614482228453421613056, 2019.>>(-70)
        assert 76, 19.<<2
        assert 76, 19.>>(-2)
        assert 2019, 2383614482228453421613056.>>70
        assert 2019, 2383614482228453421613056.<<(-70)

        assert 8785458905193172896, (0x7f3d870a761a99f4 << 3) & 0x7fffffffffffffff
        assert 1146079111924634430, 0x7f3d870a761a99f4 >> 3

        big = 999999999999999999999999999999999
        assert 0, 100 >> big
        assert 0, 100 << -big
        assert 0, big >> big
        assert 0, big << -big
        assert_error { 100 << big }
        assert_error { 100 >> -big }

        "#;
        assert_script(program);
    }

    #[test]
    fn integer_times() {
        let program = r#"
        res = []
        assert 5, 5.times {|x| res[x] = x * x}
        assert [0,1,4,9,16], res
        res = []
        assert 0, 0.times {|x| res[x] = x * x}
        assert [], res
        assert -100, -100.times { break 77 }

        assert 1001, 9999999999999999999999999.times {|x| break x if x > 1000}
        assert -9999999999999999999999999, -9999999999999999999999999.times {|x| break x if x > 1000}
        "#;
        assert_script(program);
    }

    #[test]
    fn integer_upto() {
        let program = r#"
        res = []
        assert 5, 5.upto(8) { |x| res << x * x }
        assert [25, 36, 49, 64], res
        res = []
        assert 5, 5.upto(4) { |x| res << x * x }
        assert [], res
        enum = 5.upto(8)
        assert [10, 12, 14, 16], enum.map{ |x| x * 2 }

        res = []
        assert 5, 5.downto(2) { |x| res << x }
        assert [5, 4, 3, 2], res
        res = []
        assert 5, 5.downto(8) { |x| res << x * x }
        assert [], res
        enum = 5.downto(2)
        assert [10, 8, 6, 4], enum.map{ |x| x * 2 }
        "#;
        assert_script(program);
    }

    #[test]
    fn integer_step() {
        let program = r#"
        res = 0
        4.step(20){|x| res += x}
        assert(204, res)
        res = 0
        4.step(20, 3){|x| res += x}
        assert(69, res)

        res = 0
        enum = 4.step(20, 3)
        enum.each{|x| res += x}
        assert(69, res)

        res = 0
        assert_error { 4.step(20, 0){|x| res += x} }
        "#;
        assert_script(program);
    }

    #[test]
    fn integer_quotient() {
        let program = r#"
        assert 1, 3.div(2)
        assert 1, 3.div(2.0)
        assert_error { 3.div(0.0) }
        assert_error { 3.div("Ruby") }
        assert -2, (-3).div(2)
        assert -2, (-3).div(2.0)
        "#;
        assert_script(program);
    }

    #[test]
    fn integer_index() {
        let program = r#"
        assert 0, 999999999999999999999999999999999[47]
        assert 1, 999999999999999999999999999999999[48]
        assert 1, 999999999999999999999999999999999[82]
        assert 0, 999999999999999999999999999999999[85]
        assert 1, 999999999999999999999999999999999[86]
        assert 0, 999999999999999999999999999999999[999999999999999999999999999999999999]
        assert 0, 27[2]
        assert 1, 27[0]
        assert 0, 27[999999999999999999999999999999999]
        "#;
        assert_script(program);
    }
}
