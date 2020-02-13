use crate::vm::*;
use regex::{Error, Regex};

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
    let class = ClassRef::from(id, globals.object);
    globals.add_builtin_instance_method(class, "push", regexp_push);
    Value::class(globals, class)
}

// Class methods

// Instance methods

fn regexp_push(vm: &mut VM, args: &Args, _block: Option<MethodRef>) -> VMResult {
    let mut aref = args
        .self_value
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    for i in 0..args.len() {
        aref.elements.push(args[i]);
    }
    Ok(args.self_value)
}
