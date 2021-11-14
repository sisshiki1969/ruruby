use crate::*;
use num::Integer;
use num::{bigint::ToBigInt, BigInt, Signed, ToPrimitive, Zero};
use std::convert::TryInto;
use std::ops::{BitAnd, BitOr};

pub(crate) fn init(globals: &mut Globals) -> Value {
    let class = Module::class_under(BuiltinClass::numeric());
    BUILTINS.with(|m| m.borrow_mut().integer = class.into());
    globals.set_toplevel_constant("Integer", class);
    globals.set_toplevel_constant("Fixnum", class);
    globals.set_toplevel_constant("Bignum", class);
    class.add_builtin_method_by_str(globals, "%", rem);
    class.add_builtin_method_by_str(globals, "**", exp);
    class.add_builtin_method_by_str(globals, "pow", exp);
    class.add_builtin_method_by_str(globals, "+@", plus);
    class.add_builtin_method_by_str(globals, "-@", minus);
    class.add_builtin_method_by_str(globals, "div", quotient);
    class.add_builtin_method_by_str(globals, "fdiv", fdiv);
    class.add_builtin_method_by_str(globals, "==", eq);
    class.add_builtin_method_by_str(globals, "===", eq);
    class.add_builtin_method_by_str(globals, "!=", neq);
    class.add_builtin_method_by_str(globals, "<=>", cmp);
    class.add_builtin_method_by_str(globals, "[]", index);
    class.add_builtin_method_by_str(globals, ">>", shr);
    class.add_builtin_method_by_str(globals, "<<", shl);
    class.add_builtin_method_by_str(globals, "&", band);
    class.add_builtin_method_by_str(globals, "|", bor);

    class.add_builtin_method_by_str(globals, "abs", abs);
    class.add_builtin_method_by_str(globals, "floor", floor);
    class.add_builtin_method_by_str(globals, "even?", even);
    class.add_builtin_method_by_str(globals, "odd?", odd);
    class.add_builtin_method_by_str(globals, "gcd", gcd);
    class.add_builtin_method_by_str(globals, "lcm", lcm);
    class.add_builtin_method_by_str(globals, "gcdlcm", gcdlcm);

    class.add_builtin_method_by_str(globals, "times", times);
    class.add_builtin_method_by_str(globals, "upto", upto);
    class.add_builtin_method_by_str(globals, "downto", downto);
    class.add_builtin_method_by_str(globals, "step", step);

    class.add_builtin_method_by_str(globals, "chr", chr);
    class.add_builtin_method_by_str(globals, "ord", ord);
    class.add_builtin_method_by_str(globals, "bit_length", bit_length);
    class.add_builtin_method_by_str(globals, "to_f", tof);
    class.add_builtin_method_by_str(globals, "to_i", toi);
    class.add_builtin_method_by_str(globals, "to_int", toi);
    class.add_builtin_method_by_str(globals, "size", size);
    class.add_builtin_method_by_str(globals, "next", next);
    class.add_builtin_method_by_str(globals, "succ", next);
    class.add_builtin_method_by_str(globals, "pred", pred);
    class.add_builtin_method_by_str(globals, "digits", digits);

    class.add_builtin_method_by_str(globals, "_fixnum?", fixnum);
    class.into()
}

// Class methods

// Instance methods

fn rem(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    if let Some(n) = self_val.as_bignum() {
        arith::rem_bignum(n, vm[0])
    } else if let Some(i) = self_val.as_fixnum() {
        arith::rem_fixnum(i, vm[0])
    } else {
        unreachable!()
    }
}

fn exp(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    if let Some(n) = self_val.as_bignum() {
        if let Some(rhsi) = vm[0].as_fixnum() {
            if let Ok(rhsu) = rhsi.try_into() {
                Ok(Value::bignum(n.pow(rhsu)))
            } else {
                Ok(Value::float(n.to_f64().unwrap().powf(rhsi as f64)))
            }
        } else if let Some(f) = vm[0].as_float() {
            Ok(Value::float(n.to_f64().unwrap().powf(f)))
        } else if let Some(b) = vm[0].as_bignum() {
            Ok(Value::float(n.to_f64().unwrap().powf(b.to_f64().unwrap())))
        } else {
            Err(VMError::cant_coerse(vm[0], "Integer"))
        }
    } else if let Some(i) = self_val.as_fixnum() {
        arith::exp_fixnum(i, vm[0])
    } else {
        unreachable!()
    }
}

fn plus(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    Ok(self_val)
}

fn minus(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let rec = self_val.to_real().unwrap();
    Ok((-rec).into_val())
}

fn fdiv(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    match vm[0].to_real() {
        Some(rhs) => Ok(Value::float(lhs.to_f64() / rhs.to_f64())),
        None => Err(VMError::cant_coerse(vm[0], "Numeric")),
    }
}

fn quotient(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    match vm[0].to_real() {
        Some(rhs) => {
            if rhs.is_zero() {
                return Err(RubyError::zero_div("Divided by zero."));
            }
            Ok(lhs.quotient(rhs).into_val())
        }
        None => Err(VMError::cant_coerse(vm[0], "Numeric")),
    }
}

fn eq(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    match vm[0].to_real() {
        Some(rhs) => Ok(Value::bool(lhs == rhs)),
        _ => Ok(Value::bool(false)),
    }
}

fn neq(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let lhs = self_val.to_real().unwrap();
    match vm[0].to_real() {
        Some(rhs) => Ok(Value::bool(lhs != rhs)),
        _ => Ok(Value::bool(true)),
    }
}

fn cmp(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    if let Some(i) = self_val.as_fixnum() {
        Ok(Value::from_ord(arith::cmp_fixnum(i, vm[0])))
    } else if let Some(n) = self_val.as_bignum() {
        Ok(Value::from_ord(arith::cmp_bignum(&n, vm[0])))
    } else {
        unreachable!()
    }
}

/// self[nth] -> Integer
/// NOT SUPPORTED: self[nth, len] -> Integer
/// NOT SUPPORTED: self[range] -> Integer
///
/// https://docs.ruby-lang.org/ja/latest/method/Integer/i/=5b=5d.html
fn index(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    if let Some(i) = self_val.as_fixnum() {
        if vm[0].as_bignum().is_some() {
            return Ok(Value::integer(0));
        }
        let index = vm[0].coerce_to_fixnum("Index")?;
        let val = if index < 0 || 63 < index {
            0
        } else {
            (i >> index) & 1
        };
        Ok(Value::integer(val))
    } else if let Some(i) = self_val.as_bignum() {
        if vm[0].as_bignum().is_some() {
            return Ok(Value::integer(0));
        }
        let index = vm[0].coerce_to_fixnum("Index")?;
        let val = if index < 0 {
            BigInt::from(0)
        } else {
            (i >> index) & BigInt::from(1)
        };
        Ok(Value::bignum(val))
    } else {
        unreachable!()
    }
}

fn shr(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let rhs = vm[0];
    if let Some(lhs) = self_val.as_fixnum() {
        match rhs.as_fixnum() {
            Some(rhs) => arith::shr_fixnum(lhs, rhs),
            None => shr_bignum(rhs),
        }
    } else if let Some(lhs) = self_val.as_bignum() {
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
        None => Err(VMError::no_implicit_conv(rhs, "Integer")),
    }
}

fn shl(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let rhs = vm[0];
    if let Some(lhs) = self_val.as_fixnum() {
        match rhs.as_fixnum() {
            Some(rhs) => arith::shl_fixnum(lhs, rhs),
            None => shl_bignum(rhs),
        }
    } else if let Some(lhs) = self_val.as_bignum() {
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
        None => Err(VMError::no_implicit_conv(rhs, "Integer")),
    }
}

macro_rules! bit_ops {
    ($fname:ident, $op:ident) => {
        fn $fname(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
            vm.check_args_num(1)?;
            let rhs = vm[0];
            if let Some(lhs) = self_val.as_fixnum() {
                match rhs.as_fixnum() {
                    Some(rhs) => Ok(Value::integer(lhs.$op(rhs))),
                    None => match rhs.as_bignum() {
                        Some(rhs) => Ok(Value::bignum(lhs.to_bigint().unwrap().$op(rhs))),
                        None => Err(VMError::no_implicit_conv(rhs, "Integer")),
                    },
                }
            } else if let Some(lhs) = self_val.as_bignum() {
                let res = match rhs.as_fixnum() {
                    Some(rhs) => lhs.$op(&rhs.to_bigint().unwrap()),
                    None => match rhs.as_bignum() {
                        Some(rhs) => lhs.$op(rhs),
                        None => return Err(VMError::no_implicit_conv(rhs, "Integer")),
                    },
                };
                Ok(Value::bignum(res))
            } else {
                unreachable!()
            }
        }
    };
}

bit_ops!(band, bitand);
bit_ops!(bor, bitor);

#[test]
fn integer_bitwise_ops() {
    use crate::tests::*;
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

fn times(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let block = match &args.block {
        None => {
            let id = IdentId::get_id("times");
            let val = vm.create_enumerator(id, self_val, args.into(vm))?;
            return Ok(val);
        }
        Some(block) => block,
    };
    if let Some(num) = self_val.as_fixnum() {
        if num < 1 {
            return Ok(self_val);
        };
        let iter = (0..num).map(|i| Value::fixnum(i));
        vm.eval_block_each1(block, iter, self_val)
        /*for v in (0..num).map(|i| Value::integer(i)) {
            vm.eval_block(block, &Args::new1(v))?;
        }*/
    } else if let Some(num) = self_val.as_bignum() {
        if !num.is_positive() {
            return Ok(self_val);
        };
        let iter = num::range(BigInt::zero(), num.clone()).map(|num| Value::bignum(num));
        vm.eval_block_each1(block, iter, self_val)
        /*for v in num::range(BigInt::zero(), num.clone()).map(|num| Value::bignum(num)) {
            vm.eval_block(block, &Args::new1(v))?;
        }*/
    } else {
        unreachable!()
    }
}

/// Integer#upto(min) { |n| .. } -> self
/// Integer#upto(min) -> Enumerator
/// https://docs.ruby-lang.org/ja/latest/method/Integer/i/upto.html
fn upto(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let block = match &args.block {
        None => {
            let id = IdentId::get_id("upto");
            let val = vm.create_enumerator(id, self_val, args.into(vm))?;
            return Ok(val);
        }
        Some(block) => block,
    };
    let num = self_val.as_fixnum().unwrap();
    let max = vm[0].coerce_to_fixnum("Arg")?;
    if num <= max {
        let iter = (num..max + 1).map(|i| Value::integer(i));
        vm.eval_block_each1(block, iter, self_val)
        //Ok(self_val)
    } else {
        Ok(self_val)
    }
}

/// Integer#downto(min) { |n| .. } -> self
/// Integer#downto(min) -> Enumerator
/// https://docs.ruby-lang.org/ja/latest/method/Integer/i/downto.html
fn downto(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let block = match &args.block {
        None => {
            let id = IdentId::get_id("downto");
            let val = vm.create_enumerator(id, self_val, args.into(vm))?;
            return Ok(val);
        }
        Some(block) => block,
    };
    let num = self_val.as_fixnum().unwrap();
    let min = vm[0].coerce_to_fixnum("Arg")?;
    if num >= min {
        let iter = (min..num + 1).rev().map(|i| Value::integer(i));
        //Ok(self_val)
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

fn step(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_range(1, 2)?;
    let block = match &args.block {
        None => {
            let id = IdentId::get_id("step");
            let val = vm.create_enumerator(id, self_val, args.into(vm))?;
            return Ok(val);
        }
        Some(block) => block,
    };
    let start = self_val.as_fixnum().unwrap();
    let limit = vm[0].coerce_to_fixnum("Limit")?;
    let step = if args.len() == 2 {
        let step = vm[1].coerce_to_fixnum("Step")?;
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
    //for v in iter {
    //    vm.eval_block(block, &Args::new1(v))?;
    //}
    vm.eval_block_each1(block, iter, self_val)
    //Ok(self_val)
}

/// Built-in function "chr".
fn chr(_: &mut VM, self_val: Value, _: &Args2) -> VMResult {
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

/// ord -> Integer
///
/// https://docs.ruby-lang.org/ja/latest/method/Integer/i/ord.html
fn ord(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    Ok(self_val)
}

/// bit_length -> Integer
///
/// https://docs.ruby-lang.org/ja/latest/method/Integer/i/bit_length.html
fn bit_length(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let bits = if let Some(i) = self_val.as_fixnum() {
        if i >= 0 {
            64 - i.leading_zeros()
        } else {
            64 - i.leading_ones()
        }
    } else if let Some(b) = self_val.as_bignum() {
        let all = ((b.bits() + 63) / 64 * 64) as usize;
        let lead = if !b.is_negative() {
            (0..all).into_iter().find(|x| b.bit((all - *x) as u64))
        } else {
            (0..all).into_iter().find(|x| !b.bit((all - *x) as u64))
        }
        .unwrap_or(all) as u32;
        all as u32 - lead + 1
    } else {
        unreachable!()
    };
    Ok(Value::integer(bits as i64))
}

#[test]
fn integer_bit_length() {
    use crate::tests::*;
    let program = r#"
    assert 65, (-2**64-1).bit_length
    assert 64, (-2**64).bit_length  
    assert 64, (-2**64+1).bit_length
    assert 13, (-2**12-1).bit_length
    assert 12, (-2**12).bit_length  
    assert 12, (-2**12+1).bit_length
    assert 9, -0x101.bit_length  
    assert 8, -0x100.bit_length  
    assert 8, -0xff.bit_length   
    assert 1, -2.bit_length      
    assert 0, -1.bit_length      
    assert 0, 0.bit_length       
    assert 1, 1.bit_length       
    assert 8, 0xff.bit_length    
    assert 9, 0x100.bit_length   
    assert 12, (2**12-1).bit_length
    assert 13, (2**12).bit_length  
    assert 13, (2**12+1).bit_length
    assert 64, (2**64-1).bit_length
    assert 65, (2**64).bit_length  
    assert 65, (2**64+1).bit_length
    "#;
    assert_script(program);
}

fn floor(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_range(0, 1)?;
    Ok(self_val)
}

fn abs(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_range(0, 1)?;
    if let Some(num) = self_val.as_fixnum() {
        Ok(Value::integer(num.abs()))
    } else if let Some(num) = self_val.as_bignum() {
        Ok(Value::bignum(num.abs()))
    } else {
        unreachable!()
    }
}

fn tof(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let num = self_val.to_real().unwrap();
    Ok(Value::float(num.to_f64()))
}

fn toi(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    Ok(self_val)
}

/// even? -> bool
///
/// https://docs.ruby-lang.org/ja/latest/method/Integer/i/even=3f.html
fn even(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    if let Some(i) = self_val.as_fixnum() {
        Ok(Value::bool(i.is_even()))
    } else if let Some(b) = self_val.as_bignum() {
        Ok(Value::bool(b.is_even()))
    } else {
        unreachable!()
    }
}

/// odd? -> bool
///
/// https://docs.ruby-lang.org/ja/latest/method/Integer/i/odd=3f.html
fn odd(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    if let Some(i) = self_val.as_fixnum() {
        Ok(Value::bool(i.is_odd()))
    } else if let Some(b) = self_val.as_bignum() {
        Ok(Value::bool(b.is_odd()))
    } else {
        unreachable!()
    }
}

/// gcd(n) -> Integer
///
/// https://docs.ruby-lang.org/ja/latest/method/Integer/i/gcd.html
fn gcd(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let rhs = vm[0];
    if let Some(i) = self_val.as_fixnum() {
        let res = if let Some(b2) = rhs.as_bignum() {
            Value::bignum(BigInt::from(i).gcd(b2))
        } else if let Some(i2) = rhs.as_fixnum() {
            arith::safe_gcd(&i, &i2)
        } else {
            return Err(VMError::cant_coerse(rhs, "Integer"));
        };
        Ok(res)
    } else if let Some(b) = self_val.as_bignum() {
        let res = if let Some(b2) = rhs.as_bignum() {
            b.gcd(b2)
        } else if let Some(i2) = rhs.as_fixnum() {
            b.gcd(&BigInt::from(i2))
        } else {
            return Err(VMError::cant_coerse(rhs, "Integer"));
        };
        Ok(Value::bignum(res))
    } else {
        unreachable!()
    }
}

/// lcm(n) -> Integer
///
/// https://docs.ruby-lang.org/ja/latest/method/Integer/i/lcm.html
fn lcm(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let rhs = vm[0];
    if let Some(i) = self_val.as_fixnum() {
        let res = if let Some(b2) = rhs.as_bignum() {
            Value::bignum(BigInt::from(i).lcm(b2))
        } else if let Some(i2) = rhs.as_fixnum() {
            arith::safe_lcm(&i, &i2)
        } else {
            return Err(VMError::cant_coerse(rhs, "Integer"));
        };
        Ok(res)
    } else if let Some(b) = self_val.as_bignum() {
        let res = if let Some(b2) = rhs.as_bignum() {
            b.lcm(b2)
        } else if let Some(i2) = rhs.as_fixnum() {
            b.lcm(&BigInt::from(i2))
        } else {
            return Err(VMError::cant_coerse(rhs, "Integer"));
        };
        Ok(Value::bignum(res))
    } else {
        unreachable!()
    }
}

/// gcdlcm(n) -> [Integer]
///
/// https://docs.ruby-lang.org/ja/latest/method/Integer/i/gcdlcm.html
fn gcdlcm(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let rhs = vm[0];
    if let Some(i) = self_val.as_fixnum() {
        let res = if let Some(b2) = rhs.as_bignum() {
            let (gcd, lcm) = BigInt::from(i).gcd_lcm(b2);
            vec![Value::bignum(gcd), Value::bignum(lcm)]
        } else if let Some(i2) = rhs.as_fixnum() {
            let (gcd, lcm) = arith::safe_gcd_lcm(&i, &i2);
            vec![gcd, lcm]
        } else {
            return Err(VMError::cant_coerse(rhs, "Integer"));
        };
        Ok(Value::array_from(res))
    } else if let Some(b) = self_val.as_bignum() {
        let res = if let Some(b2) = rhs.as_bignum() {
            b.gcd_lcm(b2)
        } else if let Some(i2) = rhs.as_fixnum() {
            b.gcd_lcm(&BigInt::from(i2))
        } else {
            return Err(VMError::cant_coerse(rhs, "Integer"));
        };
        Ok(Value::array_from(vec![
            Value::bignum(res.0),
            Value::bignum(res.1),
        ]))
    } else {
        unreachable!()
    }
}

#[test]
fn integer_gcd_lcm() {
    use crate::tests::*;
    let program = r#"
    assert 49, 49.gcd(245)                  
    assert 49, 49.gcd(-245)                  
    assert 1, 3.gcd(7)                  
    assert 1, 3.gcd(-7)                 
    assert 1, ((1<<31)-1).gcd((1<<61)-1)
    assert 4722366482869645213695, ((1<<72)-1).gcd((1<<144)-1)
    assert 9, 123456789.gcd((1<<144)-1)
    assert 9, ((1<<144)-1).gcd(123456789)

    assert 175, -175.gcd 0 
    assert 175, 0.gcd 175
    assert 0, 0.gcd 0

    assert 2, 2.lcm(2)
    assert 21, 3.lcm(-7)
    assert 4951760154835678088235319297, ((1<<31)-1).lcm((1<<61)-1)
    assert 284671973855620070019183339, 123456789.lcm((1<<61)-1)
    assert 284671973855620070019183339, ((1<<61)-1).lcm(123456789)
    assert 0, 3.lcm(0)
    assert 0, 0.lcm(-7)
    assert 0, 0.lcm(0)

    assert [1,21], 7.gcdlcm 3
    assert [2,4], 2.gcdlcm 4
    assert [0,0], 0.gcdlcm 0
    assert [1, 4951760154835678088235319297], ((1<<31)-1).gcdlcm((1<<61)-1)
    assert [1, 284671973855620070019183339], ((1<<61)-1).gcdlcm(123456789)
    "#;
    assert_script(program);
}

fn size(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let bits = if self_val.as_fixnum().is_some() {
        8
    } else if let Some(b) = self_val.as_bignum() {
        ((b.bits() + 63) / 64 * 8) as i64
    } else {
        unreachable!()
    };
    Ok(Value::integer(bits))
}

#[test]
fn integer_size() {
    use crate::tests::*;
    let program = r#"
    assert(8, 80.size)
    assert(256, 2**80.size)
    "#;
    assert_script(program);
}

fn next(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
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

fn pred(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    if let Some(i) = self_val.as_fixnum() {
        match i.checked_sub(1) {
            Some(i) => Ok(Value::integer(i)),
            None => Ok(Value::bignum(BigInt::from(i) - 1)),
        }
    } else if let Some(n) = self_val.as_bignum() {
        Ok(Value::bignum(n - 1))
    } else {
        unreachable!()
    }
}

#[test]
fn integer_next_pred() {
    use crate::tests::*;
    let program = r#"
    assert 19, 18.next
    assert 7777777777777777777777777777, 7777777777777777777777777776.next
    assert true, 0x3fff_ffff_ffff_ffff._fixnum?
    assert true, 0x3fff_ffff_ffff_ffff.next == 4611686018427387904
    assert false, 0x3fff_ffff_ffff_ffff.next._fixnum?

    assert 18, 19.pred
    assert 7777777777777777777777777776, 7777777777777777777777777777.pred
    assert false, 4611686018427387904._fixnum?
    assert true, 0x3fff_ffff_ffff_ffff == 4611686018427387904.pred
    assert true, 4611686018427387904.pred._fixnum?
    "#;
    assert_script(program);
}

/// digits -> Integer
/// digits(base) -> Integer
///
/// https://docs.ruby-lang.org/ja/latest/method/Integer/i/digits.html
fn digits(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_range(0, 1)?;
    let base = if args.len() == 0 {
        10
    } else {
        match vm[0].coerce_to_fixnum("Arg")? {
            i if i < 0 => return Err(RubyError::argument("Negative radix.")),
            0 => return Err(RubyError::argument("Invalid radix 0.")),
            i => i,
        }
    };
    let mut ary = vec![];
    let mut self_ = self_val.as_fixnum().unwrap();
    if self_ < 0 {
        return Err(RubyError::math_domain("Out of domain."));
    }
    loop {
        let r = self_ % base;
        self_ = self_ / base;
        if r == 0 {
            break;
        }
        ary.push(Value::integer(r));
    }
    if ary.len() == 0 {
        ary.push(Value::integer(0))
    }
    Ok(Value::array_from(ary))
}

#[test]
fn integer_digits() {
    use crate::tests::*;
    let program = r#"
    assert [4,4,4,4], 4444.digits
    assert [12,5,1,1], 4444.digits(16)
    assert [7,8,5], 4444.digits(29)
    assert [7,8,5], 4444.digits(29.34)
    assert_error {5555.digits(-5)}
    assert_error {5555.digits(0)}
    assert_error {-5555.digits}
    "#;
    assert_script(program);
}

fn fixnum(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
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

        assert 100, 100.ord
        assert 42, 42.ord

        Integer
        Bignum
        Fixnum
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

        assert 10.714285714285714, 75.fdiv 7
        assert 1.0779533505209252e+28, 75456734536464756757558868676.fdiv 7
        assert 1.0531053059310664, 75456734536464756757558868676.fdiv 71651651654866852525254452425
        assert_error { 4.fdiv "Ruby" }
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
            assert 0, 3 <=> 3
            assert 0, 300000000000000000000000000.<=> 300000000000000000000000000
            assert 0, 300000000000000000000000000 <=> 300000000000000000000000000
            assert 1, 5.<=> 3
            assert 1, 5 <=> 3
            assert 1, 500000000000000000000000000.<=> 300000000000000000000000000
            assert 1, 500000000000000000000000000 <=> 300000000000000000000000000
            assert 1, 500000000000000000000000000.<=> 300
            assert 1, 500000000000000000000000000 <=> 300
            assert 1, 5 <=> -3333333333333333333333333333
            assert 1, 5.<=> -3333333333333333333333333333
            assert -1, 3.<=> 5
            assert -1, 3 <=> 5
            assert -1, 3.<=> 5555555555555555555555555555
            assert -1, 3 <=> 5555555555555555555555555555
            assert -1, -500000000000000000000000000.<=> 300
            assert -1, -500000000000000000000000000 <=> 300
            assert -1, -5 <=> 3333333333333333333333333333
            assert -1, -5.<=> 3333333333333333333333333333
            assert 0, 3.<=> 3.0
            assert 0, 3 <=> 3.0
            assert 0, 3333333333333333333333333333.<=> 3333333333333333333333333333.0
            assert 0, 3333333333333333333333333333 <=> 3333333333333333333333333333.0
            assert 1, 5.<=> 3.9
            assert 1, 5 <=> 3.9
            assert 1, 5555555555555555555555555555.<=> 3.9
            assert 1, 5555555555555555555555555555 <=> 3.9
            assert -1, 3.<=> 5.8
            assert -1, 3 <=> 5.8
            assert -1, 333333333333333333333333333.<=> 533333333333333333333333335.8
            assert -1, 333333333333333333333333333 <=> 533333333333333333333333335.8

            assert -1, 3333333333333333333333333333 <=> (10000000000.0**10000000000)
            assert 1, 3333333333333333333333333333 <=> -(10000000000.0**10000000000)

            # assert 1, 333333333333333333333333333.<=> 333333333333333333333333335.8   Ruby
            # assert 0, 333333333333333333333333333.<=> 333333333333333333333333335.8   ruruby

            assert nil, 3.<=> "three"
            assert nil, 3 <=> "three"
            assert nil, Float::NAN.<=> Float::NAN
            assert nil, Float::NAN <=> Float::NAN
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
