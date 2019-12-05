pub use crate::vm::*;
use core::ptr::NonNull;

#[derive(Debug, Clone)]
pub struct Context {
    pub self_value: PackedValue,
    pub lvar_scope: [PackedValue; 16],
    pub iseq_ref: ISeqRef,
    pub pc: usize,
    pub outer: Option<ContextRef>,
}

pub type ContextRef = Ref<Context>;

impl Context {
    pub fn new(self_value: PackedValue, iseq_ref: ISeqRef) -> Self {
        //let lvar_num = iseq_ref.lvars;
        Context {
            self_value,
            lvar_scope: [PackedValue::nil(); 16],
            iseq_ref,
            pc: 0,
            outer: None,
        }
    }
}

impl ContextRef {
    pub fn from(self_value: PackedValue, iseq_ref: ISeqRef) -> Self {
        ContextRef::new(Context::new(self_value, iseq_ref))
    }

    pub fn new_local(info: &Context) -> Self {
        let boxed = info as *const Context as *mut Context;
        Ref(unsafe { NonNull::new_unchecked(boxed) })
    }
    pub fn dup(&self) -> Context {
        unsafe { (*self.0.as_ptr()).clone() }
    }
}
