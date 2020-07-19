#[cfg(feature = "perf")]
use crate::vm::perf::Perf;
use crate::*;
use std::clone::Clone;
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread;

#[derive(Debug)]
pub struct FiberInfo {
    pub vm: Box<VM>,
    pub inner: FiberKind,
    rec: Receiver<VMResult>,
    tx: SyncSender<usize>,
}

impl PartialEq for FiberInfo {
    fn eq(&self, other: &Self) -> bool {
        &*self.vm as *const VM == &*other.vm as *const VM && self.inner == other.inner
    }
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
#[derive(Clone, PartialEq)]
pub enum FiberKind {
    Ruby(ContextRef),
    Builtin(Value, IdentId, Args),
}

impl std::fmt::Debug for FiberKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "FiberKind")
    }
}

impl FiberInfo {
    pub fn new(
        vm: VM,
        context: ContextRef,
        rec: Receiver<VMResult>,
        tx: SyncSender<usize>,
    ) -> Self {
        FiberInfo {
            vm: Box::new(vm),
            inner: FiberKind::Ruby(context),
            rec,
            tx,
        }
    }

    pub fn new_internal(
        vm: VM,
        receiver: Value,
        method_id: IdentId,
        args: Args,
        rec: Receiver<VMResult>,
        tx: SyncSender<usize>,
    ) -> Self {
        FiberInfo {
            vm: Box::new(vm),
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
        let mut args = args.clone();
        args.block = Some(MethodRef::from(0));
        let context = Context::new_noiseq();
        vm.context_push(ContextRef::from_ref(&context));
        vm.eval_method(method, receiver, None, &args)?;
        //vm.context_pop();
        let res = Err(vm.error_stop_iteration("msg"));
        res
    }

    pub fn resume(&mut self, current_vm: &mut VM) -> VMResult {
        #[allow(unused_variables, unused_assignments, unused_mut)]
        let mut inst: u8;
        #[cfg(feature = "perf")]
        {
            inst = current_vm.perf.get_prev_inst();
        }
        match self.vm.fiberstate() {
            FiberState::Dead => {
                return Err(current_vm.error_fiber("Dead fiber called."));
            }
            FiberState::Created => {
                //eprintln!("running {:?}", VMRef::from_ref(&self.vm));
                self.vm.fiberstate_running();
                #[cfg(feature = "perf")]
                current_vm.perf.get_perf(Perf::INVALID);
                #[cfg(feature = "trace")]
                println!("===> resume(spawn)");
                let mut fiber_vm = VMRef::from_ref(&self.vm);
                let fiber_kind = self.inner.clone();
                thread::spawn(move || {
                    fiber_vm.set_allocator();
                    let res = match fiber_kind {
                        FiberKind::Ruby(context) => fiber_vm.run_context(context),
                        FiberKind::Builtin(receiver, method_id, args) => {
                            Self::enumerator_fiber(&mut fiber_vm, receiver, method_id, &args)
                        }
                    };
                    // If the fiber was finished, the fiber becomes DEAD.
                    // Return a value on the stack top to the parent fiber.
                    fiber_vm.fiberstate_dead();
                    #[cfg(feature = "trace")]
                    println!("<=== yield {:?} and terminate fiber.", res);
                    let res = match res {
                        Err(err) => match err.kind {
                            RubyErrorKind::MethodReturn(_) => Err(err.conv_localjump_err()),
                            _ => Err(err),
                        },
                        res => res,
                    };
                    #[allow(unused_variables)]
                    #[allow(unused_mut)]
                    match &fiber_vm.parent_fiber {
                        Some(ParentFiberInfo { tx, mut parent, .. }) => {
                            #[cfg(feature = "perf")]
                            parent.perf.add(&fiber_vm.perf);

                            //eprintln!("terminated & added {:?}", fiber_vm);
                            tx.send(res).unwrap();
                        }
                        None => unreachable!(),
                    };
                });
                // Wait for fiber.resume.
                let res = self.rec.recv().unwrap();
                #[cfg(feature = "perf")]
                current_vm.perf.get_perf_no_count(inst);
                res
            }
            FiberState::Running => {
                #[cfg(feature = "perf")]
                current_vm.perf.get_perf(Perf::INVALID);
                #[cfg(feature = "trace")]
                println!("===> resume");
                //eprintln!("resume {:?}", VMRef::from_ref(&self.vm));
                self.tx.send(1).unwrap();
                // Wait for fiber.resume.
                let res = self.rec.recv().unwrap();
                #[cfg(feature = "perf")]
                current_vm.perf.get_perf_no_count(inst);
                res
            }
        }
    }
}

impl GC for FiberInfo {
    fn mark(&self, alloc: &mut Allocator) {
        if self.vm.fiberstate() != FiberState::Dead {
            self.vm.mark(alloc);
        }
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
    vm.check_args_num(args.len(), 0)?;
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
}
