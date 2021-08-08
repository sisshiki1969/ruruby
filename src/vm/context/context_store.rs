use super::*;
use std::alloc::{alloc, Layout};
use std::cell::RefCell;

const INITIAL_STACK_SIZE: usize = 256;

thread_local!(
    static CONTEXT_STORE: RefCell<Vec<*mut Context>> = RefCell::new(vec![]);
);

#[derive(Debug, Clone)]
pub struct ContextStore {
    buf: *mut Context,
    sp: usize,
    sp_ptr: *mut Context,
}

impl Drop for ContextStore {
    fn drop(&mut self) {
        CONTEXT_STORE.with(|m| m.borrow_mut().push(self.buf));
        //unsafe { dealloc(self.buf as *mut _, layout) };
    }
}

impl ContextStore {
    /// Allocate new virtual stack.
    pub fn new() -> Self {
        let buf = CONTEXT_STORE.with(|m| match m.borrow_mut().pop() {
            None => {
                let layout = Layout::from_size_align(
                    INITIAL_STACK_SIZE * std::mem::size_of::<Context>(),
                    INITIAL_STACK_SIZE,
                )
                .unwrap();
                unsafe { alloc(layout) as *mut Context }
            }
            Some(buf) => buf,
        });
        Self {
            buf,
            sp: 0,
            sp_ptr: buf,
        }
    }

    /// Push `context` to the virtual stack, and return a context handle.
    pub fn push(&mut self, context: Context) -> ContextRef {
        unsafe {
            if self.sp >= INITIAL_STACK_SIZE {
                panic!("stack overflow")
            };
            let ptr = self.sp_ptr;
            std::ptr::write(ptr, context);
            self.sp += 1;
            self.sp_ptr = ptr.add(1);
            ContextRef::from_ptr(ptr)
        }
    }

    /// Push `context` to the virtual stack, and return a context handle.
    pub fn push_with(
        &mut self,
        self_value: Value,
        block: Option<Block>,
        iseq: ISeqRef,
        outer: Option<ContextRef>,
    ) -> ContextRef {
        unsafe {
            if self.sp >= INITIAL_STACK_SIZE {
                panic!("stack overflow")
            };
            let ptr = self.sp_ptr;
            let lvar_num = iseq.lvars;
            let mut lvar_ary = std::ptr::addr_of_mut!((*ptr).lvar_ary) as *mut Value;
            let lvar_vec = std::ptr::addr_of_mut!((*ptr).lvar_vec);
            //(*ptr).lvar_ary.fill(Value::nil());
            if lvar_num > LVAR_ARRAY_SIZE {
                for _ in 0..LVAR_ARRAY_SIZE {
                    std::ptr::write(lvar_ary, Value::nil());
                    lvar_ary = lvar_ary.add(1);
                }
                let v = vec![Value::nil(); lvar_num - LVAR_ARRAY_SIZE];
                std::ptr::write(lvar_vec, v);
            } else {
                for _ in 0..lvar_num {
                    std::ptr::write(lvar_ary, Value::nil());
                    lvar_ary = lvar_ary.add(1);
                }
                std::ptr::write(lvar_vec, Vec::new());
            };
            for i in &iseq.lvar.optkw {
                (*ptr)[*i] = Value::uninitialized();
            }
            (*ptr).self_value = self_value;
            (*ptr).block = block;
            (*ptr).iseq_ref = Some(iseq);
            (*ptr).outer = outer;
            (*ptr).caller = None;
            (*ptr).on_stack = CtxKind::Stack;
            //(*ptr).cur_pc = ISeqPos::from(0);
            //(*ptr).prev_pc = ISeqPos::from(0);
            //(*ptr).prev_stack_len = 0;
            (*ptr).called = false;
            (*ptr).use_value = true;
            (*ptr).module_function = false;
            self.sp += 1;
            self.sp_ptr = ptr.add(1);
            ContextRef::from_ptr(ptr)
        }
    }

    /// Pop `context` from the virtual stack.
    pub fn pop(&mut self, _context: ContextRef) {
        unsafe {
            if self.sp == 0 {
                return;
            }
            let ptr = self.sp_ptr.sub(1);
            #[cfg(debug_assertions)]
            {
                let ctx = ContextRef::from_ptr(ptr);
                match ctx.on_stack {
                    CtxKind::Stack => {
                        assert_eq!(ctx, _context);
                    }
                    CtxKind::Dead(ctx) => {
                        assert_eq!(ctx, _context);
                    }
                    _ => unreachable!(),
                }
            }
            (*ptr).lvar_vec = Vec::new();
            self.sp -= 1;
            self.sp_ptr = ptr;
        }
    }

    #[allow(dead_code)]
    #[cfg(not(tarpaulin_include))]
    pub fn dump(&self) {
        eprintln!("dump context stack");
        for i in 0..self.sp {
            eprint!("[{}]", i);
            ContextRef::from_ptr(unsafe { self.buf.add(self.sp - 1 - i) }).pp();
        }
    }
}
