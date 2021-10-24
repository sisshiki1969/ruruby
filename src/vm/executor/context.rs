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
        let frame = self.as_mfp();
        let iseq = frame.iseq();
        writeln!(
            f,
            "self:{:?} block:{:?} iseq_kind:{:?} opt:{:?} lvar:{:?}",
            self.self_val(),
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
        assert!(index < self.local_len);
        &self.frame[index]
    }
}

impl IndexMut<LvarId> for HeapContext {
    fn index_mut(&mut self, index: LvarId) -> &mut Self::Output {
        &mut self[index.as_usize()]
    }
}

impl IndexMut<usize> for HeapContext {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.local_len);
        &mut self.frame[index]
    }
}

impl Into<HeapCtxRef> for &HeapContext {
    fn into(self) -> HeapCtxRef {
        Ref::from_ref(self)
    }
}

impl GC for HeapCtxRef {
    fn mark(&self, alloc: &mut Allocator) {
        self.frame.iter().for_each(|v| v.mark(alloc));
        let frame = self.as_mfp();
        if let Some(b) = &frame.block() {
            b.mark(alloc)
        };
        match frame.outer_heap() {
            Some(c) => c.mark(alloc),
            None => {}
        }
    }
}

impl HeapContext {
    pub(crate) fn self_val(&self) -> Value {
        self.frame[self.local_len]
    }

    pub(crate) fn local_len(&self) -> usize {
        self.local_len
    }

    pub fn as_mfp(&self) -> MethodFrame {
        MethodFrame::from_ref(&self.frame[self.local_len + 1..])
    }

    pub(crate) fn as_lfp(&self) -> LocalFrame {
        LocalFrame::from_ref(&self.frame)
    }

    pub(crate) fn lfp(&self) -> LocalFrame {
        LocalFrame::decode(self.frame[self.local_len + 1 + LFP_OFFSET])
    }

    /*pub(crate) fn block(&self) -> Option<Block> {
        Block::decode(self.frame[self.local_len + 1 + BLK_OFFSET])
    }*/

    pub(crate) fn iseq(&self) -> ISeqRef {
        ISeqRef::decode(self.frame[self.local_len + 1 + ISEQ_OFFSET].as_fnum())
    }

    pub(crate) fn set_iseq(&mut self, iseq: ISeqRef) {
        let mut f = self.frame[0..self.local_len].to_vec();
        let self_val = self.self_val();
        f.resize(iseq.lvars, Value::nil());
        f.push(self_val);
        f.extend_from_slice(&self.frame[self.local_len + 1..]);
        self.frame = Pin::from(f.into_boxed_slice());
        self.local_len = iseq.lvars;

        self.frame[self.local_len + 1 + ISEQ_OFFSET] = Value::fixnum(iseq.encode());
        self.frame[self.local_len + 1 + LFP_OFFSET] = self.as_lfp().encode();
    }

    pub(crate) fn outer(&self) -> Option<HeapCtxRef> {
        match self.frame[self.local_len + 1 + DFP_OFFSET].as_fnum() {
            0 => None,
            i if i > 0 => Some(HeapCtxRef::decode(i)),
            _ => unreachable!(),
        }
    }

    pub(crate) fn method(&self) -> MethodFrame {
        MethodFrame::decode(self.frame[self.local_len + 1 + MFP_OFFSET])
    }

    //#[cfg(not(tarpaulin_include))]
    /*pub(crate) fn pp(&self) {
        println!(
            "context:{:?} outer:{:?}",
            self as *const HeapContext,
            self.outer()
        );
    }*/
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
        let local_len = iseq_ref.lvars;
        let mut frame = match lvars {
            None => vec![Value::nil(); local_len],
            Some(slice) => {
                assert_eq!(slice.len(), local_len);
                slice.to_vec()
            }
        };
        frame.push(self_value);
        frame.extend_from_slice(&VM::control_frame(
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
            LocalFrame::default(),
        ));
        let mut frame = Pin::from(frame.into_boxed_slice());
        frame[local_len + 1 + MFP_OFFSET] = match &outer {
            None => MethodFrame::from_ref(&frame[local_len + 1..]),
            Some(heap) => heap.method(),
        }
        .encode();
        frame[local_len + 1 + LFP_OFFSET] = LocalFrame::from_ref(&frame).encode();
        let mut context = HeapContext { frame, local_len };
        for i in &iseq_ref.lvar.kw {
            context[*i] = Value::uninitialized();
        }
        let h = HeapCtxRef::new(context);
        h
    }

    pub(crate) fn new_from_frame(
        frame: &[Value],
        outer: Option<HeapCtxRef>,
        local_len: usize,
    ) -> Self {
        let mut frame = Pin::from(frame.to_vec().into_boxed_slice());
        match outer {
            None => {
                frame[local_len + 1 + MFP_OFFSET] =
                    MethodFrame::from_ref(&frame[local_len + 1..]).encode();
                frame[local_len + 1 + DFP_OFFSET] = Value::fixnum(0);
            }
            Some(h) => {
                frame[local_len + 1 + MFP_OFFSET] = h.method().encode();
                frame[local_len + 1 + DFP_OFFSET] = Value::fixnum(h.encode());
            }
        }
        frame[local_len + 1 + LFP_OFFSET] = LocalFrame::from_ref(&frame).encode();
        let context = HeapContext { frame, local_len };
        HeapCtxRef::new(context)
    }

    pub(crate) fn enumerate_local_vars(&self, vec: &mut IndexSet<IdentId>) {
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
