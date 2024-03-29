use super::FiberContext;
use std::arch::asm;

pub const OFFSET: isize = 48;

#[cfg(not(tarpaulin_include))]
#[naked]
pub(super) extern "C" fn skip() {
    unsafe {
        // rdi <- *mut FiberContext
        asm!("mov rdi, [rsp+8]", "ret", options(noreturn));
    };
}

/// This function is called when the child fiber is resumed at first.
#[cfg(not(tarpaulin_include))]
#[naked]
pub(super) extern "C" fn invoke_context(_fiber: *mut FiberContext) {
    // rdi <- _fiber
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
            "ret", // new_context(&mut Fiber)
            options(noreturn)
        );
    }
}

/// This function is called when the child fiber is resumed.
#[cfg(not(tarpaulin_include))]
#[naked]
pub(super) extern "C" fn switch_context(_fiber: *mut FiberContext) {
    // rdi <- _fiber
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
            "ret",
            options(noreturn)
        );
    }
}

/// This function is called when the child fiber yielded.
#[cfg(not(tarpaulin_include))]
#[naked]
pub(super) extern "C" fn yield_context(_fiber: *mut FiberContext) {
    // rdi <- _fiber
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
            //"lea  rax, [rdi + 16]", // rax <- &f.result
            "ret",
            options(noreturn)
        );
    }
}
