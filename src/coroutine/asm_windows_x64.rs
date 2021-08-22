// See Microsoft ABI documentation.
// https://docs.microsoft.com/en-us/cpp/build/x64-calling-convention?view=msvc-160#callercallee-saved-registers
use super::FiberContext;
use crate::{VMResult, Value};

pub const OFFSET: isize = 64 + 160;

#[naked]
pub(super) extern "C" fn skip() {
    unsafe {
        // rcx <- *mut FiberContext
        // rdx <- *mut VMResult
        asm!("mov rcx, [rsp+8]", "mov rdx, rax", "ret", options(noreturn));
    };
}

#[naked]
pub(super) extern "C" fn invoke_context(
    _fiber: *mut FiberContext,
    _send_val: Value,
) -> *mut VMResult {
    // rcx <- _fiber
    // rdx <- _send_val
    unsafe {
        asm!(
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm15",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm14",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm13",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm12",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm11",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm10",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm9",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm8",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm7",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm6",
            "push r15",
            "push r14",
            "push r13",
            "push r12",
            "push rsi",
            "push rdi",
            "push rbx",
            "push rbp",
            "mov  [rcx + 8], rsp", // [f.main_rsp] <- rsp
            "mov  rsp, [rcx]",     // rsp <- f.rsp
            "pop  rbp",
            "pop  rbx",
            "pop  rdi",
            "pop  rsi",
            "pop  r12",
            "pop  r13",
            "pop  r14",
            "pop  r15",
            "movdqu xmm6, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm7, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm8, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm9, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm10, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm11, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm12, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm13, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm14, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm15, xmmword ptr [rsp]",
            "add  rsp, 16",
            "ret", // f(&mut Fiber, u64)
            options(noreturn)
        );
    }
}

#[naked]
pub(super) extern "C" fn switch_context(
    _fiber: *mut FiberContext,
    _ret_val: Value,
) -> *mut VMResult {
    // rcx <- _fiber
    // rdx <- _ret_val
    unsafe {
        asm!(
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm15",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm14",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm13",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm12",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm11",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm10",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm9",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm8",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm7",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm6",
            "push r15",
            "push r14",
            "push r13",
            "push r12",
            "push rsi",
            "push rdi",
            "push rbx",
            "push rbp",
            "mov  [rcx + 8], rsp", // [f.main_rsp] <- rsp
            "mov  rsp, [rcx]",     // rsp <- f.rsp
            "pop  rbp",
            "pop  rbx",
            "pop  rdi",
            "pop  rsi",
            "pop  r12",
            "pop  r13",
            "pop  r14",
            "pop  r15",
            "movdqu xmm6, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm7, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm8, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm9, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm10, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm11, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm12, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm13, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm14, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm15, xmmword ptr [rsp]",
            "add  rsp, 16",
            "mov  rax, rdx", // rax <- _ret_val
            "ret",
            options(noreturn)
        );
    }
}

#[naked]
pub(super) extern "C" fn yield_context(_fiber: *mut FiberContext) -> u64 {
    // rcx <- _fiber
    unsafe {
        asm!(
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm15",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm14",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm13",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm12",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm11",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm10",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm9",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm8",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm7",
            "sub  rsp, 16",
            "movdqu xmmword ptr [rsp], xmm6",
            "push r15",
            "push r14",
            "push r13",
            "push r12",
            "push rsi",
            "push rdi",
            "push rbx",
            "push rbp",
            "mov  [rcx], rsp",     // [f.rsp] <- rsp
            "mov  rsp, [rcx + 8]", // rsp <- f.main_rsp
            "pop  rbp",
            "pop  rbx",
            "pop  rdi",
            "pop  rsi",
            "pop  r12",
            "pop  r13",
            "pop  r14",
            "pop  r15",
            "movdqu xmm6, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm7, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm8, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm9, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm10, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm11, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm12, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm13, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm14, xmmword ptr [rsp]",
            "add  rsp, 16",
            "movdqu xmm15, xmmword ptr [rsp]",
            "add  rsp, 16",
            "lea  rax, [rcx + 16]", // rax <- _ret_val
            "ret",
            options(noreturn)
        );
    }
}
