use crate::vm::*;

#[derive(Debug, Clone, PartialEq)]
pub struct RangeInfo {
    pub start: PackedValue,
    pub end: PackedValue,
    pub exclude: bool,
}

pub type RangeRef = Ref<RangeInfo>;

impl RangeRef {
    pub fn new_range(start: PackedValue, end: PackedValue, exclude: bool) -> Self {
        let info = RangeInfo {
            start,
            end,
            exclude,
        };
        RangeRef::new(info)
    }
}

pub fn init_range(globals: &mut Globals) -> ClassRef {
    let id = globals.get_ident_id("Range");
    let class = ClassRef::from(id, globals.object_class);
    globals.add_builtin_instance_method(class, "map", range_map);
    globals.add_builtin_instance_method(class, "begin", range_begin);
    globals.add_builtin_instance_method(class, "first", range_first);
    globals.add_builtin_instance_method(class, "end", range_end);
    globals.add_builtin_instance_method(class, "last", range_last);
    globals.add_builtin_class_method(class, "new", range_new);
    class
}

fn range_new(
    vm: &mut VM,
    _receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let len = args.len();
    if len < 2 || 3 < len {
        return Err(vm.error_argument(format!(
            "Wrong number of arguments. (given {}, expected 2..3)",
            len
        )));
    }
    let (start, end) = (args[0], args[1]);
    let exclude_end = if len == 2 {
        false
    } else {
        vm.val_to_bool(args[2])
    };
    Ok(PackedValue::range(&vm.globals, start, end, exclude_end))
}

fn range_begin(
    _vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let range = receiver.as_range().unwrap();
    Ok(range.start)
}

fn range_end(
    _vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let range = receiver.as_range().unwrap();
    Ok(range.end)
}

fn range_first(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let range = receiver.as_range().unwrap();
    let start = range.start.as_fixnum().unwrap();
    let mut end = range.end.as_fixnum().unwrap() - if range.exclude { 1 } else { 0 };
    if args.len() == 0 {
        return Ok(range.start);
    };
    let arg = match args[0].as_fixnum() {
        Some(i) => i,
        None => return Err(vm.error_type("Must be an integer.")),
    };
    if arg < 0 {
        return Err(vm.error_argument("Negative array size"));
    };
    let mut v = vec![];
    if start + arg - 1 < end {
        end = start + arg - 1;
    };
    for i in start..=end {
        v.push(PackedValue::fixnum(i));
    }
    Ok(PackedValue::array(&vm.globals, ArrayRef::from(v)))
}

fn range_last(
    vm: &mut VM,
    receiver: PackedValue,
    args: VecArray,
    _block: Option<MethodRef>,
) -> VMResult {
    let range = receiver.as_range().unwrap();
    let mut start = range.start.as_fixnum().unwrap();
    let end = range.end.as_fixnum().unwrap() - if range.exclude { 1 } else { 0 };
    if args.len() == 0 {
        return Ok(range.end);
    };
    let arg = match args[0].as_fixnum() {
        Some(i) => i,
        None => return Err(vm.error_type("Must be an integer.")),
    };
    if arg < 0 {
        return Err(vm.error_argument("Negative array size"));
    };
    let mut v = vec![];
    if end - arg + 1 > start {
        start = end - arg + 1;
    };
    for i in start..=end {
        v.push(PackedValue::fixnum(i));
    }
    Ok(PackedValue::array(&vm.globals, ArrayRef::from(v)))
}

fn range_map(
    vm: &mut VM,
    receiver: PackedValue,
    _args: VecArray,
    block: Option<MethodRef>,
) -> VMResult {
    let range = receiver.as_range().unwrap();
    let iseq = match block {
        Some(method) => vm.globals.get_method_info(method).as_iseq(&vm)?,
        None => return Err(vm.error_argument("Currently, needs block.")),
    };
    let mut res = vec![];
    let context = vm.context();
    let start = if let Some(start) = range.start.as_fixnum() {
        start
    } else {
        return Err(vm.error_argument("Currently, start must be an Integer."));
    };
    let end = if let Some(end) = range.end.as_fixnum() {
        end + if range.exclude { 0 } else { 1 }
    } else {
        return Err(vm.error_argument("Currently, end must be an Integer."));
    };
    for i in start..end {
        vm.vm_run(
            context.self_value,
            iseq,
            Some(context),
            VecArray::new1(PackedValue::fixnum(i)),
            None,
        )?;
        res.push(vm.exec_stack.pop().unwrap());
    }
    let res = PackedValue::array(&vm.globals, ArrayRef::from(res));
    Ok(res)
}
