use super::FiberContext;
use crate::{VMResult, Value};

pub(super) const OFFSET: isize = 0xb0;

#[naked]
pub(super) extern "C" fn skip() {
    unsafe {
        // x0: *mut VMResult
        asm!(
            "mov x1, x0",
            "ldr x0, [sp, #24]", // *mut FiberContext
            "ldr lr, [sp, #16]", // guard()
            "ret",               // jump to guard()
            // x0 <- *mut FiberContext
            // x1 <- *mut VMResult
            options(noreturn)
        );
    };
}

#[naked]
pub(super) extern "C" fn invoke_context(
    _fiber: *mut FiberContext,
    _send_val: Value,
) -> *mut VMResult {
    // x0: _fiber
    // x1: _send_val
    unsafe {
        asm!(
            "sub sp, sp, #0xb0",
            "stp d8, d9, [sp, #0x00]",
            "stp d10, d11, [sp, #0x10]",
            "stp d12, d13, [sp, #0x20]",
            "stp d14, d15, [sp, #0x30]",
            "stp x19, x20, [sp, #0x40]",
            "stp x21, x22, [sp, #0x50]",
            "stp x23, x24, [sp, #0x60]",
            "stp x25, x26, [sp, #0x70]",
            "stp x27, x28, [sp, #0x80]",
            "stp fp, lr, [sp, #0x90]",
            "mov x19, sp",
            "str x19, [x0, #8]", // [f.main_rsp] <- sp
            "ldr x19, [x0]",
            "mov sp, x19", // sp <- f.rsp
            "ldp d8, d9, [sp, #0x00]",
            "ldp d10, d11, [sp, #0x10]",
            "ldp d12, d13, [sp, #0x20]",
            "ldp d14, d15, [sp, #0x30]",
            "ldp x19, x20, [sp, #0x40]",
            "ldp x21, x22, [sp, #0x50]",
            "ldp x23, x24, [sp, #0x60]",
            "ldp x25, x26, [sp, #0x70]",
            "ldp x27, x28, [sp, #0x80]",
            "ldp fp, lr, [sp, #0x90]",
            "add sp, sp, #0xb0",
            "ldr lr, [sp, #8]", // lr <- skip()
            "ldr x4, [sp]",
            "ret x4", // f(&mut Fiber, u64)
            options(noreturn)
        );
    }
}

#[naked]
pub(super) extern "C" fn switch_context(
    _fiber: *mut FiberContext,
    _ret_val: Value,
) -> *mut VMResult {
    // x0: _fiber
    // x1: _ret_val
    unsafe {
        asm!(
            "sub sp, sp, #0xb0",
            "stp d8, d9, [sp, #0x00]",
            "stp d10, d11, [sp, #0x10]",
            "stp d12, d13, [sp, #0x20]",
            "stp d14, d15, [sp, #0x30]",
            "stp x19, x20, [sp, #0x40]",
            "stp x21, x22, [sp, #0x50]",
            "stp x23, x24, [sp, #0x60]",
            "stp x25, x26, [sp, #0x70]",
            "stp x27, x28, [sp, #0x80]",
            "stp fp, lr, [sp, #0x90]",
            "mov x19, sp",       // [f.main_rsp] <- sp
            "str x19, [x0, #8]", // [f.main_rsp] <- rsp
            "ldr x19, [x0]",     // rsp <- f.rsp
            "mov sp, x19",
            "ldp d8, d9, [sp, #0x00]",
            "ldp d10, d11, [sp, #0x10]",
            "ldp d12, d13, [sp, #0x20]",
            "ldp d14, d15, [sp, #0x30]",
            "ldp x19, x20, [sp, #0x40]",
            "ldp x21, x22, [sp, #0x50]",
            "ldp x23, x24, [sp, #0x60]",
            "ldp x25, x26, [sp, #0x70]",
            "ldp x27, x28, [sp, #0x80]",
            "ldp fp, lr, [sp, #0x90]",
            "add sp, sp, #0xb0",
            "mov x0, x1", // x0 <- _ret_val
            "ret",
            options(noreturn)
        );
    }
}

#[naked]
pub(super) extern "C" fn yield_context(_fiber: *mut FiberContext) -> u64 {
    // x0: _fiber
    unsafe {
        asm!(
            "sub sp, sp, #0xb0",
            "stp d8, d9, [sp, #0x00]",
            "stp d10, d11, [sp, #0x10]",
            "stp d12, d13, [sp, #0x20]",
            "stp d14, d15, [sp, #0x30]",
            "stp x19, x20, [sp, #0x40]",
            "stp x21, x22, [sp, #0x50]",
            "stp x23, x24, [sp, #0x60]",
            "stp x25, x26, [sp, #0x70]",
            "stp x27, x28, [sp, #0x80]",
            "stp fp, lr, [sp, #0x90]",
            "mov x19, sp",
            "str x19, [x0]",     // [f.rsp] <- rsp
            "ldr x19, [x0, #8]", // rsp <- f.main_rsp
            "mov sp, x19",
            "ldp d8, d9, [sp, #0x00]",
            "ldp d10, d11, [sp, #0x10]",
            "ldp d12, d13, [sp, #0x20]",
            "ldp d14, d15, [sp, #0x30]",
            "ldp x19, x20, [sp, #0x40]",
            "ldp x21, x22, [sp, #0x50]",
            "ldp x23, x24, [sp, #0x60]",
            "ldp x25, x26, [sp, #0x70]",
            "ldp x27, x28, [sp, #0x80]",
            "ldp fp, lr, [sp, #0x90]",
            "add sp, sp, #0xb0",
            "add x0, x0, #16", // x0 <- &f.result
            "ret",
            options(noreturn)
        );
    }
}
