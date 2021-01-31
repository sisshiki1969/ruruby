#[cfg(feature = "perf")]
use crate::vm::perf::Perf;
use crate::*;
use std::clone::Clone;
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread;

#[derive(Debug)]
pub struct FiberInfo {
    pub vm: VMRef,
    pub kind: FiberKind,
    rec: Receiver<VMResult>,
    tx: SyncSender<FiberMsg>,
}

impl PartialEq for FiberInfo {
    fn eq(&self, other: &Self) -> bool {
        &*self.vm as *const VM == &*other.vm as *const VM && self.kind == other.kind
    }
}

impl Drop for FiberInfo {
    fn drop(&mut self) {
        if let Some(h) = self.vm.handle.take() {
            // FiberState::Running or DEAD
            let _id = h.thread().id();
            if !self.vm.is_dead() {
                let _ = self.tx.send(FiberMsg::Terminate);
            }
            h.join().unwrap();
            //#[cfg(debug_assertions)]
            //eprintln!("fiber disposed {:?}", _id);
        };
        self.vm.free();
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
    Fiber(ContextRef),
    Enum(Value, IdentId, Args),
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
            kind: FiberKind::Fiber(context),
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
            kind: FiberKind::Enum(receiver, method_id, args),
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
        let method = vm.get_method_from_receiver(receiver, method_id)?;
        let context = Context::new_noiseq();
        vm.context_push(ContextRef::from_ref(&context));
        vm.invoke_method(method, receiver, None, args)?;
        vm.context_pop();
        let res = Err(RubyError::stop_iteration("msg"));
        res
    }

    pub fn resume(&mut self, _current_vm: &mut VM) -> VMResult {
        #[allow(unused_variables, unused_assignments, unused_mut)]
        let mut inst: u8;
        #[cfg(feature = "perf")]
        {
            inst = _current_vm.perf.get_prev_inst();
        }
        match self.vm.fiberstate() {
            FiberState::Dead => return Err(RubyError::fiber("Dead fiber called.")),
            FiberState::Created => {
                #[cfg(feature = "perf")]
                _current_vm.perf.get_perf(Perf::INVALID);
                #[cfg(any(feature = "trace", feature = "trace-func"))]
                println!("===> resume(spawn)");
                let mut fiber_vm = VMRef::from_ref(&self.vm);
                let fiber_kind = self.kind.clone();
                //let builder = thread::Builder::new().stack_size(1024 * 1024);
                let join = thread::spawn(move || {
                    fiber_vm.fiberstate_running();
                    //#[cfg(debug_assertions)]
                    //eprintln!("running {:?}", std::thread::current().id());
                    fiber_vm.globals.set_allocator();
                    let res = match fiber_kind {
                        FiberKind::Fiber(context) => fiber_vm.run_context(context),
                        FiberKind::Enum(receiver, method_id, args) => {
                            Self::enumerator_fiber(&mut fiber_vm, receiver, method_id, &args)
                        }
                    };
                    //#[cfg(debug_assertions)]
                    //eprintln!("finished {:?} {:?}", std::thread::current().id(), res);
                    // If the fiber was finished, the fiber becomes DEAD.
                    // Return a value on the stack top to the parent fiber.
                    #[cfg(any(feature = "trace", feature = "trace-func"))]
                    println!("<=== yield {:?} and terminate fiber.", res);
                    let res = match res {
                        Err(err) => match &err.kind {
                            RubyErrorKind::MethodReturn(_) => Err(err.conv_localjump_err()),
                            RubyErrorKind::RuntimeErr { kind, .. }
                                if *kind == RuntimeErrKind::Fiber =>
                            {
                                #[cfg(feature = "perf")]
                                match &fiber_vm.parent_fiber {
                                    Some(ParentFiberInfo { mut parent, .. }) => {
                                        parent.perf.add(&fiber_vm.perf);
                                    }
                                    None => {}
                                };
                                //#[cfg(debug_assertions)]
                                //eprintln!("killed {:?}", std::thread::current().id());
                                fiber_vm.fiberstate_dead();
                                return;
                            }
                            _ => Err(err),
                        },
                        res => res,
                    };
                    fiber_vm.fiberstate_dead();
                    match &fiber_vm.parent_fiber {
                        Some(ParentFiberInfo { tx, .. }) => {
                            let _ = tx.send(res);
                        }
                        None => unreachable!(),
                    };
                    #[cfg(feature = "perf")]
                    match &fiber_vm.parent_fiber {
                        Some(ParentFiberInfo { mut parent, .. }) => {
                            parent.perf.add(&fiber_vm.perf);
                        }
                        None => {}
                    };
                    //#[cfg(debug_assertions)]
                    //eprintln!("dead {:?}", std::thread::current().id());
                });
                // Wait for Fiber.yield.
                let res = self.rec.recv().unwrap();
                self.vm.handle = Some(join);
                #[cfg(feature = "perf")]
                _current_vm.perf.get_perf_no_count(inst);
                res
            }
            FiberState::Running => {
                #[cfg(feature = "perf")]
                _current_vm.perf.get_perf(Perf::INVALID);
                #[cfg(any(feature = "trace", feature = "trace-func"))]
                println!("===> resume");
                //eprintln!("resume {:?}", VMRef::from_ref(&self.vm));
                self.tx.send(FiberMsg::Resume).unwrap();
                // Wait for Fiber.yield.
                let res = self.rec.recv().unwrap();
                #[cfg(feature = "perf")]
                _current_vm.perf.get_perf_no_count(inst);
                res
            }
        }
    }

    /// Yield args to parent fiber. (execute Fiber.yield)
    pub fn fiber_yield(vm: &mut VM, args: &Args) -> VMResult {
        let val = match args.len() {
            0 => Value::nil(),
            1 => args[0],
            _ => Value::array_from(args.to_vec()),
        };
        match &vm.parent_fiber {
            None => return Err(RubyError::fiber("Can not yield from main fiber.")),
            Some(ParentFiberInfo { tx, rx, .. }) => {
                #[cfg(feature = "perf")]
                let mut _inst: u8;
                #[cfg(feature = "perf")]
                {
                    _inst = self.perf.get_prev_inst();
                }
                #[cfg(feature = "perf")]
                self.perf.get_perf(Perf::INVALID);
                #[cfg(feature = "trace")]
                #[cfg(feature = "trace-func")]
                println!("<=== yield Ok({:?})", val);

                tx.send(Ok(val)).unwrap();
                // Wait for fiber's response
                match rx.recv() {
                    Ok(FiberMsg::Resume) => {}
                    _ => return Err(RubyError::fiber("terminated")),
                }
                #[cfg(feature = "perf")]
                self.perf.get_perf_no_count(_inst);
                // TODO: this return value is not correct. The arg of Fiber#resume should be returned.
                Ok(Value::nil())
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
        match &self.kind {
            FiberKind::Fiber(context) => context.mark(alloc),
            FiberKind::Enum(receiver, _, args) => {
                receiver.mark(alloc);
                for arg in args.iter() {
                    arg.mark(alloc);
                }
            }
        }
    }
}
