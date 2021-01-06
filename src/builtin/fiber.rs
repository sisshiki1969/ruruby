use crate::*;

pub fn init(globals: &mut Globals) -> Value {
    let mut class = ClassInfo::class_from(globals.builtins.object);
    class.add_builtin_method_by_str("inspect", inspect);
    class.add_builtin_method_by_str("resume", resume);

    let class_val = Value::class(class);
    class_val.add_builtin_class_method("new", new);
    class_val.add_builtin_class_method("yield", yield_);
    class_val
}

// Class methods

fn new(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let context = match args.expect_block()? {
        Block::Block(method, outer) => vm.create_block_context(*method, *outer)?,
        Block::Proc(proc) => proc.expect_proc(vm)?.context,
        _ => unreachable!(),
    };
    assert!(!context.on_stack);
    assert!(context.moved_to_heap == Some(context));

    //vm.globals.fibers.push(VMRef::from_ref(&new_fiber));
    let val = Value::fiber(vm, context);
    Ok(val)
}

fn yield_(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.fiber_yield(args)
}

// Instance methods

fn inspect(_: &mut VM, mut self_val: Value, _args: &Args) -> VMResult {
    let fref = self_val.expect_fiber("Expect Fiber.")?;
    let inspect = format!(
        "#<Fiber:0x{:<016x} ({:?})>",
        fref as *mut FiberInfo as u64,
        fref.vm.fiberstate(),
    );
    Ok(Value::string(inspect))
}

fn resume(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let fiber = self_val.expect_fiber("")?;
    fiber.resume(vm)
}

#[cfg(test)]
mod test1 {
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
    fn fiber_test4() {
        let program = r#"
    fib = Fiber.new do
        Fiber.yield a=b=1
        loop { 
            a,b=b,a+b
            Fiber.yield a
        }
    end

    res = *(0..7).map {
        fib.resume
    }

    assert([1,1,2,3,5,8,13,21], res)
"#;
        assert_script(program);
    }
}

#[cfg(test)]
mod test2 {
    use crate::test::*;
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
