use crate::vm::*;
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

    pub fn to_str(self) -> Result<Self, FromUtf8Error> {
        match self {
            RString::Str(_) => Ok(self),
            RString::Bytes(bytes) => match String::from_utf8(bytes) {
                Ok(s) => Ok(Self::new_string(s)),
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
    let class = ClassRef::from(id, globals.object);
    globals.add_builtin_instance_method(class, "start_with?", string_start_with);
    globals.add_builtin_instance_method(class, "to_sym", string_to_sym);
    globals.add_builtin_instance_method(class, "intern", string_to_sym);
    globals.add_builtin_instance_method(class, "split", string_split);
    globals.add_builtin_instance_method(class, "gsub", string_gsub);
    globals.add_builtin_instance_method(class, "=~", string_rmatch);
    globals.add_builtin_instance_method(class, "tr", string_tr);
    globals.add_builtin_instance_method(class, "size", string_size);
    Value::class(globals, class)
}

fn string_start_with(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let string = args.self_value.as_string().unwrap();
    let arg = match args[0].as_string() {
        Some(arg) => arg,
        None => return Err(vm.error_argument("An arg must be a String.")),
    };
    let res = string.starts_with(arg);
    Ok(Value::bool(res))
}

fn string_to_sym(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let string = args.self_value.as_string().unwrap();
    let id = vm.globals.get_ident_id(string);
    Ok(Value::symbol(id))
}

fn string_split(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 1, 2)?;
    let string = args.self_value.as_string().unwrap();
    let sep = args[0].as_string().unwrap();
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

fn string_gsub(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 2, 2)?;
    let given = args.self_value.as_string().unwrap();
    let regexp = if let Some(s) = args[0].as_string() {
        match regex::Regex::new(&regex::escape(&s)) {
            Ok(re) => re,
            Err(_) => return Err(vm.error_argument("Illegal string for RegExp.")),
        }
    } else if let Some(re) = args[0].as_regexp() {
        re.regexp.clone()
    } else {
        return Err(vm.error_argument("1st arg must be RegExp or String."));
    };
    let replace = args[1].as_string().unwrap();
    let res = regexp.replace_all(&given, replace.as_str()).to_string();
    Ok(Value::string(res))
}

fn string_rmatch(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let given = args.self_value.as_string().unwrap();
    let regexp = if let Some(re) = args[0].as_regexp() {
        re.regexp.clone()
    } else {
        return Err(vm.error_argument("1st arg must be RegExp."));
    };
    let res = match regexp.find(given) {
        Some(mat) => Value::fixnum(mat.start() as i64),
        None => Value::nil(),
    };
    Ok(res)
}

fn string_tr(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 2, 2)?;
    let rec = args.self_value.as_string().unwrap();
    let from = args[0].as_string().unwrap();
    let to = args[1].as_string().unwrap();
    let res = rec.replace(from, to);
    Ok(Value::string(res))
}

fn string_size(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let rec = args.self_value.as_string().unwrap();
    Ok(Value::fixnum(rec.chars().count() as i64))
}
