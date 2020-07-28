pub use crate::*;
use std::ops::{Index, IndexMut, Range};

const LVAR_ARRAY_SIZE: usize = 32;

#[derive(Debug, Clone)]
pub struct Context {
    pub self_value: Value,
    pub block: Option<MethodRef>,
    lvar_ary: [Value; LVAR_ARRAY_SIZE],
    lvar_vec: Vec<Value>,
    pub iseq_ref: Option<ISeqRef>,
    //pub pc: usize,
    /// Context of outer scope.
    pub outer: Option<ContextRef>,
    /// Context of caller.
    pub caller: Option<ContextRef>,
    pub on_stack: bool,
    //pub stack_len: usize,
    pub kind: ISeqKind,
}

pub type ContextRef = Ref<Context>;

impl Index<LvarId> for Context {
    type Output = Value;

    fn index(&self, index: LvarId) -> &Self::Output {
        let i = index.as_usize();
        &self[i]
    }
}

impl Index<usize> for Context {
    type Output = Value;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe {
            if index < LVAR_ARRAY_SIZE {
                self.lvar_ary.get_unchecked(index)
            } else {
                self.lvar_vec.get_unchecked(index - LVAR_ARRAY_SIZE)
            }
        }
    }
}

impl IndexMut<LvarId> for Context {
    fn index_mut(&mut self, index: LvarId) -> &mut Self::Output {
        let i = index.as_usize();
        &mut self[i]
    }
}

impl IndexMut<usize> for Context {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe {
            if index < LVAR_ARRAY_SIZE {
                self.lvar_ary.get_unchecked_mut(index)
            } else {
                self.lvar_vec.get_unchecked_mut(index - LVAR_ARRAY_SIZE)
            }
        }
    }
}

impl GC for Context {
    fn mark(&self, alloc: &mut Allocator) {
        self.self_value.mark(alloc);
        match self.iseq_ref {
            Some(iseq_ref) => {
                for i in 0..iseq_ref.lvars {
                    self[i].mark(alloc);
                }
            }
            None => {}
        }
        match self.outer {
            Some(c) => c.mark(alloc),
            None => {}
        }
    }
}

impl Context {
    pub fn new(
        self_value: Value,
        block: Option<MethodRef>,
        iseq_ref: ISeqRef,
        outer: Option<ContextRef>,
        caller: Option<ContextRef>,
    ) -> Self {
        let lvar_num = iseq_ref.lvars;
        let lvar_vec = if lvar_num > LVAR_ARRAY_SIZE {
            vec![Value::uninitialized(); lvar_num - LVAR_ARRAY_SIZE]
        } else {
            Vec::new()
        };
        Context {
            self_value,
            block,
            lvar_ary: [Value::uninitialized(); LVAR_ARRAY_SIZE],
            lvar_vec,
            iseq_ref: Some(iseq_ref),
            //pc: 0,
            outer,
            caller,
            on_stack: true,
            //stack_len: 0,
            kind: iseq_ref.kind.clone(),
        }
    }

    pub fn new_noiseq() -> Self {
        Context {
            self_value: Value::nil(),
            block: None,
            lvar_ary: [Value::uninitialized(); LVAR_ARRAY_SIZE],
            lvar_vec: vec![],
            iseq_ref: None,
            //pc: 0,
            outer: None,
            caller: None,
            on_stack: true,
            //stack_len: 0,
            kind: ISeqKind::Block,
        }
    }

    pub fn from_args(
        vm: &mut VM,
        self_value: Value,
        iseq: ISeqRef,
        args: &Args,
        outer: Option<ContextRef>,
        caller: Option<ContextRef>,
    ) -> Result<Self, RubyError> {
        if iseq.opt_flag {
            return Context::from_args_opt(vm, self_value, iseq, args, outer, caller);
        }
        let mut context = Context::new(self_value, args.block, iseq, outer, caller);
        let params = &iseq.params;
        let kw = if params.keyword_params.is_empty() {
            args.kw_arg
        } else {
            None
        };
        if !iseq.is_block() {
            let len = args.len() + if kw.is_some() { 1 } else { 0 };
            let min = params.req_params + params.post_params;
            if params.rest_param {
                vm.check_args_min(len, min)?;
            } else {
                vm.check_args_range(len, min, min + params.opt_params)?;
            }
        }
        context.set_arguments(&vm.globals, args, kw);
        match args.kw_arg {
            Some(kw_arg) if kw.is_none() => {
                let keyword = kw_arg.as_hash().unwrap();
                for (k, v) in keyword.iter() {
                    let id = k.as_symbol().unwrap();
                    match params.keyword_params.get(&id) {
                        Some(lvar) => {
                            context[*lvar] = v;
                        }
                        None => return Err(vm.error_argument("Undefined keyword.")),
                    };
                }
            }
            _ => {}
        };
        if let Some(id) = iseq.lvar.block_param() {
            context[id] = match args.block {
                Some(block) => {
                    let proc_context = vm.create_block_context(block)?;
                    Value::procobj(&vm.globals, proc_context)
                }
                None => Value::nil(),
            }
        }
        Ok(context)
    }

    pub fn from_args_opt(
        vm: &mut VM,
        self_value: Value,
        iseq: ISeqRef,
        args: &Args,
        outer: Option<ContextRef>,
        caller: Option<ContextRef>,
    ) -> Result<Self, RubyError> {
        let mut context = Context::new(self_value, args.block, iseq, outer, caller);
        let req_len = iseq.params.req_params;
        vm.check_args_num(args.len(), req_len)?;

        for i in 0..req_len {
            context[i] = args[i];
        }

        if args.kw_arg.is_some() {
            return Err(vm.error_argument("Undefined keyword."));
        };
        Ok(context)
    }

    fn set_arguments(&mut self, globals: &Globals, args: &Args, kw_arg: Option<Value>) {
        let iseq = self.iseq_ref.unwrap();
        let req_len = iseq.params.req_params;
        let post_len = iseq.params.post_params;
        match self.kind {
            ISeqKind::Block if args.len() == 1 && req_len + post_len > 1 => {
                match args[0].as_array() {
                    Some(ary) => {
                        let args = &ary.elements;
                        self.fill_arguments(globals, args, args.len(), iseq, kw_arg);
                        return;
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        self.fill_arguments(globals, args, args.len(), iseq, kw_arg);
    }

    fn fill_arguments(
        &mut self,
        globals: &Globals,
        args: &(impl Index<usize, Output = Value> + Index<Range<usize>, Output = [Value]>),
        args_len: usize,
        iseq: ISeqRef,
        kw_arg: Option<Value>,
    ) {
        let params = &iseq.params;
        let mut kw_len = if kw_arg.is_some() { 1 } else { 0 };
        let req_len = params.req_params;
        let opt_len = params.opt_params;
        let rest_len = if params.rest_param { 1 } else { 0 };
        let post_len = params.post_params;
        let post_pos = req_len + opt_len + rest_len;
        let arg_len = args_len + kw_len;
        if post_len != 0 {
            // fill post_req params.
            for i in 0..post_len - kw_len {
                self[post_pos + i] = args[arg_len - post_len + i];
            }
            if kw_len == 1 {
                // fill keyword params as a hash.
                self[post_pos + post_len - 1] = kw_arg.unwrap();
                kw_len = 0;
            }
        }
        let req_opt = std::cmp::min(opt_len + req_len, arg_len - post_len);
        if req_opt != 0 {
            // fill req and opt params.
            for i in 0..req_opt - kw_len {
                self[i] = args[i];
            }
            if kw_len == 1 {
                // fill keyword params as a hash.
                self[req_opt - 1] = kw_arg.unwrap();
                kw_len = 0;
            }
            if req_opt < req_len {
                // fill rest req params with nil.
                for i in req_opt..req_len {
                    self[i] = Value::nil();
                }
            }
        }
        if rest_len == 1 {
            let ary = if req_len + opt_len + post_len >= arg_len {
                vec![]
            } else {
                let mut v = args[req_len + opt_len..arg_len - post_len - kw_len].to_vec();
                if kw_len == 1 {
                    v.push(kw_arg.unwrap());
                }
                v
            };
            let val = Value::array_from(globals, ary);
            self[req_len + opt_len] = val;
        }
    }
}

impl ContextRef {
    pub fn from(
        self_value: Value,
        block: Option<MethodRef>,
        iseq_ref: ISeqRef,
        outer: Option<ContextRef>,
        caller: Option<ContextRef>,
    ) -> Self {
        let mut context = Context::new(self_value, block, iseq_ref, outer, caller);
        context.on_stack = false;
        ContextRef::new(context)
    }

    pub fn from_local(info: &Context) -> Self {
        Ref::from_ref(info)
    }

    pub fn adjust_lvar_size(&mut self) {
        let len = self.iseq_ref.unwrap().lvars;
        if LVAR_ARRAY_SIZE < len {
            for _ in 0..len - LVAR_ARRAY_SIZE {
                self.lvar_vec.push(Value::nil());
            }
        }
    }
}
