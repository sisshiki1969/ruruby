use crate::vm::*;
use std::string::FromUtf8Error;
//#[macro_use]
use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub enum RString {
    Str(String),
    Bytes(Vec<u8>),
}

use std::str::FromStr;
impl RString {
    pub fn new_string(string: String) -> Self {
        RString::Str(string)
    }

    pub fn new_bytes(bytes: Vec<u8>) -> Self {
        RString::Bytes(bytes)
    }

    pub fn convert_to_str(&mut self) -> Result<(), FromUtf8Error> {
        match self {
            RString::Str(_) => Ok(()),
            RString::Bytes(bytes) => match String::from_utf8(bytes.clone()) {
                Ok(s) => {
                    std::mem::replace(self, RString::Str(s));
                    Ok(())
                }
                Err(err) => Err(err),
            },
        }
    }

    pub fn parse<F: FromStr>(&self) -> Option<F> {
        match self {
            RString::Str(s) => FromStr::from_str(s).ok(),
            RString::Bytes(bytes) => match String::from_utf8(bytes.clone()) {
                Ok(s) => FromStr::from_str(&s).ok(),
                Err(_) => None,
            },
        }
    }
}

impl std::hash::Hash for RString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            RString::Str(s) => s.hash(state),
            RString::Bytes(b) => b.hash(state),
        };
    }
}

pub fn init_string(globals: &mut Globals) -> Value {
    let id = globals.get_ident_id("String");
    let class = ClassRef::from(id, globals.builtins.object);
    globals.add_builtin_instance_method(class, "+", string_add);
    globals.add_builtin_instance_method(class, "*", string_mul);
    globals.add_builtin_instance_method(class, "%", string_rem);
    globals.add_builtin_instance_method(class, "start_with?", string_start_with);
    globals.add_builtin_instance_method(class, "to_sym", string_to_sym);
    globals.add_builtin_instance_method(class, "intern", string_to_sym);
    globals.add_builtin_instance_method(class, "split", string_split);
    globals.add_builtin_instance_method(class, "sub", string_sub);
    globals.add_builtin_instance_method(class, "gsub", string_gsub);
    globals.add_builtin_instance_method(class, "gsub!", string_gsub_);
    globals.add_builtin_instance_method(class, "scan", string_scan);
    globals.add_builtin_instance_method(class, "=~", string_rmatch);
    globals.add_builtin_instance_method(class, "tr", string_tr);
    globals.add_builtin_instance_method(class, "size", string_size);
    globals.add_builtin_instance_method(class, "bytes", string_bytes);
    globals.add_builtin_instance_method(class, "chars", string_chars);
    globals.add_builtin_instance_method(class, "sum", string_sum);
    globals.add_builtin_instance_method(class, "upcase", string_upcase);
    globals.add_builtin_instance_method(class, "chomp", string_chomp);

    Value::class(globals, class)
}

fn string_add(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    expect_string!(lhs, vm, self_val);
    expect_string!(rhs, vm, args[0]);
    let res = format!("{}{}", lhs, rhs);
    Ok(Value::string(&vm.globals, res))
}

fn string_mul(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    expect_string!(lhs, vm, self_val);
    let rhs = match args[0].expect_fixnum(vm, "Rhs must be FixNum.")? {
        i if i < 0 => return Err(vm.error_argument("Negative argument.")),
        i => i as usize,
    };

    let res = lhs.repeat(rhs);
    Ok(Value::string(&vm.globals, res))
}

fn expect_char(vm: &mut VM, chars: &mut std::str::Chars) -> Result<char, RubyError> {
    let ch = match chars.next() {
        Some(ch) => ch,
        None => return Err(vm.error_argument("Invalid format character")),
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

fn string_rem(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
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
            let res = Value::string(&vm.globals, "".to_string());
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
            None => return Err(vm.error_argument("Incomplete format specifier. use '%%' instead.")),
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
            return Err(vm.error_argument("Too few arguments"));
        };
        // Specifier
        let val = arguments[arg_no];
        arg_no += 1;
        let format = match ch {
            'd' => {
                let val = val.expect_fixnum(&vm, "Invalid value for placeholder of Integer.")?;
                if zero_flag {
                    format!("{:0w$.p$}", val, w = width, p = precision)
                } else {
                    format!("{:w$.p$}", val, w = width, p = precision)
                }
            }
            'b' => {
                let val = val.expect_fixnum(&vm, "Invalid value for placeholder of Integer.")?;
                if zero_flag {
                    format!("{:0w$b}", val, w = width)
                } else {
                    format!("{:w$b}", val, w = width)
                }
            }
            'x' => {
                let val = val.expect_fixnum(&vm, "Invalid value for placeholder of Integer.")?;
                if zero_flag {
                    format!("{:0w$x}", val, w = width)
                } else {
                    format!("{:w$x}", val, w = width)
                }
            }
            'X' => {
                let val = val.expect_fixnum(&vm, "Invalid value for placeholder of Integer.")?;
                if zero_flag {
                    format!("{:0w$X}", val, w = width)
                } else {
                    format!("{:w$X}", val, w = width)
                }
            }
            'f' => {
                let val = val.expect_flonum(&vm, "Invalid value for placeholder of Float.")?;
                if zero_flag {
                    format!("{:0w$.p$}", val, w = width, p = precision)
                } else {
                    format!("{:w$.p$}", val, w = width, p = precision)
                }
            }
            _ => return Err(vm.error_argument("Invalid format character.")),
        };
        format_str.append(&mut format.chars().collect());
        next_char!(ch, chars);
    }

    let res = Value::string(&vm.globals, format_str.into_iter().collect());
    Ok(res)
}

fn string_start_with(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    expect_string!(string, vm, self_val);
    expect_string!(arg, vm, args[0]);
    let res = string.starts_with(arg);
    Ok(Value::bool(res))
}

fn string_to_sym(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    expect_string!(string, vm, self_val);
    let id = vm.globals.get_ident_id(string);
    Ok(Value::symbol(id))
}

fn string_split(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 2)?;
    expect_string!(string, vm, self_val);
    expect_string!(sep, vm, args[0]);
    let lim = if args.len() > 1 {
        args[1].expect_fixnum(vm, "Second arg must be Integer.")?
    } else {
        0
    };
    if lim == 1 {
        let vec = vec![Value::string(&vm.globals, string.to_string())];
        let ary = Value::array_from(&vm.globals, vec);
        return Ok(ary);
    } else if lim < 0 {
        let vec = string
            .split(sep)
            .map(|x| Value::string(&vm.globals, x.to_string()))
            .collect();
        let ary = Value::array_from(&vm.globals, vec);
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
        let vec = vec
            .iter()
            .map(|x| Value::string(&vm.globals, x.to_string()))
            .collect();
        let ary = Value::array_from(&vm.globals, vec);
        return Ok(ary);
    } else {
        let vec = string
            .splitn(lim as usize, sep)
            .map(|x| Value::string(&vm.globals, x.to_string()))
            .collect();
        let ary = Value::array_from(&vm.globals, vec);
        return Ok(ary);
    }
}

fn string_sub(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 2, 2)?;
    expect_string!(given, vm, self_val);
    expect_string!(replace, vm, args[1]);
    let res = if let Some(s) = args[0].as_string() {
        let re = vm.regexp_from_string(&s)?;
        Regexp::replace_one(vm, &re, given, replace)?
    } else if let Some(re) = args[0].as_regexp() {
        Regexp::replace_one(vm, &re.regexp, given, replace)?
    } else {
        return Err(vm.error_argument("1st arg must be RegExp or String."));
    };

    Ok(Value::string(&vm.globals, res))
}

fn string_gsub(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let (res, _) = gsub(vm, self_val, args)?;
    Ok(Value::string(&vm.globals, res))
}

fn string_gsub_(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let (res, changed) = gsub(vm, self_val, args)?;
    *self_val.rvalue_mut() = RValue::new_string(&vm.globals, res);
    let res = if changed { self_val } else { Value::nil() };
    Ok(res)
}

fn gsub(vm: &mut VM, self_val: Value, args: &Args) -> Result<(String, bool), RubyError> {
    match args.block {
        Some(block) => {
            vm.check_args_num(args.len(), 1, 1)?;
            expect_string!(given, vm, self_val);
            if let Some(s) = args[0].as_string() {
                let re = vm.regexp_from_string(&s)?;
                Ok(Regexp::replace_all_block(vm, &re, given, block)?)
            } else if let Some(re) = args[0].as_regexp() {
                Ok(Regexp::replace_all_block(vm, &re.regexp, given, block)?)
            } else {
                return Err(vm.error_argument("1st arg must be RegExp or String."));
            }
        }
        None => {
            vm.check_args_num(args.len(), 2, 2)?;
            expect_string!(given, vm, self_val);
            expect_string!(replace, vm, args[1]);
            if let Some(s) = args[0].as_string() {
                let re = vm.regexp_from_string(&s)?;
                Ok(Regexp::replace_all(vm, &re, given, replace)?)
            } else if let Some(re) = args[0].as_regexp() {
                Ok(Regexp::replace_all(vm, &re.regexp, given, replace)?)
            } else {
                return Err(vm.error_argument("1st arg must be RegExp or String."));
            }
        }
    }
}

fn string_scan(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    expect_string!(given, vm, self_val);
    let vec = if let Some(s) = args[0].as_string() {
        let re = vm.regexp_from_string(&s)?;
        Regexp::find_all(vm, &re, given)?
    } else if let Some(re) = args[0].as_regexp() {
        Regexp::find_all(vm, &re.regexp, given)?
    } else {
        return Err(vm.error_argument("1st arg must be RegExp or String."));
    };
    match args.block {
        Some(block) => {
            for arg in vec {
                match arg.as_array() {
                    Some(ary) => {
                        let len = ary.elements.len();
                        let mut block_args = Args::new(len);
                        for i in 0..len {
                            block_args[i] = ary.elements[i]
                        }
                        vm.eval_block(block, &block_args)?;
                    }
                    None => {
                        let block_args = Args::new1(None, arg);
                        vm.eval_block(block, &block_args)?;
                    }
                }
            }
            Ok(self_val)
        }
        None => Ok(Value::array_from(&vm.globals, vec)),
    }
}

fn string_rmatch(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    expect_string!(given, vm, self_val);
    if let Some(re) = args[0].as_regexp() {
        let res = match Regexp::find_one(vm, &re.regexp, given).unwrap() {
            Some(mat) => Value::fixnum(mat.start() as i64),
            None => Value::nil(),
        };
        return Ok(res);
    } else {
        return Err(vm.error_argument("1st arg must be RegExp."));
    };
}

fn string_tr(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 2, 2)?;
    expect_string!(rec, vm, self_val);
    expect_string!(from, vm, args[0]);
    expect_string!(to, vm, args[1]);
    let res = rec.replace(from, to);
    Ok(Value::string(&vm.globals, res))
}

fn string_size(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    expect_string!(rec, vm, self_val);
    Ok(Value::fixnum(rec.chars().count() as i64))
}

fn string_bytes(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    expect_bytes!(bytes, vm, self_val);
    let mut ary = vec![];
    for b in bytes {
        ary.push(Value::fixnum(*b as i64));
    }
    Ok(Value::array_from(&vm.globals, ary))
}

fn string_chars(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    expect_string!(string, vm, self_val);
    let ary: Vec<Value> = string
        .chars()
        .map(|c| Value::string(&vm.globals, c.to_string()))
        .collect();
    Ok(Value::array_from(&vm.globals, ary))
}

fn string_sum(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    expect_bytes!(bytes, vm, self_val);
    let mut sum = 0;
    for b in bytes {
        sum += *b as u64;
    }
    Ok(Value::fixnum((sum & ((1 << 16) - 1)) as i64))
}

fn string_upcase(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    expect_string!(string, vm, self_val);
    let res = string.to_uppercase();
    Ok(Value::string(&vm.globals, res))
}

fn string_chomp(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    expect_string!(string, vm, self_val);
    let res = string.trim_end_matches('\n').to_string();
    Ok(Value::string(&vm.globals, res))
}

#[cfg(test)]
mod test {
    use crate::test::*;

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
        "#;
        assert_script(program);
    }

    #[test]
    fn string_sum() {
        let program = r#"
        assert 394, "abcd".sum
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
    fn string_upcase() {
        let program = r#"
        assert "RUBY IS GREAT.", "ruby is great.".upcase
        "#;
        assert_script(program);
    }

    #[test]
    fn string_chomp() {
        let program = r#"
        assert "Ruby", "Ruby\n\n\n".chomp
        "#;
        assert_script(program);
    }
}
