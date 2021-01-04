use crate::vm::*;
use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let mut string_class = Value::class_under(globals.builtins.object);
    string_class.add_builtin_class_method("new", string_new);
    string_class.add_builtin_method_by_str("to_s", to_s);
    string_class.add_builtin_method_by_str("inspect", inspect);
    string_class.add_builtin_method_by_str("+", add);
    string_class.add_builtin_method_by_str("*", mul);
    string_class.add_builtin_method_by_str("%", rem);
    string_class.add_builtin_method_by_str("[]", index);
    string_class.add_builtin_method_by_str("[]=", index_assign);
    string_class.add_builtin_method_by_str("<=>", cmp);
    string_class.add_builtin_method_by_str("<<", concat);
    string_class.add_builtin_method_by_str("concat", concat);
    string_class.add_builtin_method_by_str("start_with?", start_with);
    string_class.add_builtin_method_by_str("end_with?", end_with);
    string_class.add_builtin_method_by_str("to_sym", to_sym);
    string_class.add_builtin_method_by_str("intern", to_sym);
    string_class.add_builtin_method_by_str("split", split);
    string_class.add_builtin_method_by_str("include?", include_);
    string_class.add_builtin_method_by_str("sub", sub);
    string_class.add_builtin_method_by_str("gsub", gsub);
    string_class.add_builtin_method_by_str("gsub!", gsub_);
    string_class.add_builtin_method_by_str("scan", scan);
    string_class.add_builtin_method_by_str("slice!", slice_);
    string_class.add_builtin_method_by_str("match", str_match);
    string_class.add_builtin_method_by_str("=~", rmatch);
    string_class.add_builtin_method_by_str("tr", tr);
    string_class.add_builtin_method_by_str("size", size);
    string_class.add_builtin_method_by_str("length", size);
    string_class.add_builtin_method_by_str("bytes", bytes);
    string_class.add_builtin_method_by_str("each_byte", each_byte);
    string_class.add_builtin_method_by_str("chars", chars);
    string_class.add_builtin_method_by_str("each_char", each_char);
    string_class.add_builtin_method_by_str("sum", sum);
    string_class.add_builtin_method_by_str("upcase", upcase);
    string_class.add_builtin_method_by_str("chomp", chomp);
    string_class.add_builtin_method_by_str("to_i", toi);
    string_class.add_builtin_method_by_str("<", lt);
    string_class.add_builtin_method_by_str(">", gt);
    string_class.add_builtin_method_by_str("center", center);
    string_class.add_builtin_method_by_str("ljust", ljust);
    string_class.add_builtin_method_by_str("rjust", rjust);
    string_class.add_builtin_method_by_str("next", next);
    string_class.add_builtin_method_by_str("succ", next);
    string_class.add_builtin_method_by_str("count", count);
    string_class.add_builtin_method_by_str("rstrip", rstrip);
    string_class.add_builtin_method_by_str("ord", ord);
    string_class.add_builtin_method_by_str("empty?", empty);
    string_class.add_builtin_method_by_str("codepoints", codepoints);
    string_class.add_builtin_method_by_str("frozen?", frozen_);
    string_class.add_builtin_method_by_str("lines", lines);

    string_class
}

fn string_new(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let s = if args.len() == 0 {
        String::new()
    } else {
        let mut arg = args[0];
        arg.expect_string("1st arg")?.to_string()
    };
    let mut array = Value::string(s);
    array.set_class(self_val);
    if let Some(method) = vm.globals.find_method(self_val, IdentId::INITIALIZE) {
        vm.eval_send(method, array, args)?;
    };
    Ok(array)
}

fn to_s(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let self_ = self_val.as_rstring().unwrap();
    Ok(Value::string(self_.to_s()))
}

fn inspect(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let self_ = self_val.as_rstring().unwrap();
    Ok(Value::string(self_.inspect()))
}

fn add(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let mut lhs = self_val.as_rstring().unwrap().clone();
    let rhs = args[0].as_rstring().ok_or_else(|| {
        RubyError::argument(format!("1st arg must be String. (given:{:?})", args[0]))
    })?;
    lhs.append(rhs);
    Ok(Value::string_from_rstring(lhs))
}

fn mul(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.as_rstring().unwrap();
    let count = match args[0].expect_integer("1st arg")? {
        i if i < 0 => return Err(RubyError::argument("Negative argument.")),
        i => i as usize,
    };

    let res = match lhs {
        RString::Str(s) => Value::string(s.repeat(count)),
        RString::SmallStr(s) => Value::string(s.repeat(count)),
        RString::Bytes(b) => Value::bytes(b.repeat(count)),
    };
    Ok(res)
}

fn index(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    fn conv_index(i: i64, len: usize) -> Option<usize> {
        if i >= 0 {
            if i < len as i64 {
                Some(i as usize)
            } else {
                None
            }
        } else {
            match len as i64 + i {
                n if n < 0 => None,
                n => Some(n as usize),
            }
        }
    }
    args.check_args_range(1, 2)?;
    let lhs = self_val.expect_string("Receiver")?;
    match args[0].unpack() {
        RV::Integer(i) => {
            let index = match conv_index(i, lhs.chars().count()) {
                Some(i) => i,
                None => return Ok(Value::nil()),
            };
            let len = if args.len() == 2 {
                match args[1].expect_integer("1st arg")? {
                    0 => return Ok(Value::string("")),
                    i if i < 0 => return Ok(Value::nil()),
                    i => i as usize,
                }
            } else {
                1usize
            };
            let ch: String = lhs.chars().skip(index).take(len).collect();
            if ch.len() != 0 {
                Ok(Value::string(ch))
            } else {
                Ok(Value::nil())
            }
        }
        RV::Object(oref) => match &oref.kind {
            ObjKind::Range(info) => {
                let len = lhs.chars().count();
                let (start, end) = match (info.start.as_integer(), info.end.as_integer()) {
                    (Some(start), Some(end)) => {
                        match (conv_index(start, len), conv_index(end, len)) {
                            (Some(start), Some(end)) if start > end => {
                                return Ok(Value::string(""))
                            }
                            (Some(start), Some(end)) => (start, end),
                            _ => return Ok(Value::nil()),
                        }
                    }
                    _ => return Err(RubyError::argument("Index must be Integer.")),
                };
                let s: String = lhs.chars().skip(start).take(end - start + 1).collect();
                Ok(Value::string(s))
            }
            _ => return Err(RubyError::argument("Bad type for index.")),
        },
        _ => return Err(RubyError::argument("Bad type for index.")),
    }
}

fn index_assign(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(2, 3)?;
    let string = self_val.as_mut_rstring().unwrap();
    let str_len = string.chars().count();
    let start_pos = args[0].expect_integer("1st arg")? as usize;
    let (len, mut subst_val) = match args.len() {
        2 => (0, args[1]),
        3 => (args[1].expect_integer("2nd arg")? as usize - 1, args[2]),
        _ => unreachable!(),
    };
    if start_pos >= str_len {
        return Err(RubyError::index(format!(
            "Index {:?} out of string.",
            args[0]
        )));
    }
    let (start, end) = (
        string.char_indices().nth(start_pos).unwrap().0,
        match string.char_indices().nth(start_pos + len + 1) {
            Some((p, _)) => p,
            None => string.len(),
        },
    );
    string.replace_range(start..end, subst_val.expect_string("Value")?);
    Ok(Value::nil())
}

fn cmp(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.as_rstring().unwrap();
    match lhs.cmp(args[0]) {
        Some(ord) => Ok(Value::integer(ord as i64)),
        None => Ok(Value::nil()),
    }
}

fn concat(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.as_mut_rstring().unwrap();
    let mut arg0 = args[0];
    match arg0.as_mut_rstring() {
        Some(rhs) => lhs.append(rhs),
        None => match arg0.as_integer() {
            Some(i) => {
                let rhs = RString::Bytes(vec![i as i8 as u8]);
                lhs.append(&rhs);
            }
            None => return Err(RubyError::argument("Arg must be String or Integer.")),
        },
    };
    Ok(self_val)
}

fn expect_char(_: &mut VM, chars: &mut std::str::Chars) -> Result<char, RubyError> {
    let ch = match chars.next() {
        Some(ch) => ch,
        None => return Err(RubyError::argument("Invalid termination of format string.")),
    };
    Ok(ch)
}

macro_rules! next_char {
    ($ch:ident, $chars:ident) => {
        $ch = match $chars.next() {
            Some(c) => c,
            None => break,
        };
    };
}

fn rem(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let arguments = match args[0].as_array() {
        Some(ary) => ary.elements.clone(),
        None => vec![args[0]],
    };
    let mut arg_no = 0;
    let mut format_str = vec![];
    let mut chars = self_val.as_string().unwrap().chars();
    let mut ch = match chars.next() {
        Some(ch) => ch,
        None => {
            let res = Value::string("");
            return Ok(res);
        }
    };
    loop {
        if ch != '%' {
            format_str.push(ch);
            next_char!(ch, chars);
            continue;
        }
        match chars.next() {
            Some(c) if c == '%' => {
                format_str.push('%');
                next_char!(ch, chars);
                continue;
            }
            Some(c) => ch = c,
            None => {
                return Err(RubyError::argument(
                    "Incomplete format specifier. use '%%' instead.",
                ))
            }
        };
        let mut zero_flag = false;
        // Zero-fill
        if ch == '0' {
            zero_flag = true;
            ch = expect_char(vm, &mut chars)?;
        }
        // Width
        let mut width = 0usize;
        while '0' <= ch && ch <= '9' {
            width = width * 10 + ch as usize - '0' as usize;
            ch = expect_char(vm, &mut chars)?;
        }
        // Precision
        let mut precision = 0usize;
        if ch == '.' {
            ch = expect_char(vm, &mut chars)?;
            while '0' <= ch && ch <= '9' {
                precision = precision * 10 + ch as usize - '0' as usize;
                ch = expect_char(vm, &mut chars)?;
            }
        };
        if arguments.len() <= arg_no {
            return Err(RubyError::argument("Too few arguments"));
        };
        // Specifier
        let mut val = arguments[arg_no];
        arg_no += 1;
        let format = match ch {
            'd' => {
                let val = val.expect_integer("Invalid value for placeholder of Integer.")?;
                if zero_flag {
                    format!("{:0w$.p$}", val, w = width, p = precision)
                } else {
                    format!("{:w$.p$}", val, w = width, p = precision)
                }
            }
            'b' => {
                let val = val.expect_integer("Invalid value for placeholder of Integer.")?;
                if zero_flag {
                    format!("{:0w$b}", val, w = width)
                } else {
                    format!("{:w$b}", val, w = width)
                }
            }
            'x' => {
                let val = val.expect_integer("Invalid value for placeholder of Integer.")?;
                if zero_flag {
                    format!("{:0w$x}", val, w = width)
                } else {
                    format!("{:w$x}", val, w = width)
                }
            }
            'X' => {
                let val = val.expect_integer("Value for the placeholder")?;
                if zero_flag {
                    format!("{:0w$X}", val, w = width)
                } else {
                    format!("{:w$X}", val, w = width)
                }
            }
            'f' => {
                let f = match val.as_float() {
                    Some(f) => f,
                    None => val.expect_integer("Value for the placeholder")? as f64,
                };
                if zero_flag {
                    format!("{:0w$.p$}", f, w = width, p = precision)
                } else {
                    format!("{:w$.p$}", f, w = width, p = precision)
                }
            }
            'c' => {
                let val = val.expect_integer("Value for the placeholder")?;
                let ch = char::from_u32(val as u32)
                    .ok_or_else(|| RubyError::argument("Invalid value for placeholder."))?;
                format!("{}", ch)
            }
            's' => val.expect_string("Value for placeholder")?.to_string(),
            _ => {
                return Err(RubyError::argument(format!(
                    "Invalid format character. {}",
                    ch
                )))
            }
        };
        format_str.append(&mut format.chars().collect());
        next_char!(ch, chars);
    }

    let res = Value::string(format_str.into_iter().collect::<String>());
    Ok(res)
}

fn start_with(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let string = self_val.expect_string("Receiver")?;
    let mut arg0 = args[0];
    let arg = arg0.expect_string("1st arg")?;
    let res = string.starts_with(arg);
    Ok(Value::bool(res))
}

fn end_with(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let string = self_val.expect_string("Receiver")?;
    let mut arg0 = args[0];
    let arg = arg0.expect_string("1st arg")?;
    let res = string.ends_with(arg);
    Ok(Value::bool(res))
}

fn to_sym(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let string = self_val.expect_string("Receiver")?;
    let id = IdentId::get_id(string);
    Ok(Value::symbol(id))
}

fn split(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 2)?;
    let string = self_val.expect_string("Receiver")?;
    let mut arg0 = args[0];
    let sep = arg0.expect_string("1st arg")?;
    let lim = if args.len() > 1 {
        args[1].expect_integer("Second arg must be Integer.")?
    } else {
        0
    };
    if lim == 1 {
        let vec = vec![Value::string(string)];
        let ary = Value::array_from(vec);
        return Ok(ary);
    } else if lim < 0 {
        let vec = string.split(sep).map(|x| Value::string(x)).collect();
        let ary = Value::array_from(vec);
        return Ok(ary);
    } else if lim == 0 {
        let mut vec: Vec<&str> = string.split(sep).collect();
        loop {
            match vec.last() {
                Some(s) => {
                    if s == &"" {
                        vec.pop();
                    } else {
                        break;
                    }
                }
                None => break,
            }
        }
        let vec = vec.iter().map(|x| Value::string(*x)).collect();
        let ary = Value::array_from(vec);
        return Ok(ary);
    } else {
        let vec = string
            .splitn(lim as usize, sep)
            .map(|x| Value::string(x))
            .collect();
        let ary = Value::array_from(vec);
        return Ok(ary);
    }
}

fn include_(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let mut arg = args[0];
    let rec = self_val.expect_string("Receiver")?;
    let pat = arg.expect_string("Arg")?;
    let b = rec.contains(pat);
    Ok(Value::bool(b))
}

fn sub(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 2)?;
    let given = self_val.expect_string("Receiver")?;
    let res = if args.len() == 2 {
        let mut arg1 = args[1];
        let replace = arg1.expect_string("2nd arg")?;
        RegexpInfo::replace_one(vm, args[0], given, replace)?
    } else {
        let block = args.expect_block()?;
        let (res, _) = RegexpInfo::replace_one_block(vm, args[0], given, &block)?;
        res
    };
    Ok(Value::string(res))
}

fn gsub(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let (res, _) = gsub_main(vm, self_val, args)?;
    Ok(Value::string(res))
}

fn gsub_(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let (res, changed) = gsub_main(vm, self_val, args)?;
    *self_val.rvalue_mut() = RValue::new_string(&res);
    let res = if changed { self_val } else { Value::nil() };
    Ok(res)
}

fn gsub_main(vm: &mut VM, mut self_val: Value, args: &Args) -> Result<(String, bool), RubyError> {
    match &args.block {
        Block::None => {
            args.check_args_num(2)?;
            let given = self_val.expect_string("Receiver")?;
            let mut arg1 = args[1];
            let replace = arg1.expect_string("2nd arg")?;
            RegexpInfo::replace_all(vm, args[0], given, replace)
        }
        block => {
            args.check_args_num(1)?;
            let given = self_val.expect_string("Receiver")?;
            RegexpInfo::replace_all_block(vm, args[0], given, block)
        }
    }
}

fn scan(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let given = self_val.expect_string("Receiver")?;
    let vec = if let Some(s) = args[0].as_string() {
        let re = vm.regexp_from_escaped_string(s)?;
        RegexpInfo::find_all(vm, &re, given)?
    } else if let Some(re) = args[0].as_regexp() {
        RegexpInfo::find_all(vm, &*re, given)?
    } else {
        return Err(RubyError::argument("1st arg must be RegExp or String."));
    };
    match &args.block {
        Block::None => Ok(Value::array_from(vec)),
        block => {
            vm.temp_push_vec(&vec);
            for arg in vec {
                match arg.as_array() {
                    Some(ary) => {
                        let block_args = Args::from_slice(&ary.elements);
                        vm.eval_block(block, &block_args)?;
                    }
                    None => {
                        let block_args = Args::new1(arg);
                        vm.eval_block(block, &block_args)?;
                    }
                }
            }
            Ok(self_val)
        }
    }
}

fn slice_(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    fn calc_idx(target: &str, i: i64) -> Option<usize> {
        if i >= 0 {
            Some(i as usize)
        } else {
            match target.chars().count() as i64 + i {
                idx if idx < 0 => None,
                idx => Some(idx as usize),
            }
        }
    }
    let mut self_val2 = self_val;
    args.check_args_range(1, 2)?;
    let target = self_val2.as_mut_rstring().unwrap();
    let arg0 = args[0].clone();
    match arg0.unpack() {
        RV::Integer(i) => {
            if args.len() == 1 {
                let idx = match calc_idx(target.as_string()?, i) {
                    Some(i) => i,
                    None => return Ok(Value::nil()),
                };
                let (pos, ch) = match target.char_indices().nth(idx) {
                    Some((pos, ch)) => (pos, ch),
                    None => return Ok(Value::nil()),
                };
                target.remove(pos);
                return Ok(Value::string(ch.to_string()));
            } else {
                let len = args[1].expect_integer("2nd arg")?;
                let len = if len < 0 {
                    return Ok(Value::nil());
                } else {
                    len as usize
                };
                let start = match calc_idx(target.as_string()?, i) {
                    Some(i) => i,
                    None => return Ok(Value::nil()),
                };
                let mut iter = target.char_indices().skip(start);
                let mut take = iter.by_ref().take(len).peekable();
                let start_pos = match take.peek().cloned() {
                    Some((pos, _)) => pos,
                    None => return Ok(Value::nil()),
                };
                let take: String = take.map(|(_, ch)| ch).collect();

                match iter.next() {
                    Some((end_pos, _)) => {
                        target.replace_range(start_pos..end_pos, "");
                    }
                    None => {
                        target.replace_range(start_pos.., "");
                    }
                }

                Ok(Value::string(take))
            }
        }
        RV::Object(_rvalue) => match &mut args[0].clone().rvalue_mut().kind {
            ObjKind::String(rs) => {
                args.check_args_num(1)?;
                let given = rs.as_string()?;
                *target = RString::from(&target.replacen(given, "", usize::MAX));
                Ok(Value::string(given))
            }
            ObjKind::Regexp(regexp) => {
                let given = target.as_string()?;
                let (res, cap) = regexp.replace_once(vm, given, "")?;
                let ret = match cap {
                    Some(cap) => Value::string(cap.get(0).unwrap().as_str()),
                    None => Value::nil(),
                };
                *target = RString::from(&res);
                Ok(ret)
            }
            _ => {
                return Err(RubyError::argument(
                    "First arg must be Integer, String, Regexp or Range.",
                ))
            }
        },
        _ => {
            return Err(RubyError::argument(
                "First arg must be Integer, String, Regexp or Range.",
            ))
        }
    }
}

fn rmatch(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let given = self_val.expect_string("Receiver")?;
    if let Some(re) = args[0].as_regexp() {
        let res = match RegexpInfo::find_one(vm, &re, given).unwrap() {
            Some(mat) => Value::integer(mat.start() as i64),
            None => Value::nil(),
        };
        return Ok(res);
    } else {
        return Err(RubyError::argument("1st arg must be RegExp."));
    };
}

fn str_match(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 2)?;
    let given = self_val.expect_string("Receiver")?;
    let re = args[0].expect_regexp_or_string(vm, "1st arg")?;
    let pos = if args.len() == 1 {
        0usize
    } else {
        match args[1].expect_integer("2nd arg")? {
            pos if pos >= 0 => pos as usize,
            _ => return Ok(Value::nil()),
        }
    };
    match &args.block {
        Block::None => RegexpInfo::match_one(vm, &re, given, pos),
        block => RegexpInfo::match_one_block(vm, &re, given, block, pos),
    }
}

fn tr(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(2)?;
    let rec = self_val.expect_string("Receiver")?;
    let mut arg0 = args[0];
    let mut arg1 = args[1];
    let from = arg0.expect_string("1st arg")?;
    let to = arg1.expect_string("2nd arg")?;
    let res = rec.replace(from, to);
    Ok(Value::string(res))
}

fn size(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let rec = self_val.expect_string("Receiver")?;
    Ok(Value::integer(rec.chars().count() as i64))
}

fn bytes(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    match &args.block {
        Block::None => {
            let bytes = self_val.expect_bytes("Receiver")?;
            let mut ary = vec![];
            for b in bytes {
                ary.push(Value::integer(*b as i64));
            }
            Ok(Value::array_from(ary))
        }
        block => {
            let rstr = match self_val.as_rstring() {
                Some(rstr) => rstr,
                None => return Err(RubyError::argument("Receiver must be String.")),
            };
            for b in rstr.as_bytes() {
                let byte = Value::integer(*b as i64);
                vm.eval_block(block, &Args::new1(byte))?;
            }
            Ok(self_val)
        }
    }
}

fn each_byte(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let block = args.expect_block()?;
    let rstr = match self_val.as_rstring() {
        Some(rstr) => rstr,
        None => return Err(RubyError::argument("Receiver must be String.")),
    };
    for b in rstr.as_bytes() {
        let byte = Value::integer(*b as i64);
        vm.eval_block(block, &Args::new1(byte))?;
    }
    Ok(self_val)
}

fn chars(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let string = self_val.expect_string("Receiver")?;
    let ary: Vec<Value> = string
        .chars()
        .map(|c| Value::string(c.to_string()))
        .collect();
    Ok(Value::array_from(ary))
}

fn each_char(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let block = args.expect_block()?;
    let chars = self_val.expect_string("Receiver")?;
    for c in chars.chars() {
        let char = Value::string(c.to_string());
        vm.eval_block(block, &Args::new1(char))?;
    }
    Ok(self_val)
}

fn sum(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let bytes = self_val.as_bytes().unwrap();
    let mut sum = 0;
    for b in bytes {
        sum += *b as u64;
    }
    Ok(Value::integer((sum & ((1 << 16) - 1)) as i64))
}

fn upcase(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let self_ = self_val.expect_string("Receiver")?;
    let res = self_.to_uppercase();
    Ok(Value::string(res))
}

fn chomp(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let self_ = self_val.expect_string("Receiver")?;
    let res = self_.trim_end_matches('\n').to_string();
    Ok(Value::string(res))
}

fn toi(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let self_ = match self_val.expect_string("Receiver") {
        Ok(s) => s,
        Err(_) => return Ok(Value::integer(0)),
    };
    let i: i64 = match self_.parse() {
        Ok(i) => i,
        Err(_) => 0,
    };
    Ok(Value::integer(i))
}

fn lt(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.as_rstring().unwrap();
    match lhs.cmp(args[0]) {
        Some(ord) => Ok(Value::bool(ord == std::cmp::Ordering::Less)),
        None => Err(RubyError::argument(format!(
            "Comparison of String with {:?} failed.",
            args[0]
        ))),
    }
}

fn gt(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.as_rstring().unwrap();
    match lhs.cmp(args[0]) {
        Some(ord) => Ok(Value::bool(ord == std::cmp::Ordering::Greater)),
        None => Err(RubyError::argument(format!(
            "Comparison of String with {:?} failed.",
            args[0]
        ))),
    }
}

fn gen_pad(padding: &str, len: usize) -> String {
    let pad_len = padding.chars().count();
    let pad_repeat = padding.repeat(len / pad_len);
    let pad_end: String = padding.chars().take(len % pad_len).collect();
    format!("{}{}", pad_repeat, pad_end)
}

fn center(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 2)?;
    let padding = if args.len() == 2 {
        let mut arg = args[1];
        arg.expect_string("2nd arg")?.to_string()
    } else {
        " ".to_string()
    };
    if padding.len() == 0 {
        return Err(RubyError::argument("Zero width padding."));
    };
    let lhs = self_val.as_string().unwrap();
    let width = args[0].expect_integer("1st arg")?;
    let str_len = lhs.chars().count();
    if width <= 0 || width as usize <= str_len {
        return Ok(Value::string(lhs));
    }
    let head = (width as usize - str_len) / 2;
    let tail = width as usize - str_len - head;
    return Ok(Value::string(format!(
        "{}{}{}",
        gen_pad(&padding, head),
        lhs,
        gen_pad(&padding, tail)
    )));
}

fn ljust(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 2)?;
    let padding = if args.len() == 2 {
        let mut arg = args[1];
        arg.expect_string("2nd arg")?.to_string()
    } else {
        " ".to_string()
    };
    if padding.len() == 0 {
        return Err(RubyError::argument("Zero width padding."));
    };
    let lhs = self_val.as_string().unwrap();
    let width = args[0].expect_integer("1st arg")?;
    let str_len = lhs.chars().count();
    if width <= 0 || width as usize <= str_len {
        return Ok(Value::string(lhs));
    }
    let tail = width as usize - str_len;
    Ok(Value::string(format!("{}{}", lhs, gen_pad(&padding, tail))))
}

fn rjust(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 2)?;
    let padding = if args.len() == 2 {
        let mut arg = args[1];
        arg.expect_string("2nd arg")?.to_string()
    } else {
        " ".to_string()
    };
    if padding.len() == 0 {
        return Err(RubyError::argument("Zero width padding."));
    };
    let lhs = self_val.as_string().unwrap();
    let width = args[0].expect_integer("1st arg")?;
    let str_len = lhs.chars().count();
    if width <= 0 || width as usize <= str_len {
        return Ok(Value::string(lhs));
    }
    let tail = width as usize - str_len;
    Ok(Value::string(format!("{}{}", gen_pad(&padding, tail), lhs)))
}

fn next(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    fn char_forward(ch: char, _: &mut VM) -> Result<char, RubyError> {
        std::char::from_u32((ch as u32) + 1)
            .ok_or_else(|| RubyError::argument("Error occurs in String#succ."))
    }
    args.check_args_num(0)?;
    let self_ = self_val.as_string().unwrap();
    if self_.len() == 0 {
        return Ok(Value::string(""));
    }
    let chars = self_.chars();
    let mut buf: Vec<char> = vec![];
    let mut carry_flag = true;
    for c in chars.rev() {
        if carry_flag {
            if '0' <= c && c <= '8'
                || 'a' <= c && c <= 'y'
                || 'A' <= c && c <= 'Y'
                || '０' <= c && c <= '８'
            {
                carry_flag = false;
                buf.push(char_forward(c, vm)?);
            } else if c == '9' {
                buf.push('0');
            } else if c == '９' {
                buf.push('０');
            } else if c == 'z' {
                buf.push('a');
            } else if c == 'Z' {
                buf.push('A');
            } else {
                carry_flag = false;
                buf.push(char_forward(c, vm)?);
            }
        } else {
            buf.push(c);
        }
    }
    if carry_flag {
        let c = buf.last().unwrap();
        if *c == '0' {
            buf.push('1');
        } else if *c == '０' {
            buf.push('１');
        } else if *c == 'a' {
            buf.push('a');
        } else if *c == 'A' {
            buf.push('A');
        }
    }
    let val = Value::string(buf.iter().rev().collect::<String>());
    Ok(val)
}

fn count(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let mut arg0 = args[0];
    let target = self_val.as_string().unwrap();
    let mut c = 0;
    let iter = arg0.expect_string("Args")?.chars();
    for ch in iter {
        c += target.rmatches(|x| ch == x).count();
    }
    Ok(Value::integer(c as i64))
}

fn rstrip(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let string = self_val.as_string().unwrap();
    let trim: &[_] = &[' ', '\n', '\t', '\x0d', '\x0c', '\x0b', '\x00'];
    let res = string.trim_end_matches(trim);
    Ok(Value::string(res))
}

fn ord(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let ch = match self_val.as_string().unwrap().chars().next() {
        Some(ch) => ch,
        None => return Err(RubyError::argument("Empty string.")),
    };
    Ok(Value::integer(ch as u32 as i64))
}

fn empty(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let len = match self_val.as_rstring().unwrap() {
        RString::Str(s) => s.len(),
        RString::SmallStr(s) => s.len() as usize,
        RString::Bytes(b) => b.len(),
    };
    Ok(Value::bool(len == 0))
}

fn codepoints(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    args.expect_no_block()?;
    let res = self_val
        .as_string()
        .unwrap()
        .chars()
        .map(|c| Value::integer(c as u32 as i64))
        .collect();
    Ok(Value::array_from(res))
}

fn frozen_(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(Value::false_val())
}

fn lines(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let mut receiver = self_val;
    args.check_args_num(0)?;
    args.expect_no_block()?;
    let string = receiver.expect_string("Receiver")?;
    let ary = string
        .split_inclusive('\n')
        .map(|line| Value::string(line))
        .collect();
    Ok(Value::array_from(ary))
}

#[cfg(test)]
mod test {
    use crate::test::*;

    #[test]
    fn string_test() {
        let program = r#"
        assert(true, "a" < "b")
        assert(false, "b" < "b")
        assert(false, "c" < "b")
        assert(false, "a" > "b")
        assert(false, "b" > "b")
        assert(true, "c" > "b")
        assert(-1, "a" <=> "b")
        assert(0, "b" <=> "b")
        assert(1, "b" <=> "a")
        assert_error { "a" < 9 }
        assert_error { "a" > 9 }
        assert(7, "hello世界".size)
        assert(true, "".empty?)
        assert(false, "s".empty?)
        assert(false, 2.chr.empty?)
        "#;
        assert_script(program);
    }

    #[test]
    fn string_add() {
        let program = r#"
        assert "this is a pen", "this is " + "a pen"
        "#;
        assert_script(program);
    }

    #[test]
    fn string_mul() {
        let program = r#"
        assert "rubyrubyrubyruby", "ruby" * 4
        assert "", "ruby" * 0
        "#;
        assert_script(program);
    }

    #[test]
    fn string_concat() {
        let program = r#"
        a = "Ruby"
        assert "Ruby is easy", a << " is easy"
        assert "Ruby is easy", a
        a << 33
        assert "Ruby is easy!", a
        "#;
        assert_script(program);
    }

    #[test]
    fn string_index() {
        let program = r#"
        assert "rubyruby"[3], "y" 
        assert "rubyruby"[0..2], "rub" 
        assert "rubyruby"[0..-2], "rubyrub" 
        assert "rubyruby"[2..-7], ""
        "#;
        assert_script(program);
    }

    #[test]
    fn string_index2() {
        let program = r#"
        a = "qwertyuiop"
        a[9] = "P"
        a[3,6] = "/"
        assert("qwe/P", a) 
        "#;
        assert_script(program);
    }

    #[test]
    fn string_match() {
        let program = r#"
        assert ["ll", "l"],'hello'.match('(.)\1')   # => #<MatchData "ll" 1:"l">
        assert "ll", 'hello'.match('(.)\1')[0]
        assert "ll", 'hello'.match(/(.)\1/)[0]
        assert nil, 'hello'.match('xx')

        assert "match l", 'hello'.match('(.)\1'){|e| "match #{$1}" }
        assert nil, 'hello'.match('xx'){|e| "match #{$1}" }

        assert ["hoge"], 'hoge hige hege bar'.match('h.ge', 0)     # => #<MatchData "hoge">
        assert ["hige"], 'hoge hige hege bar'.match('h.ge', 1)     # => #<MatchData "hige">
        assert nil, 'hoge hige hege bar'.match('h.ge', -1)
        assert nil, 'hoge hige hege bar'.match('h.ge', 18)     # => #<MatchData "hige">
        "#;
        assert_script(program);
    }

    #[test]
    fn string_format() {
        let program = r#"
        assert "-12-", "-%d-" % 12
        assert "-  12-", "-%4d-" % 12
        assert "-0012-", "-%04d-" % 12
        assert "-c-", "-%x-" % 12
        assert "-   c-", "-%4x-" % 12
        assert "-000c-", "-%04x-" % 12
        assert "-C-", "-%X-" % 12
        assert "-   C-", "-%4X-" % 12
        assert "-000C-", "-%04X-" % 12
        assert "-1001-", "-%b-" % 9
        assert "-  1001-", "-%6b-" % 9
        assert "-001001-", "-%06b-" % 9
        assert "12.50000", "%08.5f" % 12.5
        assert "0012.500", "%08.3f" % 12.5
        assert "1.34", "%.2f" % 1.345
        "#;
        assert_script(program);
    }

    #[test]
    fn string_start_with() {
        let program = r#"
        assert true, "ruby".start_with?("r")
        assert false, "ruby".start_with?("R")
        assert true, "魁ruby".start_with?("魁")
        "#;
        assert_script(program);
    }

    #[test]
    fn string_end_with() {
        let program = r#"
        assert true, "ruby".end_with?("by")
        assert false, "ruby".end_with?("yy")
        assert true, "ruby魂".end_with?("魂")
        "#;
        assert_script(program);
    }

    #[test]
    fn string_to_sym() {
        let program = r#"
        assert :ruby, "ruby".to_sym
        assert :rust, "rust".to_sym
        "#;
        assert_script(program);
    }

    #[test]
    fn string_split() {
        let program = r#"
        assert ["this", "is", "a", "pen"], "this is a pen       ".split(" ")
        assert ["this", "is", "a pen"], "this is a pen".split(" ", 3)
        "#;
        assert_script(program);
    }

    #[test]
    fn string_bytes() {
        let program = r#"
        assert [97, 98, 99, 100], "abcd".bytes
        assert [228, 184, 150, 231, 149, 140], "世界".bytes
        res = []
        "str".each_byte do |byte|
        res << byte
        end
        assert [115, 116, 114], res
        "#;
        assert_script(program);
    }

    #[test]
    fn string_chars() {
        let program = r#"
        assert ["a", "b", "c", "d"], "abcd".chars
        assert ["世", "界"], "世界".chars
        res = []
        "str".each_char do |byte|
        res << byte
        end
        assert ["s", "t", "r"], res
        "#;
        assert_script(program);
    }

    #[test]
    fn string_sum() {
        let program = r#"
        assert 394, "abcd".sum
        a = ""
        [114, 117, 98].map{ |elem| a += elem.chr}
        assert 329, a.sum
        "#;
        assert_script(program);
    }

    #[test]
    fn string_include() {
        let program = r#"
        assert true, "hello".include? "lo"
        assert false, "hello".include? "ol"
        "#;
        assert_script(program);
    }

    #[test]
    fn string_sub() {
        let program = r#"
        assert "abc!!g", "abcdefg".sub(/def/, "!!")
        #assert "a<<b>>cabc", "abcabc".sub(/b/, "<<\1>>")
        #assert "X<<bb>>xbb", "xxbbxbb".sub(/x+(b+)/, "X<<\1>>")
        assert "aBCabc", "abcabc".sub(/bc/) {|s| s.upcase }
        assert "abcabc", "abcabc".sub(/bd/) {|s| s.upcase }
        "#;
        assert_script(program);
    }

    #[test]
    fn string_scan() {
        let program = r#"
        assert ["fo", "ob", "ar"], "foobar".scan(/../)
        assert ["o", "o"], "foobar".scan("o")
        assert ["bar", "baz", "bar", "baz"], "foobarbazfoobarbaz".scan(/ba./)
        assert [["f"], ["o"], ["o"], ["b"], ["a"], ["r"]], "foobar".scan(/(.)/)
        assert [["ba", "r", ""], ["ba", "z", ""], ["ba", "r", ""], ["ba", "z", ""]], "foobarbazfoobarbaz".scan(/(ba)(.)()/)
        "foobarbazfoobarbaz".scan(/ba./) {|x| puts x}
        "#;
        assert_script(program);
    }

    #[test]
    fn string_slice_() {
        let program = r#"
        a = ["私の名前は一色です"] * 20
        assert "は", a[0].slice!(4)
        assert "私の名前一色です", a[0]
        assert "色", a[0].slice!(-3)
        assert "私の名前一です", a[0]
        assert nil, a[0].slice!(-9)
        assert "私の名前一です", a[0]
        assert "名前は一色", a[1].slice!(2,5)
        assert "私のです", a[1]
        assert nil, a[2].slice!(10,5)
        assert "色です", a[3].slice!(-3,5)
        assert nil, a[3].slice!(-10,5)
        a = "a"
        assert "a", a.slice!(0,1)
        assert "", a

        a = "abc agc afc"
        assert "abc", a.slice!(/a.c/)
        assert " agc afc", a

        "#;
        assert_script(program);
    }

    #[test]
    fn string_upcase() {
        let program = r#"
        assert "RUBY IS GREAT.", "ruby is great.".upcase
        a = ""
        [114, 117, 98, 121, 32, 105, 115, 32, 103, 114, 101, 97, 116, 46].map{ |elem| a += elem.chr }
        assert "RUBY IS GREAT.", a.upcase
        "#;
        assert_script(program);
    }

    #[test]
    fn string_chomp() {
        let program = r#"
        assert "Ruby", "Ruby\n\n\n".chomp
        a = ""
        [82, 117, 98, 121, 10, 10, 10].map{ |elem| a += elem.chr }
        assert "Ruby", a.chomp
        "#;
        assert_script(program);
    }

    #[test]
    fn string_toi() {
        let program = r#"
        assert 1578, "1578".to_i
        a = ""
        [49, 53, 55, 56].map{ |elem| a += elem.chr }
        assert 1578, a.to_i
        assert 0, "k".to_i
        "#;
        assert_script(program);
    }

    #[test]
    fn string_center() {
        let program = r#"
        assert("foo", "foo".center(1))
        assert("foo", "foo".center(2))
        assert("foo", "foo".center(3))
        assert("  foo  ", "foo".center(7))
        assert("  foo   ", "foo".center(8))
        assert("   foo   ", "foo".center(9))
        assert("   foo    ", "foo".center(10))
        assert("***foo****", "foo".center(10, "*"))
        assert("121foo1212", "foo".center(10, "12"))
        "#;
        assert_script(program);
    }

    #[test]
    fn string_ljust() {
        let program = r#"
        s = "戦闘妖精"
        assert_error { s.ljust }
        assert_error { s.ljust 8, "" }
        assert("戦闘妖精       ", s.ljust 11)
        assert("戦闘妖精$$$$$$$", s.ljust 11,"$")
        assert("戦闘妖精1231231", s.ljust 11,"123")
        "#;
        assert_script(program);
    }

    #[test]
    fn string_rjust() {
        let program = r#"
        s = "戦闘妖精"
        assert_error { s.rjust }
        assert_error { s.rjust 8, "" }
        assert("       戦闘妖精", s.rjust 11)
        assert("$$$$$$$戦闘妖精", s.rjust 11,"$")
        assert("1231231戦闘妖精", s.rjust 11,"123")
        "#;
        assert_script(program);
    }

    #[test]
    fn string_succ() {
        let program = r#"
        assert "aa".succ, "ab"
        assert "88".succ.succ, "90"
        assert "99".succ, "100"
        assert "ZZ".succ, "AAA"
        assert "a9".succ, "b0"
        #assert "-9".succ, "-10"
        assert ".".succ, "/"
        assert "aa".succ, "ab"
        
        # 繰り上がり
        assert "99".succ, "100"
        assert "a9".succ, "b0"
        assert "Az".succ, "Ba"
        assert "zz".succ, "aaa"
        #assert "-9".succ, "-10"
        assert "9".succ, "10"
        assert "09".succ, "10"
        assert "０".succ, "１"
        assert "９".succ, "１０"
        
        # アルファベット・数字とそれ以外の混在
        #assert "1.9.9".succ, "2.0.0"
        
        # アルファベット・数字以外のみ
        assert ".".succ, "/"
        #assert "\0".succ, "\001"
        #assert "\377".succ, "\001\000"
        "#;
        assert_script(program);
    }

    #[test]
    fn string_count() {
        let program = r#"
        assert 1, 'abcdefg'.count('c')
        assert 4, '123456789'.count('2378')
        #assert 4, '123456789'.count('2-8', '^4-6')
        "#;
        assert_script(program);
    }

    #[test]
    fn string_rstrip() {
        let program = r#"
        assert "   abc", "   abc\n".rstrip
        assert "   abc", "   abc \t\n\x00".rstrip
        assert "   abc", "   abc".rstrip
        assert "   abc", "   abc\x00".rstrip
        "#;
        assert_script(program);
    }

    #[test]
    fn string_ord() {
        let program = r#"
        assert 97, 'abcdefg'.ord
        "#;
        assert_script(program);
    }

    #[test]
    fn string_codepoints() {
        let program = r#"
        assert [104, 101, 108, 108, 111, 32, 12431, 12540, 12427, 12393], "hello わーるど".codepoints
        "#;
        assert_script(program);
    }

    #[test]
    fn frozen() {
        let program = r#"
        assert false, "Ruby".frozen?
        "#;
        assert_script(program);
    }

    #[test]
    fn lines() {
        let program = r#"
        assert ["aa\n", "bb\n", "cc\n"], "aa\nbb\ncc\n".lines
        assert ["aa\n", "bb\n", "cc"], "aa\nbb\ncc".lines
        "#;
        assert_script(program);
    }
}
