use crate::vm::*;

pub fn init_string(globals: &mut Globals) -> PackedValue {
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
    PackedValue::class(globals, class)
}

fn string_start_with(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let string = receiver.as_string().unwrap();
    let arg = match args[0].as_string() {
        Some(arg) => arg,
        None => return Err(vm.error_argument("An arg must be a String.")),
    };
    let res = string.starts_with(arg);
    Ok(PackedValue::bool(res))
}

fn string_to_sym(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let string = receiver.as_string().unwrap();
    let id = vm.globals.get_ident_id(string);
    Ok(PackedValue::symbol(id))
}

fn string_split(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    vm.check_args_num(args.len(), 1, 2)?;
    let string = receiver.as_string().unwrap();
    let sep = args[0].as_string().unwrap();
    let lim = if args.len() > 1 {
        args[1].expect_fixnum(vm, "Second arg must be Integer.")?
    } else {
        0
    };
    if lim == 1 {
        let vec = vec![PackedValue::string(string.to_string())];
        let ary = PackedValue::array_from(&vm.globals, vec);
        return Ok(ary);
    } else if lim < 0 {
        let vec = string
            .split(sep)
            .map(|x| PackedValue::string(x.to_string()))
            .collect();
        let ary = PackedValue::array_from(&vm.globals, vec);
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
            .map(|x| PackedValue::string(x.to_string()))
            .collect();
        let ary = PackedValue::array_from(&vm.globals, vec);
        return Ok(ary);
    } else {
        let vec = string
            .splitn(lim as usize, sep)
            .map(|x| PackedValue::string(x.to_string()))
            .collect();
        let ary = PackedValue::array_from(&vm.globals, vec);
        return Ok(ary);
    }
}

fn string_gsub(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    vm.check_args_num(args.len(), 1, 2)?;
    let given = receiver.as_string().unwrap();
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
    Ok(PackedValue::string(res))
}

fn string_rmatch(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let given = receiver.as_string().unwrap();
    let regexp = if let Some(re) = args[0].as_regexp() {
        re.regexp.clone()
    } else {
        return Err(vm.error_argument("1st arg must be RegExp."));
    };
    let res = match regexp.find(given) {
        Some(mat) => PackedValue::fixnum(mat.start() as i64),
        None => PackedValue::nil(),
    };
    Ok(res)
}

fn string_tr(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    vm.check_args_num(args.len(), 2, 2)?;
    let rec = receiver.as_string().unwrap();
    let from = args[0].as_string().unwrap();
    let to = args[1].as_string().unwrap();
    let res = rec.replace(from, to);
    Ok(PackedValue::string(res))
}

fn string_size(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let rec = receiver.as_string().unwrap();
    Ok(PackedValue::fixnum(rec.chars().count() as i64))
}
