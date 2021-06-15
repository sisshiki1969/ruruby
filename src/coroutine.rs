use crate::*;
#[cfg(all(unix, target_arch = "aarch64"))]
mod asm_arm64;
#[cfg(all(windows, target_arch = "x86_64"))]
mod asm_windows_x64;
#[cfg(all(unix, target_arch = "x86_64"))]
mod asm_x64;
mod stack;
#[cfg(all(unix, target_arch = "aarch64"))]
use asm_arm64 as asm;
#[cfg(all(windows, target_arch = "x86_64"))]
use asm_windows_x64 as asm;
#[cfg(all(unix, target_arch = "x86_64"))]
use asm_x64 as asm;
use stack::*;

#[derive(PartialEq, Eq, Debug)]
pub enum FiberState {
    Created,
    Running,
    Dead,
}

#[derive(Clone, PartialEq, Debug)]
pub enum FiberKind {
    Fiber(ContextRef),
    Enum(Box<EnumInfo>),
}

impl GC for FiberKind {
    fn mark(&self, alloc: &mut Allocator) {
        match self {
            FiberKind::Fiber(context) => context.mark(alloc),
            FiberKind::Enum(info) => info.mark(alloc),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct EnumInfo {
    pub receiver: Value,
    pub method: IdentId,
    pub args: Args,
}

impl GC for EnumInfo {
    fn mark(&self, alloc: &mut Allocator) {
        self.receiver.mark(alloc);
        self.args.mark(alloc);
    }
}

impl EnumInfo {
    /// This BuiltinFunc is called in the fiber thread of a enumerator.
    /// `vm`: VM of created fiber.
    pub fn enumerator_fiber(&self, vm: &mut VM) -> VMResult {
        let method = vm.get_method_from_receiver(self.receiver, self.method)?;
        let context = ContextRef::new_noiseq();
        vm.context_push(context);
        vm.invoke_func(method, self.receiver, None, &self.args)?;
        vm.context_pop();
        Err(RubyError::stop_iteration("Iteration reached an end."))
    }
}

#[derive(Debug, PartialEq)]
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
        //eprintln!("dropped!");
        self.vm.free();
        self.stack.deallocate();
    }
}

impl GC for FiberContext {
    fn mark(&self, alloc: &mut Allocator) {
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
    pub fn vm(&self) -> VMRef {
        unsafe { (*self.0).vm }
    }

    pub fn kind(&self) -> &FiberKind {
        unsafe { &(*self.0).kind }
    }

    /// Yield args to parent fiber. (execute Fiber.yield)
    pub fn fiber_yield(vm: &mut VM, args: &Args) -> VMResult {
        let val = match args.len() {
            0 => Value::nil(),
            1 => args[0],
            _ => Value::array_from(args.to_vec()),
        };
        match vm.handle {
            None => Err(RubyError::fiber("Can not yield from main fiber.")),
            Some(handle) => {
                #[cfg(feature = "perf")]
                vm.globals.perf.get_perf(Perf::INVALID);
                #[cfg(any(feature = "trace", feature = "trace-func"))]
                println!("<=== yield Ok({:?})", val);
                let send_val = Box::into_raw(Box::new(Ok(val)));
                let val = asm::yield_context(handle.0, send_val);
                Ok(Value::from(val))
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
    pub fn initialize(&mut self) {
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

    pub fn new_fiber(vm: VM, context: ContextRef) -> Box<Self> {
        let vmref = VMRef::new(vm);
        Box::new(FiberContext::new(vmref, FiberKind::Fiber(context)))
    }

    pub fn new_enumerator(vm: VM, info: EnumInfo) -> Box<Self> {
        let vmref = VMRef::new(vm);
        Box::new(FiberContext::new(vmref, FiberKind::Enum(Box::new(info))))
    }
}

impl FiberContext {
    /// Resume child fiber.
    pub fn resume(&mut self, val: Value) -> VMResult {
        #[cfg(any(feature = "trace", feature = "trace-func"))]
        println!("===> resume");
        let ptr = self as _;
        match self.state {
            FiberState::Dead => Err(RubyError::fiber("Dead fiber called.")),
            FiberState::Created => {
                self.initialize();
                unsafe { *Box::from_raw(asm::invoke_context(ptr, val)) }
            }
            FiberState::Running => unsafe { *Box::from_raw(asm::switch_context(ptr, val)) },
        }
    }
}
