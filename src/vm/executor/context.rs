pub use crate::*;
use indexmap::IndexSet;
use std::ops::{Index, IndexMut};

#[derive(Clone, PartialEq)]
pub struct HeapContext {
    pub frame: Box<[Value; RUBY_FRAME_LEN]>,
    pub self_value: Value,
    lvar: Vec<Value>,
}

impl std::fmt::Debug for HeapContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let frame = self.as_mfp();
        let iseq = frame.iseq();
        writeln!(
            f,
            "self:{:?} block:{:?} iseq_kind:{:?} opt:{:?} lvar:{:?}",
            self.self_value,
            frame.block(),
            iseq.kind,
            iseq.opt_flag,
            iseq.lvar
        )?;
        for i in 0..iseq.lvars {
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
        let frame = self.as_mfp();
        if let Some(b) = &frame.block() {
            b.mark(alloc)
        };
        match frame.outer() {
            Some(c) => c.mark(alloc),
            None => {}
        }
    }
}

impl HeapContext {
    pub fn as_mfp(&self) -> MethodFrame {
        MethodFrame::from_ref(&self.frame)
    }

    pub fn as_lfp(&self) -> LocalFrame {
        LocalFrame::from_ref(&self.lvar)
    }

    pub fn lfp(&self) -> LocalFrame {
        LocalFrame::decode(self.frame[LFP_OFFSET])
    }

    pub fn block(&self) -> Option<Block> {
        Block::decode(self.frame[BLK_OFFSET])
    }

    pub fn iseq(&self) -> ISeqRef {
        ISeqRef::decode(self.frame[ISEQ_OFFSET].as_fnum())
    }

    pub fn set_iseq(&mut self, iseq: ISeqRef) {
        self.frame[ISEQ_OFFSET] = Value::fixnum(iseq.encode());
        self.lvar.resize(iseq.lvars, Value::nil());
        self.frame[LFP_OFFSET] = LocalFrame::from_ref(&self.lvar).encode();
    }

    pub fn outer(&self) -> Option<HeapCtxRef> {
        match self.frame[DFP_OFFSET].as_fnum() {
            0 => None,
            i => Some(HeapCtxRef::decode(i)),
        }
    }

    pub fn method(&self) -> MethodFrame {
        MethodFrame::decode(self.frame[MFP_OFFSET])
    }

    #[cfg(not(tarpaulin_include))]
    pub fn pp(&self) {
        println!(
            "context:{:?} outer:{:?}",
            self as *const HeapContext,
            self.outer()
        );
    }
}

impl HeapCtxRef {
    pub fn new_heap(
        flag: i64,
        self_value: Value,
        block: Option<Block>,
        iseq_ref: ISeqRef,
        outer: Option<HeapCtxRef>,
        lvars: Option<&[Value]>,
    ) -> Self {
        let lvar_num = iseq_ref.lvars;
        if let Some(lvars) = lvars {
            assert_eq!(lvars.len(), lvar_num);
        }
        let lvar = match lvars {
            None => vec![Value::nil(); lvar_num],
            Some(slice) => slice.to_vec(),
        };
        let mut frame = Box::new(VM::control_frame(
            flag,
            0,
            Value::fixnum(0),
            None,
            match &outer {
                None => 0,
                Some(h) => h.encode(),
            },
            iseq_ref,
            block.as_ref(),
            LocalFrame::from_ref(&lvar),
        ));
        frame[MFP_OFFSET] = match &outer {
            None => MethodFrame::from_ref(&frame),
            Some(heap) => heap.method(),
        }
        .encode();
        let mut context = HeapContext {
            frame,
            self_value,
            lvar,
        };
        for i in &iseq_ref.lvar.kw {
            context[*i] = Value::uninitialized();
        }
        HeapCtxRef::new(context)
    }

    pub fn enumerate_local_vars(&self, vec: &mut IndexSet<IdentId>) {
        let mut ctx = Some(*self);
        while let Some(c) = ctx {
            let iseq = c.iseq();
            for v in iseq.lvar.table() {
                vec.insert(*v);
            }
            ctx = c.outer();
        }
    }
}
