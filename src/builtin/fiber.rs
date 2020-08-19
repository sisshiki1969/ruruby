use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let id = IdentId::get_id("Fiber");
    let class = ClassRef::from(id, globals.builtins.object);
    let val = Value::class(globals, class);
    globals.add_builtin_instance_method(class, "inspect", inspect);
    globals.add_builtin_instance_method(class, "resume", resume);
    globals.add_builtin_class_method(val, "new", new);
    globals.add_builtin_class_method(val, "yield", yield_);
    val
}

// Class methods

fn new(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(self_val, args.len(), 0)?;
    let method = vm.expect_block(args.block)?;
    let context = vm.create_block_context(method)?;
    let (tx0, rx0) = std::sync::mpsc::sync_channel(0);
    let (tx1, rx1) = std::sync::mpsc::sync_channel(0);
    let new_fiber = vm.create_fiber(tx0, rx1);
    //vm.globals.fibers.push(VMRef::from_ref(&new_fiber));
    let val = Value::fiber(&vm.globals, new_fiber, context, rx0, tx1);
    Ok(val)
}

fn yield_(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.fiber_yield(args)
}

// Instance methods

fn inspect(vm: &mut VM, mut self_val: Value, _args: &Args) -> VMResult {
    let fref = self_val.expect_fiber(vm, "Expect Fiber.")?;
    let inspect = format!(
        "#<Fiber:0x{:<016x} ({:?})>",
        fref as *mut FiberInfo as u64,
        fref.vm.fiberstate(),
    );
    Ok(Value::string(&vm.globals.builtins, inspect))
}

fn resume(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(self_val, args.len(), 0)?;
    let fiber = self_val.expect_fiber(vm, "")?;
    fiber.resume(vm)
}

#[cfg(test)]
mod test {
    use crate::test::*;
    #[test]
    fn fiber_test1() {
        let program = r#"
        def enum2gen(enum)
            Fiber.new do
                enum.each{|i|
                    Fiber.yield(i)
                }
            end
        end

        g = enum2gen(1..5)

        assert(1, g.resume)
        assert(2, g.resume)
        assert(3, g.resume)
        assert(4, g.resume)
        assert(5, g.resume)
        assert(1..5, g.resume)
        assert_error { g.resume }
        "#;
        assert_script(program);
    }

    #[test]
    fn fiber_test2() {
        let program = r#"
        f = Fiber.new do
            30.times {|x|
                Fiber.yield x
            }
        end
        assert(0, f.resume)
        assert(1, f.resume)
        assert(2, f.resume)
        assert(3, f.resume)
        assert(4, f.resume)
        "#;
        assert_script(program);
    }

    #[test]
    fn fiber_test3() {
        let program = r#"
        f = Fiber.new {}
        assert(nil, f.resume)
        f = Fiber.new { 5 }
        assert(5, f.resume)
        f = Fiber.new { return 5 }
        assert_error { f.resume }
        "#;
        assert_script(program);
    }

    #[test]
    fn fiber_gc_test1() {
        let program = r#"
        10000.times do |x|
            f = Fiber.new { Fiber.yield([x.to_s] * 10000) }
        end
        "#;
        assert_script(program);
    }

    #[test]
    fn fiber_gc_test2() {
        let program = r#"
        10000.times do |x|
            f = Fiber.new { Fiber.yield([x.to_s] * 10000) }
            f.resume
        end
        "#;
        assert_script(program);
    }

    #[test]
    fn fiber_gc_test3() {
        let program = r#"
        10000.times do |x|
            f = Fiber.new { Fiber.yield([x.to_s] * 10000) }
            f.resume
            f.resume
        end
        "#;
        assert_script(program);
    }
}
