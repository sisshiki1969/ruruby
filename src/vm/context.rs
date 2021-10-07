pub use crate::*;
pub use context_store::ContextStore;
use indexmap::IndexSet;
use std::ops::{Index, IndexMut};

mod context_store;

#[derive(Clone)]
pub struct HeapContext {
    pub self_value: Value,
    pub block: Option<Block>,
    pub lvar: Vec<Value>,
    pub iseq_ref: ISeqRef,
    /// Context of outer scope.
    pub outer: Option<HeapCtxRef>,
    pub on_stack: CtxKind,
    pub cur_pc: ISeqPos,
}

impl std::fmt::Debug for HeapContext {
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
    Dead(HeapCtxRef),
}

pub type HeapCtxRef = Ref<HeapContext>;

impl Index<LvarId> for HeapContext {
    type Output = Value;

    fn index(&self, index: LvarId) -> &Self::Output {
        &self[index.as_usize()]
    }
}

impl Index<usize> for HeapContext {
    type Output = Value;

    fn index(&self, index: usize) -> &Self::Output {
        &self.lvar[index]
    }
}

impl IndexMut<LvarId> for HeapContext {
    fn index_mut(&mut self, index: LvarId) -> &mut Self::Output {
        &mut self[index.as_usize()]
    }
}

impl IndexMut<usize> for HeapContext {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.lvar[index]
    }
}

impl Into<HeapCtxRef> for &HeapContext {
    fn into(self) -> HeapCtxRef {
        Ref::from_ref(self)
    }
}

impl GC for HeapCtxRef {
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

impl HeapContext {
    fn new(
        self_value: Value,
        block: Option<Block>,
        iseq_ref: ISeqRef,
        outer: Option<HeapCtxRef>,
    ) -> Self {
        let lvar_num = iseq_ref.lvars;
        HeapContext {
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
            self.on_stack, self as *const HeapContext, self.outer
        );
    }
}

impl HeapCtxRef {
    pub fn new_heap(
        self_value: Value,
        block: Option<Block>,
        iseq_ref: ISeqRef,
        outer: Option<HeapCtxRef>,
    ) -> Self {
        let mut context = HeapContext::new(self_value, block, iseq_ref, outer);
        context.on_stack = CtxKind::FromHeap;
        for i in &iseq_ref.lvar.kw {
            context[*i] = Value::uninitialized();
        }
        HeapCtxRef::new(context)
    }

    pub fn method_context(&self) -> HeapCtxRef {
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
    pub(super) fn move_to_heap(mut self) -> HeapCtxRef {
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
