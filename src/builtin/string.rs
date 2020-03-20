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
    globals.add_builtin_instance_method(class, "start_with?", string_start_with);
    globals.add_builtin_instance_method(class, "to_sym", string_to_sym);
    globals.add_builtin_instance_method(class, "intern", string_to_sym);
    globals.add_builtin_instance_method(class, "split", string_split);
    globals.add_builtin_instance_method(class, "sub", string_sub);
    globals.add_builtin_instance_method(class, "gsub", string_gsub);
    globals.add_builtin_instance_method(class, "scan", string_scan);
    globals.add_builtin_instance_method(class, "=~", string_rmatch);
    globals.add_builtin_instance_method(class, "tr", string_tr);
    globals.add_builtin_instance_method(class, "size", string_size);
    globals.add_builtin_instance_method(class, "bytes", string_bytes);
    globals.add_builtin_instance_method(class, "chars", string_chars);
    globals.add_builtin_instance_method(class, "sum", string_sum);

    Value::class(globals, class)
}

fn string_add(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let lhs = expect_string!(vm, args.self_value);
    let rhs = expect_string!(vm, args[0]);
    let res = format!("{}{}", lhs, rhs);
    Ok(Value::string(&vm.globals, res))
}

fn string_mul(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let lhs = expect_string!(vm, args.self_value);
    let rhs = match args[0].expect_fixnum(vm, "Rhs must be FixNum.")? {
        i if i < 0 => return Err(vm.error_argument("Negative argument.")),
        i => i as usize,
    };

    let res = lhs.repeat(rhs);
    Ok(Value::string(&vm.globals, res))
}

fn string_start_with(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let string = expect_string!(vm, args.self_value);
    let arg = expect_string!(vm, args[0]);
    let res = string.starts_with(&arg);
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
        let vec = vec![Value::string(&vm.globals, string.to_string())];
        let ary = Value::array_from(&vm.globals, vec);
        return Ok(ary);
    } else if lim < 0 {
        let vec = string
            .split(&sep)
            .map(|x| Value::string(&vm.globals, x.to_string()))
            .collect();
        let ary = Value::array_from(&vm.globals, vec);
        return Ok(ary);
    } else if lim == 0 {
        let mut vec: Vec<&str> = string.split(&sep).collect();
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
            .splitn(lim as usize, &sep)
            .map(|x| Value::string(&vm.globals, x.to_string()))
            .collect();
        let ary = Value::array_from(&vm.globals, vec);
        return Ok(ary);
    }
}

fn string_sub(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 2, 2)?;
    let given = expect_string!(vm, args.self_value);
    let replace = expect_string!(vm, args[1]);
    let res = if let Some(s) = as_string!(args[0]) {
        let re = vm.regexp_from_string(&s)?;
        Regexp::replace_one(vm, &re, &given, &replace)
    } else if let Some(re) = args[0].as_regexp() {
        Regexp::replace_one(vm, &re.regexp, &given, &replace)
    } else {
        return Err(vm.error_argument("1st arg must be RegExp or String."));
    };
    let res = match res {
        Ok(res) => res,
        Err(err) => return Err(vm.error_argument(format!("capture failed. {}", err))),
    };

    Ok(Value::string(&vm.globals, res))
}

fn string_gsub(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 2, 2)?;
    let given = expect_string!(vm, args.self_value);
    let replace = expect_string!(vm, args[1]);
    let res = if let Some(s) = as_string!(args[0]) {
        let re = vm.regexp_from_string(&s)?;
        Regexp::replace_all(vm, &re, &given, &replace)
    } else if let Some(re) = args[0].as_regexp() {
        Regexp::replace_all(vm, &re.regexp, &given, &replace)
    } else {
        return Err(vm.error_argument("1st arg must be RegExp or String."));
    };
    let res = match res {
        Ok(res) => res,
        Err(err) => return Err(vm.error_internal(format!("Capture failed. {}", err))),
    };

    Ok(Value::string(&vm.globals, res))
}

fn string_scan(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let given = expect_string!(vm, args.self_value);
    let vec = if let Some(s) = as_string!(args[0]) {
        let re = vm.regexp_from_string(&s)?;
        Regexp::find_all(vm, &re, &given)
    } else if let Some(re) = args[0].as_regexp() {
        Regexp::find_all(vm, &re.regexp, &given)
    } else {
        return Err(vm.error_argument("1st arg must be RegExp or String."));
    }?;
    match args.block {
        Some(block) => {
            let self_value = vm.context().self_value;
            for arg in vec {
                let block_args = Args::new1(self_value, None, arg);
                vm.eval_block(block, &block_args)?;
            }
            Ok(args.self_value)
        }
        None => Ok(Value::array_from(&vm.globals, vec)),
    }
}

fn string_rmatch(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let given = expect_string!(vm, args.self_value);
    if let Some(re) = args[0].as_regexp() {
        let res = match Regexp::find_one(vm, &re.regexp, &given).unwrap() {
            Some(mat) => Value::fixnum(mat.start() as i64),
            None => Value::nil(),
        };
        return Ok(res);
    } else {
        return Err(vm.error_argument("1st arg must be RegExp."));
    };
}

fn string_tr(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 2, 2)?;
    let rec = expect_string!(vm, args.self_value);
    let from = expect_string!(vm, args[0]);
    let to = expect_string!(vm, args[1]);
    let res = rec.replace(&from, &to);
    Ok(Value::string(&vm.globals, res))
}

fn string_size(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let rec = expect_string!(vm, args.self_value);
    Ok(Value::fixnum(rec.chars().count() as i64))
}

fn string_bytes(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let bytes = expect_bytes!(vm, args.self_value);
    let mut ary = vec![];
    for b in bytes {
        ary.push(Value::fixnum(b as i64));
    }
    Ok(Value::array_from(&vm.globals, ary))
}

fn string_chars(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let string = expect_string!(vm, args.self_value);
    let ary: Vec<Value> = string
        .chars()
        .map(|c| Value::string(&vm.globals, c.to_string()))
        .collect();
    Ok(Value::array_from(&vm.globals, ary))
}

fn string_sum(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let bytes = expect_bytes!(vm, args.self_value);
    let mut sum = 0;
    for b in bytes {
        sum += b as u64;
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
}
