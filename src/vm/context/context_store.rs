use super::*;
use std::alloc::{alloc, Layout};
use std::cell::RefCell;

const INITIAL_STACK_SIZE: usize = 256;

thread_local!(
    static CONTEXT_STORE: RefCell<Vec<*mut HeapContext>> = RefCell::new(vec![]);
);

#[derive(Debug, Clone)]
pub struct ContextStore {
    buf: *mut HeapContext,
    sp: usize,
    sp_ptr: *mut HeapContext,
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
                    INITIAL_STACK_SIZE * std::mem::size_of::<HeapContext>(),
                    INITIAL_STACK_SIZE,
                )
                .unwrap();
                unsafe { alloc(layout) as *mut HeapContext }
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
    pub fn push_with(
        &mut self,
        self_value: Value,
        block: Option<Block>,
        iseq: ISeqRef,
        outer: Option<HeapCtxRef>,
    ) -> HeapCtxRef {
        unsafe {
            if self.sp >= INITIAL_STACK_SIZE {
                panic!("stack overflow")
            };
            let ptr = self.sp_ptr;
            //let lvar_num = iseq.lvars;
            let lvar_vec = std::ptr::addr_of_mut!((*ptr).lvar);
            let v = vec![];
            std::ptr::write(lvar_vec, v);
            /*for i in &iseq.lvar.kw {
                (*ptr)[*i] = Value::uninitialized();
            }*/
            (*ptr).self_value = self_value;
            (*ptr).block = block;
            (*ptr).iseq_ref = iseq;
            (*ptr).outer = outer;
            (*ptr).on_stack = CtxKind::Stack;
            self.sp += 1;
            self.sp_ptr = ptr.add(1);
            HeapCtxRef::from_ptr(ptr)
        }
    }

    /// Pop `context` from the virtual stack.
    pub fn pop(&mut self, _context: HeapCtxRef) {
        unsafe {
            if self.sp == 0 {
                return;
            }
            let ptr = self.sp_ptr.sub(1);
            #[cfg(debug_assertions)]
            {
                let ctx = HeapCtxRef::from_ptr(ptr);
                match ctx.on_stack {
                    CtxKind::Stack => assert_eq!(ctx, _context),
                    CtxKind::Dead(ctx) => assert_eq!(ctx, _context),
                    _ => unreachable!(),
                }
            }
            (*ptr).lvar.clear();
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
            HeapCtxRef::from_ptr(unsafe { self.buf.add(self.sp - 1 - i) }).pp();
        }
    }
}
