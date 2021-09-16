pub use crate::*;
pub use context_store::ContextStore;
use indexmap::IndexSet;
use std::ops::{Index, IndexMut, Range};

mod context_store;

const LVAR_ARRAY_SIZE: usize = 16;

#[derive(Debug, Clone)]
pub struct Context {
    pub self_value: Value,
    pub block: Option<Block>,
    lvar_ary: [Value; LVAR_ARRAY_SIZE],
    lvar_vec: Vec<Value>,
    pub iseq_ref: ISeqRef,
    /// Context of outer scope.
    pub outer: Option<ContextRef>,
    /// Previous context.
    pub caller: Option<ContextRef>,
    pub on_stack: CtxKind,
    pub cur_pc: ISeqPos,
    pub module_function: bool,
    pub delegate_args: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CtxKind {
    FromHeap,
    Heap,
    Stack,
    Dead(ContextRef),
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

impl GC for ContextRef {
    fn mark(&self, alloc: &mut Allocator) {
        self.self_value.mark(alloc);
        for i in 0..self.iseq_ref.lvars {
            self[i].mark(alloc);
        }
        if let Some(b) = &self.block {
            b.mark(alloc)
        };
        if let Some(v) = self.delegate_args {
            v.mark(alloc)
        }
        match self.outer {
            Some(c) => c.mark(alloc),
            None => {}
        }
    }
}

impl Context {
    fn new(
        self_value: Value,
        block: Option<Block>,
        iseq_ref: ISeqRef,
        outer: Option<ContextRef>,
    ) -> Self {
        let lvar_num = iseq_ref.lvars;
        let lvar_vec = if lvar_num > LVAR_ARRAY_SIZE {
            vec![Value::nil(); lvar_num - LVAR_ARRAY_SIZE]
        } else {
            Vec::new()
        };
        Context {
            self_value,
            block,
            lvar_ary: [Value::nil(); LVAR_ARRAY_SIZE],
            lvar_vec,
            iseq_ref,
            outer,
            caller: None,
            on_stack: CtxKind::Stack,
            cur_pc: ISeqPos::from(0),
            module_function: false,
            delegate_args: None,
        }
    }

    pub fn on_heap(&self) -> bool {
        match self.on_stack {
            CtxKind::FromHeap | CtxKind::Heap => true,
            _ => false,
        }
    }

    pub fn from_heap(&self) -> bool {
        self.on_stack == CtxKind::FromHeap
    }

    pub fn alive(&self) -> bool {
        match self.on_stack {
            CtxKind::Dead(_) => false,
            _ => true,
        }
    }

    pub fn is_method(&self) -> bool {
        self.iseq_ref.is_method()
    }

    #[cfg(feature = "trace")]
    pub fn dump(&self) {
        eprintln!(
            "{:?} context:{:?} outer:{:?}",
            self.on_stack, self as *const Context, self.outer
        );
        eprintln!("  iseq: {:?}", self.iseq_ref);
        eprintln!("  self: {:#?}", self.self_value);
        for (i, id) in self.iseq_ref.lvar.table().iter().enumerate() {
            eprintln!("  lvar({}): {:?} {:#?}", i, *id, self[i]);
        }
        eprintln!("  delegate: {:?}", self.delegate_args);
    }

    #[cfg(not(tarpaulin_include))]
    pub fn pp(&self) {
        println!(
            "{:?} context:{:?} outer:{:?}",
            self.on_stack, self as *const Context, self.outer
        );
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
        self.copy_from_slice(0, slice);
    }

    fn fill(&mut self, range: Range<usize>, val: Value) {
        for i in range {
            self[i] = val;
        }
    }

    fn set_arguments(&mut self, args: &[Value], kw_arg: Value) {
        let iseq = self.iseq_ref;
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
        if self.iseq_ref.lvar.delegate_param && req_opt < arg_len {
            let v = args[req_opt..arg_len].to_vec();
            self.delegate_args = Some(Value::array_from(v));
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

    pub fn fill_arguments_opt(&mut self, args: &[Value], req_len: usize) {
        let args_len = args.len();
        if req_len <= args_len {
            // fill req params.
            self.copy_from_slice0(&args[0..req_len]);
        } else {
            // fill req params.
            self.copy_from_slice0(args);
            // fill the remaining req params with nil.
            self.fill(args_len..req_len, Value::nil());
        }
    }

    fn from_args_opt_block(&mut self, params: &ISeqParams, args: &[Value]) {
        let args_len = args.len();
        let req_len = params.req;
        if args_len == 1 && req_len > 1 {
            if let Some(ary) = args[0].as_array() {
                // if a single array argument is given for the block with multiple formal parameters,
                // the arguments must be expanded.
                self.fill_arguments_opt(&ary.elements, req_len);
                return;
            };
        }

        self.fill_arguments_opt(args, req_len);
    }
}

impl ContextRef {
    pub fn new_heap(
        self_value: Value,
        block: Option<Block>,
        iseq_ref: ISeqRef,
        outer: Option<ContextRef>,
    ) -> Self {
        let mut context = Context::new(self_value, block, iseq_ref, outer);
        context.on_stack = CtxKind::FromHeap;
        for i in &iseq_ref.lvar.optkw {
            context[*i] = Value::uninitialized();
        }
        ContextRef::new(context)
    }

    pub fn get_current(self) -> Self {
        match self.on_stack {
            CtxKind::Dead(c) => c,
            _ => self,
        }
    }

    pub fn from_block(
        vm: &mut VM,
        iseq: ISeqRef,
        args: &Args2,
        outer: ContextRef,
        use_value: bool,
    ) -> Result<Self, RubyError> {
        if iseq.opt_flag {
            let context =
                ContextRef::from_opt_block(vm, iseq, &args, outer.get_current(), use_value);
            Ok(context)
        } else {
            ContextRef::from_noopt(vm, iseq, &args, outer.get_current(), use_value)
        }
    }

    pub fn from(
        vm: &mut VM,
        iseq: ISeqRef,
        args: &Args2,
        outer: impl Into<Option<ContextRef>>,
        use_value: bool,
    ) -> Result<Self, RubyError> {
        if iseq.opt_flag {
            let context = if !args.kw_arg.is_nil() {
                return Err(RubyError::argument("Undefined keyword."));
            } else if iseq.is_block() {
                Self::from_opt_block(vm, iseq, args, outer, use_value)
            } else {
                Self::from_opt_method(vm, iseq, args, outer, use_value)?
            };
            Ok(context)
        } else {
            Self::from_noopt(vm, iseq, args, outer, use_value)
        }
    }

    fn from_opt_block(
        vm: &mut VM,
        iseq: ISeqRef,
        args: &Args2,
        outer: impl Into<Option<ContextRef>>,
        use_value: bool,
    ) -> Self {
        let mut context = vm.new_stack_context_with(
            args.block.clone(),
            iseq,
            outer.into(),
            args.len(),
            use_value,
        );
        context.from_args_opt_block(&iseq.params, vm.args());
        context
    }

    fn from_opt_method(
        vm: &mut VM,
        iseq: ISeqRef,
        args: &Args2,
        outer: impl Into<Option<ContextRef>>,
        use_value: bool,
    ) -> Result<Self, RubyError> {
        let req_len = iseq.params.req;
        args.check_args_num(req_len)?;
        let mut context = vm.new_stack_context_with(
            args.block.clone(),
            iseq,
            outer.into(),
            args.len(),
            use_value,
        );
        context.copy_from_slice0(vm.args());
        Ok(context)
    }

    pub fn from_noopt(
        vm: &mut VM,
        iseq: ISeqRef,
        args: &Args2,
        outer: impl Into<Option<ContextRef>>,
        use_value: bool,
    ) -> Result<Self, RubyError> {
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
            } else if params.delegate.is_none() {
                let len = args.len() + kw;
                if min > len || len > min + params.opt {
                    return Err(RubyError::argument_wrong_range(len, min, min + params.opt));
                }
            } else {
                let len = args.len() + kw;
                if min > len {
                    return Err(RubyError::argument(format!(
                        "Wrong number of arguments. (given {}, expected {}+)",
                        len, min
                    )));
                }
            }
        }

        let mut context = vm.new_stack_context_with(
            args.block.clone(),
            iseq,
            outer.into(),
            args.len(),
            use_value,
        );
        context.set_arguments(vm.args(), kw);
        if params.kwrest || keyword_flag {
            let mut kwrest = FxIndexMap::default();
            if keyword_flag {
                let keyword = args.kw_arg.as_hash().unwrap();
                for (k, v) in keyword.iter() {
                    let id = k.as_symbol().unwrap();
                    match params.keyword.get(&id) {
                        Some(lvar) => context[*lvar] = v,
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
                Some(Block::Block(method, outer)) => vm.create_proc_from_block(*method, *outer),
                Some(Block::Proc(proc)) => *proc,
                None => Value::nil(),
            };
        }
        Ok(context)
    }

    pub fn enumerate_local_vars(&self, vec: &mut IndexSet<IdentId>) {
        let mut ctx = Some(*self);
        while let Some(c) = ctx {
            let iseq = c.iseq_ref;
            for v in iseq.lvar.table() {
                vec.insert(*v);
            }
            ctx = c.outer;
        }
    }

    /// Move a context on the stack to the heap.
    pub(super) fn move_to_heap(mut self) -> ContextRef {
        if self.on_heap() {
            return self;
        }
        assert!(self.alive());
        let mut heap = self.dup();
        heap.on_stack = CtxKind::Heap;
        self.on_stack = CtxKind::Dead(heap);

        match heap.outer {
            Some(c) => {
                if c.on_stack == CtxKind::Stack {
                    let c_heap = c.move_to_heap();
                    heap.outer = Some(c_heap);
                }
            }
            None => {}
        }
        heap
    }
}
