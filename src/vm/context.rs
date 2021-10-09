pub use crate::*;
use indexmap::IndexSet;
use std::ops::{Index, IndexMut};

#[derive(Clone, PartialEq)]
pub struct HeapContext {
    pub self_value: Value,
    pub block: Option<Block>,
    pub lvar: Vec<Value>,
    pub iseq_ref: ISeqRef,
    /// Outer context.
    pub outer: Option<HeapCtxRef>,
    /// Method context.
    pub method: Option<HeapCtxRef>,
}

impl std::fmt::Debug for HeapContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        writeln!(
            f,
            "self:{:?} block:{:?} iseq_kind:{:?} opt:{:?} lvar:{:?}",
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
    pub fn set_iseq(&mut self, iseq: ISeqRef) {
        self.iseq_ref = iseq;
        self.lvar.resize(iseq.lvars, Value::nil());
    }

    #[cfg(not(tarpaulin_include))]
    pub fn pp(&self) {
        println!(
            "context:{:?} outer:{:?}",
            self as *const HeapContext, self.outer
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
        let lvar_num = iseq_ref.lvars;
        let mut context = HeapContext {
            self_value,
            block,
            lvar: vec![Value::nil(); lvar_num],
            iseq_ref,
            outer,
            method: outer.map(|h| h.method.unwrap()),
        };
        for i in &iseq_ref.lvar.kw {
            context[*i] = Value::uninitialized();
        }
        let mut r = HeapCtxRef::new(context);
        if r.method.is_none() {
            r.method = Some(r);
        }
        r
    }

    pub fn method_context(&self) -> HeapCtxRef {
        self.method.unwrap()
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
}
