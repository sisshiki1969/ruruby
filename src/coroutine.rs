use crate::*;
use region::{protect, Protection};
use std::alloc::{alloc, Layout};

const DEFAULT_STACK_SIZE: usize = 1024 * 1024 * 2;

#[derive(PartialEq, Eq, Debug)]
pub enum FiberState {
    Created,
    Running,
    Dead,
}

#[derive(Debug)]
#[repr(C)]
pub struct FiberContext {
    rsp: u64,
    main_rsp: u64,
    stack: *mut u8,
    pub state: FiberState,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct FiberHandle(*mut FiberContext);

impl FiberHandle {
    /// Yield from current child fiber.
    pub fn fiber_yield(&mut self, send_val: VMResult) -> Value {
        let send_val = Box::into_raw(Box::new(send_val));
        let val = yield_context(self.0, send_val);
        Value::from(val)
    }
}

impl FiberContext {
    //         stack end
    //     +----------------+
    // -8  |  *mut Runtime  |
    //     +----------------+
    // -16 |     guard      |
    //     +----------------+
    // -24 |      skip      |
    //     +----------------+
    // -32 |       f        |
    //     +----------------+
    //     |   callee-save  |
    // -80 |    registers   |
    //     +----------------+
    pub fn spawn(f: fn(FiberHandle, Value) -> *mut VMResult) -> Self {
        let mut fiber = FiberContext::new();
        unsafe {
            let s_ptr = fiber.get_stack_end();
            (s_ptr.offset(-8) as *mut u64).write(&fiber as *const _ as u64);
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
    fn new() -> Self {
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
        }
    }

    fn get_stack_end(&mut self) -> *mut u8 {
        unsafe { self.stack.offset(DEFAULT_STACK_SIZE as isize) }
    }
}

impl FiberContext {
    /// Resume child fiber.
    pub fn fiber_resume(&mut self, val: Value) -> Option<VMResult> {
        let ptr = self as _;
        if self.state == FiberState::Dead {
            eprintln!("The fiber is dead.");
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
        let mut fiber1 = FiberContext::spawn(|mut handle, val| {
            let mut fiber2 = FiberContext::spawn(|mut handle, val| {
                println!("CHILD2 STARTING with {:?}", val);
                for i in 0..5 {
                    let res = handle.fiber_yield(Ok(Value::integer(val.as_integer().unwrap() * i)));
                    println!("CHILD2 value: {:?}", res);
                }
                println!("CHILD2 FINISHED");
                Box::into_raw(Box::new(Ok(Value::integer(123))))
            });
            println!("CHILD1 STARTING with {:?}", val);
            for i in 0..4 {
                let res = handle.fiber_yield(Ok(Value::integer(11 * i)));
                println!("CHILD1 {:?}", res);
                assert_eq!(Value::integer(100 * i + 100), res);
                fiber2.fiber_resume(Value::integer(50 * i));
            }
            println!("CHILD1 FINISHED");
            Box::into_raw(Box::new(Ok(Value::integer(456))))
        });

        println!("MAIN STARTING");
        for i in 0..6 {
            println!("MAIN counter: {}", i);
            let res = fiber1.fiber_resume(Value::integer(100 * i));
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
