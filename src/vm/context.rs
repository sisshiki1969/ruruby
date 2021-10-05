pub use crate::*;
pub use context_store::ContextStore;
use indexmap::IndexSet;
use std::ops::{Index, IndexMut};

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
        for i in &iseq_ref.lvar.kw {
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
        let base = self.stack_len() - args.len();
        let outer = outer.into();
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
            self.prepare_block_args(iseq, base);
        }

        let mut context = self.push_with(self_value, args.block.clone(), iseq, outer);
        self.fill_positional_arguments(base, iseq);
        // Handling keyword arguments and a keyword rest paramter.
        if params.kwrest || ordinary_kwarg {
            self.fill_keyword_arguments(base, iseq, args.kw_arg, ordinary_kwarg)?;
        };

        self.stack_push(self_value);
        self.prepare_frame(self.stack_len() - base - 1, use_value, context, outer, iseq);
        // Handling block paramter.
        if let Some(id) = iseq.lvar.block_param() {
            self.fill_block_argument(base, id, &args.block);
        }
        context.lvar = self.args().to_vec();

        #[cfg(feature = "trace")]
        self.dump_current_frame();
        Ok(())
    }
}
