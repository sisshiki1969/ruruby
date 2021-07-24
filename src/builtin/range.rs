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

    pub fn to_s(&self, vm: &mut VM) -> Result<String, RubyError> {
        let start = self.start.val_to_s(vm)?;
        let end = self.end.val_to_s(vm)?;
        let sym = if self.exclude { "..." } else { ".." };
        Ok(format!("{}{}{}", start, sym, end))
    }

    pub fn inspect(&self, vm: &mut VM) -> Result<String, RubyError> {
        let start = vm.val_inspect(self.start)?;
        let end = vm.val_inspect(self.end)?;
        let sym = if self.exclude { "..." } else { ".." };
        Ok(format!("{}{}{}", start, sym, end))
    }
}

pub fn init() -> Value {
    let class = Module::class_under_object();
    BuiltinClass::set_toplevel_constant("Range", class);
    class.add_builtin_method_by_str("to_s", to_s);
    class.add_builtin_method_by_str("inspect", inspect);
    class.add_builtin_method_by_str("map", map);
    class.add_builtin_method_by_str("flat_map", flat_map);
    class.add_builtin_method_by_str("each", each);
    class.add_builtin_method_by_str("all?", all);
    class.add_builtin_method_by_str("begin", begin);
    class.add_builtin_method_by_str("first", first);
    class.add_builtin_method_by_str("end", end);
    class.add_builtin_method_by_str("last", last);
    class.add_builtin_method_by_str("to_a", to_a);
    class.add_builtin_method_by_str("exclude_end?", exclude_end);
    class.add_builtin_method_by_str("include?", include);

    class.add_builtin_class_method("new", range_new);
    class.into()
}

fn range_new(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let len = args.len();
    args.check_args_range(2, 3)?;
    let (start, end) = (args[0], args[1]);
    let exclude_end = if len == 2 { false } else { args[2].to_bool() };
    vm.create_range(start, end, exclude_end)
}

fn to_s(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    let res = range.to_s(vm)?;
    Ok(Value::string(res))
}

fn inspect(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    let res = range.inspect(vm)?;
    Ok(Value::string(res))
}

fn begin(_vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    Ok(range.start)
}

fn end(_vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    Ok(range.end)
}

fn first(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let range = self_val.as_range().unwrap();
    let start = range.start.as_integer().unwrap();
    let mut end = range.end.as_integer().unwrap() - if range.exclude { 1 } else { 0 };
    if args.len() == 0 {
        return Ok(range.start);
    };
    let arg = args[0].expect_integer("Argument")?;
    if arg < 0 {
        return Err(RubyError::argument("Negative array size"));
    };
    let mut v = vec![];
    if start + arg - 1 < end {
        end = start + arg - 1;
    };
    for i in start..=end {
        v.push(Value::integer(i));
    }
    Ok(Value::array_from(v))
}

fn last(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let range = self_val.as_range().unwrap();
    let mut start = range.start.as_integer().unwrap();
    let end = range.end.as_integer().unwrap() - if range.exclude { 1 } else { 0 };
    if args.len() == 0 {
        return Ok(range.end);
    };
    let arg = args[0].expect_integer("Argument")?;
    if arg < 0 {
        return Err(RubyError::argument("Negative array size"));
    };
    let mut v = vec![];
    if end - arg + 1 > start {
        start = end - arg + 1;
    };
    for i in start..=end {
        v.push(Value::integer(i));
    }
    Ok(Value::array_from(v))
}

fn map(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let range = self_val.as_range().unwrap();
    let block = args.expect_block()?;
    let start = range.start.expect_integer("Start")?;
    let end = range.end.expect_integer("End")? + if range.exclude { 0 } else { 1 };
    let mut arg = Args::new(1);
    let mut res = vec![];
    for i in start..end {
        arg[0] = Value::integer(i);
        let val = vm.eval_block(&block, &arg)?;
        vm.temp_push(val);
        res.push(val);
    }
    let res = Value::array_from(res);
    Ok(res)
}

fn flat_map(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let range = self_val.as_range().unwrap();
    let block = args.expect_block()?;
    let start = range.start.expect_integer("Start")?;
    let end = range.end.expect_integer("End")? + if range.exclude { 0 } else { 1 };
    let mut arg = Args::new(1);
    let mut res = vec![];
    for i in start..end {
        arg[0] = Value::integer(i);
        let val = vm.eval_block(&block, &arg)?;
        vm.temp_push(val);
        match val.as_array() {
            Some(aref) => {
                let mut other = aref.elements.clone();
                res.append(&mut other);
            }
            None => res.push(val),
        };
    }
    let res = Value::array_from(res);
    Ok(res)
}

fn each(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let range = self_val.as_range().unwrap();
    let method = match &args.block {
        Block::None => {
            // return Enumerator
            let id = IdentId::EACH;
            let e = vm.create_enumerator(id, self_val, args.clone())?;
            return Ok(e);
        }
        method => method,
    };
    let start = range.start.expect_integer("Start")?;
    let end = range.end.expect_integer("End")? + if range.exclude { 0 } else { 1 };

    /*for i in start..end {
        args[0] = Value::integer(i);
        vm.eval_block_iter(method, &args)?;
    }*/

    let iter = (start..end).map(|i| Value::integer(i));
    vm.eval_block_each1(method, iter)?;
    Ok(self_val)
}

fn all(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let range = self_val.as_range().unwrap();
    let block = args.expect_block()?;
    let start = range.start.expect_integer("Start")?;
    let end = range.end.expect_integer("End")? + if range.exclude { 0 } else { 1 };
    for i in start..end {
        let arg = Args::new1(Value::integer(i));
        let res = vm.eval_block(&block, &arg)?;
        if !res.to_bool() {
            return Ok(Value::false_val());
        }
    }
    Ok(Value::true_val())
}

fn to_a(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let RangeInfo {
        start,
        end,
        exclude,
    } = *self_val.as_range().unwrap();
    if let Some(start) = start.as_integer() {
        /*let start = range.start.expect_integer(format!(
            "Can not iterate from {}",
            range.start.get_class_name()
        ))?;*/
        let end = end.expect_integer("Range.end")?;
        let v = if exclude { start..end } else { start..end + 1 }
            .map(|i| Value::integer(i))
            .collect();
        Ok(Value::array_from(v))
    } else if let Some(start) = start.as_string() {
        let mut end = end;
        let end = end.expect_string("Range.end")?;
        // single character
        if start.is_ascii() && end.is_ascii() && start.len() == 1 && end.len() == 1 {
            let (start, end) = (start.as_bytes()[0], end.as_bytes()[0]);
            if start > end || start == end && exclude {
                return Ok(Value::array_empty());
            }
            let v = if exclude { start..end } else { start..end + 1 }
                .map(|b| Value::string((b as char).to_string()))
                .collect();
            Ok(Value::array_from(v))
        } else {
            unimplemented!()
        }
    } else {
        unimplemented!()
    }
}

fn exclude_end(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let range = self_val.as_range().unwrap();
    Ok(Value::bool(range.exclude))
}

fn include(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let range = self_val.as_range().unwrap();
    match range.start.unpack() {
        RV::Integer(start) => {
            let start = Real::Integer(start);
            let end = range.end.to_real().unwrap();
            let val = match args[0].to_real() {
                Some(real) => real,
                None => return Ok(Value::false_val()),
            };
            let b = val.included(&start, &end, range.exclude);
            Ok(Value::bool(b))
        }
        RV::Float(start) => {
            let start = Real::Float(start);
            let end = range.end.to_real().unwrap();
            let val = match args[0].to_real() {
                Some(real) => real,
                None => return Ok(Value::false_val()),
            };
            let b = val.included(&start, &end, range.exclude);
            Ok(Value::bool(b))
        }
        _ => {
            if !vm.eval_send(IdentId::_LE, range.start, args)?.to_bool() {
                return Ok(Value::false_val());
            };
            let b = if range.exclude {
                vm.eval_send(IdentId::_GT, range.end, args)?.to_bool()
            } else {
                vm.eval_send(IdentId::_GE, range.end, args)?.to_bool()
            };
            Ok(Value::bool(b))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::*;

    #[test]
    fn range_check() {
        let program = r#"
            0..3.2
            Object..Class
            "a".."z"
            assert_error { 0.."a" }
        "#;
        assert_script(program);
    }

    #[test]
    fn range_toa() {
        let program = r#"
            assert [2, 3, 4, 5], (2..5).to_a
            assert [2, 3, 4], (2...5).to_a
            assert ["Z", "[", "\\", "]", "^", "_", "`", "a"], ("Z".."a").to_a
            assert ["Z", "[", "\\", "]", "^", "_", "`"], ("Z"..."a").to_a
        "#;
        assert_script(program);
    }

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
            assert(true, (5..7).all? {|v| v > 0 })
            assert(false, (-1..3).all? {|v| v > 0 })
            assert(true, (0...3).exclude_end?)
            assert(false, (0..3).exclude_end?)
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

    #[test]
    fn range_include() {
        let program = r#"
        assert(true, (3..7).include? 3)
        assert(true, (3..7).include? 7)
        assert(true, (3..7).include? 5)
        assert(true, (3..7).include? 5.7)
        assert(true, (3..7).include? 7.0)
        assert(false, (3..7).include? 0)
        assert(false, (3..7).include? 7.1)
        assert(false, (3..7).include? "6")

        assert(true, (3...7).include? 3)
        assert(false, (3...7).include? 7)
        assert(true, (3...7).include? 5.7)

        assert(true, (3.3..7.1).include? 3.3)
        assert(true, (3.3..7.1).include? 7.1)
        assert(true, (3.3..7.1).include? 4.5)
        assert(true, (3.3..7.1).include? 7)
        assert(false, (3.3..7.1).include? 3.2)
        assert(false, (3.3..7.1).include? 7.2)
        assert(false, (3.3..7.1).include? 3)
        assert(false, (3.3..7.1).include?(:a))

        assert(true, (3.3...7.1).include? 3.3)
        assert(false, (3.3...7.1).include? 7.1)
        assert(true, (3.3...7.1).include? 4.5)
        assert(false, (3.3...7.0).include? 7)
        "#;
        assert_script(program);
    }

    #[test]
    fn range_include2() {
        let program = r#"
        class Foo
            attr_accessor :x
            include Comparable
            def initialize(x)
                @x = x
            end
            def <=>(other)
                self.x<=>other.x
            end
        end

        assert true, (Foo.new(3)..Foo.new(6)).include? Foo.new(3)
        assert true, (Foo.new(3)..Foo.new(6)).include? Foo.new(6)
        assert false, (Foo.new(3)..Foo.new(6)).include? Foo.new(0)
        assert false, (Foo.new(3)..Foo.new(6)).include? Foo.new(7)
        "#;
        assert_script(program);
    }
}
