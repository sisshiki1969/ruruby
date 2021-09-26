use crate::error::RubyError;
use crate::*;
use fxhash::FxHashSet;

pub fn init() -> Value {
    let class = Module::class_under_object();
    BuiltinClass::set_toplevel_constant("Array", class);
    class.add_builtin_method_by_str("inspect", inspect);
    class.add_builtin_method_by_str("to_s", inspect);
    class.add_builtin_method_by_str("to_a", toa);
    class.add_builtin_method_by_str("length", length);
    class.add_builtin_method_by_str("size", length);
    class.add_builtin_method_by_str("empty?", empty);
    class.add_builtin_method_by_str("[]", get_elem);
    class.add_builtin_method_by_str("at", at);
    class.add_builtin_method_by_str("[]=", set_elem);
    class.add_builtin_method_by_str("push", push);
    class.add_builtin_method_by_str("<<", push);
    class.add_builtin_method_by_str("pop", pop);
    class.add_builtin_method_by_str("*", mul);
    class.add_builtin_method_by_str("+", add);
    class.add_builtin_method_by_str("-", sub);
    class.add_builtin_method_by_str("<=>", cmp);

    class.add_builtin_method_by_str("shift", shift);
    class.add_builtin_method_by_str("unshift", unshift);

    class.add_builtin_method_by_str("concat", concat);
    class.add_builtin_method_by_str("map", map);
    class.add_builtin_method_by_str("collect", map);
    class.add_builtin_method_by_str("map!", map_);
    class.add_builtin_method_by_str("collect!", map_);
    class.add_builtin_method_by_str("flat_map", flat_map);
    class.add_builtin_method_by_str("each", each);
    class.add_builtin_method_by_str("each_index", each_index);
    class.add_builtin_method_by_str("each_with_index", each_with_index);
    class.add_builtin_method_by_str("partition", partition);

    class.add_builtin_method_by_str("include?", include);
    class.add_builtin_method_by_str("reverse", reverse);
    class.add_builtin_method_by_str("reverse!", reverse_);
    class.add_builtin_method_by_str("rotate!", rotate_);
    class.add_builtin_method_by_str("compact", compact);
    class.add_builtin_method_by_str("compact!", compact_);

    class.add_builtin_method_by_str("transpose", transpose);
    class.add_builtin_method_by_str("min", min);
    class.add_builtin_method_by_str("fill", fill);
    class.add_builtin_method_by_str("clear", clear);
    class.add_builtin_method_by_str("uniq!", uniq_);
    class.add_builtin_method_by_str("uniq", uniq);
    class.add_builtin_method_by_str("any?", any_);
    class.add_builtin_method_by_str("all?", all_);

    class.add_builtin_method_by_str("slice!", slice_);
    class.add_builtin_method_by_str("max", max);
    class.add_builtin_method_by_str("first", first);
    class.add_builtin_method_by_str("last", last);
    class.add_builtin_method_by_str("dup", dup);
    class.add_builtin_method_by_str("clone", dup);
    class.add_builtin_method_by_str("pack", pack);
    class.add_builtin_method_by_str("join", join);
    class.add_builtin_method_by_str("drop", drop);
    class.add_builtin_method_by_str("zip", zip);
    class.add_builtin_method_by_str("grep", grep);
    class.add_builtin_method_by_str("sort", sort);
    class.add_builtin_method_by_str("sort_by", sort_by);
    class.add_builtin_method_by_str("count", count);
    class.add_builtin_method_by_str("inject", inject);
    class.add_builtin_method_by_str("reduce", inject);
    class.add_builtin_method_by_str("find_index", find_index);
    class.add_builtin_method_by_str("index", find_index);

    class.add_builtin_method_by_str("reject", reject);
    class.add_builtin_method_by_str("find", find);
    class.add_builtin_method_by_str("detect", find);
    class.add_builtin_method_by_str("select", select);
    class.add_builtin_method_by_str("filter", select);
    class.add_builtin_method_by_str("bsearch", bsearch);
    class.add_builtin_method_by_str("bsearch_index", bsearch_index);
    class.add_builtin_method_by_str("delete", delete);
    class.add_builtin_method_by_str("flatten", flatten);
    class.add_builtin_method_by_str("flatten!", flatten_);

    class.add_builtin_class_method("new", array_new);
    class.add_builtin_class_method("allocate", array_allocate);
    class.add_builtin_class_method("[]", array_elem);
    class.into()
}

// Class methods

fn array_new(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 2)?;
    let self_val = self_val.into_module();
    let array_vec = match args.len() {
        0 => vec![],
        1 => match args[0].unpack() {
            RV::Integer(num) => {
                if num < 0 {
                    return Err(RubyError::argument("Negative array size."));
                };
                vec![Value::nil(); num as usize]
            }
            RV::Object(oref) => match &oref.kind {
                ObjKind::Array(aref) => aref.elements.clone(),
                _ => {
                    return Err(RubyError::wrong_type(
                        "1st arg",
                        "Integer or Array",
                        args[0],
                    ))
                }
            },
            _ => {
                return Err(RubyError::wrong_type(
                    "1st arg",
                    "Integer or Array",
                    args[0],
                ))
            }
        },
        2 => {
            let num = args[0].coerce_to_fixnum("1st arg")?;
            if num < 0 {
                return Err(RubyError::argument("Negative array size."));
            };
            vec![args[1]; num as usize]
        }
        _ => unreachable!(),
    };
    let array = Value::array_from_with_class(array_vec, self_val);
    if let Some(method) = MethodRepo::find_method(self_val, IdentId::INITIALIZE) {
        vm.eval_method(method, array, args)?;
    };
    Ok(array)
}

fn array_allocate(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let array = Value::array_from_with_class(vec![], self_val.into_module());
    Ok(array)
}

fn array_elem(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let array = Value::array_from_with_class(args.to_vec(), self_val.into_module());
    Ok(array)
}

// Instance methods

fn inspect(vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    fn checked_inspect(vm: &mut VM, self_val: Value, elem: Value) -> Result<String, RubyError> {
        if elem.id() == self_val.id() {
            Ok("[...]".to_string())
        } else {
            vm.val_inspect(elem)
        }
    }
    let ary = self_val.into_array();
    let s = match ary.elements.len() {
        0 => "[]".to_string(),
        len => {
            let mut result = checked_inspect(vm, self_val, ary.elements[0])?;
            for i in 1..len {
                result = format!(
                    "{}, {}",
                    result,
                    checked_inspect(vm, self_val, ary.elements[i])?
                );
            }
            format! {"[{}]", result}
        }
    };
    Ok(Value::string(s))
}

fn toa(_: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    let array = BuiltinClass::array();
    if self_val.get_class().id() == array.id() {
        return Ok(self_val);
    };
    let new_val = self_val.shallow_dup();
    new_val.set_class(array);
    Ok(new_val)
}

/// self[nth] -> object | nil
/// self[range] -> Array | nil
/// self[start, length] -> Array | nil
///
/// https://docs.ruby-lang.org/ja/latest/method/Array/i/=5b=5d.html
fn get_elem(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    self_val.into_array().get_elem(args)
}

/// at(nth) -> object | nil
///
/// https://docs.ruby-lang.org/ja/latest/method/Array/i/=5b=5d.html
fn at(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    self_val.into_array().get_elem1(args[0])
}

/// self[nth] = val
/// self[range] = val
/// self[start, length] = val
///
/// https://docs.ruby-lang.org/ja/latest/method/Array/i/=5b=5d=3d.html
fn set_elem(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    self_val.into_array().set_elem(args)
}

fn cmp(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    if self_val.id() == args[0].id() {
        return Ok(Value::integer(0));
    }
    let lhs = self_val.into_array();
    let rhs = match args[0].as_array() {
        Some(aref) => aref,
        None => return Ok(Value::nil()),
    };
    if lhs.len() >= rhs.len() {
        for (i, rhs_v) in rhs.iter().enumerate() {
            match vm.eval_compare(*rhs_v, lhs[i])?.as_fixnum() {
                Some(0) => {}
                Some(ord) => return Ok(Value::integer(ord)),
                None => return Ok(Value::nil()),
            }
        }
        if lhs.len() == rhs.len() {
            Ok(Value::integer(0))
        } else {
            Ok(Value::integer(1))
        }
    } else {
        for (i, lhs_v) in lhs.iter().enumerate() {
            match vm.eval_compare(rhs[i], *lhs_v)?.as_fixnum() {
                Some(0) => {}
                Some(ord) => return Ok(Value::integer(ord)),
                None => return Ok(Value::nil()),
            }
        }
        Ok(Value::integer(-1))
    }
}

fn push(_vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let mut ary = self_val.into_array();
    for arg in args.iter() {
        ary.push(*arg);
    }
    Ok(self_val)
}

fn pop(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let mut ary = self_val.into_array();
    let res = ary.pop().unwrap_or_default();
    Ok(res)
}

fn shift(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let mut array_flag = false;
    let num = if args.len() == 0 {
        0
    } else {
        let i = args[0].coerce_to_fixnum("1st arg")?;
        if i < 0 {
            return Err(RubyError::argument("Negative array size."));
        }
        array_flag = true;
        i as usize
    };

    let mut ary = self_val.into_array();
    if array_flag {
        if ary.len() < num {
            return Ok(Value::array_empty().into());
        }
        let new = ary.split_off(num);
        let res = ary[0..num].to_vec();
        ary.elements = new;
        Ok(Value::array_from(res))
    } else {
        if ary.len() == 0 {
            return Ok(Value::nil());
        }
        let new = ary.split_off(1);
        let res = ary[0];
        ary.elements = new;
        Ok(res)
    }
}

fn unshift(_vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    if args.len() == 0 {
        return Ok(self_val);
    }
    let mut new = args[0..args.len()].to_owned();
    let mut ary = self_val.into_array();
    new.append(&mut ary.elements);
    ary.elements = new;
    Ok(self_val)
}

fn length(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let res = Value::integer(self_val.into_array().len() as i64);
    Ok(res)
}

fn empty(_vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    let aref = self_val.into_array();
    let res = Value::bool(aref.is_empty());
    Ok(res)
}

fn mul(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let ary = self_val.into_array();
    if let Some(num) = args[0].as_fixnum() {
        let v = match num {
            i if i < 0 => return Err(RubyError::argument("Negative argument.")),
            i => ary.elements.repeat(i as usize),
        };
        let res = Value::array_from_with_class(v, self_val.get_class());
        Ok(res.into())
    } else if let Some(s) = args[0].as_string() {
        match ary.len() {
            0 => return Ok(Value::string("")),
            1 => {
                let res = ary[0].val_to_s(vm)?;
                return Ok(Value::string(res));
            }
            _ => {
                let mut res = ary[0].val_to_s(vm)?.to_string();
                for i in 1..ary.len() {
                    let elem = ary[i].val_to_s(vm)?;
                    res = res + s + &elem;
                }
                return Ok(Value::string(res));
            }
        };
    } else {
        return Err(RubyError::no_implicit_conv(args[0], "Integer"));
    }
}

fn add(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let mut lhs = self_val.into_array().elements.clone();
    let mut arg0 = args[0];
    let mut rhs = arg0.expect_array("Argument")?.elements.clone();
    lhs.append(&mut rhs);
    Ok(Value::array_from(lhs))
}

fn concat(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let mut lhs = self_val.into_array();
    let mut arg0 = args[0];
    let mut rhs = arg0.expect_array("Argument")?.elements.clone();
    lhs.append(&mut rhs);
    Ok(self_val)
}

fn sub(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs_v = &self_val.expect_array("Receiver")?.elements;
    let mut arg0 = args[0];
    let rhs_v = &arg0.expect_array("Argument")?.elements;
    let mut v = vec![];
    for lhs in lhs_v {
        let mut flag = true;
        for rhs in rhs_v {
            if vm.eval_eq2(*lhs, *rhs)? {
                flag = false;
                break;
            }
        }
        if flag {
            v.push(*lhs)
        }
    }
    Ok(Value::array_from(v))
}

macro_rules! to_enum_id {
    ($vm:ident, $self_val:ident, $args:ident, $id:expr) => {
        match &$args.block {
            None => {
                let val = $vm.create_enumerator($id, $self_val, $args.clone())?;
                return Ok(val);
            }
            Some(block) => block,
        }
    };
}

macro_rules! to_enum_str {
    ($vm:ident, $self_val:ident, $args:ident, $id:expr) => {
        match &$args.block {
            None => {
                let val = $vm.create_enumerator(IdentId::get_id($id), $self_val, $args.clone())?;
                return Ok(val);
            }
            Some(block) => block,
        }
    };
}

fn map(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.into_array();
    let method = to_enum_id!(vm, self_val, args, IdentId::MAP);

    let temp_len = vm.temp_len();

    let mut i = 0;
    let mut arg = Args::new(1);
    while i < aref.len() {
        arg[0] = aref[i];
        let res = vm.eval_block(method, &arg)?;
        vm.temp_push(res);
        i += 1;
    }

    let res = vm.temp_pop_vec(temp_len);
    Ok(Value::array_from(res))
}

fn map_(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let mut aref = self_val.into_array();
    let method = to_enum_id!(vm, self_val, args, IdentId::MAP);

    let mut i = 0;
    let mut arg = Args::new(1);
    while i < aref.len() {
        arg[0] = aref[i];
        aref[i] = vm.eval_block(method, &arg)?;
        i += 1;
    }

    Ok(self_val)
}

fn flat_map(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let block = args.expect_block()?;
    let param_num = block.to_iseq().params.req;
    let mut arg = Args::new(param_num);

    let aref = self_val.into_array();
    let mut res = vec![];
    for elem in &aref.elements {
        if param_num == 0 {
        } else if param_num == 1 {
            arg[0] = *elem;
        } else {
            match elem.as_array() {
                Some(ary) => arg.copy_from_slice(&ary.elements[0..param_num]),
                None => arg[0] = *elem,
            }
        }

        let ary = vm.eval_block(&block, &arg)?;
        vm.temp_push(ary);
        match ary.as_array() {
            Some(mut ary) => {
                res.append(&mut ary.elements);
            }
            None => res.push(ary),
        }
    }
    let res = Value::array_from(res);
    Ok(res)
}

/// each {|item| .... } -> self
/// each -> Enumerator
///
/// https://docs.ruby-lang.org/ja/latest/method/Array/i/each.html
fn each(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let method = to_enum_id!(vm, self_val, args, IdentId::EACH);
    let aref = self_val.into_array();
    let mut i = 0;
    let mut arg = Args::new(1);
    while i < aref.len() {
        arg[0] = aref[i];
        vm.eval_block(method, &arg)?;
        i += 1;
    }
    Ok(self_val)
}

/// each_index {|index| .... } -> self
/// each_index -> Enumerator
///
/// https://docs.ruby-lang.org/ja/latest/method/Array/i/each_index.html
fn each_index(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let method = to_enum_str!(vm, self_val, args, "each_index");
    let aref = self_val.into_array();
    let mut i = 0;
    let mut arg = Args::new(1);
    while i < aref.len() {
        arg[0] = Value::integer(i as i64);
        vm.eval_block(method, &arg)?;
        i += 1;
    }
    Ok(self_val)
}

/// Enumerable#each_with_index
/// each_with_index(*args) -> Enumerator
/// each_with_index(*args) {|item, index| ... } -> self
///
/// https://docs.ruby-lang.org/ja/latest/method/Enumerable/i/each_with_index.html
fn each_with_index(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let method = to_enum_str!(vm, self_val, args, "each_with_index");
    let aref = self_val.into_array();
    let mut i = 0;
    let mut arg = Args::new(2);
    while i < aref.len() {
        arg[0] = aref[i];
        arg[1] = Value::integer(i as i64);
        vm.eval_block(method, &arg)?;
        i += 1;
    }
    Ok(self_val)
}

fn partition(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let method = to_enum_str!(vm, self_val, args, "partition");
    let aref = self_val.into_array();
    let mut arg = Args::new(1);
    let mut res_true = vec![];
    let mut res_false = vec![];
    for i in &aref.elements {
        arg[0] = *i;
        if vm.eval_block(method, &arg)?.to_bool() {
            res_true.push(*i);
        } else {
            res_false.push(*i);
        };
    }
    let ary = vec![
        Value::array_from(res_true).into(),
        Value::array_from(res_false).into(),
    ];
    Ok(Value::array_from(ary))
}

fn include(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let target = args[0];
    let aref = self_val.into_array();
    for item in aref.elements.iter() {
        if vm.eval_eq2(*item, target)? {
            return Ok(Value::true_val());
        }
    }
    Ok(Value::false_val())
}

fn reverse(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.into_array();
    let mut res = aref.elements.clone();
    res.reverse();
    Ok(Value::array_from(res))
}

fn reverse_(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let mut aref = self_val.into_array();
    aref.elements.reverse();
    Ok(self_val)
}

fn rotate_(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let i = if args.len() == 0 {
        1
    } else {
        match args[0].as_fixnum() {
            Some(i) => i,
            None => return Err(RubyError::argument("Must be Integer.")),
        }
    };
    let mut aref = self_val.into_array();
    if i == 0 {
        Ok(self_val)
    } else if i > 0 {
        let i = i % (aref.elements.len() as i64);
        let mut vec = &mut aref.clone().elements;
        let mut vec2 = vec.split_off(i as usize);
        vec2.append(&mut vec);
        aref.elements = vec2;
        Ok(self_val)
    } else {
        let len = aref.elements.len() as i64;
        let i = (-i) % len;
        let mut vec = &mut aref.clone().elements;
        let mut vec2 = vec.split_off((len - i) as usize);
        vec2.append(&mut vec);
        aref.elements = vec2;
        Ok(self_val)
    }
}

fn compact(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.into_array();
    let ary = aref
        .elements
        .iter()
        .filter(|x| !x.is_nil())
        .cloned()
        .collect();
    Ok(Value::array_from(ary))
}

fn compact_(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let mut aref = self_val.into_array();
    let mut flag = false;
    aref.elements.retain(|x| {
        let b = !x.is_nil();
        if !b {
            flag = true
        };
        b
    });
    if flag {
        Ok(self_val)
    } else {
        Ok(Value::nil())
    }
}

fn transpose(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let mut aref = self_val.into_array();
    if aref.elements.len() == 0 {
        return Ok(Value::array_empty().into());
    }
    let mut vec = vec![];
    for elem in &mut aref.elements {
        let ary = elem
            .as_array()
            .ok_or(RubyError::argument(
                "Each element of receiver must be an array.",
            ))?
            .elements
            .clone();
        vec.push(ary);
    }
    let len = vec[0].len();
    let mut trans = vec![];
    for i in 0..len {
        let mut temp = vec![];
        for v in &vec {
            if v.len() != len {
                return Err(RubyError::index("Element size differs."));
            }
            temp.push(v[i]);
        }
        let ary = Value::array_from(temp).into();
        trans.push(ary);
    }
    //aref.elements.reverse();
    let res = Value::array_from(trans).into();
    Ok(res)
}

fn min(_: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    fn to_float(val: Value) -> Result<f64, RubyError> {
        if let Some(i) = val.as_fixnum() {
            Ok(i as f64)
        } else if let Some(f) = val.as_flonum() {
            Ok(f)
        } else {
            Err(RubyError::typeerr(
                "Currently, each element must be Numeric.",
            ))
        }
    }

    let aref = self_val.into_array();
    if aref.elements.len() == 0 {
        return Ok(Value::nil());
    }
    let mut min_obj = aref.elements[0];
    let mut min = to_float(min_obj)?;
    for elem in &aref.elements {
        let elem_f = to_float(*elem)?;
        if elem_f < min {
            min_obj = *elem;
            min = elem_f;
        }
    }

    return Ok(min_obj);
}

fn max(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.into_array();
    if aref.elements.len() == 0 {
        return Ok(Value::nil());
    }
    let mut max = aref.elements[0];
    for elem in &aref.elements {
        if vm.eval_gt2(max, *elem)? {
            max = *elem;
        };
    }
    Ok(max)
}

fn fill(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let mut aref = self_val.into_array();
    for elem in &mut aref.elements {
        *elem = args[0];
    }
    Ok(self_val)
}

fn clear(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let mut aref = self_val.into_array();
    aref.elements.clear();
    Ok(self_val)
}

fn uniq(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.into_array();
    let mut h = FxHashSet::default();
    let mut v = vec![];
    match &args.block {
        None => {
            for elem in &aref.elements {
                if h.insert(HashKey(*elem)) {
                    v.push(*elem);
                };
            }
        }
        Some(block) => {
            let mut block_args = Args::new(1);
            for elem in &aref.elements {
                block_args[0] = *elem;
                let res = vm.eval_block(block, &block_args)?;
                if h.insert(HashKey(res)) {
                    v.push(*elem);
                };
            }
        }
    };
    Ok(Value::array_from(v))
}

fn uniq_(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let mut h = FxHashSet::default();
    match &args.block {
        None => {
            let mut aref = self_val.into_array();
            aref.retain(|x| Ok(h.insert(HashKey(*x))))?;
            Ok(self_val)
        }
        Some(block) => {
            let mut aref = self_val.into_array();
            let mut block_args = Args::new(1);
            aref.retain(|x| {
                block_args[0] = *x;
                let res = vm.eval_block(block, &block_args)?;
                vm.temp_push(res);
                Ok(h.insert(HashKey(res)))
            })?;
            Ok(self_val)
        }
    }
}

fn slice_(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(2)?;
    let start = args[0].coerce_to_fixnum("Currently, first arg must be Integer.")?;
    if start < 0 {
        return Err(RubyError::argument("First arg must be positive value."));
    };
    let len = args[1].coerce_to_fixnum("Currently, second arg must be Integer")?;
    if len < 0 {
        return Err(RubyError::argument("Second arg must be positive value."));
    };
    let start = start as usize;
    let len = len as usize;
    let mut aref = self_val.into_array();
    let new = aref.elements.drain(start..start + len).collect();
    Ok(Value::array_from(new))
}

fn first(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.into_array();
    if aref.elements.len() == 0 {
        Ok(Value::nil())
    } else {
        Ok(aref.elements[0])
    }
}

fn last(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.into_array();
    if aref.elements.len() == 0 {
        Ok(Value::nil())
    } else {
        Ok(*aref.elements.last().unwrap())
    }
}

fn dup(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.into_array();
    Ok(Value::array_from(aref.elements.clone()))
}

fn pack(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let aref = self_val.into_array();
    let mut v = vec![];
    for elem in &aref.elements {
        let i = match elem.as_fixnum() {
            Some(i) => i as i8 as u8,
            None => return Err(RubyError::argument("Must be Array of Integer.")),
        };
        v.push(i);
    }
    Ok(Value::bytes(v))
}

fn join(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let sep = if args.len() == 0 {
        ""
    } else {
        match args[0].as_string() {
            Some(s) => s,
            None => return Err(RubyError::argument("Seperator must be String.")),
        }
    };
    let aref = self_val.into_array();
    let mut res = String::new();
    for elem in &aref.elements {
        let s = elem.val_to_s(vm)?;
        if res.is_empty() {
            res = s.into_owned();
        } else {
            res = res + sep + &s;
        }
    }
    Ok(Value::string(res))
}

fn drop(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let aref = self_val.into_array();
    let num = args[0].coerce_to_fixnum("An argument must be Integer.")? as usize;
    if num >= aref.len() {
        return Err(RubyError::argument(format!("An argument too big. {}", num)));
    };
    let ary = &aref.elements[num..];
    Ok(Value::array_from(ary.to_vec()))
}

fn zip(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let self_ary = self_val.into_array();
    let mut args_ary = vec![];
    for a in args.iter() {
        args_ary.push(a.clone().expect_array("Args")?.elements.clone());
    }
    let mut ary = vec![];
    for (i, val) in self_ary.elements.iter().enumerate() {
        let mut vec = vec![*val];
        for args in &args_ary {
            if i < args.len() {
                vec.push(args[i]);
            } else {
                vec.push(Value::nil());
            }
        }
        let zip = Value::array_from(vec).into();
        ary.push(zip);
    }
    match &args.block {
        None => Ok(Value::array_from(ary)),
        Some(block) => {
            let mut arg = Args::new(1);
            vm.temp_push_vec(&ary);
            for val in ary {
                arg[0] = val;
                vm.eval_block(block, &arg)?;
            }
            Ok(Value::nil())
        }
    }
}

fn grep(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let aref = self_val.into_array();
    let ary = match &args.block {
        None => aref
            .elements
            .iter()
            .filter_map(|x| match vm.eval_teq(*x, args[0]) {
                Ok(true) => Some(*x),
                _ => None,
            })
            .collect(),
        _ => unimplemented!(),
    };
    Ok(Value::array_from(ary))
}

/// Array#sort -> Array
/// Array#sort { |a, b| .. } -> Array
/// https://docs.ruby-lang.org/ja/latest/method/Array/i/sort.html
fn sort(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    //use std::cmp::Ordering;
    args.check_args_num(0)?;
    let mut ary = self_val.expect_array("Receiver")?.elements.clone();
    match &args.block {
        None => {
            vm.sort_array(&mut ary)?;
        }
        Some(block) => {
            let mut args = Args::new(2);
            ary.sort_by(|a, b| {
                args[0] = *a;
                args[1] = *b;
                vm.eval_block(block, &args).unwrap().to_ordering()
            });
        }
    };
    Ok(Value::array_from(ary))
}

/// Enumerator#sort { |item| .. } -> Array
/// https://docs.ruby-lang.org/ja/latest/method/Enumerable/i/sort_by.html
fn sort_by(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let block = args.expect_block()?;
    let mut ary = vec![];
    {
        let mut args = Args::new(1);
        for v in &self_val.as_array().unwrap().elements {
            args[0] = *v;
            let v1 = vm.eval_block(block, &args)?;
            ary.push((*v, v1));
        }
    }
    ary.sort_by(|a, b| vm.eval_compare(b.1, a.1).unwrap().to_ordering());

    Ok(Value::array_from(ary.iter().map(|x| x.0).collect()))
}

fn any_(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let aref = self_val.into_array();
    if args.len() == 1 {
        if args.block.is_some() {
            eprintln!("warning: given block not used");
        }
        for v in aref.elements.iter() {
            if vm.eval_teq(*v, args[0])? {
                return Ok(Value::true_val());
            };
        }
        return Ok(Value::false_val());
    }
    args.check_args_num(0)?;

    match &args.block {
        None => {
            for v in aref.elements.iter() {
                if v.to_bool() {
                    return Ok(Value::true_val());
                };
            }
        }
        Some(block) => {
            let mut args = Args::new(1);
            for v in aref.elements.iter() {
                args[0] = *v;
                if vm.eval_block(block, &args)?.to_bool() {
                    return Ok(Value::true_val());
                };
            }
        }
    }
    Ok(Value::false_val())
}

fn all_(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let aref = self_val.into_array();
    if args.len() == 1 {
        if args.block.is_some() {
            eprintln!("warning: given block not used");
        }
        for v in aref.elements.iter() {
            if !vm.eval_teq(*v, args[0])? {
                return Ok(Value::false_val());
            };
        }
        return Ok(Value::true_val());
    }
    args.check_args_num(0)?;

    match &args.block {
        None => {
            for v in aref.elements.iter() {
                if !v.to_bool() {
                    return Ok(Value::false_val());
                };
            }
        }
        Some(block) => {
            let mut args = Args::new(1);
            for v in aref.elements.iter() {
                args[0] = *v;
                if !vm.eval_block(block, &args)?.to_bool() {
                    return Ok(Value::false_val());
                };
            }
        }
    }
    Ok(Value::true_val())
}

fn count(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    if args.block.is_some() {
        return Err(RubyError::argument("Currently, block is not supported."));
    }
    let ary = self_val.expect_array("").unwrap();
    match args.len() {
        0 => {
            let len = ary.len() as i64;
            Ok(Value::integer(len))
        }
        1 => {
            let other = args[0];
            let mut count = 0;
            for elem in &ary.elements {
                if vm.eval_eq2(*elem, other)? {
                    count += 1;
                }
            }
            Ok(Value::integer(count))
        }
        _ => unreachable!(),
    }
}

fn inject(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let block = args.expect_block()?;
    let ary = self_val.expect_array("").unwrap();
    let mut res = args[0];
    let mut args = Args::new(2);
    for elem in ary.elements.iter() {
        args[0] = res;
        args[1] = *elem;
        res = vm.eval_block(block, &args)?;
    }
    Ok(res)
}

fn find_index(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let ary = self_val.expect_array("").unwrap();
    if args.len() == 1 {
        if args.block.is_some() {
            eprintln!("Warning: given block not used.")
        };
        for (i, v) in ary.elements.iter().enumerate() {
            if v.eq(&args[0]) {
                return Ok(Value::integer(i as i64));
            };
        }
        return Ok(Value::nil());
    };
    let block = to_enum_str!(vm, self_val, args, "find_index");
    let mut args = Args::new(1);
    for (i, elem) in ary.elements.iter().enumerate() {
        args[0] = *elem;
        if vm.eval_block(&block, &args)?.to_bool() {
            return Ok(Value::integer(i as i64));
        };
    }
    Ok(Value::nil())
}

fn reject(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let ary = self_val.into_array();
    let block = to_enum_str!(vm, self_val, args, "reject");
    let mut args = Args::new(1);
    let mut res = vec![];
    for elem in ary.elements.iter() {
        args[0] = *elem;
        if !vm.eval_block(&block, &args)?.to_bool() {
            res.push(*elem);
        };
    }
    Ok(Value::array_from(res))
}

fn select(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let ary = self_val.into_array();
    let block = to_enum_str!(vm, self_val, args, "select");
    let mut args = Args::new(1);
    let mut res = vec![];
    for elem in ary.elements.iter() {
        args[0] = *elem;
        if vm.eval_block(&block, &args)?.to_bool() {
            res.push(*elem);
        };
    }
    Ok(Value::array_from(res))
}

fn find(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let ary = self_val.into_array();
    let block = to_enum_str!(vm, self_val, args, "find");
    let mut args = Args::new(1);
    for elem in ary.elements.iter() {
        args[0] = *elem;
        if vm.eval_block(&block, &args)?.to_bool() {
            return Ok(*elem);
        };
    }
    Ok(Value::nil())
}

fn binary_search(vm: &mut VM, ary: Array, block: &Block) -> Result<Option<usize>, RubyError> {
    if ary.len() == 0 {
        return Ok(None);
    };
    let mut args = Args::new(1);
    let mut i_min = 0;
    let mut i_max = ary.len() - 1;
    args[0] = ary.elements[0];
    if vm.eval_block(block, &args)?.expect_bool_nil_num()? {
        return Ok(Some(0));
    };
    args[0] = ary.elements[i_max];
    if !vm.eval_block(block, &args)?.expect_bool_nil_num()? {
        return Ok(None);
    };

    loop {
        let i_mid = i_min + (i_max - i_min) / 2;
        if i_mid == i_min {
            return Ok(Some(i_max));
        };
        args[0] = ary.elements[i_mid];
        if vm.eval_block(block, &args)?.expect_bool_nil_num()? {
            i_max = i_mid;
        } else {
            i_min = i_mid;
        };
    }
}

/// bsearch { |x| ... } -> object | nil
/// bsearch -> Enumerator
/// https://docs.ruby-lang.org/ja/latest/method/Array/i/bsearch.html
fn bsearch(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let ary = self_val.into_array();
    let block = to_enum_str!(vm, self_val, args, "bsearch");
    match binary_search(vm, ary, block)? {
        Some(i) => Ok(ary.elements[i]),
        None => Ok(Value::nil()),
    }
}

/// bsearch_index { |x| ... } -> Integer | nil
/// bsearch_index -> Enumerator
///
///https://docs.ruby-lang.org/ja/latest/method/Array/i/bsearch_index.html
fn bsearch_index(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let ary = self_val.into_array();
    let block = to_enum_str!(vm, self_val, args, "bsearch_index");
    match binary_search(vm, ary, block)? {
        Some(i) => Ok(Value::integer(i as i64)),
        None => Ok(Value::nil()),
    }
}

fn delete(_vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let arg = args[0];
    args.expect_no_block()?;
    let mut aref = self_val.into_array();
    let mut removed = None;
    aref.retain(|x| {
        if x.eq(&arg) {
            removed = Some(*x);
            Ok(false)
        } else {
            Ok(true)
        }
    })?;
    Ok(removed.unwrap_or(Value::nil()))
}

fn flatten(_vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let level = if args.len() == 0 {
        None
    } else {
        let i = args[0].coerce_to_fixnum("1st arg")?;
        if i < 0 {
            None
        } else {
            Some(i as usize)
        }
    };
    let mut res = vec![];
    for v in &self_val.into_array().elements {
        ary_flatten(*v, &mut res, level, self_val)?;
    }
    Ok(Value::array_from(res))
}

fn flatten_(_vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let level = if args.len() == 0 {
        None
    } else {
        let i = args[0].coerce_to_fixnum("1st arg")?;
        if i < 0 {
            None
        } else {
            Some(i as usize)
        }
    };
    let mut res = vec![];
    let mut flag = false;
    for v in self_val.into_array().iter() {
        flag |= ary_flatten(*v, &mut res, level, self_val)?;
    }
    self_val.into_array().elements = res;
    Ok(if flag { self_val } else { Value::nil() })
}

fn ary_flatten(
    val: Value,
    res: &mut Vec<Value>,
    level: Option<usize>,
    origin: Value,
) -> Result<bool, RubyError> {
    let mut flag = false;
    match level {
        None => match val.as_array() {
            Some(ainfo) => {
                if val.id() == origin.id() {
                    return Err(RubyError::argument("Tried to flatten recursive array."));
                };
                flag = true;
                for v in &ainfo.elements {
                    ary_flatten(*v, res, None, origin)?;
                }
            }
            None => res.push(val),
        },
        Some(0) => res.push(val),
        Some(level) => match val.as_array() {
            Some(ainfo) => {
                flag = true;
                for v in &ainfo.elements {
                    ary_flatten(*v, res, Some(level - 1), origin)?;
                }
            }
            None => res.push(val),
        },
    }
    Ok(flag)
}

#[cfg(test)]
mod tests {
    use crate::tests::*;

    #[test]
    fn array() {
        let program = r##"
        a=[1,2,3,4]
        assert 3, a[2];
        a[1]=14
        assert [1,14,3,4], a
        a.pop()
        assert [1,14,3], a
        a.push(7,8,9)
        assert [1,14,3,7,8,9], a
        a=[1,2,3,4]
        b=Array.new(a)
        assert a, b
        b[2]=100
        assert [1,2,3,4], a
        assert [1,2,100,4], b
        assert 4, a.length
        assert 4, b.length
        assert true, [].empty?
        assert false, a.empty?
        a = [1,2,3]
        b = [4,5]
        assert [1,2,3,4,5], a.concat(b)
        assert [1,2,3,4,5], a
        assert [4,5], b
        assert "[1, 2]", [1,2].inspect
        assert "[1, 2]", [1,2].to_s
        a = []
        a << a
        assert "[[...]]", a.inspect
        assert "[[...]]", a.to_s
        a = [1,2]
        a << a
        assert "[1, 2, [...]]", a.inspect
        assert "[1, 2, [...]]", a.to_s
    "##;
        assert_script(program);
    }

    #[test]
    fn array1() {
        let program = "
        assert [], [1,2,3]*0 
        assert [1,2,3]*1, [1,2,3]
        assert [nil,nil,nil,nil,nil], [nil]*5 
        assert [1,2,3,3,4,5], [1,2,3]+[3,4,5] 
        assert [1,2], [1,2,3]-[3,4,5] 
    ";
        assert_script(program);
    }

    #[test]
    fn array2() {
        let program = "
        a = [1,2,3,4,5,6,7]
        b = [3,9]
        c = [3,3]
        assert 3, a[2]
        assert [4,5,6,7], a[3,9]
        assert [4,5,6,7], a[*b] 
        assert [4,5,6], a[3,3]
        assert [4,5,6], a[*c] 
        assert nil, a[7]
        assert [], a[7,3]
    ";
        assert_script(program);
    }

    #[test]
    fn array3() {
        let program = "
        a = [1,2,3,4,5,6,7]
        assert [3,4,5], a[2,3]
        a[2,3] = 100
        assert [1,2,100,6,7], a
        a[1,3] = [10,11,12,13,14,15,16,17]
        assert [1,10,11,12,13,14,15,16,17,7], a
        a[4,5] = []
        assert [1,10,11,12,7], a
        a[1,4] = [0]
        assert [1,0], a
        ";
        assert_script(program);
    }

    #[test]
    fn array_set_elem_range() {
        let program = r##"
        ary = [0, 1, 2, 3, 4, 5]
        ary[0..2] = ["a", "b"]
        assert ["a", "b", 3, 4, 5], ary
        
        ary = [0, 1, 2]
        ary[5..6] = "x"
        assert [0, 1, 2, nil, nil, "x"], ary
        
        ary = [0, 1, 2, 3, 4, 5]
        ary[1..3] = "x"
        assert [0, "x", 4, 5], ary

        ary = [0, 1, 2, 3, 4, 5]
        ary[2..4] = []
        assert [0, 1, 5], ary

        ary = [0, 1, 2, 3, 4, 5]
        ary[2..0] = ["a", "b", "c"]
        assert [0, 1, "a", "b", "c", 2, 3, 4, 5], ary

        a = [0, 1, 2, 3, 4, 5]
        assert_error { a[-10..10] = 1 }       #=> RangeError
        "##;
        assert_script(program);
    }

    #[test]
    fn array_set_elem2() {
        let program = "
        a = *(1..5)
        assert 9, a[7,2] = 9
        assert [1,2,3,4,5,nil,nil,9], a
        
        a = *(1..5)
        assert 9, a[3,3] = 9
        assert [1,2,3,9], a
        
        a = *(1..5)
        assert 9, a[3,1] = 9
        assert [1,2,3,9,5], a

        a = *(1..5)
        assert 9, a[4,0] = 9
        assert [1,2,3,4,9,5], a

        a = *(1..5)
        assert [9,9,9], a[7,4] = [9,9,9]
        assert [1,2,3,4,5,nil,nil,9,9,9], a

        a = *(1..5)
        assert [9,9,9], a[3,4] = [9,9,9]
        assert [1,2,3,9,9,9], a

        a = *(1..5)
        assert [9,9,9], a[1,2] = [9,9,9]
        assert [1,9,9,9,4,5], a
        ";
        assert_script(program);
    }

    #[test]
    fn array_bracket() {
        let program = "
        class MyArray < Array
        end
        a = MyArray[0,1,2]
        assert(1, a[1])
        assert(MyArray, a.class)
        ";
        assert_script(program);
    }

    #[test]
    fn array_range() {
        let program = r##"
        a = [ "a", "b", "c", "d", "e" ]
        assert a[0..1], ["a", "b"]
        assert a[0...1], ["a"]
        assert a[0..-1], ["a", "b", "c", "d", "e"]
        assert a[-2..-1], ["d", "e"]
        assert a[-2..4], ["d", "e"]  #(start は末尾から -2 番目、end は先頭から (4+1) 番目となる。)
        assert a[0..10], ["a", "b", "c", "d", "e"]
        assert a[10..11], nil
        assert a[2..1], []
        assert a[-1..-2], []
        assert a[5..10], []

        # 特殊なケース。first が自身の長さと同じ場合には以下のようになります。
        #a[5]                   #=> nil
        #a[5, 1]                #=> []
        "##;
        assert_script(program);
    }

    #[test]
    fn array_toa() {
        let program = "
        a = [1,2,3]
        assert Array, a.class
        assert [1,2,3], a.to_a
        assert true, a == a.to_a
        class B < Array; end
        b = B.new([1,2,3])
        assert B, b.class
        assert Array, b.to_a.class
        assert true, b == b.to_a
        assert false, b.object_id == b.to_a.object_id
        ";
        assert_script(program);
    }

    #[test]
    fn array_shift() {
        let program = "
        a = [0, 1, 2, 3, 4]
        assert 0, a.shift
        assert [1, 2, 3, 4], a
        assert [1], a.shift(1)
        assert [2,3], a.shift(2)
        assert [4], a
        assert nil, [].shift
        assert [],  [].shift(1)

        a = [1,2,3]
        a.unshift 0
        assert [0, 1, 2, 3], a
        a.unshift [0]
        assert [[0], 0, 1, 2, 3], a
        a.unshift 1, 2
        assert [1, 2, [0], 0, 1, 2, 3], a
        ";
        assert_script(program);
    }

    #[test]
    fn array_cmp() {
        let program = "
        assert 0, [1,2,3,4] <=> [1,2,3,4]
        assert 1, [1,2,3,4] <=> [1,2,3]
        assert -1, [1,2,3,4] <=> [1,2,3,4,5]
        assert 1, [1,2,3,4] <=> [-1,2,3,4,5,6]
        assert -1, [1,2,3,4] <=> [6,2]
        assert nil, [1,2,3,4] <=> 8
        ";
        assert_script(program);
    }

    #[test]
    fn array_mul() {
        let program = r#"
        assert [1,2,3,1,2,3,1,2,3], [1,2,3] * 3 
        assert_error { [1,2,3] * nil } 
        assert_error { [1,2,3] * -1 } 

        class MyArray < Array; end
        assert MyArray[1,2,1,2], MyArray[1,2] * 2

        assert "1,2,3", [1,2,3] * ","
        assert "Ruby", ["Ruby"] * ","
        a = ["Ruby"] * 5
        assert ["Ruby", "Ruby", "Ruby", "Ruby", "Ruby"], a
        a[2] = "mruby"
        assert ["Ruby", "Ruby", "mruby", "Ruby", "Ruby"], a
        "#;
        assert_script(program);
    }

    #[test]
    fn array_push() {
        let program = r#"
        a = [1,2,3]
        a << 4
        a << "Ruby"
        assert([1,2,3,4,"Ruby"], a)
        "#;
        assert_script(program);
    }

    #[test]
    fn array_sort() {
        let program = r#"
        assert [-3,2,6], [6,2,-3].sort
        assert [-3,2.34,6.3], [6.3,2.34,-3].sort
        assert_error {[1,2.5,nil].sort}

        assert ["a","b","c","d","e"], ["d","a","e","c","b"].sort
        assert ["10","11","7","8","9"], ["9","7","10","11","8"].sort
        assert  ["7","8","9","10","11"],  ["9","7","10","11","8"].sort{ |a, b| a.to_i <=> b.to_i }

        assert ["BAR", "bar", "FOO", "foo"], ["BAR", "FOO", "bar", "foo"].sort_by { |v| v.downcase }
        "#;
        assert_script(program);
    }

    #[test]
    fn array_min_max() {
        let program = r#"
        assert nil, [].min
        #assert [], [].min(1)
        assert 2, [2, 5, 3.7].min
        #assert [2, 3], [2, 5, 3].min(2)
        assert nil, [].max
        #assert [], [].max(1)
        assert 5, [2, 5, 3].max
        #assert [5, 3], [2.1, 5, 3].max(2)
        "#;
        assert_script(program);
    }

    #[test]
    fn array_map() {
        let program = "
        a = [1,2,3]
        assert(a.map {|| 3 }, [3,3,3])
        assert(a.map {|x| x*3 }, [3,6,9])
        assert(a.map do |x| x*3 end, [3,6,9])
        assert(a, [1,2,3])
        b = [1, [2, 3], 4, [5, 6, 7]]
        assert [2, 2, 3, 2, 3, 8, 5, 6, 7, 5, 6, 7], b.flat_map{|x| x * 2}
        e = [1,2,3].map
        assert 1, e.next
        assert 2, e.next
        assert 3, e.next
        assert_error { e.next }
        ";
        assert_script(program);
    }

    #[test]
    fn array_map_() {
        let program = "
        a = [1,2,3]
        a.map! {|| 3 }
        assert [3,3,3], a
        a = [1,2,3]
        a.map! {|x| x*3 }
        assert [3,6,9], a
        a = [1,2,3]
        a.map! do |x| x*3 end
        assert [3,6,9], a
        b = [1, [2, 3], 4, [5, 6, 7]]
        #b.flat_map!{|x| x * 2}
        #assert b, [2, 2, 3, 2, 3, 8, 5, 6, 7, 5, 6, 7]
        e = [1,2,3].map!
        assert 1, e.next
        assert 2, e.next
        assert 3, e.next
        assert_error { e.next }
        ";
        assert_script(program);
    }

    #[test]
    fn array_each() {
        let program = "
        a = [1,2,3]
        b = 0
        assert([1,2,3], a.each {|x| b+=x })
        assert(6, b)

        a = (0..4).to_a
        b = []
        a.each do |x|
          if x == 2
            a.shift
          end
          b << x
        end
        assert([1,2,3,4], a)
        assert([0,1,2,4], b)

        a = [1,2,3]
        b = []
        assert([1,2,3], a.each_with_index {|x, i| b << [x,i] })
        assert([[1, 0], [2, 1], [3, 2]], b)

        a = (0..3).to_a
        b = []
        a.each_with_index do |x, i|
          if i == 2
            a.shift
          end
          b << [x, i]
        end
        assert([1,2,3], a)
        assert([[0,0], [1,1], [2,2]], b)

        b = []
        a = [4,3,2,1].each_index {|x| b << x*2}
        assert [4,3,2,1], a
        assert [0,2,4,6], b
    ";
        assert_script(program);
    }

    #[test]
    fn partition() {
        let program = "
        a = [10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0].partition {|i| i % 3 == 0 }
        assert [[9, 6, 3, 0], [10, 8, 7, 5, 4, 2, 1]], a
    ";
        assert_script(program);
    }

    #[test]
    fn array_include() {
        let program = r#"
        a = ["ruby","rust","java"]
        assert(true, a.include?("ruby"))
        assert(true, a.include?("rust"))
        assert(false, a.include?("c++"))
        assert(false, a.include?(:ruby))
    "#;
        assert_script(program);
    }

    #[test]
    fn array_reverse() {
        let program = "
    a = [1,2,3,4,5]
    assert([5,4,3,2,1], a.reverse)
    assert([1,2,3,4,5], a)
    assert([5,4,3,2,1], a.reverse!)
    assert([5,4,3,2,1], a)
    ";
        assert_script(program);
    }

    #[test]
    fn array_rotate() {
        let program = r#"
        a = ["a","b","c","d"]
        assert ["b","c","d","a"], a.rotate!
        assert ["b","c","d","a"], a
        assert ["d","a","b","c"], a.rotate!(2)
        assert ["a","b","c","d"], a.rotate!(-3) 
    "#;
        assert_script(program);
    }

    #[test]
    fn array_compact() {
        let program = r#"
        ary = [1, nil, 2, nil, 3, nil]
        assert [1, 2, 3], ary.compact 
        assert [1, nil, 2, nil, 3, nil], ary 
        assert [1, 2, 3], ary.compact!
        assert [1, 2, 3], ary
        assert nil, ary.compact!
    "#;
        assert_script(program);
    }

    #[test]
    fn array_transpose() {
        let program = r#"
        assert [[1, 3, 5], [2, 4, 6]], [[1,2],[3,4],[5,6]].transpose
        assert [], [].transpose

        assert_error { [1,2,3].transpose }
        assert_error { [[1,2],[3,4,5],[6,7]].transpose }
        "#;
        assert_script(program);
    }

    #[test]
    fn array_zip() {
        let program = r#"
        assert [[1,4,7],[2,5,8],[3,6,9]], [1,2,3].zip([4,5,6],[7,8,9])
        assert [[1,:a,:A],[2,:b,:B]], [1,2].zip([:a,:b,:c],[:A,:B,:C,:D])
        assert [[1,:a,:A],[2,:b,:B],[3,:c,:C],[4,nil,:D],[5,nil,nil]], [1,2,3,4,5].zip([:a,:b,:c],[:A,:B,:C,:D])
        ans = []
        [1,2,3].zip([4,5,6], [7,8,9]) {|ary|
            ans.push(ary)
        }
        assert [[1,4,7],[2,5,8],[3,6,9]], ans
        "#;
        assert_script(program);
    }

    #[test]
    fn fill() {
        let program = r#"
        a = [1,2,3,4]
        assert ["Ruby","Ruby","Ruby","Ruby"], a.fill("Ruby")
        assert ["Ruby","Ruby","Ruby","Ruby"], a
        "#;
        assert_script(program);
    }

    #[test]
    fn array_methods() {
        let program = r#"
        a = [1,2,3,4]
        assert [], a.clear
        assert [], a
        assert 1, [1,2,3,4].first
        assert 4, [1,2,3,4].last
        assert nil, [].first
        assert nil, [].last
        a = ["a","b","c"]
        assert ["b","c"], a.slice!(1, 2)
        assert ["a"], a
        a = ["a","b","c"]
        assert [], a.slice!(1, 0)
        assert ["a","b","c"], a
        "#;
        assert_script(program);
    }

    #[test]
    fn uniq() {
        let program = r#"
        a = [1,2,3,4,3,2,1,0,3.0]
        assert [1,2,3,4,0,3.0], a.uniq
        assert [1,2,3,4,3,2,1,0,3.0], a
        100.times {|i|
          if [1,2,3,3.0] != a.uniq {|x| x % 3 }
            raise StandardError.new("assert failed in #{i}")
          end
        }
        assert_error { a.uniq {|x| if x == 3 then raise end; x} }
        assert [1,2,3,4,3,2,1,0,3.0], a
        100.times {|i|
          a = [1,2,3,4,3,2,1,0,3.0]
          if [1,2,3,3.0] != a.uniq! {|x| x % 3 }
            raise StandardError.new("assert failed in #{i}")
          end
        }
        "#;
        assert_script(program);
    }

    #[test]
    fn count() {
        let program = r#"
        a = [1, "1", 1.0, :one, 1, :one, 53.7, [1]]
        assert 8, a.count
        assert 3, a.count(1)
        assert 2, a.count(:one)
        "#;
        assert_script(program);
    }

    #[test]
    fn any_() {
        let program = r#"
        assert false, [1, 2, 3].any? {|v| v > 3 }
        assert true, [1, 2, 3].any? {|v| v > 1 }
        assert false, [].any? {|v| v > 0 }
        assert false, %w[ant bear cat].any?(/d/)
        assert true, %w[ant bear cat].any?(/ear/)
        assert true, [nil, true, 99].any?(Integer)
        assert false, [nil, true, 99].any?(String)
        assert false, [nil, false, nil].any?
        assert true, [nil, true, 99].any?
        assert false, [].any?
        "#;
        assert_script(program);
    }

    #[test]
    fn all_() {
        let program = r#"
        assert true, [5, 6, 7].all? {|v| v > 0 }
        assert false, [5, -1, 7].all? {|v| v > 0 }
        assert true, [].all? {|v| v > 0 }
        assert true, [-50, 77, 99].all?(Integer)
        assert false, [3, 111, :d].all?(Integer)
        assert false, %w[ant bear cat].all?(/t/)
        assert true, %w[ant bear cat].all?(/a/)
        "#;
        assert_script(program);
    }

    #[test]
    fn inject() {
        let program = r#"
        assert 14, [2, 3, 4, 5].inject(0) {|result, item| result + item }
        assert 54, [2, 3, 4, 5].inject(0) {|result, item| result + item**2 }
        "#;
        assert_script(program);
    }

    #[test]
    fn index() {
        let program = r#"
        assert 0, [1, 2, 3, 4, 5].index(1)
        assert 3, [1, 2, 3, 4, 5].index(4)
        assert nil, [1, 2, 3, 4, 5].index(2.5)
        assert 0, [3, 0, 0, 1, 0].index {|v| v > 0}
        assert 3, [0, 0, 0, 1, 0].index {|v| v > 0}
        assert nil, [-40, -1, -5, -11, 0].index {|v| v > 0}
        "#;
        assert_script(program);
    }

    #[test]
    fn reject() {
        let program = r#"
        assert [1, 3, 5], [1, 2, 3, 4, 5, 6].reject {|i| i % 2 == 0 }
        "#;
        assert_script(program);
    }

    #[test]
    fn select() {
        let program = r#"
        assert [1, 3, 5], [1, 2, 3, 4, 5, 6].select {|i| i % 2 != 0 }
        "#;
        assert_script(program);
    }

    #[test]
    fn find() {
        let program = r#"
        assert 3, [1, 2, 3, 4, 5].find {|i| i % 3 == 0 }
        assert nil, [2, 2, 2, 2, 2].find {|i| i % 3 == 0 }
        "#;
        assert_script(program);
    }

    #[test]
    fn bsearch() {
        let program = r#"
        ary = [0, 4, 7, 10, 12]
        assert 4, ary.bsearch {|x| x >=  4 } # => 4
        assert 7, ary.bsearch {|x| x >=  6 } # => 7
        assert 0, ary.bsearch {|x| x >= -1 } # => 0
        assert nil, ary.bsearch {|x| x >= 100 } # => nil
        assert nil, ary.bsearch { false }
        assert nil, ary.bsearch { nil }
        assert 0, ary.bsearch { true }
        assert nil, ary.bsearch { 1 }
        assert 0, ary.bsearch { 0 }
        assert 0, ary.bsearch { 0.0 }
        "#;
        assert_script(program);
    }

    #[test]
    fn bsearch_index() {
        let program = r#"
        ary = [0, 4, 7, 10, 12]
        assert 1, ary.bsearch_index {|x| x >=  4 }
        assert 2, ary.bsearch_index {|x| x >=  6 }
        assert 0, ary.bsearch_index {|x| x >= -1 }
        assert nil, ary.bsearch_index {|x| x >= 100 }
        "#;
        assert_script(program);
    }

    #[test]
    fn delete() {
        let program = r#"
        ary = [0, nil, 7, 1, "hard", 1.0]
        assert 1.0, ary.delete(1)
        assert [0, nil, 7, "hard"], ary
        "#;
        assert_script(program);
    }

    #[test]
    fn flatten() {
        let program = r#"
        a = [1, [2, 3, [4], 5]]
        assert [1, 2, 3, 4, 5], a.flatten
        assert [1, [2, 3, [4], 5]], a
        assert [1, [2, 3, [4], 5]], a.flatten(0)
        assert [1, 2, 3, [4], 5], a.flatten(1)
        assert [1, 2, 3, 4, 5], a.flatten(2)
        assert [1, 2, 3, 4, 5], a.flatten(3)

        a = [1]
        a << a
        assert_error { a.flatten }
        assert_error { a.flatten(-1) }
        assert_error { a.flatten("1") }
        assert [1, a], a.flatten(0)
        assert [1, 1, a], a.flatten(1)

        a = [1, [2, 3, [4], 5]]
        assert [1, 2, 3, 4, 5], a.flatten!
        assert [1, 2, 3, 4, 5], a
        assert nil, a.flatten!

        a = [1, [2, 3, [4], 5]]
        assert nil, a.flatten!(0)
        assert [1, 2, 3, [4], 5], a.flatten!(1)
        assert [1, 2, 3, 4, 5], a.flatten!(2)
        assert nil, a.flatten!(2)

        a = [1]
        a << a
        assert_error { a.flatten! }
        assert_error { a.flatten!(-1) }
        assert_error { a.flatten!("1") }
        assert [1, a], a
        assert nil, a.flatten!(0)
        assert [1, 1, 1, a], a.flatten!(2)
        "#;
        assert_script(program);
    }
}
