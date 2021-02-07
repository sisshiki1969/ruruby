use super::FiberContext;
use crate::VMResult;

pub const OFFSET: isize = -80;

#[naked]
pub(super) extern "C" fn skip() {
    unsafe {
        // rdi <- *mut FiberContext
        // rsi <- *mut VMResult
        asm!("mov rdi, [rsp+8]", "mov rsi, rax", "ret", options(noreturn));
    };
}

#[naked]
#[inline(never)]
pub(super) extern "C" fn invoke_context(
    _fiber: *mut FiberContext,
    _send_val: u64,
) -> *mut VMResult {
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
pub(super) extern "C" fn switch_context(_fiber: *mut FiberContext, _ret_val: u64) -> *mut VMResult {
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
pub(super) extern "C" fn yield_context(_fiber: *mut FiberContext, _ret_val: *mut VMResult) -> u64 {
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
