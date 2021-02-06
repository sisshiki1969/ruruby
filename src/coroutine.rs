use crate::*;
use region::{protect, Protection};
use std::alloc::{alloc, Layout};

const DEFAULT_STACK_SIZE: usize = 1024 * 128;

#[derive(PartialEq, Eq, Debug)]
pub enum FiberState {
    Created,
    Running,
    Dead,
}

#[derive(Clone, PartialEq)]
pub enum FiberKind {
    Fiber(ContextRef),
    Enum(Box<EnumInfo>),
}

impl std::fmt::Debug for FiberKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "FiberKind")
    }
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

#[derive(Debug, PartialEq)]
#[repr(C)]
pub struct FiberContext {
    rsp: u64,
    main_rsp: u64,
    stack: *mut u8,
    pub state: FiberState,
    pub vm: VMRef,
    pub kind: FiberKind,
}

impl Drop for FiberContext {
    fn drop(&mut self) {
        self.vm.free();
        let layout = Layout::from_size_align(DEFAULT_STACK_SIZE, 16).expect("Bad Layout.");
        unsafe { std::alloc::dealloc(self.stack, layout) };
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
    /// Yield from current child fiber.
    pub fn yield_(&mut self, send_val: VMResult) -> Value {
        let send_val = Box::into_raw(Box::new(send_val));
        let val = yield_context(self.0, send_val);
        Value::from(val)
    }

    pub fn vm(&self) -> VMRef {
        unsafe { (*self.0).vm }
    }

    pub fn kind(&self) -> FiberKind {
        unsafe { (*self.0).kind.clone() }
    }

    /// Yield args to parent fiber. (execute Fiber.yield)
    pub fn fiber_yield(vm: &mut VM, args: &Args) -> VMResult {
        let val = match args.len() {
            0 => Value::nil(),
            1 => args[0],
            _ => Value::array_from(args.to_vec()),
        };
        match vm.handle {
            None => return Err(RubyError::fiber("Can not yield from main fiber.")),
            Some(mut handle) => {
                #[cfg(feature = "perf")]
                vm.globals.perf.get_perf(Perf::INVALID);
                #[cfg(feature = "trace")]
                #[cfg(feature = "trace-func")]
                println!("<=== yield Ok({:?})", val);
                Ok(handle.yield_(Ok(val)))
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
    pub fn spawn(
        vm: VMRef,
        kind: FiberKind,
        f: fn(FiberHandle, Value) -> *mut VMResult,
    ) -> Box<Self> {
        let mut fiber = Box::new(FiberContext::new(vm, kind));
        //eprintln!("spawn() pointer: {:?}", &*fiber as *const _);
        unsafe {
            let s_ptr = fiber.get_stack_end();
            // &fiber points to the caller's stack.
            (s_ptr.offset(-8) as *mut u64).write(&*fiber as *const _ as u64);
            (s_ptr.offset(-16) as *mut u64).write(guard as u64);
            // this is a dummy function for 16bytes-align.
            (s_ptr.offset(-24) as *mut u64).write(skip as u64);
            (s_ptr.offset(-32) as *mut u64).write(f as u64);
            // 48 bytes to store registers.
            fiber.rsp = s_ptr.offset(-80) as u64;
        }
        fiber
    }
}

impl FiberContext {
    fn new(vm: VMRef, kind: FiberKind) -> Self {
        let layout = Layout::from_size_align(DEFAULT_STACK_SIZE, 16).expect("Bad Layout.");
        let stack = unsafe { alloc(layout) };
        unsafe {
            protect(stack, DEFAULT_STACK_SIZE, Protection::READ_WRITE).expect("Mprotect failed.");
        }
        FiberContext {
            rsp: 0,
            main_rsp: 0,
            stack,
            state: FiberState::Created,
            vm,
            kind,
        }
    }

    pub fn new_fiber(vm: VM, context: ContextRef) -> Box<Self> {
        let vmref = VMRef::new(vm);
        Self::spawn(vmref, FiberKind::Fiber(context), Self::new_context)
    }

    pub fn new_enumerator(vm: VM, info: EnumInfo) -> Box<Self> {
        let vmref = VMRef::new(vm);
        Self::spawn(vmref, FiberKind::Enum(Box::new(info)), Self::new_context)
    }

    fn get_stack_end(&mut self) -> *mut u8 {
        unsafe { self.stack.offset(DEFAULT_STACK_SIZE as isize) }
    }

    fn new_context(handle: FiberHandle, _val: Value) -> *mut VMResult {
        let mut fiber_vm = handle.vm();
        fiber_vm.handle = Some(handle);
        let res = match handle.kind() {
            FiberKind::Fiber(context) => fiber_vm.run_context(context),
            FiberKind::Enum(info) => Self::enumerator_fiber(&mut fiber_vm, &info),
        };
        #[cfg(any(feature = "trace", feature = "trace-func"))]
        println!("<=== yield {:?} and terminate fiber.", res);
        let res = match res {
            Err(err) => match &err.kind {
                RubyErrorKind::MethodReturn(_) => Err(err.conv_localjump_err()),
                _ => Err(err),
            },
            res => res,
        };
        Box::into_raw(Box::new(res))
    }

    /// This BuiltinFunc is called in the fiber thread of a enumerator.
    /// `vm`: VM of created fiber.
    pub fn enumerator_fiber(vm: &mut VM, info: &EnumInfo) -> VMResult {
        let method = vm.get_method_from_receiver(info.receiver, info.method)?;
        let context = Context::new_noiseq();
        vm.context_push(ContextRef::from_ref(&context));
        vm.invoke_method(method, info.receiver, None, &info.args)?;
        vm.context_pop();
        let res = Err(RubyError::stop_iteration("msg"));
        res
    }
}

impl FiberContext {
    /// Resume child fiber.
    pub fn resume_(&mut self, val: Value) -> Option<VMResult> {
        let ptr = self as _;
        if self.state == FiberState::Dead {
            return None;
        }
        if self.state == FiberState::Created {
            self.state = FiberState::Running;
            let res = unsafe { Box::from_raw(invoke_context(ptr, val.get())) };
            Some(*res)
        } else {
            let res = unsafe { Box::from_raw(switch_context(self as _, val.get())) };
            Some(*res)
        }
    }

    pub fn resume(&mut self, _globals: &mut Globals) -> VMResult {
        #[allow(unused_variables, unused_assignments, unused_mut)]
        #[cfg(any(feature = "trace", feature = "trace-func"))]
        println!("===> resume");
        match self.resume_(Value::nil()) {
            None => return Err(RubyError::fiber("Dead fiber called.")),
            Some(res) => return res,
        };
    }
}

extern "C" fn guard(fiber: *mut FiberContext, val: *mut VMResult) {
    unsafe {
        (*fiber).state = FiberState::Dead;
    }
    yield_context(fiber, val);
}

#[naked]
extern "C" fn skip() {
    unsafe {
        // rdi <- *mut FiberContext
        // rsi <- *mut VMResult
        asm!("mov rdi, [rsp+8]", "mov rsi, rax", "ret", options(noreturn));
    };
}

#[naked]
#[inline(never)]
extern "C" fn invoke_context(_fiber: *mut FiberContext, _send_val: u64) -> *mut VMResult {
    // rdi <- _fiber
    // rsi <- _send_val
    unsafe {
        asm!(
            "push r15",
            "push r14",
            "push r13",
            "push r12",
            "push rbx",
            "push rbp",
            "mov  [rdi + 8], rsp", // [f.main_rsp] <- rsp
            "mov  rsp, [rdi]",     // rsp <- f.rsp
            "pop  rbp",
            "pop  rbx",
            "pop  r12",
            "pop  r13",
            "pop  r14",
            "pop  r15",
            "ret", // f(&mut Fiber, u64)
            options(noreturn)
        );
    }
}

#[naked]
#[inline(never)]
extern "C" fn switch_context(_fiber: *mut FiberContext, _ret_val: u64) -> *mut VMResult {
    // rdi <- _fiber
    // rsi <- _ret_val
    unsafe {
        asm!(
            "push r15",
            "push r14",
            "push r13",
            "push r12",
            "push rbx",
            "push rbp",
            "mov  [rdi + 8], rsp", // [f.main_rsp] <- rsp
            "mov  rsp, [rdi]",     // rsp <- f.rsp
            "pop  rbp",
            "pop  rbx",
            "pop  r12",
            "pop  r13",
            "pop  r14",
            "pop  r15",
            "mov  rax, rsi", // rax <- _ret_val
            "ret",
            options(noreturn)
        );
    }
}

#[naked]
#[inline(never)]
extern "C" fn yield_context(_fiber: *mut FiberContext, _ret_val: *mut VMResult) -> u64 {
    // rdi <- _fiber
    // rsi <- _ret_val
    unsafe {
        asm!(
            "push r15",
            "push r14",
            "push r13",
            "push r12",
            "push rbx",
            "push rbp",
            "mov  [rdi], rsp",     // [f.rsp] <- rsp
            "mov  rsp, [rdi + 8]", // rsp <- f.main_rsp
            "pop  rbp",
            "pop  rbx",
            "pop  r12",
            "pop  r13",
            "pop  r14",
            "pop  r15",
            "mov  rax, rsi", // rax <- _ret_val
            "ret",
            options(noreturn)
        );
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn coroutine_test() {
        let vm = VMRef::from(0);
        let kind = FiberKind::Fiber(ContextRef::from(0));
        let mut fiber1 = FiberContext::spawn(vm, kind, |mut handle, val| {
            let vm = VMRef::from(0);
            let kind = FiberKind::Fiber(ContextRef::from(0));
            let mut fiber2 = FiberContext::spawn(vm, kind, |mut handle, val| {
                println!("CHILD2 STARTING with {:?}", val);
                for i in 0..5 {
                    let res = handle.yield_(Ok(Value::integer(val.as_integer().unwrap() * i)));
                    println!("CHILD2 value: {:?}", res);
                }
                println!("CHILD2 FINISHED");
                Box::into_raw(Box::new(Ok(Value::integer(123))))
            });
            eprintln!("obtained fiber2: {:?}", &*fiber2 as *const _);
            println!("CHILD1 STARTING with {:?}", val);
            for i in 0..4 {
                let res = handle.yield_(Ok(Value::integer(11 * i)));
                println!("CHILD1 {:?}", res);
                assert_eq!(Value::integer(100 * i + 100), res);
                fiber2.resume_(Value::integer(50 * i));
            }
            println!("CHILD1 FINISHED");
            Box::into_raw(Box::new(Ok(Value::integer(456))))
        });
        eprintln!("obtained fiber1: {:?}", &*fiber1 as *const _);

        println!("MAIN STARTING");
        for i in 0..6 {
            println!("MAIN counter: {}", i);
            let res = fiber1.resume_(Value::integer(100 * i));
            println!("response: {:?}", res);
            match i {
                i if i < 4 => assert_eq!(Some(Ok(Value::integer(i * 11))), res),
                4 => assert_eq!(Some(Ok(Value::integer(456))), res),
                _ => assert_eq!(None, res),
            }
            //eprintln!("CHILD1: {:?}", res);
        }
        println!("MAIN FINISHED");
    }
}
