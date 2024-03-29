use crate::coroutine::*;
use crate::*;

pub(crate) fn init(globals: &mut Globals) -> Value {
    let class = Module::class_under_object();
    globals.set_toplevel_constant("Fiber", class);
    class.add_builtin_method_by_str(globals, "inspect", inspect);
    class.add_builtin_method_by_str(globals, "resume", resume);

    class.add_builtin_class_method(globals, "new", new);
    class.add_builtin_class_method(globals, "yield", yield_);
    class.into()
}

// Class methods

fn new(vm: &mut VM, _self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    let val = Value::fiber(vm, args.expect_block()?);
    Ok(val)
}

fn yield_(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    FiberHandle::fiber_yield(vm, args)
}

// Instance methods

fn inspect(_: &mut VM, mut self_val: Value, _args: &Args2) -> VMResult {
    let fref = self_val.expect_fiber("Expect Fiber.")?;
    let inspect = format!(
        "#<Fiber:0x{:<016x} ({:?})>",
        fref as *mut _ as u64, fref.state,
    );
    Ok(Value::string(inspect))
}

fn resume(vm: &mut VM, mut self_val: Value, args: &Args2) -> VMResult {
    args.check_args_range(0, 1)?;
    let fiber = self_val.expect_fiber("")?;
    fiber.resume(vm.args().get(0).cloned().unwrap_or(Value::nil()))
}

#[cfg(test)]
mod test1 {
    use crate::tests::*;
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

    #[test]
    fn fiber_test5() {
        let program = r#"
        f = Fiber.new do |x|
          Fiber.yield x * 7
          loop do
            x = 5 * (Fiber.yield x)
          end
        end
        assert(700, f.resume 100)
        assert(100, f.resume 30)
        assert(75, f.resume 15)
        assert(0, f.resume 0)
        assert(5, f.resume 1)
        "#;
        assert_script(program);
    }
}

#[cfg(test)]
mod test2 {
    use crate::tests::*;
    #[test]
    fn fiber_gc_test1() {
        let program = r#"
        100.times do |x|
            f = Fiber.new { Fiber.yield([x.to_s] * 100) }
        end
        "#;
        assert_script(program);
    }

    #[test]
    fn fiber_gc_test2() {
        let program = r#"
        100.times do |x|
            f = Fiber.new { Fiber.yield([x.to_s] * 100) }
            f.resume
        end
        "#;
        assert_script(program);
    }

    #[test]
    fn fiber_gc_test3() {
        let program = r#"
        100.times do |x|
            f = Fiber.new { Fiber.yield([x.to_s] * 100) }
            f.resume
            f.resume
        end
        "#;
        assert_script(program);
    }
}
