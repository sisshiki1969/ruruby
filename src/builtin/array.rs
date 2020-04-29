use crate::error::RubyError;
use crate::*;

pub fn init_array(globals: &mut Globals) -> Value {
    let array_id = globals.get_ident_id("Array");
    let class = ClassRef::from(array_id, globals.builtins.object);
    let obj = Value::class(globals, class);
    globals.add_builtin_instance_method(class, "[]=", array_set_elem);
    globals.add_builtin_instance_method(class, "push", array_push);
    globals.add_builtin_instance_method(class, "<<", array_push);
    globals.add_builtin_instance_method(class, "pop", array_pop);
    globals.add_builtin_instance_method(class, "shift", array_shift);
    globals.add_builtin_instance_method(class, "unshift", array_unshift);
    globals.add_builtin_instance_method(class, "length", array_length);
    globals.add_builtin_instance_method(class, "size", array_length);
    globals.add_builtin_instance_method(class, "empty?", array_empty);
    globals.add_builtin_instance_method(class, "*", array_mul);
    globals.add_builtin_instance_method(class, "+", array_add);
    globals.add_builtin_instance_method(class, "concat", array_concat);
    globals.add_builtin_instance_method(class, "-", array_sub);
    globals.add_builtin_instance_method(class, "map", array_map);
    globals.add_builtin_instance_method(class, "flat_map", array_flat_map);
    globals.add_builtin_instance_method(class, "each", array_each);
    globals.add_builtin_instance_method(class, "include?", array_include);
    globals.add_builtin_instance_method(class, "reverse", array_reverse);
    globals.add_builtin_instance_method(class, "reverse!", array_reverse_);
    globals.add_builtin_instance_method(class, "rotate!", array_rotate_);
    globals.add_builtin_instance_method(class, "transpose", array_transpose);
    globals.add_builtin_instance_method(class, "min", array_min);
    globals.add_builtin_instance_method(class, "fill", array_fill);
    globals.add_builtin_instance_method(class, "clear", array_clear);
    globals.add_builtin_instance_method(class, "uniq!", array_uniq_);
    globals.add_builtin_instance_method(class, "slice!", array_slice_);
    globals.add_builtin_instance_method(class, "max", array_max);
    globals.add_builtin_instance_method(class, "first", array_first);
    globals.add_builtin_instance_method(class, "last", array_last);
    globals.add_builtin_instance_method(class, "dup", array_dup);
    globals.add_builtin_instance_method(class, "clone", array_dup);
    globals.add_builtin_instance_method(class, "pack", array_pack);
    globals.add_builtin_instance_method(class, "join", array_join);
    globals.add_builtin_instance_method(class, "drop", array_drop);
    globals.add_builtin_instance_method(class, "zip", array_zip);
    globals.add_builtin_class_method(obj, "new", array_new);
    obj
}

// Class methods

fn array_new(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 2)?;
    let array_vec = match args.len() {
        0 => vec![],
        1 => match args[0].unpack() {
            RV::FixNum(num) if num >= 0 => vec![Value::nil(); num as usize],
            RV::Object(oref) => match oref.kind {
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
    let array = Value::array_from(&vm.globals, array_vec);
    Ok(array)
}

// Instance methods

fn array_set_elem(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let mut aref = vm.expect_array(self_val, "Receiver")?;
    let val = aref.set_elem(vm, args)?;
    Ok(val)
}

fn array_push(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let mut aref = vm.expect_array(self_val, "Receiver")?;
    for arg in args.get_slice(0, args.len()) {
        aref.elements.push(*arg);
    }
    Ok(self_val)
}

fn array_pop(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let mut aref = vm.expect_array(self_val, "Receiver")?;
    let res = aref.elements.pop().unwrap_or_default();
    Ok(res)
}

fn array_shift(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let mut aref = vm.expect_array(self_val, "Receiver")?;
    let new = aref.elements.split_off(1);
    let res = aref.elements[0];
    aref.elements = new;
    Ok(res)
}

fn array_unshift(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    //vm.check_args_num(args.len(), 0, 0)?;
    if args.len() == 0 {
        return Ok(self_val);
    }
    let mut new = args.get_slice(0, args.len()).to_owned();
    let mut aref = vm.expect_array(self_val, "Receiver")?;
    new.append(&mut aref.elements);
    aref.elements = new;
    Ok(self_val)
}

fn array_length(vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    let aref = vm.expect_array(self_val, "Receiver")?;
    let res = Value::fixnum(aref.elements.len() as i64);
    Ok(res)
}

fn array_empty(vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    let aref = vm.expect_array(self_val, "Receiver")?;
    let res = Value::bool(aref.elements.is_empty());
    Ok(res)
}

fn array_mul(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let aref = vm.expect_array(self_val, "Receiver")?;
    let v = match args[0].unpack() {
        RV::FixNum(num) => match num {
            i if i < 0 => return Err(vm.error_argument("Negative argument.")),
            0 => vec![],
            1 => aref.elements.clone(),
            _ => {
                let len = aref.elements.len();
                let src = &aref.elements[0..len];
                let mut v = vec![Value::nil(); len * num as usize];
                //println!("dest:{:?} src:{:?}", aref.elements, src);
                let mut i = 0;
                for _ in 0..num {
                    v[i..i + len].copy_from_slice(src);
                    i += len;
                }
                v
            }
        },
        _ => return Err(vm.error_nomethod(" ")),
    };
    let res = Value::array_from(&vm.globals, v);
    Ok(res)
}

fn array_add(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let mut lhs = vm.expect_array(self_val, "Receiver")?.elements.clone();
    let mut rhs = vm.expect_array(args[0], "Argument")?.elements.clone();
    lhs.append(&mut rhs);
    Ok(Value::array_from(&vm.globals, lhs))
}

fn array_concat(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let mut lhs = vm.expect_array(self_val, "Receiver")?;
    let mut rhs = vm.expect_array(args[0], "Argument")?.elements.clone();
    lhs.elements.append(&mut rhs);
    Ok(self_val)
}

fn array_sub(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let lhs_v = &vm.expect_array(self_val, "Receiver")?.elements;
    let rhs_v = &vm.expect_array(args[0], "Argument")?.elements;
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
    Ok(Value::array_from(&vm.globals, v))
}

fn array_map(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let aref = vm.expect_array(self_val, "Receiver")?;
    let method = match args.block {
        Some(method) => method,
        None => {
            let id = vm.globals.get_ident_id("map");
            let val = Value::enumerator(&vm.globals, id, self_val, args.clone());
            return Ok(val);
        }
    };

    let mut res = vec![];
    let mut args = Args::new1(None, Value::nil());

    for elem in &aref.elements {
        args[0] = *elem;
        let val = vm.eval_block(method, &args)?;
        res.push(val);
    }

    let res = Value::array_from(&vm.globals, res);
    Ok(res)
}

fn array_flat_map(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let aref = vm.expect_array(self_val, "Receiver")?;
    let method = vm.expect_block(args.block)?;
    let mut res = vec![];
    let param_num = vm.get_iseq(method)?.req_params;
    let mut arg = Args::new(param_num);
    for elem in &aref.elements {
        if param_num == 0 {
        } else if param_num == 1 {
            arg[0] = *elem;
        } else {
            match elem.as_array() {
                Some(ary) => {
                    for i in 0..param_num {
                        arg[i] = ary.elements[i];
                    }
                }
                None => arg[0] = *elem,
            }
        }

        let ary = vm.eval_block(method, &arg)?;
        match ary.as_array() {
            Some(mut ary) => {
                res.append(&mut ary.elements);
            }
            None => res.push(ary),
        }
    }
    let res = Value::array_from(&vm.globals, res);
    Ok(res)
}

fn array_each(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let aref = vm.expect_array(self_val, "Receiver")?;
    //let method = vm.expect_block(args.block)?;

    let method = match args.block {
        Some(method) => method,
        None => {
            let id = vm.globals.get_ident_id("each");
            let val = Value::enumerator(&vm.globals, id, self_val, args.clone());
            return Ok(val);
        }
    };

    let mut arg = Args::new(vm.get_iseq(method)?.req_params);
    for i in &aref.elements {
        match i.as_array() {
            Some(aref) if arg.len() != 1 => {
                for j in 0..arg.len() {
                    arg[j] = if j < aref.elements.len() {
                        aref.elements[j]
                    } else {
                        Value::nil()
                    };
                }
            }
            _ => {
                arg[0] = *i;
                for j in 1..arg.len() {
                    arg[j] = Value::nil();
                }
            }
        };

        vm.eval_block(method, &arg)?;
    }
    Ok(self_val)
}

fn array_include(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let target = args[0];
    let aref = vm.expect_array(self_val, "Receiver")?;
    for item in aref.elements.iter() {
        if vm.eval_eq(*item, target)? {
            return Ok(Value::true_val());
        }
    }
    Ok(Value::false_val())
}

fn array_reverse(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let aref = vm.expect_array(self_val, "Receiver")?;
    let mut res = aref.elements.clone();
    res.reverse();
    Ok(Value::array_from(&vm.globals, res))
}

fn array_reverse_(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let mut aref = vm.expect_array(self_val, "Receiver")?;
    aref.elements.reverse();
    Ok(self_val)
}

fn array_rotate_(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 1)?;
    let i = if args.len() == 0 {
        1
    } else {
        match args[0].as_fixnum() {
            Some(i) => i,
            None => return Err(vm.error_argument("Must be Integer.")),
        }
    };
    let mut aref = vm.expect_array(self_val, "Receiver")?;
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

fn array_transpose(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let aref = vm.expect_array(self_val, "Receiver")?;
    if aref.elements.len() == 0 {
        return Ok(Value::array_from(&vm.globals, vec![]));
    }
    let mut vec = vec![];
    for elem in &aref.elements {
        let ary = elem
            .as_array()
            .ok_or(vm.error_nomethod("Each element of receiver must be an array."))?
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
                return Err(vm.error_index("Element size differs."));
            }
            temp.push(v[i]);
        }
        let ary = Value::array_from(&vm.globals, temp);
        trans.push(ary);
    }
    //aref.elements.reverse();
    let res = Value::array_from(&vm.globals, trans);
    Ok(res)
}

fn array_min(vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    fn to_float(vm: &VM, val: Value) -> Result<f64, RubyError> {
        if val.is_packed_fixnum() {
            Ok(val.as_packed_fixnum() as f64)
        } else if val.is_packed_num() {
            Ok(val.as_packed_flonum())
        } else {
            Err(vm.error_type("Currently, each element must be Numeric."))
        }
    }

    let aref = vm.expect_array(self_val, "Receiver")?;
    if aref.elements.len() == 0 {
        return Ok(Value::nil());
    }
    let mut min_obj = aref.elements[0];
    let mut min = to_float(vm, min_obj)?;
    for elem in &aref.elements {
        let elem_f = to_float(vm, *elem)?;
        if elem_f < min {
            min_obj = *elem;
            min = elem_f;
        }
    }

    return Ok(min_obj);
}

fn array_fill(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let mut aref = vm.expect_array(self_val, "Receiver")?;
    for elem in &mut aref.elements {
        *elem = args[0];
    }
    Ok(self_val)
}

fn array_clear(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let mut aref = vm.expect_array(self_val, "Receiver")?;
    aref.elements.clear();
    Ok(self_val)
}

fn array_uniq_(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let mut aref = vm.expect_array(self_val, "Receiver")?;
    let mut set = std::collections::HashSet::new();
    match args.block {
        None => {
            aref.elements.retain(|x| set.insert(*x));
            Ok(self_val)
        }
        Some(block) => {
            aref.elements.retain(|x| {
                let block_args = Args::new1(None, *x);
                let res = vm.eval_block(block, &block_args).unwrap();
                set.insert(res)
            });
            Ok(self_val)
        }
    }
}

fn array_slice_(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 2, 2)?;
    let start = args[0].expect_fixnum(vm, "Currently, first arg must be Integer.")?;
    if start < 0 {
        return Err(vm.error_argument("First arg must be positive value."));
    };
    let len = args[1].expect_fixnum(vm, "Currently, second arg must be Integer")?;
    if len < 0 {
        return Err(vm.error_argument("Second arg must be positive value."));
    };
    let start = start as usize;
    let len = len as usize;
    let mut aref = vm.expect_array(self_val, "Receiver")?;
    let new = aref.elements.drain(start..start + len).collect();
    Ok(Value::array_from(&vm.globals, new))
}

fn array_max(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let aref = vm.expect_array(self_val, "Receiver")?;
    if aref.elements.len() == 0 {
        return Ok(Value::nil());
    }
    let mut max = aref.elements[0];
    for elem in &aref.elements {
        if vm.eval_gt(max, *elem)? == Value::true_val() {
            max = *elem;
        };
    }
    Ok(max)
}

fn array_first(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let aref = vm.expect_array(self_val, "Receiver")?;
    if aref.elements.len() == 0 {
        Ok(Value::nil())
    } else {
        Ok(aref.elements[0])
    }
}

fn array_last(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let aref = vm.expect_array(self_val, "Receiver")?;
    if aref.elements.len() == 0 {
        Ok(Value::nil())
    } else {
        Ok(*aref.elements.last().unwrap())
    }
}

fn array_dup(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 0)?;
    let aref = vm.expect_array(self_val, "Receiver")?;
    Ok(Value::array_from(&vm.globals, aref.elements.clone()))
}

fn array_pack(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 1)?;
    let aref = vm.expect_array(self_val, "Receiver")?;
    let mut v = vec![];
    for elem in &aref.elements {
        let i = match elem.as_fixnum() {
            Some(i) => i as i8 as u8,
            None => return Err(vm.error_argument("Must be Array of Integer.")),
        };
        v.push(i);
    }
    Ok(Value::bytes(&vm.globals, v))
}

fn array_join(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0, 1)?;
    let sep = if args.len() == 0 {
        ""
    } else {
        match args[0].as_string() {
            Some(s) => s,
            None => return Err(vm.error_argument("Seperator must be String.")),
        }
    };
    let aref = vm.expect_array(self_val, "Receiver")?;
    let mut res = "".to_string();
    for elem in &aref.elements {
        let s = vm.val_to_s(*elem);
        if res.is_empty() {
            res = s.to_owned();
        } else {
            res = res + sep + s.as_str();
        }
    }
    Ok(Value::string(&vm.globals, res))
}

fn array_drop(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1, 1)?;
    let aref = vm.expect_array(self_val, "Receiver")?;
    let num = args[0].expect_fixnum(vm, "An argument must be Integer.")? as usize;
    let ary = &aref.elements[num..aref.elements.len()];
    Ok(Value::array_from(&vm.globals, ary.to_vec()))
}

fn array_zip(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    //vm.check_args_num(args.len(), 1, 1)?;
    let self_ary = vm.expect_array(self_val, "Receiver")?;
    let mut args_ary = vec![];
    for a in args.iter() {
        args_ary.push(vm.expect_array(*a, "Args")?.elements.clone());
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
        let zip = Value::array_from(&vm.globals, vec);
        ary.push(zip);
    }
    Ok(Value::array_from(&vm.globals, ary))
}

#[cfg(test)]
mod tests {
    use crate::test::*;

    #[test]
    fn array_zip() {
        let program = r#"
        assert [[1,4,7],[2,5,8],[3,6,9]], [1,2,3].zip([4,5,6],[7,8,9])
        assert [[1,:a,:A],[2,:b,:B]], [1,2].zip([:a,:b,:c],[:A,:B,:C,:D])
        assert [[1,:a,:A],[2,:b,:B],[3,:c,:C],[4,nil,:D],[5,nil,nil]], [1,2,3,4,5].zip([:a,:b,:c],[:A,:B,:C,:D])
        "#;
        assert_script(program);
    }
}
