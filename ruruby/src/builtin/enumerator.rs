use crate::coroutine::*;
use crate::*;

pub(crate) fn init(globals: &mut Globals) -> Value {
    let class = Module::class_under_object();
    globals.set_toplevel_constant("Enumerator", class);
    class.add_builtin_method_by_str(globals, "next", next);
    class.add_builtin_method_by_str(globals, "each", each);
    class.add_builtin_method_by_str(globals, "map", map);
    class.add_builtin_method_by_str(globals, "collect", map);
    class.add_builtin_method_by_str(globals, "with_index", with_index);
    class.add_builtin_method_by_str(globals, "inspect", inspect);

    class.add_builtin_class_method(globals, "new", enum_new);
    class.into()
}

// Class methods

fn enum_new(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    vm.check_args_min(1)?;
    if args.block.is_some() {
        return Err(RubyError::argument("Block is not allowed."));
    };
    let receiver = vm[0];
    let (method, new_args) = if args.len() == 1 {
        let method = IdentId::EACH;
        let new_args = Args::new0();
        (method, new_args)
    } else {
        if !vm[1].is_packed_symbol() {
            return Err(RubyError::argument("2nd arg must be Symbol."));
        };
        let method = vm[1].as_packed_symbol();
        let new_args = Args::from_slice(&vm.args()[2..]);
        (method, new_args)
    };
    let val = vm.create_enumerator(method, receiver, new_args)?;
    Ok(val)
}

pub(crate) fn enumerator_iterate(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    FiberHandle::fiber_yield(vm, args)
}

// Instance methods

fn inspect(vm: &mut VM, mut self_val: Value, _args: &Args2) -> VMResult {
    let eref = self_val.as_enumerator().unwrap();
    let (receiver, method, args) = match &eref.kind {
        FiberKind::Enum(info) => (info.receiver, info.method, &info.args),
        _ => unreachable!(),
    };

    let arg_string = {
        match args.len() {
            0 => "".to_string(),
            1 => format!(" {:?}", vm[0]),
            _ => {
                let mut s = format!(" {:?}", vm[0]);
                for i in 1..args.len() {
                    s = format!("{},{:?}", s, args[i]);
                }
                s
            }
        }
    };

    let receiver_string = vm.val_inspect(receiver)?;
    let inspect = format!(
        "#<Enumerator: {}:{:?}{}>",
        receiver_string, method, arg_string
    );
    Ok(Value::string(inspect))
}

fn next(vm: &mut VM, mut self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let eref = self_val.as_enumerator().unwrap();
    if args.block.is_some() {
        return Err(RubyError::argument("Block is not allowed."));
    };
    if eref.state == FiberState::Dead {
        return Err(RubyError::stop_iteration("Iteration reached an end."));
    };
    match eref.resume(Value::nil()) {
        Ok(val) => Ok(val),
        /*Err(err) if err.is_stop_iteration() => {
            return Err(RubyError::stop_iteration("Iteration reached an end."))
        }*/
        Err(err) => Err(err),
    }
}

fn each(vm: &mut VM, mut self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let eref = self_val.as_enumerator().unwrap();
    // A new fiber must be constructed for each method call.
    let block = match &args.block {
        None => return Ok(self_val),
        Some(block) => block,
    };
    let mut fiber = vm.dup_enum(eref);
    loop {
        let val = match fiber.resume(Value::nil()) {
            Ok(val) => val,
            Err(err) if err.is_stop_iteration() => break,
            Err(err) => return Err(err),
        };
        vm.eval_block1(block, val)?;
    }
    let mut recv = match &eref.kind {
        FiberKind::Enum(einfo) => einfo.receiver,
        _ => unreachable!(),
    };
    loop {
        recv = match recv.as_enumerator() {
            Some(eref) => match &eref.kind {
                FiberKind::Enum(einfo) => einfo.receiver,
                _ => unreachable!(),
            },
            None => break,
        };
    }
    Ok(recv)
}

fn map(vm: &mut VM, mut self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let eref = self_val.as_enumerator().unwrap();
    let mut info = vm.dup_enum(eref);
    let block = match &args.block {
        None => {
            // return Enumerator
            let id = IdentId::MAP;
            let e = vm.create_enumerator(id, self_val, args.into(vm))?;
            return Ok(e);
        }
        Some(block) => block,
    };
    let len = vm.temp_len();
    loop {
        let val = match info.resume(Value::nil()) {
            Ok(val) => val,
            Err(err) if err.is_stop_iteration() => break,
            Err(err) => return Err(err),
        };
        let res = vm.eval_block1(block, val)?;
        vm.temp_push(res);
    }
    Ok(Value::array_from(vm.temp_pop_vec(len)))
}

fn with_index(vm: &mut VM, mut self_val: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let eref = self_val.as_enumerator().unwrap();
    let mut info = vm.dup_enum(eref);
    let block = match &args.block {
        None => {
            // return Enumerator
            let id = IdentId::get_id("with_index");
            let e = vm.create_enumerator(id, self_val, args.into(vm))?;
            return Ok(e);
        }
        Some(block) => block,
    };

    let mut c = 0;
    let len = vm.temp_len();
    loop {
        let val = match info.resume(Value::nil()) {
            Ok(val) => val,
            Err(err) => {
                if err.is_stop_iteration() {
                    break;
                } else {
                    return Err(err);
                }
            }
        };
        let res = vm.eval_block2(block, val, Value::integer(c))?;
        vm.temp_push(res);
        c += 1;
    }
    Ok(Value::array_from(vm.temp_pop_vec(len)))
}

#[cfg(test)]
mod test {
    use crate::tests::*;

    #[test]
    fn enumerator_next_each() {
        let program = r###"
        e = Enumerator.new(1..3)
        assert(1, e.next)
        assert(2, e.next)
        a = []
        e.each do |x|
            a << x
        end
        assert([1,2,3], a)
        assert(3, e.next)
        assert_error { e.next }

        e = Enumerator.new([1,2,3], :each)
        assert("#<Enumerator: [1, 2, 3]:each>", e.inspect)
        assert(1, e.next)
        assert(2, e.next)
        assert(3, e.next)
        assert_error { e.next }

        "###;
        assert_script(program);
    }

    #[test]
    fn enumerator_map0() {
        let program = r#"
            assert [0, 5, 12, 21], (4..7).each.with_index.map{|x,y| x * y}
            "#;
        assert_script(program);
    }

    #[test]
    fn enumerator_map2() {
        let program = r#"
            assert [8, 10, 12, 14], (4..7).each.map{|x| x * 2}
            "#;
        assert_script(program);
    }

    #[test]
    fn enumerator_map3() {
        let program = r#"
            a = []
            assert (4..7), (4..7).each.with_index.each{|x,y| a << [x,y]}
            assert [[4, 0], [5, 1], [6, 2], [7, 3]], a
            "#;
        assert_script(program);
    }

    #[test]
    fn enumerator_with_index() {
        let program = r#"
        ans = %w(This is a Ruby.).map.with_index {|x| x }
        assert ["This", "is", "a", "Ruby."], ans
        ans = %w(This is a Ruby.).map.with_index {|x,y| [x,y] }
        assert [["This", 0], ["is", 1], ["a", 2], ["Ruby.", 3]], ans
        ans = %w(This is a Ruby.).map.with_index {|x,y,z| [x,y,z] }
        assert [["This", 0, nil], ["is", 1, nil], ["a", 2, nil], ["Ruby.", 3, nil]], ans
        "#;
        assert_script(program);
    }
}
