pub use crate::*;
use indexmap::IndexSet;
use std::ops::{Index, IndexMut};
use std::pin::Pin;

#[derive(Clone, PartialEq)]
pub struct HeapContext {
    frame: Pin<Box<[Value]>>,
    local_len: usize,
}

impl std::fmt::Debug for HeapContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let frame = self.as_dfp();
        let iseq = frame.iseq();
        writeln!(
            f,
            "self:{:?} iseq_kind:{:?} opt:{:?} lvar:{:?}",
            self.self_val(),
            iseq.kind,
            iseq.opt_flag,
            iseq.lvar
        )?;
        for i in 0..iseq.lvars {
            write!(f, "[{:?}] ", self[i])?;
        }
        writeln!(f)?;
        Ok(())
    }
}

pub type HeapCtxRef = Ref<HeapContext>;

impl Index<usize> for HeapContext {
    type Output = Value;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.local_len);
        &self.frame[index]
    }
}

impl IndexMut<usize> for HeapContext {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.local_len + 1);
        &mut self.frame[index]
    }
}

impl Into<HeapCtxRef> for &HeapContext {
    fn into(self) -> HeapCtxRef {
        Ref::from_ref(self)
    }
}

impl GC<RValue> for HeapCtxRef {
    fn mark(&self, alloc: &mut Allocator<RValue>) {
        self.as_dfp().mark(alloc);
    }
}

impl HeapContext {
    pub(crate) fn self_val(&self) -> Value {
        self.frame[self.local_len + 1]
    }

    pub fn as_dfp(&self) -> DynamicFrame {
        DynamicFrame::from_ref(&self.frame[self.local_len + 2..])
    }

    fn as_lfp(&self) -> LocalFrame {
        LocalFrame::from_ref(&self.frame[1..])
    }

    pub(crate) fn lfp(&self) -> LocalFrame {
        self.as_dfp().lfp()
        //LocalFrame::decode(self.frame[self.local_len + 2 + LFP_OFFSET])
    }

    pub fn iseq(&self) -> ISeqRef {
        self.as_dfp().iseq()
        //ISeqRef::decode(self.frame[self.local_len + 2 + ISEQ_OFFSET].as_fnum())
    }

    pub(crate) fn set_iseq(&mut self, iseq: ISeqRef) {
        let mut f = self.frame[0..self.local_len + 1].to_vec();
        let self_val = self.self_val();
        f.resize(iseq.lvars + 1, Value::nil());
        f.push(self_val);
        f.extend_from_slice(&self.frame[self.local_len + 2..]);
        self.frame = Pin::from(f.into_boxed_slice());
        self.local_len = iseq.lvars;

        self.frame[self.local_len + 2 + ISEQ_OFFSET] = Value::fixnum(iseq.encode());
        self.frame[self.local_len + 2 + LFP_OFFSET] = self.as_lfp().encode();
    }

    pub(crate) fn outer(&self) -> Option<DynamicFrame> {
        self.as_dfp().dfp()
        //DynamicFrame::decode(self.frame[self.local_len + 2 + DFP_OFFSET])
    }
}

impl HeapCtxRef {
    pub fn new_heap(self_value: Value, iseq_ref: ISeqRef, outer: Option<DynamicFrame>) -> Self {
        let local_len = iseq_ref.lvars;
        let mut frame = vec![Value::nil(); local_len + 1];
        frame[0] = self_value;
        frame.push(self_value);
        frame.extend_from_slice(&VM::heap_control_frame(outer, iseq_ref));
        let mut frame = Pin::from(frame.into_boxed_slice());
        frame[local_len + 2 + MFP_OFFSET] = match &outer {
            None => ControlFrame::from_ref(&frame[local_len + 2..]),
            Some(heap) => heap.mfp(),
        }
        .encode();
        frame[local_len + 2 + LFP_OFFSET] = LocalFrame::from_ref(&frame[1..]).encode();
        let heap = HeapCtxRef::new(HeapContext { frame, local_len });
        let mut lfp = heap.as_lfp();
        for i in &iseq_ref.lvar.kw {
            lfp[*i] = Value::uninitialized();
        }
        heap
    }

    pub(crate) fn new_from_frame(
        self_value: Value,
        frame: &[Value],
        outer: Option<DynamicFrame>,
        local_len: usize,
    ) -> Self {
        let mut f = vec![self_value];
        f.extend_from_slice(frame);
        let mut frame = Pin::from(f.into_boxed_slice());
        match outer {
            None => {
                frame[local_len + 2 + MFP_OFFSET] =
                    ControlFrame::from_ref(&frame[local_len + 2..]).encode();
                frame[local_len + 2 + DFP_OFFSET] = DynamicFrame::encode(None);
            }
            Some(outer) => {
                frame[local_len + 2 + MFP_OFFSET] = outer.mfp().encode();
                frame[local_len + 2 + DFP_OFFSET] = DynamicFrame::encode(Some(outer));
            }
        }
        frame[local_len + 2 + LFP_OFFSET] = LocalFrame::from_ref(&frame[1..]).encode();
        let context = HeapContext { frame, local_len };
        HeapCtxRef::new(context)
    }

    pub(crate) fn enumerate_local_vars(&self, vec: &mut IndexSet<IdentId>) {
        let mut ctx = Some(self.as_dfp());
        while let Some(c) = ctx {
            let iseq = c.iseq();
            for v in iseq.lvar.table() {
                vec.insert(*v);
            }
            ctx = c.outer();
        }
    }
}
