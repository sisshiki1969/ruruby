//use crate::error::RubyError;
use crate::*;
use fxhash::FxHashSet;

pub(crate) fn init(globals: &mut Globals) -> Value {
    let class = Module::class_under_object();
    globals.set_toplevel_constant("Array", class);
    class.add_builtin_method_by_str(globals, "inspect", inspect);
    class.add_builtin_method_by_str(globals, "to_s", inspect);
    class.add_builtin_method_by_str(globals, "to_a", toa);
    class.add_builtin_method_by_str(globals, "length", length);
    class.add_builtin_method_by_str(globals, "size", length);
    class.add_builtin_method_by_str(globals, "empty?", empty);
    class.add_builtin_method_by_str(globals, "[]", get_elem);
    class.add_builtin_method_by_str(globals, "at", at);
    class.add_builtin_method_by_str(globals, "[]=", set_elem);
    class.add_builtin_method_by_str(globals, "push", push);
    class.add_builtin_method_by_str(globals, "<<", push);
    class.add_builtin_method_by_str(globals, "pop", pop);
    class.add_builtin_method_by_str(globals, "*", mul);
    class.add_builtin_method_by_str(globals, "+", add);
    class.add_builtin_method_by_str(globals, "-", sub);
    class.add_builtin_method_by_str(globals, "<=>", cmp);

    class.add_builtin_method_by_str(globals, "shift", shift);
    class.add_builtin_method_by_str(globals, "unshift", unshift);

    class.add_builtin_method_by_str(globals, "concat", concat);
    class.add_builtin_method_by_str(globals, "map", map);
    class.add_builtin_method_by_str(globals, "collect", map);
    class.add_builtin_method_by_str(globals, "map!", map_);
    class.add_builtin_method_by_str(globals, "collect!", map_);
    class.add_builtin_method_by_str(globals, "flat_map", flat_map);
    class.add_builtin_method_by_str(globals, "each", each);
    class.add_builtin_method_by_str(globals, "each_index", each_index);
    class.add_builtin_method_by_str(globals, "each_with_index", each_with_index);
    class.add_builtin_method_by_str(globals, "partition", partition);

    class.add_builtin_method_by_str(globals, "include?", include);
    class.add_builtin_method_by_str(globals, "reverse", reverse);
    class.add_builtin_method_by_str(globals, "reverse!", reverse_);
    class.add_builtin_method_by_str(globals, "rotate!", rotate_);
    class.add_builtin_method_by_str(globals, "compact", compact);
    class.add_builtin_method_by_str(globals, "compact!", compact_);

    class.add_builtin_method_by_str(globals, "transpose", transpose);
    class.add_builtin_method_by_str(globals, "min", min);
    class.add_builtin_method_by_str(globals, "fill", fill);
    class.add_builtin_method_by_str(globals, "clear", clear);
    class.add_builtin_method_by_str(globals, "uniq!", uniq_);
    class.add_builtin_method_by_str(globals, "uniq", uniq);
    class.add_builtin_method_by_str(globals, "any?", any_);
    class.add_builtin_method_by_str(globals, "all?", all_);

    class.add_builtin_method_by_str(globals, "slice!", slice_);
    class.add_builtin_method_by_str(globals, "max", max);
    class.add_builtin_method_by_str(globals, "first", first);
    class.add_builtin_method_by_str(globals, "last", last);
    class.add_builtin_method_by_str(globals, "dup", dup);
    class.add_builtin_method_by_str(globals, "clone", dup);
    class.add_builtin_method_by_str(globals, "pack", pack);
    class.add_builtin_method_by_str(globals, "join", join);
    class.add_builtin_method_by_str(globals, "drop", drop);
    class.add_builtin_method_by_str(globals, "zip", zip);
    class.add_builtin_method_by_str(globals, "grep", grep);
    class.add_builtin_method_by_str(globals, "sort", sort);
    class.add_builtin_method_by_str(globals, "sort_by", sort_by);
    class.add_builtin_method_by_str(globals, "count", count);
    class.add_builtin_method_by_str(globals, "inject", inject);
    class.add_builtin_method_by_str(globals, "reduce", inject);
    class.add_builtin_method_by_str(globals, "find_index", find_index);
    class.add_builtin_method_by_str(globals, "index", find_index);

    class.add_builtin_method_by_str(globals, "reject", reject);
    class.add_builtin_method_by_str(globals, "find", find);
    class.add_builtin_method_by_str(globals, "detect", find);
    class.add_builtin_method_by_str(globals, "select", select);
    class.add_builtin_method_by_str(globals, "filter", select);
    class.add_builtin_method_by_str(globals, "bsearch", bsearch);
    class.add_builtin_method_by_str(globals, "bsearch_index", bsearch_index);
    class.add_builtin_method_by_str(globals, "delete", delete);
    class.add_builtin_method_by_str(globals, "flatten", flatten);
    class.add_builtin_method_by_str(globals, "flatten!", flatten_);

    class.add_builtin_class_method(globals, "new", array_new);
    class.add_builtin_class_method(globals, "allocate", array_allocate);
    class.add_builtin_class_method(globals, "[]", array_elem);
    class.into()
}

// Class methods

fn array_new(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_range(0, 2)?;
    let self_val = self_val.into_module();
    let array_vec = match vm.args_len() {
        0 => vec![],
        1 => match vm[0].unpack() {
            RV::Integer(num) => {
                if num < 0 {
                    return Err(RubyError::argument("Negative array size."));
                };
                vec![Value::nil(); num as usize]
            }
            RV::Object(oref) => match oref.kind() {
                ObjKind::ARRAY => oref.array().to_vec(),
                _ => return Err(VMError::wrong_type("1st arg", "Integer or Array", vm[0])),
            },
            _ => return Err(VMError::wrong_type("1st arg", "Integer or Array", vm[0])),
        },
        2 => {
            let num = vm[0].coerce_to_fixnum("1st arg")?;
            if num < 0 {
                return Err(RubyError::argument("Negative array size."));
            };
            vec![vm[1]; num as usize]
        }
        _ => unreachable!(),
    };
    let array = Value::array_from_with_class(array_vec, self_val);
    if let Some(method) = vm
        .globals
        .methods
        .find_method(self_val, IdentId::INITIALIZE)
    {
        let range = vm.args_range();
        vm.eval_method_range(method, array, range, &args)?;
    };
    Ok(array)
}

fn array_allocate(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let array = Value::array_from_with_class(vec![], self_val.into_module());
    Ok(array)
}

fn array_elem(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    let array = Value::array_from_with_class(vm.args().to_vec(), self_val.into_module());
    Ok(array)
}

// Instance methods

fn inspect(vm: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    fn checked_inspect(vm: &mut VM, self_val: Value, elem: Value) -> Result<String, RubyError> {
        if elem.id() == self_val.id() {
            Ok("[...]".to_string())
        } else {
            vm.val_inspect(elem)
        }
    }
    let ary = self_val.into_array();
    let s = match ary.len() {
        0 => "[]".to_string(),
        len => {
            let mut result = checked_inspect(vm, self_val, ary[0])?;
            for i in 1..len {
                result = format!("{}, {}", result, checked_inspect(vm, self_val, ary[i])?);
            }
            format! {"[{}]", result}
        }
    };
    Ok(Value::string(s))
}

fn toa(_: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
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
fn get_elem(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_range(1, 2)?;
    self_val.into_array().get_elem(vm.args())
}

/// at(nth) -> object | nil
///
/// https://docs.ruby-lang.org/ja/latest/method/Array/i/=5b=5d.html
fn at(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    self_val.into_array().get_elem1(vm[0])
}

/// self[nth] = val
/// self[range] = val
/// self[start, length] = val
///
/// https://docs.ruby-lang.org/ja/latest/method/Array/i/=5b=5d=3d.html
fn set_elem(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_range(2, 3)?;
    self_val.into_array().set_elem(vm.args())
}

fn cmp(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    if self_val.id() == vm[0].id() {
        return Ok(Value::integer(0));
    }
    let lhs = self_val.into_array();
    let rhs = match vm[0].as_array() {
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

fn push(vm: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    let mut ary = self_val.into_array();
    for arg in vm.args() {
        ary.push(*arg);
    }
    Ok(self_val)
}

fn pop(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let mut ary = self_val.into_array();
    let res = ary.pop().unwrap_or_default();
    Ok(res)
}

/// shift -> object | nil
/// shift(n) -> Array
/// https://docs.ruby-lang.org/ja/latest/method/Array/i/shift.html
fn shift(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_range(0, 1)?;
    let mut array_flag = false;
    let num = if vm.args_len() == 0 {
        0
    } else {
        let i = vm[0].coerce_to_fixnum("1st arg")?;
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
        let res = ary.to_vec();
        *ary = ArrayInfo::new(new);
        Ok(Value::array_from(res))
    } else {
        if ary.len() == 0 {
            return Ok(Value::nil());
        }
        let res = ary[0];
        *ary = ArrayInfo::new(ary[1..].to_vec());
        Ok(res)
    }
}

fn unshift(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    if vm.args_len() == 0 {
        return Ok(self_val);
    }
    let mut new = vm.args().to_owned();
    let ary = &mut *self_val.into_array();
    new.extend_from_slice(&ary);
    *ary = ArrayInfo::new(new);
    Ok(self_val)
}

fn length(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let res = Value::integer(self_val.into_array().len() as i64);
    Ok(res)
}

fn empty(_vm: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    let aref = self_val.into_array();
    let res = Value::bool(aref.is_empty());
    Ok(res)
}

fn mul(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let ary = self_val.into_array();
    if let Some(num) = vm[0].as_fixnum() {
        let v = match num {
            i if i < 0 => return Err(RubyError::argument("Negative argument.")),
            i => ary.repeat(i as usize),
        };
        let res = Value::array_from_with_class(v, self_val.get_class());
        Ok(res.into())
    } else if let Some(s) = vm[0].clone().as_string() {
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
        return Err(VMError::no_implicit_conv(vm[0], "Integer"));
    }
}

fn add(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let mut lhs = self_val.into_array().to_vec();
    let mut arg0 = vm[0];
    let mut rhs = arg0.expect_array("Argument")?.to_vec();
    lhs.append(&mut rhs);
    Ok(Value::array_from(lhs))
}

fn concat(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let lhs = &mut *self_val.into_array();
    let mut arg0 = vm[0];
    let rhs = &**arg0.expect_array("Argument")?;
    lhs.extend_from_slice(rhs);
    Ok(self_val)
}

fn sub(vm: &mut VM, mut self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let lhs_v = &**self_val.expect_array("Receiver")?;
    let mut arg0 = vm[0];
    let rhs_v = &**arg0.expect_array("Argument")?;
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
                let val = $vm.create_enumerator($id, $self_val, $args.into($vm))?;
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
                let val =
                    $vm.create_enumerator(IdentId::get_id($id), $self_val, $args.into($vm))?;
                return Ok(val);
            }
            Some(block) => block,
        }
    };
}

fn map(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let aref = self_val.into_array();
    let method = to_enum_id!(vm, self_val, args, IdentId::MAP);

    let temp_len = vm.temp_len();

    let mut i = 0;
    while i < aref.len() {
        let res = vm.eval_block1(method, aref[i])?;
        vm.temp_push(res);
        i += 1;
    }

    let res = vm.temp_pop_vec(temp_len);
    Ok(Value::array_from(res))
}

fn map_(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let mut aref = self_val.into_array();
    let method = to_enum_id!(vm, self_val, args, IdentId::MAP);

    let mut i = 0;
    while i < aref.len() {
        aref[i] = vm.eval_block1(method, aref[i])?;
        i += 1;
    }

    Ok(self_val)
}

fn flat_map(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    let block = args.expect_block()?;
    let param_num = block.to_iseq(&vm.globals).params.req;
    let aref = vm.self_value().into_array();
    let temp_len = vm.temp_len();
    for elem in &**aref {
        let ary = if param_num == 0 {
            vm.eval_block0(&block)?
        } else if param_num == 1 {
            vm.eval_block1(&block, *elem)?
        } else {
            match elem.as_array() {
                Some(ary) => {
                    //arg.copy_from_slice(&ary.elements[0..param_num]);
                    vm.eval_block(&block, &ary[0..param_num], &Args2::new(param_num))?
                }
                None => vm.eval_block1(&block, *elem)?,
            }
        };

        match ary.as_array() {
            Some(ary) => vm.temp_extend_from_slice(&**ary),
            None => vm.temp_push(ary),
        }
    }
    let val = Value::array_from(vm.temp_pop_vec(temp_len));
    Ok(val)
}

/// each {|item| .... } -> self
/// each -> Enumerator
///
/// https://docs.ruby-lang.org/ja/latest/method/Array/i/each.html
fn each(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let method = to_enum_id!(vm, self_val, args, IdentId::EACH);
    let aref = self_val.into_array();
    let mut i = 0;
    while i < aref.len() {
        vm.eval_block1(method, aref[i])?;
        i += 1;
    }
    Ok(self_val)
}

/// each_index {|index| .... } -> self
/// each_index -> Enumerator
///
/// https://docs.ruby-lang.org/ja/latest/method/Array/i/each_index.html
fn each_index(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let method = to_enum_str!(vm, self_val, args, "each_index");
    let aref = self_val.into_array();
    let mut i = 0;
    while i < aref.len() {
        vm.eval_block1(method, Value::integer(i as i64))?;
        i += 1;
    }
    Ok(self_val)
}

/// Enumerable#each_with_index
/// each_with_index(*args) -> Enumerator
/// each_with_index(*args) {|item, index| ... } -> self
///
/// https://docs.ruby-lang.org/ja/latest/method/Enumerable/i/each_with_index.html
fn each_with_index(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let method = to_enum_str!(vm, self_val, args, "each_with_index");
    let aref = self_val.into_array();
    let mut i = 0;
    while i < aref.len() {
        vm.eval_block2(method, aref[i], Value::integer(i as i64))?;
        i += 1;
    }
    Ok(self_val)
}

fn partition(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let method = to_enum_str!(vm, self_val, args, "partition");
    let aref = self_val.into_array();
    let mut res_true = vec![];
    let mut res_false = vec![];
    for i in &**aref {
        if vm.eval_block1(method, *i)?.to_bool() {
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

fn include(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let target = vm[0];
    let aref = self_val.into_array();
    for item in aref.iter() {
        if vm.eval_eq2(*item, target)? {
            return Ok(Value::true_val());
        }
    }
    Ok(Value::false_val())
}

fn reverse(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let aref = self_val.into_array();
    let mut res = aref.to_vec();
    res.reverse();
    Ok(Value::array_from(res))
}

fn reverse_(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let mut aref = self_val.into_array();
    aref.reverse();
    Ok(self_val)
}

/// rotate!(cnt = 1) -> Array
/// https://docs.ruby-lang.org/ja/latest/method/Array/i/rotate=21.html
fn rotate_(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_range(0, 1)?;
    let i = if vm.args_len() == 0 {
        1
    } else {
        match vm[0].as_fixnum() {
            Some(i) => i,
            None => return Err(VMError::cant_coerse(vm[0], "Integer")),
        }
    };
    let mut aref = self_val.into_array();
    if i == 0 || aref.is_empty() {
        Ok(self_val)
    } else if i > 0 {
        let i = i % (aref.len() as i64);
        aref.rotate_left(i as usize);
        Ok(self_val)
    } else {
        let len = aref.len() as i64;
        let i = (-i) % len;
        aref.rotate_right(i as usize);
        Ok(self_val)
    }
}

fn compact(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let aref = self_val.into_array();
    let ary = aref.iter().filter(|x| !x.is_nil()).cloned().collect();
    Ok(Value::array_from(ary))
}

fn compact_(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let mut aref = self_val.into_array();
    let mut flag = false;
    aref.retain(|x| {
        let b = !x.is_nil();
        if !b {
            flag = true
        };
        Ok(b)
    })?;
    if flag {
        Ok(self_val)
    } else {
        Ok(Value::nil())
    }
}

fn transpose(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let mut aref = self_val.into_array();
    if aref.len() == 0 {
        return Ok(Value::array_empty().into());
    }
    let mut vec = vec![];
    for elem in &mut **aref {
        let ary = elem
            .as_array()
            .ok_or(RubyError::argument(
                "Each element of receiver must be an array.",
            ))?
            .to_vec();
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

fn min(_: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
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
    if aref.len() == 0 {
        return Ok(Value::nil());
    }
    let mut min_obj = aref[0];
    let mut min = to_float(min_obj)?;
    for elem in &**aref {
        let elem_f = to_float(*elem)?;
        if elem_f < min {
            min_obj = *elem;
            min = elem_f;
        }
    }

    return Ok(min_obj);
}

fn max(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let aref = self_val.into_array();
    if aref.len() == 0 {
        return Ok(Value::nil());
    }
    let mut max = aref[0];
    for elem in &**aref {
        if vm.eval_gt2(max, *elem)? {
            max = *elem;
        };
    }
    Ok(max)
}

fn fill(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let mut aref = self_val.into_array();
    for elem in &mut **aref {
        *elem = vm[0];
    }
    Ok(self_val)
}

fn clear(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let mut aref = self_val.into_array();
    aref.clear();
    Ok(self_val)
}

/// uniq -> Array
/// uniq {|item| ... } -> Array
/// https://docs.ruby-lang.org/ja/latest/method/Array/i/uniq.html
fn uniq(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.into_array();
    let mut h = FxHashSet::default();
    let mut v = vec![];
    match &args.block {
        None => {
            let mut recursive = false;
            for elem in &**aref {
                if self_val.id() == elem.id() {
                    if !recursive {
                        v.push(*elem);
                        recursive = true;
                    }
                } else if h.insert(HashKey(*elem)) {
                    v.push(*elem);
                }
            }
        }
        Some(block) => {
            for elem in &**aref {
                let res = vm.eval_block1(block, *elem)?;
                if h.insert(HashKey(res)) {
                    v.push(*elem);
                };
            }
        }
    };
    Ok(Value::array_from(v))
}

/// uniq! -> self | nil
/// uniq! {|item| ... } -> self | nil
/// https://docs.ruby-lang.org/ja/latest/method/Array/i/uniq.html
fn uniq_(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    let mut h = FxHashSet::default();
    let deleted = match &args.block {
        None => {
            let mut aref = self_val.into_array();
            let mut recursive = false;
            aref.retain(|x| {
                if self_val.id() == x.id() {
                    if !recursive {
                        //h.insert(HashKey(*x));
                        recursive = true;
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                } else {
                    Ok(h.insert(HashKey(*x)))
                }
            })?
        }
        Some(block) => {
            let mut aref = self_val.into_array();
            aref.retain(|x| {
                let res = vm.eval_block1(block, *x)?;
                vm.temp_push(res);
                Ok(h.insert(HashKey(res)))
            })?
        }
    };
    if deleted {
        Ok(self_val)
    } else {
        Ok(Value::nil())
    }
}

/// slice!(nth) -> object | nil         NOT SUPPORTED
/// slice!(start, len) -> Array | nil
/// slice!(range) -> Array | nil        NOT SUPPORTED
/// https://docs.ruby-lang.org/ja/latest/method/Array/i/slice=21.html
fn slice_(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(2)?;
    let start = vm[0].coerce_to_fixnum("Currently, first arg must be Integer.")?;
    if start < 0 {
        return Err(RubyError::argument("First arg must be positive value."));
    };
    let len = vm[1].coerce_to_fixnum("Currently, second arg must be Integer")?;
    if len < 0 {
        return Err(RubyError::argument("Second arg must be positive value."));
    };
    let start = start as usize;
    let len = len as usize;
    let mut aref = self_val.into_array();
    let ary_len = aref.len();
    if ary_len < start {
        return Ok(Value::nil());
    }
    if ary_len <= start || len == 0 {
        return Ok(Value::array_from(vec![]));
    }
    let end = if ary_len < start + len {
        ary_len
    } else {
        start + len
    };
    let new = aref.drain(start..end);
    Ok(Value::array_from(new))
}

fn first(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let aref = self_val.into_array();
    if aref.len() == 0 {
        Ok(Value::nil())
    } else {
        Ok(aref[0])
    }
}

fn last(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let aref = self_val.into_array();
    if aref.len() == 0 {
        Ok(Value::nil())
    } else {
        Ok(*aref.last().unwrap())
    }
}

fn dup(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let aref = self_val.into_array();
    Ok(Value::array_from(aref.to_vec()))
}

fn pack(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_range(0, 1)?;
    let aref = self_val.into_array();
    let mut v = vec![];
    for elem in &**aref {
        let i = match elem.as_fixnum() {
            Some(i) => i as i8 as u8,
            None => return Err(RubyError::argument("Must be Array of Integer.")),
        };
        v.push(i);
    }
    Ok(Value::bytes(v))
}

fn join(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_range(0, 1)?;
    let sep = if vm.args_len() == 0 {
        "".to_string()
    } else {
        match vm[0].as_string() {
            Some(s) => s.to_string(),
            None => return Err(RubyError::argument("Seperator must be String.")),
        }
    };
    let aref = self_val.into_array();
    let mut res = String::new();
    for elem in &**aref {
        let s = elem.val_to_s(vm)?;
        if res.is_empty() {
            res = s.into_owned();
        } else {
            res = res + &sep + &s;
        }
    }
    Ok(Value::string(res))
}

fn drop(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let aref = self_val.into_array();
    let num = vm[0].coerce_to_fixnum("An argument must be Integer.")? as usize;
    if num >= aref.len() {
        return Err(RubyError::argument(format!("An argument too big. {}", num)));
    };
    let ary = &aref[num..];
    Ok(Value::array_from(ary.to_vec()))
}

fn zip(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    let self_ary = self_val.into_array();
    let mut args_ary = vec![];
    for a in vm.args() {
        args_ary.push(a.clone().expect_array("Args")?.to_vec());
    }
    let mut ary = vec![];
    for (i, val) in self_ary.iter().enumerate() {
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
            vm.temp_extend_from_slice(&ary);
            for val in ary {
                vm.eval_block1(block, val)?;
            }
            Ok(Value::nil())
        }
    }
}

fn grep(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let aref = self_val.into_array();
    let ary = match &args.block {
        None => aref
            .iter()
            .filter_map(|x| match vm.eval_teq(*x, vm[0]) {
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
fn sort(vm: &mut VM, mut self_val: Value, args: &Args2) -> VMResult {
    //use std::cmp::Ordering;
    vm.check_args_num(0)?;
    let mut ary = self_val.expect_array("Receiver")?.to_vec();
    match &args.block {
        None => {
            vm.sort_array(&mut ary)?;
        }
        Some(block) => {
            vm.sort_by(&mut ary, |vm, a, b| {
                vm.eval_block2(block, *a, *b)?.to_ordering()
            })?;
        }
    };
    Ok(Value::array_from(ary))
}

/// Enumerator#sort { |item| .. } -> Array
/// https://docs.ruby-lang.org/ja/latest/method/Enumerable/i/sort_by.html
fn sort_by(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let block = args.expect_block()?;
    let mut ary = vec![];
    {
        for v in &**self_val.as_array().unwrap() {
            let v1 = vm.eval_block1(block, *v)?;
            vm.temp_push(v1);
            ary.push((*v, v1));
        }
    }
    vm.sort_by(&mut ary, |vm, a, b| {
        Ok(vm.eval_compare(b.1, a.1)?.to_ordering()?)
    })?;

    Ok(Value::array_from(ary.iter().map(|x| x.0).collect()))
}

fn any_(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    let aref = self_val.into_array();
    if vm.args_len() == 1 {
        if args.block.is_some() {
            eprintln!("warning: given block not used");
        }
        for v in aref.iter() {
            if vm.eval_teq(*v, vm[0])? {
                return Ok(Value::true_val());
            };
        }
        return Ok(Value::false_val());
    }
    vm.check_args_num(0)?;

    match &args.block {
        None => {
            for v in aref.iter() {
                if v.to_bool() {
                    return Ok(Value::true_val());
                };
            }
        }
        Some(block) => {
            for v in aref.iter() {
                if vm.eval_block1(block, *v)?.to_bool() {
                    return Ok(Value::true_val());
                };
            }
        }
    }
    Ok(Value::false_val())
}

fn all_(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    let aref = self_val.into_array();
    if vm.args_len() == 1 {
        if args.block.is_some() {
            eprintln!("warning: given block not used");
        }
        for v in aref.iter() {
            if !vm.eval_teq(*v, vm[0])? {
                return Ok(Value::false_val());
            };
        }
        return Ok(Value::true_val());
    }
    vm.check_args_num(0)?;

    match &args.block {
        None => {
            for v in aref.iter() {
                if !v.to_bool() {
                    return Ok(Value::false_val());
                };
            }
        }
        Some(block) => {
            for v in aref.iter() {
                if !vm.eval_block1(block, *v)?.to_bool() {
                    return Ok(Value::false_val());
                };
            }
        }
    }
    Ok(Value::true_val())
}

fn count(vm: &mut VM, mut self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_range(0, 1)?;
    if args.block.is_some() {
        return Err(RubyError::argument("Currently, block is not supported."));
    }
    let ary = self_val.expect_array("").unwrap();
    match vm.args_len() {
        0 => {
            let len = ary.len() as i64;
            Ok(Value::integer(len))
        }
        1 => {
            let other = vm[0];
            let mut count = 0;
            for elem in &**ary {
                if vm.eval_eq2(*elem, other)? {
                    count += 1;
                }
            }
            Ok(Value::integer(count))
        }
        _ => unreachable!(),
    }
}

fn inject(vm: &mut VM, mut self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let block = args.expect_block()?;
    let ary = self_val.expect_array("").unwrap();
    let mut res = vm[0];
    for elem in ary.iter() {
        res = vm.eval_block2(block, res, *elem)?;
    }
    Ok(res)
}

fn find_index(vm: &mut VM, mut self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_range(0, 1)?;
    let ary = self_val.expect_array("").unwrap();
    if vm.args_len() == 1 {
        if args.block.is_some() {
            eprintln!("Warning: given block not used.")
        };
        for (i, v) in ary.iter().enumerate() {
            if v.eq(&vm[0]) {
                return Ok(Value::integer(i as i64));
            };
        }
        return Ok(Value::nil());
    };
    let block = to_enum_str!(vm, self_val, args, "find_index");
    for (i, elem) in ary.iter().enumerate() {
        if vm.eval_block1(&block, *elem)?.to_bool() {
            return Ok(Value::integer(i as i64));
        };
    }
    Ok(Value::nil())
}

fn reject(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let ary = self_val.into_array();
    let block = to_enum_str!(vm, self_val, args, "reject");
    let mut res = vec![];
    for elem in ary.iter() {
        if !vm.eval_block1(&block, *elem)?.to_bool() {
            res.push(*elem);
        };
    }
    Ok(Value::array_from(res))
}

fn select(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let ary = self_val.into_array();
    let block = to_enum_str!(vm, self_val, args, "select");
    let mut res = vec![];
    for elem in ary.iter() {
        if vm.eval_block1(&block, *elem)?.to_bool() {
            res.push(*elem);
        };
    }
    Ok(Value::array_from(res))
}

fn find(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let ary = self_val.into_array();
    let block = to_enum_str!(vm, self_val, args, "find");
    for elem in ary.iter() {
        if vm.eval_block1(&block, *elem)?.to_bool() {
            return Ok(*elem);
        };
    }
    Ok(Value::nil())
}

fn binary_search(vm: &mut VM, ary: Array, block: &Block) -> Result<Option<usize>, RubyError> {
    if ary.len() == 0 {
        return Ok(None);
    };
    let mut i_min = 0;
    let mut i_max = ary.len() - 1;
    if vm.eval_block1(block, ary[0])?.expect_bool_nil_num()? {
        return Ok(Some(0));
    };
    if !vm.eval_block1(block, ary[i_max])?.expect_bool_nil_num()? {
        return Ok(None);
    };

    loop {
        let i_mid = i_min + (i_max - i_min) / 2;
        if i_mid == i_min {
            return Ok(Some(i_max));
        };
        if vm.eval_block1(block, ary[i_mid])?.expect_bool_nil_num()? {
            i_max = i_mid;
        } else {
            i_min = i_mid;
        };
    }
}

/// bsearch { |x| ... } -> object | nil
/// bsearch -> Enumerator
/// https://docs.ruby-lang.org/ja/latest/method/Array/i/bsearch.html
fn bsearch(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let ary = self_val.into_array();
    let block = to_enum_str!(vm, self_val, args, "bsearch");
    match binary_search(vm, ary, block)? {
        Some(i) => Ok(ary[i]),
        None => Ok(Value::nil()),
    }
}

/// bsearch_index { |x| ... } -> Integer | nil
/// bsearch_index -> Enumerator
///
///https://docs.ruby-lang.org/ja/latest/method/Array/i/bsearch_index.html
fn bsearch_index(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let ary = self_val.into_array();
    let block = to_enum_str!(vm, self_val, args, "bsearch_index");
    match binary_search(vm, ary, block)? {
        Some(i) => Ok(Value::integer(i as i64)),
        None => Ok(Value::nil()),
    }
}

fn delete(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let arg = vm[0];
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

fn flatten(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_range(0, 1)?;
    let level = if vm.args_len() == 0 {
        None
    } else {
        let i = vm[0].coerce_to_fixnum("1st arg")?;
        if i < 0 {
            None
        } else {
            Some(i as usize)
        }
    };
    let mut res = vec![];
    for v in &**self_val.into_array() {
        ary_flatten(*v, &mut res, level, self_val)?;
    }
    Ok(Value::array_from(res))
}

fn flatten_(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_range(0, 1)?;
    let level = if vm.args_len() == 0 {
        None
    } else {
        let i = vm[0].coerce_to_fixnum("1st arg")?;
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
    *self_val.into_array() = ArrayInfo::new(res);
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
                for v in &**ainfo {
                    ary_flatten(*v, res, None, origin)?;
                }
            }
            None => res.push(val),
        },
        Some(0) => res.push(val),
        Some(level) => match val.as_array() {
            Some(ainfo) => {
                flag = true;
                for v in &**ainfo {
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
        assert ["1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16", "17", "18", "19", "20", "21", "22", "23", "24", "25"],
        ["14", "4", "21", "5", "24", "17", "3", "23", "9", "12", "20", "11", "18", "10", "13", "15", "16", "8", "19", "1", "22", "7", "2", "6", "25"].sort_by {|v| v.to_i}
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
        a = []
        assert a, a.rotate!
        assert a.object_id(), a.rotate!.object_id()
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
        assert [], a.slice!(2, 0)
        assert [], a.slice!(3, 0)
        assert nil, a.slice!(4, 0)
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
        r = []
        r << r
        r << r
        assert [r], r.uniq

        assert [1,2,3,4,3,2,1,0,3.0], a
        100.times {|i|
          a = [1,2,3,4,3,2,1,0,3.0]
          if [1,2,3,3.0] != a.uniq! {|x| x % 3 }
            raise StandardError.new("assert failed in #{i}")
          end
        }
        assert nil, [1,2,3,4].uniq!
        assert [1,2,3,4], [1,2,3,4,1].uniq!
        a = []
        a << a
        a << a
        assert [a], a.uniq!
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
        assert [1, 3, 5], [1, 2, 3, 4, 5, 6].reject(&:even?)
        "#;
        assert_script(program);
    }

    #[test]
    fn select() {
        let program = r#"
        assert [1, 3, 5], [1, 2, 3, 4, 5, 6].select {|i| i % 2 != 0 }
        assert [1, 3, 5], [1, 2, 3, 4, 5, 6].select(&:odd?)
        "#;
        assert_script(program);
    }

    #[test]
    fn find() {
        let program = r#"
        assert 3, [1, 2, 3, 4, 5].find {|i| i % 3 == 0 }
        assert nil, [2, 2, 2, 2, 2].find {|i| i % 3 == 0 }
        assert 9, [2, 2, 2, 9, 2].find(&:odd?)
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
