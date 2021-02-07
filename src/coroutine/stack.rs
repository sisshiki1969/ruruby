use super::*;
use region::{protect, Protection};
use std::{
    alloc::{alloc, Layout},
    cell::RefCell,
};

const DEFAULT_STACK_SIZE: usize = 1024 * 64;

thread_local! {
    static STACK_STORE: RefCell<Vec<*mut u8>> = RefCell::new(vec![]);
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(transparent)]
pub struct Stack(*mut u8);

impl Stack {
    pub fn allocate() -> Self {
        STACK_STORE.with(|m| {
            let mut v = m.borrow_mut();
            match v.pop() {
                None => {
                    let stack = unsafe {
                        alloc(Layout::from_size_align(DEFAULT_STACK_SIZE, 16).expect("Bad Layout."))
                    };
                    unsafe {
                        protect(stack, DEFAULT_STACK_SIZE, Protection::READ_WRITE)
                            .expect("Mprotect failed.");
                    }
                    Self(stack)
                }
                Some(stack) => Self(stack),
            }
        })
    }

    pub fn deallocate(self) {
        STACK_STORE.with(|m| m.borrow_mut().push(self.0));
        /*unsafe {
            std::alloc::dealloc(
                self.0,
                Layout::from_size_align(DEFAULT_STACK_SIZE, 16).expect("Bad Layout."),
            )
        };*/
    }

    pub fn init(&mut self, fiber: *const FiberContext) -> u64 {
        unsafe {
            let s_ptr = self.0.offset(DEFAULT_STACK_SIZE as isize);
            (s_ptr.offset(-8) as *mut u64).write(fiber as u64);
            (s_ptr.offset(-16) as *mut u64).write(guard as u64);
            // this is a dummy function for 16bytes-align.
            (s_ptr.offset(-24) as *mut u64).write(asm::skip as u64);
            (s_ptr.offset(-32) as *mut u64).write(new_context as u64);
            // 48 bytes to store registers.
            s_ptr.offset(-80) as u64
        }
    }
}

extern "C" fn new_context(handle: FiberHandle, _val: Value) -> *mut VMResult {
    let mut fiber_vm = handle.vm();
    fiber_vm.handle = Some(handle);
    let res = match handle.kind() {
        FiberKind::Fiber(context) => fiber_vm.run_context(*context),
        FiberKind::Enum(info) => info.enumerator_fiber(&mut fiber_vm),
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

extern "C" fn guard(fiber: *mut FiberContext, val: *mut VMResult) {
    unsafe {
        (*fiber).state = FiberState::Dead;
    }
    asm::yield_context(fiber, val);
}
