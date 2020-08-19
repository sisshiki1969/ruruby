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

    pub fn to_s(&self, vm: &mut VM) -> String {
        let start = vm.val_to_s(self.start);
        let end = vm.val_to_s(self.end);
        let sym = if self.exclude { "..." } else { ".." };
        format!("{}{}{}", start, sym, end)
    }

    pub fn inspect(&self, vm: &mut VM) -> String {
        let start = vm.val_inspect(self.start);
        let end = vm.val_inspect(self.end);
        let sym = if self.exclude { "..." } else { ".." };
        format!("{}{}{}", start, sym, end)
    }
}

pub fn init(globals: &mut Globals) -> Value {
    let id = IdentId::get_id("Range");
    let class = ClassRef::from(id, globals.builtins.object);
    let obj = Value::class(globals, class);
    globals.add_builtin_instance_method(class, "to_s", to_s);
    globals.add_builtin_instance_method(class, "inspect", inspect);
    globals.add_builtin_instance_method(class, "map", map);
    globals.add_builtin_instance_method(class, "flat_map", flat_map);
    globals.add_builtin_instance_method(class, "each", each);
    globals.add_builtin_instance_method(class, "all?", all);
    globals.add_builtin_instance_method(class, "begin", begin);
    globals.add_builtin_instance_method(class, "first", first);
    globals.add_builtin_instance_method(class, "end", end);
    globals.add_builtin_instance_method(class, "last", last);
    globals.add_builtin_instance_method(class, "to_a", to_a);
    globals.add_builtin_class_method(obj, "new", range_new);
    obj
}

fn range_new(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let len = args.len();
    vm.check_args_range(len, 2, 3)?;
    let (start, end) = (args[0], args[1]);
    let exclude_end = if len == 2 { false } else { args[2].to_bool() };
    Ok(Value::range(&vm.globals, start, end, exclude_end))
}

fn to_s(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    let res = range.to_s(vm);
    Ok(Value::string(&vm.globals.builtins, res))
}

fn inspect(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    let res = range.inspect(vm);
    Ok(Value::string(&vm.globals.builtins, res))
}

fn begin(_vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    Ok(range.start)
}

fn end(_vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    Ok(range.end)
}

fn first(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
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

fn last(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
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

fn map(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    let method = vm.expect_block(args.block)?;
    let start = range.start.expect_integer(&vm, "Start")?;
    let end = range.end.expect_integer(&vm, "End")? + if range.exclude { 0 } else { 1 };
    let mut arg = Args::new1(Value::nil());
    let mut res = vec![];
    for i in start..end {
        arg[0] = Value::fixnum(i);
        let val = vm.eval_block(method, &arg)?;
        vm.temp_push(val);
        res.push(val);
    }
    let res = Value::array_from(&vm.globals, res);
    Ok(res)
}

fn flat_map(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    let method = vm.expect_block(args.block)?;
    let start = range.start.expect_integer(&vm, "Start")?;
    let end = range.end.expect_integer(&vm, "End")? + if range.exclude { 0 } else { 1 };
    let mut arg = Args::new1(Value::nil());
    let mut res = vec![];
    for i in start..end {
        arg[0] = Value::fixnum(i);
        let val = vm.eval_block(method, &arg)?;
        vm.temp_push(val);
        match val.as_array() {
            Some(aref) => {
                let mut other = aref.elements.clone();
                res.append(&mut other);
            }
            None => res.push(val),
        };
    }
    let res = Value::array_from(&vm.globals, res);
    Ok(res)
}

fn each(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    let method = match args.block {
        Some(method) => method,
        None => {
            // return Enumerator
            let id = IdentId::get_id("each");
            let e = vm.create_enumerator(id, self_val, args.clone())?;
            return Ok(e);
        }
    };
    let start = range.start.expect_integer(&vm, "Start")?;
    let end = range.end.expect_integer(&vm, "End")? + if range.exclude { 0 } else { 1 };
    for i in start..end {
        let arg = Args::new1(Value::fixnum(i));
        vm.eval_block(method, &arg)?;
    }
    Ok(self_val)
}

fn all(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    let method = vm.expect_block(args.block)?;
    let start = range.start.expect_integer(&vm, "Start")?;
    let end = range.end.expect_integer(&vm, "End")? + if range.exclude { 0 } else { 1 };
    for i in start..end {
        let arg = Args::new1(Value::fixnum(i));
        let res = vm.eval_block(method, &arg)?;
        if !res.to_bool() {
            return Ok(Value::false_val());
        }
    }
    Ok(Value::true_val())
}

fn to_a(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
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

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn range_test() {
        let program = r#"
            assert(3, (3..100).begin)
            assert(100, (3..100).end)
            assert("3..100", (3..100).to_s)
            assert("3..100", (3..100).inspect)
            assert([6, 8, 10], (3..5).map{|x| x * 2})
            assert(
                [2, 4, 6, 8],
                [[1, 2], [3, 4]].flat_map{|i| i.map{|j| j * 2}}
            )
            assert([2, 3, 4, 5], (2..5).to_a)
            assert(true, (5..7).all? {|v| v > 0 })
            assert(false, (-1..3).all? {|v| v > 0 })
        "#;
        assert_script(program);
    }

    #[test]
    fn range1() {
        let program = "
            assert(Range.new(5,10), 5..10)
            assert(Range.new(5,10, false), 5..10)
            assert(Range.new(5,10, true), 5...10)";
        assert_script(program);
    }

    #[test]
    fn range2() {
        let program = "
            assert(Range.new(5,10).first, 5)
            assert(Range.new(5,10).first(4), [5,6,7,8])
            assert(Range.new(5,10).first(100), [5,6,7,8,9,10])
            assert(Range.new(5,10,true).first(4), [5,6,7,8])
            assert(Range.new(5,10,true).first(100), [5,6,7,8,9])
            assert(Range.new(5,10).last, 10)
            assert(Range.new(5,10).last(4), [7,8,9,10])
            assert(Range.new(5,10).last(100), [5,6,7,8,9,10])
            assert(Range.new(5,10,true).last(4), [6,7,8,9])
            assert(Range.new(5,10,true).last(100), [5,6,7,8,9])";
        assert_script(program);
    }
}
