pub use crate::*;
use std::ops::{Index, IndexMut, Range};

const LVAR_ARRAY_SIZE: usize = 4;

#[derive(Debug, Clone)]
pub struct Context {
    pub self_value: Value,
    pub block: Block,
    lvar_ary: [Value; LVAR_ARRAY_SIZE],
    lvar_vec: Vec<Value>,
    pub iseq_ref: Option<ISeqRef>,
    /// Context of outer scope.
    pub outer: Option<ContextRef>,
    pub moved_to_heap: Option<ContextRef>,
    pub on_stack: bool,
    pub kind: ISeqKind,
}

pub type ContextRef = Ref<Context>;

impl Index<LvarId> for Context {
    type Output = Value;

    fn index(&self, index: LvarId) -> &Self::Output {
        &self[index.as_usize()]
    }
}

impl Index<usize> for Context {
    type Output = Value;

    fn index(&self, index: usize) -> &Self::Output {
        if index < LVAR_ARRAY_SIZE {
            &self.lvar_ary[index]
        } else {
            &self.lvar_vec[index - LVAR_ARRAY_SIZE]
        }
    }
}

impl IndexMut<LvarId> for Context {
    fn index_mut(&mut self, index: LvarId) -> &mut Self::Output {
        &mut self[index.as_usize()]
    }
}

impl IndexMut<usize> for Context {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index < LVAR_ARRAY_SIZE {
            &mut self.lvar_ary[index]
        } else {
            &mut self.lvar_vec[index - LVAR_ARRAY_SIZE]
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
        if let (true, Some(_heap)) = (self.on_stack, self.moved_to_heap) {
            panic!("Warining: ref to stack for heap-allocated context.");
        }
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
            Block::Proc(proc) => proc.mark(alloc),
            Block::Block(_, outer) => outer.get_current().mark(alloc),
            Block::None => {}
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
        block: Block,
        iseq_ref: ISeqRef,
        outer: Option<ContextRef>,
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
            outer,
            moved_to_heap: None,
            on_stack: true,
            kind: iseq_ref.kind,
        }
    }

    pub fn new_noiseq() -> Self {
        Context {
            self_value: Value::nil(),
            block: Block::None,
            lvar_ary: [Value::uninitialized(); LVAR_ARRAY_SIZE],
            lvar_vec: vec![],
            iseq_ref: None,
            outer: None,
            moved_to_heap: None,
            on_stack: true,
            kind: ISeqKind::Block,
        }
    }

    pub fn dump(&self) {
        println!(
            "{} context:{:?} outer:{:?}",
            if self.on_stack { "STACK" } else { "HEAP " },
            self as *const Context,
            self.outer
        );
        assert!(!self.on_stack || self.moved_to_heap.is_none());
        println!("  self: {:#?}", self.self_value);
        match self.iseq_ref {
            Some(iseq_ref) => {
                for i in 0..iseq_ref.lvars {
                    let id = i.into();
                    let (k, _) = iseq_ref
                        .lvar
                        .table()
                        .iter()
                        .find(|(_, v)| **v == id)
                        .unwrap();
                    println!("  lvar({}): {:?} {:#?}", id.as_u32(), k, self[id]);
                }
            }
            None => {}
        }
    }

    fn copy_from_slice(&mut self, index: usize, slice: &[Value]) {
        let len = slice.len();
        if index + len <= LVAR_ARRAY_SIZE {
            self.lvar_ary[index..index + len].copy_from_slice(slice);
        } else if index >= LVAR_ARRAY_SIZE {
            self.lvar_vec[index - LVAR_ARRAY_SIZE..index + len - LVAR_ARRAY_SIZE]
                .copy_from_slice(slice)
        } else {
            self.lvar_ary[index..LVAR_ARRAY_SIZE]
                .copy_from_slice(&slice[..LVAR_ARRAY_SIZE - index]);
            self.lvar_vec[0..index + len - LVAR_ARRAY_SIZE]
                .copy_from_slice(&slice[LVAR_ARRAY_SIZE - index..])
        }
    }

    pub fn copy_from_slice0(&mut self, slice: &[Value]) {
        let len = slice.len();
        if len <= LVAR_ARRAY_SIZE {
            self.lvar_ary[0..len].copy_from_slice(slice);
        } else {
            self.lvar_ary[0..LVAR_ARRAY_SIZE].copy_from_slice(&slice[..LVAR_ARRAY_SIZE]);
            self.lvar_vec[0..len - LVAR_ARRAY_SIZE].copy_from_slice(&slice[LVAR_ARRAY_SIZE..])
        }
    }

    fn fill(&mut self, range: Range<usize>, val: Value) {
        for i in range {
            self[i] = val;
        }
    }

    pub fn from_args(
        vm: &mut VM,
        self_value: Value,
        iseq: ISeqRef,
        args: &Args,
        outer: Option<ContextRef>,
    ) -> Result<Self, RubyError> {
        let mut context = Context::new(self_value, args.block.clone(), iseq, outer);
        if iseq.opt_flag {
            if !args.kw_arg.is_nil() {
                return Err(RubyError::argument("Undefined keyword."));
            };
            if iseq.is_block() {
                context.from_args_opt_block(&iseq.params, args)?;
            } else {
                let req_len = iseq.params.req;
                args.check_args_num(req_len)?;
                context.copy_from_slice0(args);
            }
            return Ok(context);
        }
        let params = &iseq.params;
        let mut keyword_flag = false;
        let kw = if params.keyword.is_empty() && !params.kwrest {
            // if no keyword param nor kwrest param exists in formal parameters,
            // make Hash.
            args.kw_arg
        } else {
            keyword_flag = !args.kw_arg.is_nil();
            Value::nil()
        };
        if !iseq.is_block() {
            let min = params.req + params.post;
            let kw = if kw.is_nil() { 0 } else { 1 };
            if params.rest.is_some() {
                if min > kw {
                    args.check_args_min(min - kw)?;
                }
            } else {
                args.check_args_range_ofs(kw, min, min + params.opt)?;
            }
        }

        context.set_arguments(args, kw);
        if params.kwrest || keyword_flag {
            let mut kwrest = FxHashMap::default();
            if keyword_flag {
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
                                return Err(RubyError::argument("Undefined keyword."));
                            }
                        }
                    };
                }
            };
            if let Some(id) = iseq.lvar.kwrest_param() {
                context[id] = Value::hash_from_map(kwrest);
            }
        };
        if let Some(id) = iseq.lvar.block_param() {
            context[id] = match &args.block {
                Block::Block(method, ctx) => {
                    let proc_context = vm.create_block_context(*method, *ctx)?;
                    Value::procobj(proc_context)
                }
                Block::Proc(proc) => *proc,
                Block::None => Value::nil(),
            }
        }
        Ok(context)
    }

    fn set_arguments(&mut self, args: &Args, kw_arg: Value) {
        let iseq = self.iseq_ref.unwrap();
        let req_len = iseq.params.req;
        let post_len = iseq.params.post;
        if iseq.is_block() && args.len() == 1 && req_len + post_len > 1 {
            if let Some(ary) = args[0].as_array() {
                self.fill_arguments(&ary.elements, &iseq.params, kw_arg);
                return;
            }
        }

        self.fill_arguments(args, &iseq.params, kw_arg);
    }

    fn fill_arguments(&mut self, args: &[Value], params: &ISeqParams, kw_arg: Value) {
        let args_len = args.len();
        let mut kw_len = if kw_arg.is_nil() { 0 } else { 1 };
        let req_len = params.req;
        let rest_len = if params.rest == Some(true) { 1 } else { 0 };
        let post_len = params.post;
        let arg_len = args_len + kw_len - post_len;
        let optreq_len = req_len + params.opt;
        if post_len != 0 {
            // fill post_req params.
            let post_pos = optreq_len + rest_len;
            self.copy_from_slice(post_pos, &args[arg_len..args_len]);
            if kw_len == 1 {
                // fill keyword params as a hash.
                self[post_pos + post_len - 1] = kw_arg;
                kw_len = 0;
            }
        }
        let req_opt = std::cmp::min(optreq_len, arg_len);
        if req_opt != 0 {
            // fill req and opt params.
            self.copy_from_slice0(&args[0..req_opt - kw_len]);
            if kw_len == 1 {
                // fill keyword params as a hash.
                self[req_opt - 1] = kw_arg;
                kw_len = 0;
            }
            if req_opt < req_len {
                // fill rest req params with nil.
                self.fill(req_opt..req_len, Value::nil());
            }
        }
        if rest_len == 1 {
            let ary = if optreq_len >= arg_len {
                vec![]
            } else {
                let mut v = args[optreq_len..arg_len - kw_len].to_vec();
                if kw_len == 1 {
                    v.push(kw_arg);
                }
                v
            };
            self[optreq_len] = Value::array_from(ary);
        }
    }

    fn from_args_opt_block(&mut self, params: &ISeqParams, args: &Args) -> Result<(), RubyError> {
        #[inline]
        fn fill_arguments_opt(context: &mut Context, args: &[Value], req_len: usize) {
            let args_len = args.len();
            if req_len <= args_len {
                // fill req params.
                context.copy_from_slice0(&args[0..req_len]);
            } else {
                // fill req params.
                context.copy_from_slice0(args);
                // fill the remaining req params with nil.
                context.fill(args_len..req_len, Value::nil());
            }
        }

        let args_len = args.len();
        let req_len = params.req;
        if args_len == 1 && req_len > 1 {
            if let Some(ary) = args[0].as_array() {
                // if a single array argument is given for the block with multiple formal parameters,
                // the arguments must be expanded.
                fill_arguments_opt(self, &ary.elements, req_len);
                return Ok(());
            };
        }

        fill_arguments_opt(self, args, req_len);
        Ok(())
    }
}

impl ContextRef {
    pub fn new_heap(
        self_value: Value,
        block: Block,
        iseq_ref: ISeqRef,
        outer: Option<ContextRef>,
    ) -> Self {
        let mut context = Context::new(self_value, block, iseq_ref, outer);
        context.on_stack = false;
        let mut ctxref = ContextRef::new(context);
        ctxref.moved_to_heap = Some(ctxref);
        ctxref
    }

    pub fn get_current(self) -> Self {
        match self.moved_to_heap {
            None => self,
            Some(heap) => heap,
        }
    }

    pub fn move_to_heap(mut self) -> Self {
        if !self.on_stack {
            return self;
        };
        let mut heap_context = self.dup();
        heap_context.on_stack = false;
        heap_context.moved_to_heap = Some(heap_context);
        self.moved_to_heap = Some(heap_context);
        heap_context
    }

    pub fn adjust_lvar_size(&mut self) {
        let len = self.iseq_ref.unwrap().lvars;
        if LVAR_ARRAY_SIZE != len {
            //panic!();
        }
    }
}
