use super::*;
use region::{protect, Protection};
use std::{
    alloc::{alloc, dealloc, Layout},
    cell::RefCell,
};

const DEFAULT_STACK_SIZE: usize = 1024 * 128;

thread_local!(
    static STACK_STORE: RefCell<Vec<*mut u8>> = RefCell::new(vec![]);
);

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(transparent)]
pub struct Stack(*mut u8);

impl Stack {
    pub fn default() -> Self {
        Self(0 as _)
    }

    pub fn allocate() -> Self {
        STACK_STORE.with(|m| match m.borrow_mut().pop() {
            None => unsafe {
                let stack =
                    alloc(Layout::from_size_align(DEFAULT_STACK_SIZE, 16).expect("Bad Layout."));
                protect(stack, DEFAULT_STACK_SIZE, Protection::READ_WRITE)
                    .expect("Mprotect failed.");
                Self(stack)
            },
            Some(stack) => Self(stack),
        })
    }

    pub fn deallocate(&mut self) {
        if self.0 as u64 == 0 {
            return;
        }
        STACK_STORE.with(|m| {
            let mut m = m.borrow_mut();
            if m.len() < 4 {
                m.push(self.0);
            } else {
                unsafe {
                    dealloc(
                        self.0,
                        Layout::from_size_align(DEFAULT_STACK_SIZE, 16).expect("Bad Layout."),
                    )
                };
            }
            self.0 = 0 as _;
        });
    }

    pub fn init(&mut self, fiber: *const FiberContext) -> u64 {
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
            fiber_vm.stack_push(context.self_value);
            fiber_vm.prepare_frame(
                1,
                true,
                context,
                context.outer.map(|c| c.into()),
                context.iseq_ref,
            );
            if context.iseq_ref.lvars > 0 {
                context[0] = val;
            }
            #[cfg(feature = "trace")]
            fiber_vm.dump_current_frame();
            match fiber_vm.run_loop() {
                Ok(()) => Ok(fiber_vm.stack_pop()),
                Err(err) => Err(err),
            }
        }
        FiberKind::Enum(info) => fiber_vm.enumerator_fiber(info.receiver, &info.args, info.method),
    };
    #[cfg(any(feature = "trace", feature = "trace-func"))]
    {
        eprintln!("<=== yield {:?} and terminate fiber.", res);
    }
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
        (*fiber).stack.deallocate();
        //(*fiber).result = (*val).clone();
    }
    asm::yield_context(fiber);
}
