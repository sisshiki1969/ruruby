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
    tx: SyncSender<FiberMsg>,
}

impl PartialEq for FiberInfo {
    fn eq(&self, other: &Self) -> bool {
        &*self.vm as *const VM == &*other.vm as *const VM && self.inner == other.inner
    }
}

pub enum FiberMsg {
    Resume,
    Terminate,
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
        tx: SyncSender<FiberMsg>,
    ) -> Self {
        FiberInfo {
            vm: VMRef::new(vm),
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
        tx: SyncSender<FiberMsg>,
    ) -> Self {
        FiberInfo {
            vm: VMRef::new(vm),
            inner: FiberKind::Builtin(receiver, method_id, args),
            rec,
            tx,
        }
    }

    pub fn free(&mut self) {
        //self.vm.free();
        /*match &mut self.inner {
            FiberKind::Ruby(c) => c.free(),
            _ => {}
        }*/
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
                self.vm.fiberstate_running();
                #[cfg(feature = "perf")]
                current_vm.perf.get_perf(Perf::INVALID);
                #[cfg(feature = "trace")]
                println!("===> resume(spawn)");
                let mut fiber_vm = VMRef::from_ref(&self.vm);
                let fiber_kind = self.inner.clone();
                thread::spawn(move || {
                    #[cfg(debug_assertions)]
                    eprintln!("running {:?}", std::thread::current().id());
                    fiber_vm.set_allocator();
                    let res = match fiber_kind {
                        FiberKind::Ruby(context) => fiber_vm.run_context(context),
                        FiberKind::Builtin(receiver, method_id, args) => {
                            Self::enumerator_fiber(&mut fiber_vm, receiver, method_id, &args)
                        }
                    };
                    #[cfg(debug_assertions)]
                    eprintln!("finished {:?} {:?}", std::thread::current().id(), res);
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
                        Some(ParentFiberInfo { tx, rx, mut parent }) => {
                            #[cfg(feature = "perf")]
                            parent.perf.add(&fiber_vm.perf);
                            tx.send(res).unwrap();
                        }
                        None => unreachable!(),
                    };
                    #[cfg(debug_assertions)]
                    eprintln!("killed {:?}", std::thread::current().id());
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
                self.tx.send(FiberMsg::Resume).unwrap();
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
        if self.vm.is_dead() {
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
