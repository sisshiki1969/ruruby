use crate::*;

pub fn init(_globals: &mut Globals) -> Value {
    let id = IdentId::get_id("Enumerator");
    let mut class = ClassRef::from(id, BuiltinClass::object());
    class.add_builtin_method_by_str("next", next);
    class.add_builtin_method_by_str("each", each);
    class.add_builtin_method_by_str("map", map);
    class.add_builtin_method_by_str("collect", map);
    class.add_builtin_method_by_str("with_index", with_index);
    class.add_builtin_method_by_str("inspect", inspect);
    let mut class = Value::class(class);
    class.add_builtin_class_method("new", enum_new);
    class
}

// Class methods

fn enum_new(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_min(args.len(), 1)?;
    if args.block.is_some() {
        return Err(vm.error_argument("Block is not allowed."));
    };
    let receiver = args[0];
    let (method, new_args) = if args.len() == 1 {
        let method = IdentId::get_id("each");
        let mut new_args = Args::new0();
        new_args.block = Some(*METHODREF_ENUM);
        (method, new_args)
    } else {
        if !args[1].is_packed_symbol() {
            return Err(vm.error_argument("2nd arg must be Symbol."));
        };
        let method = args[1].as_packed_symbol();
        let mut new_args = Args::new(args.len() - 2);
        for i in 0..args.len() - 2 {
            new_args[i] = args[i + 2];
        }
        new_args.block = Some(*METHODREF_ENUM);
        (method, new_args)
    };
    let val = vm.create_enumerator(method, receiver, new_args)?;
    Ok(val)
}

pub fn enumerator_iterate(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.fiber_yield(args)
}

// Instance methods

fn inspect(vm: &mut VM, mut self_val: Value, _args: &Args) -> VMResult {
    let eref = self_val.as_enumerator().unwrap();
    let (receiver, method, args) = match &eref.kind {
        FiberKind::Enum(receiver, method, args) => (receiver, method, args),
        _ => unreachable!(),
    };

    let arg_string = {
        match args.len() {
            0 => "".to_string(),
            1 => format!(" {:?}", args[0]),
            _ => {
                let mut s = format!(" {:?}", args[0]);
                for i in 1..args.len() {
                    s = format!("{},{:?}", s, args[i]);
                }
                s
            }
        }
    };

    let receiver_string = vm.val_inspect(*receiver);
    let inspect = format!(
        "#<Enumerator: {}:{:?}{}>",
        receiver_string, method, arg_string
    );
    Ok(Value::string(inspect))
}

fn next(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let eref = self_val.as_enumerator().unwrap();
    if args.block.is_some() {
        return Err(vm.error_argument("Block is not allowed."));
    };
    if eref.vm.is_dead() {
        return Err(vm.error_stop_iteration("Iteration reached an end."));
    };
    match eref.resume(vm) {
        Ok(val) => Ok(val),
        Err(err) if err.is_stop_iteration() => {
            return Err(vm.error_stop_iteration("Iteration reached an end."))
        }
        Err(err) => Err(err),
    }
}

fn each(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let eref = self_val.as_enumerator().unwrap();
    // A new fiber must be constructed for each method call.
    let mut info = vm.dup_enum(eref);
    let block = match args.block {
        Some(method) => method,
        None => {
            return Ok(self_val);
        }
    };
    let mut args = Args::new(1);
    loop {
        let val = match info.resume(vm) {
            Ok(val) => val,
            Err(err) if err.is_stop_iteration() => break,
            Err(err) => return Err(err),
        };
        args[0] = val;
        vm.eval_block(block, &args)?;
    }

    info.free();
    match info.kind {
        FiberKind::Enum(receiver, _, _) => Ok(receiver),
        _ => unreachable!(),
    }
}

fn map(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let eref = self_val.as_enumerator().unwrap();
    let mut info = vm.dup_enum(eref);
    let block = match args.block {
        Some(method) => method,
        None => {
            // return Enumerator
            let id = IdentId::get_id("map");
            let e = vm.create_enumerator(id, self_val, args.clone())?;
            return Ok(e);
        }
    };
    let mut args = Args::new(1);
    let mut ary = vec![];
    loop {
        let val = match info.resume(vm) {
            Ok(val) => val,
            Err(err) if err.is_stop_iteration() => break,
            Err(err) => return Err(err),
        };
        args[0] = val;
        let res = vm.eval_block(block, &args)?;
        ary.push(res);
        vm.temp_push(res);
    }
    info.free();
    Ok(Value::array_from(ary))
}

fn with_index(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let eref = self_val.as_enumerator().unwrap();
    let mut info = vm.dup_enum(eref);
    //let fref = &mut eref.fiber;
    let block = match args.block {
        Some(method) => method,
        None => {
            // return Enumerator
            let id = IdentId::get_id("with_index");
            let e = vm.create_enumerator(id, self_val, args.clone())?;
            return Ok(e);
        }
    };

    let mut args = Args::new(2);
    let mut c = 0;
    let mut ary = vec![];
    loop {
        let val = match info.resume(vm) {
            Ok(val) => val,
            Err(err) => {
                if err.is_stop_iteration() {
                    break;
                } else {
                    return Err(err);
                }
            }
        };
        args[0] = val;
        args[1] = Value::integer(c);
        let res = vm.eval_block(block, &args)?;
        vm.temp_push(res);
        ary.push(res);
        c += 1;
    }
    info.free();
    Ok(Value::array_from(ary))
}

#[cfg(test)]
mod test {
    use crate::test::*;

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
    fn enumerator_map() {
        let program = r#"
            assert [0, 5, 12, 21], (4..7).each.with_index.map{|x,y| x * y}
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
