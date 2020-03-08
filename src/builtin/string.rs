use crate::vm::*;
use fancy_regex::Regex;
use std::string::FromUtf8Error;

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
    globals.add_builtin_instance_method(class, "start_with?", string_start_with);
    globals.add_builtin_instance_method(class, "to_sym", string_to_sym);
    globals.add_builtin_instance_method(class, "intern", string_to_sym);
    globals.add_builtin_instance_method(class, "split", string_split);
    globals.add_builtin_instance_method(class, "sub", string_sub);
    globals.add_builtin_instance_method(class, "gsub", string_gsub);
    globals.add_builtin_instance_method(class, "=~", string_rmatch);
    globals.add_builtin_instance_method(class, "tr", string_tr);
    globals.add_builtin_instance_method(class, "size", string_size);
    globals.add_builtin_instance_method(class, "bytes", string_bytes);
    globals.add_builtin_instance_method(class, "sum", string_sum);

    Value::class(globals, class)
}

macro_rules! expect_string {
    ($vm:ident, $val:expr) => {
        match $val.as_string() {
            Some(s) => s,
            None => return Err($vm.error_argument("Must be a String.")),
        };
    };
}

fn string_add(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let lhs = expect_string!(vm, args.self_value);
    let rhs = expect_string!(vm, args[0]);
    let res = format!("{}{}", lhs, rhs);
    Ok(Value::string(res))
}

fn string_mul(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let lhs = expect_string!(vm, args.self_value);
    let rhs = match args[0].expect_fixnum(vm, "Rhs must be FixNum.")? {
        i if i < 0 => return Err(vm.error_argument("Negative argument.")),
        i => i as usize,
    };

    let res = lhs.repeat(rhs);
    Ok(Value::string(res))
}

fn string_start_with(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let string = expect_string!(vm, args.self_value);
    let arg = expect_string!(vm, args[0]);
    let res = string.starts_with(arg);
    Ok(Value::bool(res))
}

fn string_to_sym(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let string = expect_string!(vm, args.self_value);
    let id = vm.globals.get_ident_id(string);
    Ok(Value::symbol(id))
}

fn string_split(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 2)?;
    let string = expect_string!(vm, args.self_value);
    let sep = expect_string!(vm, args[0]);
    let lim = if args.len() > 1 {
        args[1].expect_fixnum(vm, "Second arg must be Integer.")?
    } else {
        0
    };
    if lim == 1 {
        let vec = vec![Value::string(string.to_string())];
        let ary = Value::array_from(&vm.globals, vec);
        return Ok(ary);
    } else if lim < 0 {
        let vec = string
            .split(sep)
            .map(|x| Value::string(x.to_string()))
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
        let vec = vec.iter().map(|x| Value::string(x.to_string())).collect();
        let ary = Value::array_from(&vm.globals, vec);
        return Ok(ary);
    } else {
        let vec = string
            .splitn(lim as usize, sep)
            .map(|x| Value::string(x.to_string()))
            .collect();
        let ary = Value::array_from(&vm.globals, vec);
        return Ok(ary);
    }
}

fn replace_one(re: &Regex, given: &String, replace: &String) -> Result<String, String> {
    let res = match re.captures(given) {
        Ok(None) => given.to_string(),
        Ok(Some(captures)) => {
            let mut res = given.to_string();
            let c = captures.get(0).unwrap();
            let mut rep = "".to_string();
            let mut escape = false;
            for ch in replace.chars() {
                if escape {
                    match ch {
                        '0'..='9' => {
                            let i = ch as usize - '0' as usize;
                            match captures.get(i) {
                                Some(m) => rep += m.as_str(),
                                None => {}
                            };
                        }
                        _ => rep.push(ch),
                    };
                    escape = false;
                } else {
                    if ch != '\\' {
                        rep.push(ch);
                    } else {
                        escape = true;
                    };
                }
            }
            res.replace_range(c.start()..c.end(), &rep);
            res
        }
        Err(err) => return Err(format!("{:?}", err)),
    };
    Ok(res)
}

fn string_sub(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 2, 2)?;
    let given = expect_string!(vm, args.self_value);
    let replace = expect_string!(vm, args[1]);
    let res = if let Some(s) = args[0].as_string() {
        match fancy_regex::Regex::new(&regex::escape(&s)) {
            Ok(re) => replace_one(&re, given, replace),
            Err(_) => return Err(vm.error_argument("Illegal string for RegExp.")),
        }
    } else if let Some(re) = args[0].as_regexp() {
        replace_one(&re.regexp, given, replace)
    } else {
        return Err(vm.error_argument("1st arg must be RegExp or String."));
    };
    let res = match res {
        Ok(res) => res,
        Err(err) => return Err(vm.error_argument(format!("capture failed. {}", err))),
    };

    Ok(Value::string(res))
}

fn string_gsub(vm: &mut VM, args: &Args) -> VMResult {
    fn replace_all(re: &Regex, given: &String, replace: &String) -> Result<String, String> {
        let mut range = vec![];
        let mut i = 0;
        loop {
            match re.captures_from_pos(given, i) {
                Ok(None) => break,
                Ok(Some(captures)) => {
                    let c = captures.get(0).unwrap();
                    i = c.end();
                    range.push((c.start(), c.end()));
                }
                Err(err) => return Err(format!("{:?}", err)),
            };
        }
        let mut res = given.to_string();
        for (start, end) in range.iter().rev() {
            res.replace_range(start..end, replace);
        }
        Ok(res)
    }

    vm.check_args_num(args.len(), 2, 2)?;
    let given = expect_string!(vm, args.self_value);
    let replace = expect_string!(vm, args[1]);
    let res = if let Some(s) = args[0].as_string() {
        match fancy_regex::Regex::new(&regex::escape(&s)) {
            Ok(re) => replace_all(&re, given, replace),
            Err(_) => return Err(vm.error_argument("Illegal string for RegExp.")),
        }
    } else if let Some(re) = args[0].as_regexp() {
        replace_all(&re.regexp, given, replace)
    } else {
        return Err(vm.error_argument("1st arg must be RegExp or String."));
    };
    let res = match res {
        Ok(res) => res,
        Err(err) => return Err(vm.error_argument(format!("capture failed. {}", err))),
    };

    Ok(Value::string(res))
}

fn string_rmatch(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let given = args.self_value.as_string().unwrap();
    let matched = if let Some(re) = args[0].as_regexp() {
        re.regexp.find(given).unwrap()
    } else {
        return Err(vm.error_argument("1st arg must be RegExp."));
    };
    let res = match matched {
        Some(mat) => Value::fixnum(mat.start() as i64),
        None => Value::nil(),
    };
    Ok(res)
}

fn string_tr(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 2, 2)?;
    let rec = args.self_value.as_string().unwrap();
    let from = args[0].as_string().unwrap();
    let to = args[1].as_string().unwrap();
    let res = rec.replace(from, to);
    Ok(Value::string(res))
}

fn string_size(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let rec = args.self_value.as_string().unwrap();
    Ok(Value::fixnum(rec.chars().count() as i64))
}

fn string_bytes(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let bytes = match args.self_value.as_bytes() {
        Some(bytes) => bytes,
        None => return Err(vm.error_type("Receiver must be String.")),
    };
    let mut ary = vec![];
    for b in bytes {
        ary.push(Value::fixnum(*b as i64));
    }
    Ok(Value::array_from(&vm.globals, ary))
}

fn string_sum(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let bytes = match args.self_value.as_bytes() {
        Some(bytes) => bytes,
        None => return Err(vm.error_type("Receiver must be String.")),
    };
    let mut sum = 0;
    for b in bytes {
        sum += *b as u64;
    }
    Ok(Value::fixnum((sum & ((1 << 16) - 1)) as i64))
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
}
