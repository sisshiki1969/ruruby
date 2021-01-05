use crate::error::RubyError;
use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let mut class = Value::class_under(globals.builtins.object);
    class.add_builtin_method_by_str("inspect", inspect);
    class.add_builtin_method_by_str("to_s", inspect);
    class.add_builtin_method_by_str("to_a", toa);
    class.add_builtin_method_by_str("length", length);
    class.add_builtin_method_by_str("size", length);
    class.add_builtin_method_by_str("empty?", empty);
    class.add_builtin_method_by_str("[]", get_elem);
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
    class.add_builtin_method_by_str("flat_map", flat_map);
    class.add_builtin_method_by_str("each", each);
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
    class
}

// Class methods

fn array_new(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 2)?;
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
                _ => return Err(RubyError::typeerr("1st arg must be Integer or Array.")),
            },
            _ => return Err(RubyError::typeerr("1st arg must be Integer or Array.")),
        },
        2 => {
            let num = args[0].expect_integer("1st arg")?;
            if num < 0 {
                return Err(RubyError::argument("Negative array size."));
            };
            vec![args[1]; num as usize]
        }
        _ => unreachable!(),
    };
    let array = Value::array_from_with_class(array_vec, self_val);
    if let Some(method) = vm.globals.find_method(self_val, IdentId::INITIALIZE) {
        vm.eval_send(method, array, args)?;
    };
    Ok(array)
}

fn array_allocate(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let array = Value::array_from_with_class(vec![], self_val);
    Ok(array)
}

fn array_elem(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let array = Value::array_from_with_class(args.to_vec(), self_val);
    Ok(array)
}

// Instance methods

fn inspect(vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    let aref = self_val.as_array().unwrap();
    let s = aref.to_s(vm)?;
    Ok(Value::string(s))
}

fn toa(vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    let array = vm.globals.builtins.array;
    if self_val.get_class().id() == array.id() {
        return Ok(self_val);
    };
    let mut new_val = self_val.dup();
    new_val.set_class(array);
    Ok(new_val)
}

fn get_elem(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    let aref = self_val.as_mut_array().unwrap();
    let val = aref.get_elem(args)?;
    Ok(val)
}

fn set_elem(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    let aref = self_val.as_mut_array().unwrap();
    let val = aref.set_elem(args)?;
    Ok(val)
}

fn cmp(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = &self_val.as_array().unwrap().elements;
    let rhs = match args[0].as_array() {
        Some(aref) => &aref.elements,
        None => return Ok(Value::nil()),
    };
    if lhs.len() >= rhs.len() {
        for (i, rhs_v) in rhs.iter().enumerate() {
            match vm.eval_compare(*rhs_v, lhs[i])?.as_integer() {
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
            match vm.eval_compare(rhs[i], *lhs_v)?.as_integer() {
                Some(0) => {}
                Some(ord) => return Ok(Value::integer(ord)),
                None => return Ok(Value::nil()),
            }
        }
        Ok(Value::integer(-1))
    }
}

fn push(_vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    let aref = self_val.as_mut_array().unwrap();
    for arg in args.iter() {
        aref.elements.push(*arg);
    }
    Ok(self_val)
}

fn pop(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.as_mut_array().unwrap();
    let res = aref.elements.pop().unwrap_or_default();
    Ok(res)
}

fn shift(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let mut array_flag = false;
    let num = if args.len() == 0 {
        0
    } else {
        let i = args[0].expect_integer("1st arg")?;
        if i < 0 {
            return Err(RubyError::argument("Negative array size."));
        }
        array_flag = true;
        i as usize
    };

    let mut aref = self_val.as_mut_array().unwrap();
    if array_flag {
        if aref.elements.len() < num {
            return Ok(Value::array_from(vec![]));
        }
        let new = aref.elements.split_off(num);
        let res = aref.elements[0..num].to_vec();
        aref.elements = new;
        Ok(Value::array_from(res))
    } else {
        if aref.elements.len() == 0 {
            return Ok(Value::nil());
        }
        let new = aref.elements.split_off(1);
        let res = aref.elements[0];
        aref.elements = new;
        Ok(res)
    }
}

fn unshift(_vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    if args.len() == 0 {
        return Ok(self_val);
    }
    let mut new = args[0..args.len()].to_owned();
    let mut aref = self_val.as_mut_array().unwrap();
    new.append(&mut aref.elements);
    aref.elements = new;
    Ok(self_val)
}

fn length(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    //eprintln!("{:?} {}", self_val, self_val.get_class_name());
    let aref = self_val.as_array().unwrap();
    let res = Value::integer(aref.elements.len() as i64);
    Ok(res)
}

fn empty(_vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    let aref = self_val.as_array().unwrap();
    let res = Value::bool(aref.elements.is_empty());
    Ok(res)
}

fn mul(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let aref = self_val.as_array().unwrap();
    if let Some(num) = args[0].as_integer() {
        let v = match num {
            i if i < 0 => return Err(RubyError::argument("Negative argument.")),
            0 => vec![],
            1 => aref.elements.clone(),
            _ => {
                let len = aref.elements.len();
                let src = &aref.elements[0..len];
                let mut v = vec![Value::nil(); len * num as usize];
                let mut i = 0;
                for _ in 0..num {
                    for src_val in src[0..len].iter() {
                        v[i] = src_val.dup();
                        i += 1;
                    }
                }
                v
            }
        };
        let res = Value::array_from_with_class(v, self_val.get_class());
        Ok(res)
    } else if let Some(s) = args[0].as_string() {
        match aref.elements.len() {
            0 => return Ok(Value::string("")),
            1 => {
                let res = aref.elements[0].val_to_s(vm)?;
                return Ok(Value::string(res));
            }
            _ => {
                let mut res = aref.elements[0].val_to_s(vm)?.to_string();
                for i in 1..aref.elements.len() {
                    let elem = aref.elements[i].val_to_s(vm)?;
                    res = res + s + &elem;
                }
                return Ok(Value::string(res));
            }
        };
    } else {
        return Err(RubyError::typeerr(format!(
            "No implicit conversion from {:?} to Integer.",
            args[0]
        )));
    }
}

fn add(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let mut lhs = self_val.expect_array("Receiver")?.elements.clone();
    let mut arg0 = args[0];
    let mut rhs = arg0.expect_array("Argument")?.elements.clone();
    lhs.append(&mut rhs);
    Ok(Value::array_from(lhs))
}

fn concat(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let lhs = self_val.as_mut_array().unwrap();
    let mut arg0 = args[0];
    let mut rhs = arg0.expect_array("Argument")?.elements.clone();
    lhs.elements.append(&mut rhs);
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
            if vm.eval_eq(*lhs, *rhs)? {
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
            Block::None => {
                let val = $vm.create_enumerator($id, $self_val, $args.clone())?;
                return Ok(val);
            }
            block => block,
        }
    };
}

macro_rules! to_enum_str {
    ($vm:ident, $self_val:ident, $args:ident, $id:expr) => {
        match &$args.block {
            Block::None => {
                let val = $vm.create_enumerator(IdentId::get_id($id), $self_val, $args.clone())?;
                return Ok(val);
            }
            block => block,
        }
    };
}

fn map(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.as_array().unwrap();
    let block = to_enum_id!(vm, self_val, args, IdentId::MAP);
    let mut args = Args::new(1);

    let mut res = vec![];
    for elem in &aref.elements {
        args[0] = *elem;
        let val = vm.eval_block(block, &args)?;
        vm.temp_push(val);
        res.push(val);
    }

    let res = Value::array_from(res);
    Ok(res)
}

fn flat_map(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let block = args.expect_block()?;
    let param_num = block.to_iseq().params.req;
    let mut arg = Args::new(param_num);

    let aref = self_val.as_array().unwrap();
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

        let mut ary = vm.eval_block(&block, &arg)?;
        vm.temp_push(ary);
        match ary.as_mut_array() {
            Some(ary) => {
                res.append(&mut ary.elements);
            }
            None => res.push(ary),
        }
    }
    let res = Value::array_from(res);
    Ok(res)
}

fn each(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let method = to_enum_id!(vm, self_val, args, IdentId::EACH);
    let aref = self_val.as_array().unwrap();
    let mut arg = Args::new(1);
    for elem in &aref.elements {
        arg[0] = *elem;
        vm.eval_block(method, &arg)?;
    }
    Ok(self_val)
}

fn each_with_index(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let method = to_enum_str!(vm, self_val, args, "each_with_index");
    let aref = self_val.as_array().unwrap();
    let mut arg = Args::new(2);
    for (i, v) in aref.elements.iter().enumerate() {
        arg[0] = *v;
        arg[1] = Value::integer(i as i64);
        vm.eval_block(method, &arg)?;
    }
    Ok(self_val)
}

fn partition(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let method = to_enum_str!(vm, self_val, args, "partition");
    let aref = self_val.as_array().unwrap();
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
    let ary = vec![Value::array_from(res_true), Value::array_from(res_false)];
    Ok(Value::array_from(ary))
}

fn include(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let target = args[0];
    let aref = self_val.as_array().unwrap();
    for item in aref.elements.iter() {
        if vm.eval_eq(*item, target)? {
            return Ok(Value::true_val());
        }
    }
    Ok(Value::false_val())
}

fn reverse(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.as_array().unwrap();
    let mut res = aref.elements.clone();
    res.reverse();
    Ok(Value::array_from(res))
}

fn reverse_(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.as_mut_array().unwrap();
    aref.elements.reverse();
    Ok(self_val)
}

fn rotate_(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let i = if args.len() == 0 {
        1
    } else {
        match args[0].as_integer() {
            Some(i) => i,
            None => return Err(RubyError::argument("Must be Integer.")),
        }
    };
    let mut aref = self_val.as_mut_array().unwrap();
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

fn compact(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.as_mut_array().unwrap();
    let ary = aref
        .elements
        .iter()
        .filter(|x| !x.is_nil())
        .cloned()
        .collect();
    Ok(Value::array_from(ary))
}

fn compact_(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.as_mut_array().unwrap();
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

fn transpose(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.as_mut_array().unwrap();
    if aref.elements.len() == 0 {
        return Ok(Value::array_from(vec![]));
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
        let ary = Value::array_from(temp);
        trans.push(ary);
    }
    //aref.elements.reverse();
    let res = Value::array_from(trans);
    Ok(res)
}

fn min(_: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    fn to_float(val: Value) -> Result<f64, RubyError> {
        if val.is_packed_fixnum() {
            Ok(val.as_packed_fixnum() as f64)
        } else if val.is_packed_num() {
            Ok(val.as_packed_flonum())
        } else {
            Err(RubyError::typeerr(
                "Currently, each element must be Numeric.",
            ))
        }
    }

    let aref = self_val.as_array().unwrap();
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
    let aref = self_val.as_array().unwrap();
    if aref.elements.len() == 0 {
        return Ok(Value::nil());
    }
    let mut max = aref.elements[0];
    for elem in &aref.elements {
        if vm.eval_gt(max, *elem)? {
            max = *elem;
        };
    }
    Ok(max)
}

fn fill(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let aref = self_val.as_mut_array().unwrap();
    for elem in &mut aref.elements {
        *elem = args[0];
    }
    Ok(self_val)
}

fn clear(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.as_mut_array().unwrap();
    aref.elements.clear();
    Ok(self_val)
}

fn uniq_(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;

    let mut set = std::collections::HashSet::new();
    match &args.block {
        Block::None => {
            let aref = self_val.as_mut_array().unwrap();
            aref.elements.retain(|x| set.insert(HashKey(*x)));
            Ok(self_val)
        }
        block => {
            let aref = self_val.as_mut_array().unwrap();
            let mut block_args = Args::new(1);
            aref.elements.retain(|x| {
                block_args[0] = *x;
                let res = vm.eval_block(block, &block_args).unwrap();
                vm.temp_push(res);
                set.insert(HashKey(res))
            });
            Ok(self_val)
        }
    }
}

fn slice_(_: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(2)?;
    let start = args[0].expect_integer("Currently, first arg must be Integer.")?;
    if start < 0 {
        return Err(RubyError::argument("First arg must be positive value."));
    };
    let len = args[1].expect_integer("Currently, second arg must be Integer")?;
    if len < 0 {
        return Err(RubyError::argument("Second arg must be positive value."));
    };
    let start = start as usize;
    let len = len as usize;
    let aref = self_val.as_mut_array().unwrap();
    let new = aref.elements.drain(start..start + len).collect();
    Ok(Value::array_from(new))
}

fn first(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.as_array().unwrap();
    if aref.elements.len() == 0 {
        Ok(Value::nil())
    } else {
        Ok(aref.elements[0])
    }
}

fn last(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.as_array().unwrap();
    if aref.elements.len() == 0 {
        Ok(Value::nil())
    } else {
        Ok(*aref.elements.last().unwrap())
    }
}

fn dup(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.as_array().unwrap();
    Ok(Value::array_from(aref.elements.clone()))
}

fn pack(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let aref = self_val.as_array().unwrap();
    let mut v = vec![];
    for elem in &aref.elements {
        let i = match elem.as_integer() {
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
    let aref = self_val.as_array().unwrap();
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
    let aref = self_val.as_array().unwrap();
    let num = args[0].expect_integer("An argument must be Integer.")? as usize;
    if num >= aref.len() {
        return Err(RubyError::argument(format!("An argument too big. {}", num)));
    };
    let ary = &aref.elements[num..];
    Ok(Value::array_from(ary.to_vec()))
}

fn zip(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let self_ary = self_val.as_array().unwrap();
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
        let zip = Value::array_from(vec);
        ary.push(zip);
    }
    match &args.block {
        Block::None => Ok(Value::array_from(ary)),
        block => {
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
    let aref = self_val.as_array().unwrap();
    let ary = match &args.block {
        Block::None => aref
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

fn sort(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    //use std::cmp::Ordering;
    args.check_args_num(0)?;
    let mut ary = self_val.expect_array("Receiver")?.elements.clone();
    match &args.block {
        Block::None => {
            vm.sort_array(&mut ary)?;
        }
        _ => return Err(RubyError::argument("Currently, can not use block.")),
    };
    Ok(Value::array_from(ary))
}

use fxhash::FxHashSet;
fn uniq(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let aref = self_val.as_array().unwrap();
    let mut h: FxHashSet<HashKey> = FxHashSet::default();
    let mut v = vec![];
    match &args.block {
        Block::None => {
            for elem in &aref.elements {
                if h.insert(HashKey(*elem)) {
                    v.push(*elem);
                };
            }
        }
        _ => return Err(RubyError::argument("Currently, can not use block.")),
    };
    Ok(Value::array_from(v))
}

fn any_(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let aref = self_val.as_array().unwrap();
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
        Block::None => {
            for v in aref.elements.iter() {
                if v.to_bool() {
                    return Ok(Value::true_val());
                };
            }
        }
        method => {
            let mut args = Args::new(1);
            for v in aref.elements.iter() {
                args[0] = *v;
                if vm.eval_block(method, &args)?.to_bool() {
                    return Ok(Value::true_val());
                };
            }
        }
    }
    Ok(Value::false_val())
}

fn all_(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let aref = self_val.as_array().unwrap();
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
        Block::None => {
            for v in aref.elements.iter() {
                if !v.to_bool() {
                    return Ok(Value::false_val());
                };
            }
        }
        method => {
            let mut args = Args::new(1);
            for v in aref.elements.iter() {
                args[0] = *v;
                if !vm.eval_block(method, &args)?.to_bool() {
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
                if vm.eval_eq(*elem, other)? {
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
    let block = match &args.block {
        Block::None => return Err(RubyError::argument("Currently, block is neccessory.")),
        block => block,
    };
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
            if *v == args[0] {
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

fn reject(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let ary = self_val.as_mut_array().unwrap();
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

fn select(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let ary = self_val.as_mut_array().unwrap();
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

fn find(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let ary = self_val.as_mut_array().unwrap();
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

fn binary_search(
    vm: &mut VM,
    ary: &mut ArrayInfo,
    block: &Block,
) -> Result<Option<usize>, RubyError> {
    if ary.len() == 0 {
        return Ok(None);
    };
    let mut args = Args::new(1);
    let mut i_min = 0;
    let mut i_max = ary.len() - 1;
    args[0] = ary.elements[0];
    if vm.eval_block(block, &args)?.to_bool() {
        return Ok(Some(0));
    };
    args[0] = ary.elements[i_max];
    if !vm.eval_block(block, &args)?.to_bool() {
        return Ok(None);
    };

    loop {
        let i_mid = i_min + (i_max - i_min) / 2;
        if i_mid == i_min {
            return Ok(Some(i_max));
        };
        args[0] = ary.elements[i_mid];
        if vm.eval_block(block, &args)?.to_bool() {
            i_max = i_mid;
        } else {
            i_min = i_mid;
        };
    }
}

fn bsearch(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let ary = self_val.as_mut_array().unwrap();
    let block = to_enum_str!(vm, self_val, args, "bsearch");
    match binary_search(vm, ary, block)? {
        Some(i) => Ok(ary.elements[i]),
        None => Ok(Value::nil()),
    }
}

fn bsearch_index(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let ary = self_val.as_mut_array().unwrap();
    let block = to_enum_str!(vm, self_val, args, "bsearch_index");
    match binary_search(vm, ary, block)? {
        Some(i) => Ok(Value::integer(i as i64)),
        None => Ok(Value::nil()),
    }
}

fn delete(_vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let arg = args[0];
    args.expect_no_block()?;
    let ary = &mut self_val.as_mut_array().unwrap().elements;
    let mut removed = None;
    ary.retain(|x| {
        if x.eq(&arg) {
            removed = Some(*x);
            false
        } else {
            true
        }
    });
    Ok(removed.unwrap_or(Value::nil()))
}

fn flatten(_vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let level = if args.len() == 0 {
        None
    } else {
        let i = args[0].expect_integer("1st arg")?;
        if i < 0 {
            None
        } else {
            Some(i as usize)
        }
    };
    let mut res = vec![];
    for v in &self_val.as_array().unwrap().elements {
        ary_flatten(*v, &mut res, level, self_val)?;
    }
    Ok(Value::array_from(res))
}

fn flatten_(_vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let level = if args.len() == 0 {
        None
    } else {
        let i = args[0].expect_integer("1st arg")?;
        if i < 0 {
            None
        } else {
            Some(i as usize)
        }
    };
    let mut res = vec![];
    let mut flag = false;
    for v in &self_val.as_array().unwrap().elements {
        flag |= ary_flatten(*v, &mut res, level, self_val)?;
    }
    self_val.as_mut_array().unwrap().elements = res;
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
    use crate::test::*;

    #[test]
    fn array() {
        let program = "
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
    ";
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
        assert([-3,2,6], [6,2,-3].sort)
        assert([-3,2.34,6.3], [6.3,2.34,-3].sort)
        assert_error {[1,2.5,nil].sort}
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
    fn array_each() {
        let program = "
        a = [1,2,3]
        b = 0
        assert([1,2,3], a.each {|x| b+=x })
        assert(6, b)

        a = [1,2,3]
        b = []
        assert([1,2,3], a.each_with_index {|x, i| b << [x,i] })
        assert([[1, 0], [2, 1], [3, 2]], b)
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

        assert [1,2,3,3.0], a.uniq! {|x| x % 3 }
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
