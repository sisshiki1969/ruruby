pub use crate::*;
use std::ops::{Index, IndexMut, Range};

const LVAR_ARRAY_SIZE: usize = 4;

#[derive(Debug, Clone)]
pub struct Context {
    pub self_value: Value,
    pub block: Option<Block>,
    lvar_ary: [Value; LVAR_ARRAY_SIZE],
    lvar_vec: Vec<Value>,
    pub iseq_ref: Option<ISeqRef>,
    /// Context of outer scope.
    pub outer: Option<ContextRef>,
    /// Context of caller.
    pub caller: Option<ContextRef>,
    pub on_stack: bool,
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

impl Into<ContextRef> for &Context {
    fn into(self) -> ContextRef {
        Ref::from_ref(self)
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
        match self.block {
            Some(Block::Proc(proc)) => proc.mark(alloc),
            _ => {}
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
        block: Option<Block>,
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
            kind: iseq_ref.kind,
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
    ) -> Result<Self, RubyError> {
        let caller = vm.latest_context();
        if iseq.opt_flag {
            if !args.kw_arg.is_nil() {
                return Err(VM::error_argument("Undefined keyword."));
            };
            let mut context = Context::new(self_value, args.block.clone(), iseq, outer, caller);
            if iseq.is_block() {
                context.from_args_opt_block(&iseq.params, args)?;
            } else {
                let req_len = iseq.params.req;
                args.check_args_num(req_len)?;
                for i in 0..req_len {
                    context[i] = args[i];
                }
            }
            return Ok(context);
        }
        let mut context = Context::new(self_value, args.block.clone(), iseq, outer, caller);
        let params = &iseq.params;
        let kw = if params.keyword.is_empty() && !params.kwrest {
            // if no keyword param nor kwrest param exists in formal parameters,
            // make Hash.
            args.kw_arg
        } else {
            Value::nil()
        };
        if !iseq.is_block() {
            let min = params.req + params.post;
            let kw = if kw.is_nil() { 0 } else { 1 };
            if params.rest {
                if min > kw {
                    args.check_args_min(min - kw)?;
                }
            } else {
                args.check_args_range_ofs(kw, min, min + params.opt)?;
            }
        }
        context.set_arguments(args, kw);
        let mut kwrest = FxHashMap::default();
        if !args.kw_arg.is_nil() && kw.is_nil() {
            let keyword = args.kw_arg.as_hash().unwrap();
            for (k, v) in keyword.iter() {
                let id = k.as_symbol().unwrap();
                match params.keyword.get(&id) {
                    Some(lvar) => {
                        context[*lvar] = v;
                    }
                    None => {
                        if params.kwrest {
                            kwrest.insert(HashKey(k), v);
                        } else {
                            return Err(VM::error_argument("Undefined keyword."));
                        }
                    }
                };
            }
        };
        if let Some(id) = iseq.lvar.kwrest_param() {
            context[id] = Value::hash_from_map(kwrest);
        }
        if let Some(id) = iseq.lvar.block_param() {
            context[id] = match &args.block {
                Some(Block::Method(method)) => {
                    let proc_context = vm.create_block_context(*method)?;
                    Value::procobj(proc_context)
                }
                Some(Block::Proc(proc)) => *proc,
                None => Value::nil(),
            }
        }
        Ok(context)
    }

    fn from_args_opt_block(&mut self, params: &ISeqParams, args: &Args) -> Result<(), RubyError> {
        #[inline]
        fn fill_arguments_opt(
            context: &mut Context,
            args: &(impl Index<usize, Output = Value> + Index<Range<usize>, Output = [Value]>),
            args_len: usize,
            req_len: usize,
        ) {
            if req_len <= args_len {
                // fill req params.
                for i in 0..req_len {
                    context[i] = args[i];
                }
            } else {
                // fill req params.
                for i in 0..args_len {
                    context[i] = args[i];
                }
                // fill the remaining req params with nil.
                for i in args_len..req_len {
                    context[i] = Value::nil();
                }
            }
        }

        let args_len = args.len();
        let req_len = params.req;

        if args_len == 1 && req_len > 1 {
            match args[0].as_array() {
                // if a single array argument is given for the block which has multiple parameters,
                // the arguments must be expanded.
                Some(ary) => {
                    let args = &ary.elements;
                    fill_arguments_opt(self, args, args.len(), req_len);
                    return Ok(());
                }
                _ => {}
            }
        }

        fill_arguments_opt(self, args, args_len, req_len);
        Ok(())
    }

    fn set_arguments(&mut self, args: &Args, kw_arg: Value) {
        let iseq = self.iseq_ref.unwrap();
        let req_len = iseq.params.req;
        let post_len = iseq.params.post;
        if iseq.is_block() && args.len() == 1 && req_len + post_len > 1 {
            match args[0].as_array() {
                Some(ary) => {
                    let args = &ary.elements;
                    self.fill_arguments(args, args.len(), &iseq.params, kw_arg);
                    return;
                }
                _ => {}
            }
        }

        self.fill_arguments(args, args.len(), &iseq.params, kw_arg);
    }

    fn fill_arguments(
        &mut self,
        args: &(impl Index<usize, Output = Value> + Index<Range<usize>, Output = [Value]>),
        args_len: usize,
        params: &ISeqParams,
        kw_arg: Value,
    ) {
        //let params = &iseq.params;
        let mut kw_len = if kw_arg.is_nil() { 0 } else { 1 };
        let req_len = params.req;
        let opt_len = params.opt;
        let rest_len = if params.rest { 1 } else { 0 };
        let post_len = params.post;
        let arg_len = args_len + kw_len;
        if post_len != 0 {
            // fill post_req params.
            let post_pos = req_len + opt_len + rest_len;
            for i in 0..post_len - kw_len {
                self[post_pos + i] = args[arg_len - post_len + i];
            }
            if kw_len == 1 {
                // fill keyword params as a hash.
                self[post_pos + post_len - 1] = kw_arg;
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
                self[req_opt - 1] = kw_arg;
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
                    v.push(kw_arg);
                }
                v
            };
            self[req_len + opt_len] = Value::array_from(ary);
        }
    }
}

impl ContextRef {
    pub fn new_heap(
        self_value: Value,
        block: Option<Block>,
        iseq_ref: ISeqRef,
        outer: Option<ContextRef>,
        caller: Option<ContextRef>,
    ) -> Self {
        let mut context = Context::new(self_value, block, iseq_ref, outer, caller);
        context.on_stack = false;
        ContextRef::new(context)
    }

    pub fn adjust_lvar_size(&mut self) {
        let len = self.iseq_ref.unwrap().lvars;
        if LVAR_ARRAY_SIZE != len {
            //panic!();
        }
    }

    pub fn get_outermost(&self) -> ContextRef {
        let mut context = *self;
        loop {
            context = match context.outer {
                Some(context) => context,
                None => return context,
            };
        }
    }
}
