#[cfg(feature = "perf")]
use crate::vm::perf::Perf;
use crate::*;
use std::clone::Clone;
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread;

#[derive(Debug)]
pub struct FiberInfo {
    pub vm: VMRef,
    pub inner: FiberKind,
    rec: Receiver<VMResult>,
    tx: SyncSender<usize>,
}
/*
impl Clone for FiberInfo {
    fn clone(&self) -> Self {
        let vm = self.vm;
        let parent_vm = match &vm.parent_fiber {
            Some(info) => info.parent,
            None => unreachable!(),
        };
        let (tx0, rx0) = std::sync::mpsc::sync_channel(0);
        let (tx1, rx1) = std::sync::mpsc::sync_channel(0);
        let fiber_vm = VMRef::new(parent_vm.create_fiber(tx0, rx1));
        FiberInfo {
            vm: fiber_vm,
            inner: self.inner.clone(),
            rec: rx0,
            tx: tx1,
        }
    }
}
*/
#[derive(Clone)]
pub enum FiberKind {
    Ruby(ContextRef),
    Builtin(Value, IdentId, Args),
}

impl std::fmt::Debug for FiberKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "FiberKind")
    }
}

pub type FiberRef = Ref<FiberInfo>;

impl FiberInfo {
    pub fn new(
        vm: VMRef,
        context: ContextRef,
        rec: Receiver<VMResult>,
        tx: SyncSender<usize>,
    ) -> Self {
        FiberInfo {
            vm,
            inner: FiberKind::Ruby(context),
            rec,
            tx,
        }
    }

    pub fn new_internal(
        vm: VMRef,
        receiver: Value,
        method_id: IdentId,
        args: Args,
        rec: Receiver<VMResult>,
        tx: SyncSender<usize>,
    ) -> Self {
        FiberInfo {
            vm,
            inner: FiberKind::Builtin(receiver, method_id, args),
            rec,
            tx,
        }
    }

    /// This BuiltinFunc is called in the fiber thread of a enumerator.
    /// `vm`: VM of created fiber.
    pub fn enumerator_fiber(
        vm: &mut VM,
        receiver: Value,
        method_id: IdentId,
        args: &Args,
    ) -> VMResult {
        let method = vm.get_method(receiver, method_id)?;
        // let context = ContextRef::new(Context::new_noiseq());
        // vm.context_push(context);
        let mut args = args.clone();
        args.block = Some(MethodRef::from(0));
        vm.eval_method(method, receiver, None, &args)?;
        let res = Err(vm.error_stop_iteration("msg"));
        // vm.context_pop();
        res
    }

    pub fn resume(&mut self, vm: &mut VM) -> VMResult {
        let mut fiber_vm = self.vm;
        match fiber_vm.fiberstate() {
            FiberState::Dead => {
                return Err(vm.error_fiber("Dead fiber called."));
            }
            FiberState::Created => {
                fiber_vm.fiberstate_running();
                #[cfg(feature = "perf")]
                fiber_vm.perf.get_perf(Perf::INVALID);
                #[cfg(feature = "trace")]
                println!("===> resume(spawn)");
                let mut vm2 = fiber_vm;
                let fiber_kind = self.inner.clone();
                thread::spawn(move || {
                    vm2.set_allocator();
                    let res = match fiber_kind {
                        FiberKind::Ruby(context) => vm2.run_context(context),
                        FiberKind::Builtin(receiver, method_id, args) => {
                            Self::enumerator_fiber(&mut vm2, receiver, method_id, &args)
                        }
                    };
                    // If the fiber was finished, the fiber becomes DEAD.
                    // Return a value on the stack top to the parent fiber.
                    vm2.fiberstate_dead();
                    #[cfg(feature = "trace")]
                    println!("<=== yield {:?} and terminate fiber.", res);
                    let res = match res {
                        Err(err) => match err.kind {
                            RubyErrorKind::MethodReturn(_) => Err(err.conv_localjump_err()),
                            _ => Err(err),
                        },
                        res => res,
                    };
                    match &vm2.parent_fiber {
                        Some(ParentFiberInfo { tx, .. }) => {
                            tx.send(res).unwrap();
                        }
                        None => unreachable!(),
                    };
                });
                // Wait for fiber.resume.
                self.rec.recv().unwrap()
            }
            FiberState::Running => {
                #[cfg(feature = "perf")]
                vm.perf.get_perf(Perf::INVALID);
                #[cfg(feature = "trace")]
                println!("===> resume");

                self.tx.send(1).unwrap();
                // Wait for fiber.resume.
                self.rec.recv().unwrap()
            }
        }
    }
}

impl GC for FiberInfo {
    fn mark(&self, alloc: &mut Allocator) {
        if self.vm.fiberstate() == FiberState::Dead {
            return;
        }
        self.vm.mark(alloc);
        match &self.inner {
            FiberKind::Ruby(context) => context.mark(alloc),
            FiberKind::Builtin(receiver, _, args) => {
                receiver.mark(alloc);
                for arg in args.iter() {
                    arg.mark(alloc);
                }
            }
        }
    }
}

pub fn init_fiber(globals: &mut Globals) -> Value {
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

fn new(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let method = vm.expect_block(args.block)?;
    let context = vm.create_block_context(method)?;
    let (tx0, rx0) = std::sync::mpsc::sync_channel(0);
    let (tx1, rx1) = std::sync::mpsc::sync_channel(0);
    let new_fiber = VMRef::new(vm.create_fiber(tx0, rx1));
    let val = Value::fiber(&vm.globals, new_fiber, context, rx0, tx1);
    Ok(val)
}

fn yield_(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let val = match args.len() {
        0 => Value::nil(),
        1 => args[0],
        _ => Value::array_from(&vm.globals, args.to_vec()),
    };
    vm.fiber_yield(val)
}

// Instance methods

fn inspect(vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    let fref = vm.expect_fiber(self_val, "Expect Fiber.")?;
    let inspect = format!(
        "#<Fiber:0x{:<016x} ({:?})>",
        fref.id(),
        fref.vm.fiberstate(),
    );
    Ok(Value::string(&vm.globals, inspect))
}

fn resume(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let mut fiber = vm.expect_fiber(self_val, "")?;
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
}
