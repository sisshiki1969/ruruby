use crate::vm::*;
use regex::Regex;

#[derive(Debug, Clone)]
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

    pub fn from_string(reg_str: &String) -> Result<Self, regex::Error> {
        let regex = Regex::new(reg_str)?;
        Ok(RegexpRef::new(RegexpInfo::new(regex)))
    }
}

pub fn init_regexp(globals: &mut Globals) -> ClassRef {
    let id = globals.get_ident_id("Regexp");
    let regexp_class = ClassRef::from(id, globals.regexp_class);
    globals.add_builtin_instance_method(regexp_class, "push", regexp::regexp_push);
    //    globals.add_builtin_class_method(array_class, "new", array::array_new);
    regexp_class
}

// Class methods
/*
fn array_new(
    vm: &mut VM,
    _receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    vm.check_args_num(args.len(), 0, 2)?;
    let array_vec = match args.len() {
        0 => vec![],
        1 => match args[0].unpack() {
            Value::FixNum(num) if num >= 0 => vec![PackedValue::nil(); num as usize],
            Value::Object(oref) => match oref.kind {
                ObjKind::Array(aref) => aref.elements.clone(),
                _ => return Err(vm.error_nomethod("Invalid arguments")),
            },
            _ => return Err(vm.error_nomethod("Invalid arguments")),
        },
        2 => {
            let arg_num = args[0]
                .as_fixnum()
                .ok_or(vm.error_nomethod("Invalid arguments"))?;
            vec![args[1]; arg_num as usize]
        }
        _ => unreachable!(),
    };
    let array = PackedValue::array_from(&vm.globals, array_vec);
    Ok(array)
}
*/
// Instance methods

fn regexp_push(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let mut aref = receiver
        .as_array()
        .ok_or(vm.error_nomethod("Receiver must be an array."))?;
    for arg in args.iter() {
        aref.elements.push(arg.clone());
    }
    Ok(receiver)
}
