use super::*;
use once_cell::sync::Lazy;
use region::{protect, Protection};
use std::alloc::{GlobalAlloc, Layout, LayoutError, System};
use std::sync::Mutex;

const DEFAULT_STACK_SIZE: usize = 1024 * 512;
const STACK_LAYOUT: Result<Layout, LayoutError> =
    Layout::from_size_align(DEFAULT_STACK_SIZE, 0x1000);

static STACK_STORE: Lazy<Mutex<Vec<Stack>>> = Lazy::new(|| Mutex::new(Vec::new()));

///
/// Machine stack handle for Fiber.
///
/// `Stack` is a wrapper of a raw pointer which points to the top of machine stack area
/// of a Fiber object.
/// The stack area is newly allocated when a Fiber object is 'resume'd at the first time
/// after created.
///  
/// Default size of the stack area is DEFAULT_SIZE bytes(currently, 512KiB).
///
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(transparent)]
pub struct Stack(*mut u8);

unsafe impl Sync for Stack {}
unsafe impl Send for Stack {}

impl Stack {
    pub(crate) fn default() -> Self {
        Self(std::ptr::null_mut())
    }

    /// Allocate new stack area.
    ///
    /// If some `Stack` were saved in `STACK_STORE`, the newest one is returned.
    /// Otherwise, allocate new `Stack` and return it.
    pub(crate) fn allocate() -> Self {
        match STACK_STORE.lock().unwrap().pop() {
            None => unsafe {
                let stack = System.alloc(STACK_LAYOUT.unwrap());
                protect(stack, DEFAULT_STACK_SIZE, Protection::READ_WRITE)
                    .expect("Mprotect failed.");
                Stack(stack)
            },
            Some(stack) => stack,
        }
    }

    /// Deallocate `Stack`.
    ///
    /// Currently, when a Fiber object was disposed by GC, associated `Stack` is returned
    /// to `STACK_STORE`.
    pub(crate) fn deallocate(&mut self) {
        if self.0.is_null() {
            return;
        }
        STACK_STORE.lock().unwrap().push(*self);
        self.0 = std::ptr::null_mut();
    }

    /// Initialize `Stack`.
    ///
    /// Addresses of some functions are stored at the bottom of the stack area.
    ///
    /// - `new_context` is to be called when the Fiber coroutine is 'resume'd at the first time.
    /// - `guard` is to be called when 'resume'd **after** the Fiber coroutine execution was finished.
    /// - a pointer which points FiberContext is placed at the very bottom of the stack.
    pub(crate) fn init(&mut self, fiber: *const FiberContext) -> u64 {
        unsafe {
            let s_ptr = self.0.offset(DEFAULT_STACK_SIZE as isize);
            (s_ptr.offset(-8) as *mut u64).write(fiber as u64);
            (s_ptr.offset(-16) as *mut u64).write(guard as u64);
            // this is a dummy function for 16bytes-align.
            (s_ptr.offset(-24) as *mut u64).write(asm::skip as u64);
            (s_ptr.offset(-32) as *mut u64).write(new_context as u64);
            // more bytes to store registers.
            s_ptr.offset(-32 - asm::OFFSET) as u64
        }
    }
}

extern "C" fn new_context(handle: FiberHandle) {
    let mut fiber_vm = handle.vm();
    fiber_vm.handle = Some(handle);
    let res = match handle.kind() {
        FiberKind::Fiber(mut context) => {
            let val = fiber_vm.stack_top();
            fiber_vm.stack_push(context.self_val());
            fiber_vm.prepare_frame_from_heap(context);
            if context.iseq().lvars > 0 {
                context[0] = val;
            }
            fiber_vm.cfp.set_heap(context);
            fiber_vm.run_loop()
        }
        FiberKind::Enum(info) => fiber_vm.enumerator_fiber(info.receiver, &info.args, info.method),
    };
    #[cfg(feature = "trace")]
    eprintln!("<=== yield {:?} and terminate fiber.", res);
    let res = match res {
        Err(err) => match &err.kind {
            RubyErrorKind::MethodReturn => Err(err.conv_localjump_err()),
            _ => Err(err),
        },
        res => res,
    };
    handle.vm().globals.fiber_result = res;
    /*unsafe {
        //(*handle.0).result = res;
        //&mut (*handle.0).result
    }*/
}

extern "C" fn guard(fiber: *mut FiberContext) {
    unsafe {
        (*fiber).state = FiberState::Dead;
        //(*fiber).stack.deallocate();
        //(*fiber).result = (*val).clone();
    }
    asm::yield_context(fiber);
}
