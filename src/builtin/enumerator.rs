use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct EnumInfo {
    method: IdentId,
    receiver: Value,
    args: Args,
    fiber: Value,
}

impl EnumInfo {
    pub fn new(method: IdentId, receiver: Value, args: Args, fiber: Value) -> Self {
        EnumInfo {
            method,
            receiver,
            args,
            fiber,
        }
    }

    pub fn next(&mut self, vm: &mut VM) -> VMResult {
        let mut fiber = self.fiber.as_fiber().unwrap();
        fiber.resume(vm)
    }
}

impl GC for EnumInfo {
    fn mark(&self, alloc: &mut Allocator) {
        self.receiver.mark(alloc);
        self.fiber.mark(alloc);
        self.args.iter().for_each(|v| v.mark(alloc));
    }
}

pub fn init_enumerator(globals: &mut Globals) -> Value {
    let id = IdentId::get_id("Enumerator");
    let class = ClassRef::from(id, globals.builtins.object);
    globals.add_builtin_instance_method(class, "each", each);
    globals.add_builtin_instance_method(class, "map", map);
    globals.add_builtin_instance_method(class, "collect", map);
    globals.add_builtin_instance_method(class, "with_index", with_index);
    globals.add_builtin_instance_method(class, "inspect", inspect);
    let class = Value::class(globals, class);
    globals.add_builtin_class_method(class, "new", enum_new);
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
        let new_args = Args::new0();
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
        new_args.block = None;
        (method, new_args)
    };
    let val = vm.create_enumerator(method, receiver, new_args)?;
    Ok(val)
}

// Instance methods

fn inspect(vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    let eref = self_val.expect_enumerator(vm, "Expect Enumerator.")?;
    let arg_string = {
        match eref.args.len() {
            0 => "".to_string(),
            1 => format!("{:?}", eref.args[0]),
            _ => {
                let mut s = format!("{:?}", eref.args[0]);
                for i in 1..eref.args.len() {
                    s = format!("{},{:?}", s, eref.args[i]);
                }
                s
            }
        }
    };
    let inspect = format!(
        "#<Enumerator: {:?}:{}({})>",
        eref.receiver,
        IdentId::get_ident_name(eref.method),
        arg_string
    );
    Ok(Value::string(&vm.globals, inspect))
}

fn each(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let eref = self_val.expect_enumerator(vm, "Expect Enumerator.")?;
    let mut fref = eref.fiber.as_fiber().unwrap();
    let block = match args.block {
        Some(method) => method,
        None => {
            return Ok(self_val);
        }
    };
    let mut args = Args::new(1);
    loop {
        let val = match fref.resume(vm) {
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
        vm.eval_block(block, &args)?;
    }

    Ok(eref.receiver)
}

fn map(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let eref = self_val.expect_enumerator(vm, "Expect Enumerator.")?;
    let mut fref = eref.fiber.as_fiber().unwrap();
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
        let val = match fref.resume(vm) {
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
        let res = vm.eval_block(block, &args)?;
        ary.push(res);
        vm.temp_push(res);
    }
    Ok(Value::array_from(&vm.globals, ary))
}

fn with_index(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let eref = self_val.expect_enumerator(vm, "Expect Enumerator.")?;
    let mut fref = eref.fiber.as_fiber().unwrap();
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
        let val = match fref.resume(vm) {
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
        args[1] = Value::fixnum(c);
        let res = vm.eval_block(block, &args)?;
        vm.temp_push(res);
        ary.push(res);
        c += 1;
    }
    Ok(Value::array_from(&vm.globals, ary))
}

#[cfg(test)]
mod test {
    use crate::test::*;

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
