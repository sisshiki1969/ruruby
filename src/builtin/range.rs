use crate::*;

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct RangeInfo {
    pub start: Value,
    pub end: Value,
    pub exclude: bool,
}

impl RangeInfo {
    pub fn new(start: Value, end: Value, exclude: bool) -> Self {
        RangeInfo {
            start,
            end,
            exclude,
        }
    }
}

pub fn init_range(globals: &mut Globals) -> Value {
    let id = globals.get_ident_id("Range");
    let class = ClassRef::from(id, globals.builtins.object);
    let obj = Value::class(globals, class);
    globals.add_builtin_instance_method(class, "map", range_map);
    globals.add_builtin_instance_method(class, "each", range_each);
    globals.add_builtin_instance_method(class, "all?", range_all);
    globals.add_builtin_instance_method(class, "begin", range_begin);
    globals.add_builtin_instance_method(class, "first", range_first);
    globals.add_builtin_instance_method(class, "end", range_end);
    globals.add_builtin_instance_method(class, "last", range_last);
    globals.add_builtin_instance_method(class, "to_a", range_toa);
    globals.add_builtin_class_method(obj, "new", range_new);
    obj
}

fn range_new(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let len = args.len();
    vm.check_args_num(len, 2, 3)?;
    let (start, end) = (args[0], args[1]);
    let exclude_end = if len == 2 {
        false
    } else {
        vm.val_to_bool(args[2])
    };
    Ok(Value::range(&vm.globals, start, end, exclude_end))
}

fn range_begin(_vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    Ok(range.start)
}

fn range_end(_vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    Ok(range.end)
}

fn range_first(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    let start = range.start.as_fixnum().unwrap();
    let mut end = range.end.as_fixnum().unwrap() - if range.exclude { 1 } else { 0 };
    if args.len() == 0 {
        return Ok(range.start);
    };
    let arg = args[0].expect_integer(&vm, "Argument")?;
    if arg < 0 {
        return Err(vm.error_argument("Negative array size"));
    };
    let mut v = vec![];
    if start + arg - 1 < end {
        end = start + arg - 1;
    };
    for i in start..=end {
        v.push(Value::fixnum(i));
    }
    Ok(Value::array_from(&vm.globals, v))
}

fn range_last(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    let mut start = range.start.as_fixnum().unwrap();
    let end = range.end.as_fixnum().unwrap() - if range.exclude { 1 } else { 0 };
    if args.len() == 0 {
        return Ok(range.end);
    };
    let arg = args[0].expect_integer(&vm, "Argument")?;
    if arg < 0 {
        return Err(vm.error_argument("Negative array size"));
    };
    let mut v = vec![];
    if end - arg + 1 > start {
        start = end - arg + 1;
    };
    for i in start..=end {
        v.push(Value::fixnum(i));
    }
    Ok(Value::array_from(&vm.globals, v))
}

fn range_map(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    let method = vm.expect_block(args.block)?;
    let mut res = vec![];
    let start = range.start.expect_integer(&vm, "Start")?;
    let end = range.end.expect_integer(&vm, "End")? + if range.exclude { 0 } else { 1 };
    for i in start..end {
        let arg = Args::new1(Value::fixnum(i));
        let val = vm.eval_block(method, &arg)?;
        res.push(val);
    }
    let res = Value::array_from(&vm.globals, res);
    Ok(res)
}

fn range_each(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    let method = vm.expect_block(args.block)?;
    let start = range.start.expect_integer(&vm, "Start")?;
    let end = range.end.expect_integer(&vm, "End")? + if range.exclude { 0 } else { 1 };
    for i in start..end {
        let arg = Args::new1(Value::fixnum(i));
        vm.eval_block(method, &arg)?;
    }
    Ok(self_val)
}

fn range_all(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    let method = vm.expect_block(args.block)?;
    let start = range.start.expect_integer(&vm, "Start")?;
    let end = range.end.expect_integer(&vm, "End")? + if range.exclude { 0 } else { 1 };
    for i in start..end {
        let arg = Args::new1(Value::fixnum(i));
        let res = vm.eval_block(method, &arg)?;
        if !vm.val_to_bool(res) {
            return Ok(Value::false_val());
        }
    }
    Ok(Value::true_val())
}

fn range_toa(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    let start = range.start.expect_integer(&vm, "Range.start")?;
    let end = range.end.expect_integer(&vm, "Range.end")?;
    let mut v = vec![];
    if range.exclude {
        for i in start..end {
            v.push(Value::fixnum(i));
        }
    } else {
        for i in start..=end {
            v.push(Value::fixnum(i));
        }
    }
    Ok(Value::array_from(&vm.globals, v))
}
