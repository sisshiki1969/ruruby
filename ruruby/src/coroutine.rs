use crate::*;
#[cfg(not(tarpaulin_include))]
#[cfg(all(unix, target_arch = "aarch64"))]
#[path = "coroutine/asm_arm64.rs"]
mod asm;
#[cfg(not(tarpaulin_include))]
#[cfg(all(windows, target_arch = "x86_64"))]
#[path = "coroutine/asm_windows_x64.rs"]
mod asm;
#[cfg(not(tarpaulin_include))]
#[cfg(all(unix, target_arch = "x86_64"))]
#[path = "coroutine/asm_x64.rs"]
mod asm;
mod stack;
use stack::*;

#[derive(PartialEq, Eq, Debug)]
pub enum FiberState {
    Created,
    Running,
    Dead,
}

#[derive(Clone, Debug)]
pub enum FiberKind {
    Fiber(HeapCtxRef),
    Enum(Box<EnumInfo>),
}

impl GC<RValue> for FiberKind {
    fn mark(&self, alloc: &mut Allocator<RValue>) {
        match self {
            FiberKind::Fiber(context) => context.mark(alloc),
            FiberKind::Enum(info) => info.mark(alloc),
        }
    }
}

#[derive(Clone, Debug)]
pub struct EnumInfo {
    pub receiver: Value,
    pub method: IdentId,
    pub args: Args,
}

impl GC<RValue> for EnumInfo {
    fn mark(&self, alloc: &mut Allocator<RValue>) {
        self.receiver.mark(alloc);
        self.args.mark(alloc);
    }
}

#[derive(Debug)]
#[repr(C)]
pub struct FiberContext {
    rsp: u64,
    main_rsp: u64,
    stack: Stack,
    pub state: FiberState,
    pub vm: VMRef,
    pub kind: FiberKind,
}

impl Drop for FiberContext {
    fn drop(&mut self) {
        self.vm.free();
        self.stack.deallocate();
    }
}

impl GC<RValue> for FiberContext {
    fn mark(&self, alloc: &mut Allocator<RValue>) {
        if self.state == FiberState::Dead {
            return;
        }
        self.vm.mark(alloc);
        self.kind.mark(alloc);
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct FiberHandle(*mut FiberContext);

impl FiberHandle {
    pub(crate) fn vm(&self) -> VMRef {
        unsafe { (*self.0).vm }
    }

    pub(crate) fn kind(&self) -> &FiberKind {
        unsafe { &(*self.0).kind }
    }

    /// Yield args to parent fiber. (execute Fiber.yield)
    pub(crate) fn fiber_yield(vm: &mut VM, args: &Args2) -> VMResult {
        let val = match args.len() {
            0 => Value::nil(),
            1 => vm[0],
            _ => Value::array_from(vm.args().to_vec()),
        };
        match vm.handle {
            None => Err(RubyError::fiber("Can not yield from main fiber.")),
            Some(handle) => {
                #[cfg(feature = "perf")]
                vm.globals.perf.get_perf(Perf::INVALID);
                #[cfg(feature = "trace")]
                eprintln!("<=== yield Ok({:?})", val);
                vm.globals.fiber_result = VMResult::Ok(val);
                asm::yield_context(handle.0);
                let val = vm.stack_pop();
                Ok(val)
            }
        }
    }
}

impl FiberContext {
    //            stack end
    //     +---------------------+
    // -8  |  *mut FiberContext  |
    //     +---------------------+
    // -16 |        guard        |
    //     +---------------------+
    // -24 |        skip         |
    //     +---------------------+
    // -32 |          f          |
    //     +---------------------+
    //     |                     |
    //     |     callee-save     |
    //     |      registers      |
    // -80 |                     | <-sp
    //     +---------------------+
    //
    // Note: Size for callee-saved registers varies by platform.
    pub(crate) fn initialize(&mut self) {
        let ptr = self as *const _;
        self.stack = Stack::allocate();
        self.rsp = self.stack.init(ptr);
        self.state = FiberState::Running;
    }
}

impl FiberContext {
    fn new(vm: VMRef, kind: FiberKind) -> Self {
        FiberContext {
            rsp: 0,
            main_rsp: 0,
            stack: Stack::default(),
            state: FiberState::Created,
            vm,
            kind,
        }
    }

    pub(crate) fn new_fiber(vm: VM, context: HeapCtxRef) -> Self {
        let vmref = VMRef::new(vm);
        FiberContext::new(vmref, FiberKind::Fiber(context))
    }

    pub(crate) fn new_enumerator(vm: VM, info: EnumInfo) -> Self {
        let vmref = VMRef::new(vm);
        FiberContext::new(vmref, FiberKind::Enum(Box::new(info)))
    }
}

impl FiberContext {
    /// Resume child fiber.
    pub(crate) fn resume(&mut self, val: Value) -> VMResult {
        #[cfg(feature = "trace")]
        eprintln!("===> resume");
        let ptr = self as _;
        match self.state {
            FiberState::Dead => Err(RubyError::fiber("Dead fiber called.")),
            FiberState::Created => {
                self.initialize();
                self.vm.stack_push(val);
                asm::invoke_context(ptr);
                self.vm.globals.fiber_result.clone()
            }
            FiberState::Running => {
                self.vm.stack_push(val);
                asm::switch_context(ptr);
                self.vm.globals.fiber_result.clone()
            }
        }
    }
}
