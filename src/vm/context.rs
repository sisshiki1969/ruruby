pub use crate::vm::*;
use core::ptr::NonNull;

const LVAR_ARRAY_SIZE: usize = 8;

#[derive(Debug, Clone)]
pub struct Context {
    pub self_value: PackedValue,
    pub block: Option<MethodRef>,
    lvar_scope: [PackedValue; LVAR_ARRAY_SIZE],
    ext_lvar: Vec<PackedValue>,
    pub iseq_ref: ISeqRef,
    pub pc: usize,
    pub outer: Option<ContextRef>,
    pub on_stack: bool,
}

pub type ContextRef = Ref<Context>;

impl Context {
    pub fn new(
        self_value: PackedValue,
        block: Option<MethodRef>,
        iseq_ref: ISeqRef,
        outer: Option<ContextRef>,
    ) -> Self {
        let lvar_num = iseq_ref.lvars;
        let ext_lvar = if lvar_num > LVAR_ARRAY_SIZE {
            vec![PackedValue::nil(); lvar_num - LVAR_ARRAY_SIZE]
        } else {
            Vec::new()
        };
        Context {
            self_value,
            block,
            lvar_scope: [PackedValue::nil(); LVAR_ARRAY_SIZE],
            ext_lvar,
            iseq_ref,
            pc: 0,
            outer,
            on_stack: true,
        }
    }

    pub fn get_lvar(&self, id: LvarId) -> PackedValue {
        let id = id.as_usize();
        if id < LVAR_ARRAY_SIZE {
            self.lvar_scope[id]
        } else {
            self.ext_lvar[id - LVAR_ARRAY_SIZE]
        }
    }

    pub fn get_mut_lvar(&mut self, id: LvarId) -> &mut PackedValue {
        let id = id.as_usize();
        if id < LVAR_ARRAY_SIZE {
            &mut self.lvar_scope[id]
        } else {
            &mut self.ext_lvar[id - LVAR_ARRAY_SIZE]
        }
    }

    pub fn set_arguments(&mut self, args: VecArray) {
        let arg_len = std::cmp::min(args.len(), self.iseq_ref.params.len());
        if arg_len <= LVAR_ARRAY_SIZE {
            self.lvar_scope[0..arg_len].clone_from_slice(args.get_slice(0, arg_len));
        } else {
            self.lvar_scope[0..LVAR_ARRAY_SIZE]
                .clone_from_slice(args.get_slice(0, LVAR_ARRAY_SIZE));
            self.ext_lvar[0..arg_len - LVAR_ARRAY_SIZE]
                .clone_from_slice(args.get_slice(LVAR_ARRAY_SIZE, arg_len));
        }
    }
}

impl ContextRef {
    pub fn from(
        self_value: PackedValue,
        block: Option<MethodRef>,
        iseq_ref: ISeqRef,
        outer: Option<ContextRef>,
    ) -> Self {
        ContextRef::new(Context::new(self_value, block, iseq_ref, outer))
    }

    pub fn new_local(info: &Context) -> Self {
        let boxed = info as *const Context as *mut Context;
        Ref(unsafe { NonNull::new_unchecked(boxed) })
    }

    pub fn dup_context(&self) -> Context {
        unsafe { (*self.0.as_ptr()).clone() }
    }

    pub fn adjust_lvar_size(&mut self) {
        let len = self.iseq_ref.lvars;
        if LVAR_ARRAY_SIZE < len {
            for _ in 0..len - LVAR_ARRAY_SIZE {
                self.ext_lvar.push(PackedValue::nil());
            }
        }
    }
}
