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
            vec![PackedValue::uninitialized(); lvar_num - LVAR_ARRAY_SIZE]
        } else {
            Vec::new()
        };
        Context {
            self_value,
            block,
            lvar_scope: [PackedValue::uninitialized(); LVAR_ARRAY_SIZE],
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

    pub fn set_lvar(&mut self, id: usize, val: PackedValue) {
        if id < LVAR_ARRAY_SIZE {
            self.lvar_scope[id] = val;
        } else {
            self.ext_lvar[id - LVAR_ARRAY_SIZE] = val;
        }
    }

    pub fn get_mut_lvar(&mut self, id: LvarId) -> &mut PackedValue {
        let id = id.as_usize();
        if id < LVAR_ARRAY_SIZE {
            &mut self.lvar_scope[id]
        } else {
            eprintln!(
                "id:{} LVAR_ARRAY_SIZE:{} ext_lvar:{}",
                id,
                LVAR_ARRAY_SIZE,
                self.ext_lvar.len()
            );
            &mut self.ext_lvar[id - LVAR_ARRAY_SIZE]
        }
    }

    pub fn set_arguments(&mut self, globals: &Globals, args: VecArray) {
        let arg_len = args.len();
        let req_len = self.iseq_ref.req_params;
        let opt_len = self.iseq_ref.opt_params;
        let rest_len = if self.iseq_ref.rest_param { 1 } else { 0 };
        let post_len = self.iseq_ref.post_params;
        let post_pos = req_len + opt_len + rest_len;
        for i in 0..std::cmp::min(opt_len + req_len, arg_len - post_len) {
            self.set_lvar(i, args[i]);
        }
        for i in 0..post_len {
            self.set_lvar(post_pos + i, args[arg_len - post_len + i]);
        }
        if rest_len == 1 {
            let ary = if req_len + opt_len + post_len >= arg_len {
                vec![]
            } else {
                args.get_slice(req_len + opt_len, arg_len - post_len)
                    .to_vec()
            };
            let val = PackedValue::array_from(globals, ary);
            self.set_lvar(req_len + opt_len, val);
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
