pub use crate::*;
pub use context_store::ContextStore;
use indexmap::IndexSet;
use std::ops::{Index, IndexMut, Range};

mod context_store;

#[derive(Clone)]
pub struct Context {
    pub self_value: Value,
    pub block: Option<Block>,
    lvar: Vec<Value>,
    pub iseq_ref: ISeqRef,
    /// Context of outer scope.
    pub outer: Option<ContextRef>,
    pub on_stack: CtxKind,
    pub cur_pc: ISeqPos,
    pub module_function: bool,
    pub delegate_args: Option<Value>,
}

impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(
            f,
            "{:?} self:{:?} block:{:?} iseq_kind:{:?} opt:{:?} lvar:{:?}",
            self.on_stack,
            self.self_value,
            self.block,
            self.iseq_ref.kind,
            self.iseq_ref.opt_flag,
            self.iseq_ref.lvar
        )?;
        for i in 0..self.iseq_ref.lvars {
            write!(f, "[{:?}] ", self[i])?;
        }
        writeln!(f, "")?;
        Ok(())
    }
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
        &self.lvar[index]
    }
}

impl IndexMut<LvarId> for Context {
    fn index_mut(&mut self, index: LvarId) -> &mut Self::Output {
        &mut self[index.as_usize()]
    }
}

impl IndexMut<usize> for Context {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.lvar[index]
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
        self.lvar.iter().for_each(|v| v.mark(alloc));
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
        Context {
            self_value,
            block,
            lvar: vec![Value::nil(); lvar_num],
            iseq_ref,
            outer,
            on_stack: CtxKind::Stack,
            cur_pc: ISeqPos::from(0),
            module_function: false,
            delegate_args: None,
        }
    }

    pub fn set_iseq(&mut self, iseq: ISeqRef) {
        self.iseq_ref = iseq;
        self.lvar.resize(iseq.lvars, Value::nil());
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

    #[cfg(not(tarpaulin_include))]
    pub fn pp(&self) {
        println!(
            "{:?} context:{:?} outer:{:?}",
            self.on_stack, self as *const Context, self.outer
        );
    }

    fn copy_from_slice(&mut self, index: usize, slice: &[Value]) {
        let len = slice.len();
        self.lvar[index..index + len].copy_from_slice(slice);
    }

    fn fill(&mut self, range: Range<usize>, val: Value) {
        for i in range {
            self[i] = val;
        }
    }

    fn fill_positional_arguments(&mut self, args: &[Value], params: &ISeqParams) {
        let args_len = args.len();
        let req_len = params.req;
        let rest_len = if params.rest == Some(true) { 1 } else { 0 };
        let post_len = params.post;
        let no_post_len = args_len - post_len;
        let optreq_len = req_len + params.opt;

        if optreq_len < no_post_len {
            // fill req and opt params.
            self.copy_from_slice(0, &args[0..optreq_len]);
            if self.iseq_ref.lvar.delegate_param {
                let v = args[optreq_len..no_post_len].to_vec();
                self.delegate_args = Some(Value::array_from(v));
            }
            if rest_len == 1 {
                let ary = args[optreq_len..no_post_len].to_vec();
                self[optreq_len] = Value::array_from(ary);
            }
            // fill post_req params.
            self.copy_from_slice(optreq_len + rest_len, &args[no_post_len..args_len]);
        } else {
            // fill req and opt params.
            self.copy_from_slice(0, &args[0..no_post_len]);
            // fill post_req params.
            self.copy_from_slice(optreq_len + rest_len, &args[no_post_len..args_len]);
            if no_post_len < req_len {
                // fill rest req params with nil.
                self.fill(no_post_len..req_len, Value::nil());
            }
            if rest_len == 1 {
                self[optreq_len] = Value::array_from(vec![]);
            }
        }
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

    pub fn method_context(&self) -> ContextRef {
        let mut context = *self;
        while let Some(c) = context.outer {
            context = c;
        }
        context
    }

    pub fn get_current(self) -> Self {
        match self.on_stack {
            CtxKind::Dead(c) => c,
            _ => self,
        }
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

impl VM {
    pub fn push_frame(
        &mut self,
        iseq: ISeqRef,
        args: &Args2,
        outer: impl Into<Option<ContextRef>>,
        use_value: bool,
    ) -> Result<(), RubyError> {
        let self_value = self.stack_pop();
        let params = &iseq.params;
        let args_pos = self.stack_len() - args.len();
        let (positional_kwarg, ordinary_kwarg) = if params.keyword.is_empty() && !params.kwrest {
            // Note that Ruby 3.0 doesn’t behave differently when calling a method which doesn’t accept keyword
            // arguments with keyword arguments.
            // For instance, the following case is not going to be deprecated and will keep working in Ruby 3.0.
            // The keyword arguments are still treated as a positional Hash argument.
            //
            // def foo(kwargs = {})
            //   kwargs
            // end
            // foo(k: 1) #=> {:k=>1}
            //
            // https://www.ruby-lang.org/en/news/2019/12/12/separation-of-positional-and-keyword-arguments-in-ruby-3-0/
            if !args.kw_arg.is_nil() {
                self.stack_push(args.kw_arg);
            }
            (!args.kw_arg.is_nil(), false)
        } else {
            (false, !args.kw_arg.is_nil())
        };
        if !iseq.is_block() {
            params.check_arity(positional_kwarg, args)?;
        } else {
            self.prepare_block_args(iseq, args_pos);
        }

        let mut context = self.push_with(self_value, args.block.clone(), iseq, outer.into());
        self.stack_push(self_value);
        self.prepare_frame(self.stack_len() - args_pos - 1, use_value, context, iseq);

        context.fill_positional_arguments(self.args(), &iseq.params);
        // Handling keyword arguments and a keyword rest paramter.
        if params.kwrest || ordinary_kwarg {
            let mut kwrest = FxIndexMap::default();
            if ordinary_kwarg {
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
        // Handling block paramter.
        if let Some(id) = iseq.lvar.block_param() {
            context[id] = args
                .block
                .as_ref()
                .map_or(Value::nil(), |block| self.create_proc(&block));
        }
        #[cfg(feature = "trace")]
        self.dump_current_frame();
        Ok(())
    }
}
