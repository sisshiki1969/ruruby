use crate::vm::*;
use fancy_regex::{Error, Regex};

#[derive(Debug)]
pub struct RegexpInfo {
    pub regexp: Regex,
}

impl RegexpInfo {
    pub fn new(regexp: Regex) -> Self {
        RegexpInfo { regexp }
    }
}

pub type RegexpRef = Ref<RegexpInfo>;

impl RegexpRef {
    pub fn from(reg: Regex) -> Self {
        RegexpRef::new(RegexpInfo::new(reg))
    }

    pub fn from_string(reg_str: &String) -> Result<Self, Error> {
        let regex = Regex::new(reg_str)?;
        Ok(RegexpRef::new(RegexpInfo::new(regex)))
    }
}

pub fn init_regexp(globals: &mut Globals) -> Value {
    let id = globals.get_ident_id("Regexp");
    let classref = ClassRef::from(id, globals.builtins.object);
    let regexp = Value::class(globals, classref);
    globals.add_builtin_class_method(regexp, "new", regexp_new);
    globals.add_builtin_class_method(regexp, "compile", regexp_new);
    globals.add_builtin_class_method(regexp, "escape", regexp_escape);
    globals.add_builtin_class_method(regexp, "quote", regexp_escape);
    regexp
}

// Class methods

fn regexp_new(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let val = match args[0].as_string() {
        Some(string) => vm.create_regexp(string)?,
        None => return Err(vm.error_argument("Must be String")),
    };
    Ok(val)
}

fn regexp_escape(vm: &mut VM, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let res = match args[0].as_string() {
        Some(s) => regex::escape(s),
        None => return Err(vm.error_argument("Must be String")),
    };
    let regexp = Value::string(res);
    Ok(regexp)
}

// Instance methods
