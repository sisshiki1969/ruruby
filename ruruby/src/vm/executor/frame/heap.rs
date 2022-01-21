use super::*;
pub use crate::*;
use std::pin::Pin;

#[derive(Clone, PartialEq)]
pub struct HeapContext {
    frame: Pin<Box<[Value]>>,
    ep: EnvFrame,
}

impl std::fmt::Debug for HeapContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let ep = self.as_ep();
        let iseq = ep.iseq();
        writeln!(
            f,
            "self:{:?} iseq_kind:{:?} opt:{:?} lvar:{:?}",
            self.self_val(),
            iseq.kind,
            iseq.opt_flag,
            iseq.lvar
        )?;
        for i in 0..iseq.lvars {
            write!(f, "[{:?}] ", ep[i as isize])?;
        }
        writeln!(f)?;
        Ok(())
    }
}

pub type HeapCtxRef = Ref<HeapContext>;

impl Into<HeapCtxRef> for &HeapContext {
    fn into(self) -> HeapCtxRef {
        Ref::from_ref(self)
    }
}

impl GC<RValue> for HeapCtxRef {
    fn mark(&self, alloc: &mut Allocator<RValue>) {
        self.as_ep().mark(alloc);
    }
}

impl HeapContext {
    pub(crate) fn self_val(&self) -> Value {
        self.frame[0]
    }

    pub(crate) fn as_ep(&self) -> EnvFrame {
        self.ep
    }
}

impl HeapCtxRef {
    pub fn new_heap(self_value: Value, iseq_ref: ISeqRef, outer: Option<EnvFrame>) -> Self {
        let local_len = iseq_ref.lvars;
        let mut frame = vec![Value::nil(); local_len + 1];
        frame[0] = self_value;
        frame.push(self_value);
        frame.extend_from_slice(&control_frame(
            ControlFrame::default(),
            EnvFrame::default(),
            VM::ruby_flag(true, local_len),
        ));
        frame.extend_from_slice(&heap_env_frame(outer, iseq_ref));
        let frame = Pin::from(frame.into_boxed_slice());
        let mut ep = EnvFrame::from_ref(&frame[local_len + 2]);
        ep[EV_EP] = ep.enc();
        ep[EV_MFP] = match &outer {
            None => ep.enc(),
            Some(heap) => heap.mfp().enc(),
        };
        let mut lfp = ep.get_lfp();
        for i in &iseq_ref.lvar.kw {
            lfp[*i] = Value::uninitialized();
        }
        HeapCtxRef::new(HeapContext { frame, ep })
    }

    pub fn new_binding(self_value: Value, iseq_ref: ISeqRef, outer: Option<EnvFrame>) -> EnvFrame {
        let local_len = iseq_ref.lvars;
        assert!(local_len < 64);
        let mut frame = vec![Value::nil(); 64];
        frame[64 - local_len - 1] = self_value;
        frame.push(self_value);
        frame.extend_from_slice(&control_frame(
            ControlFrame::default(),
            EnvFrame::default(),
            VM::ruby_flag(true, local_len),
        ));
        frame.extend_from_slice(&heap_env_frame(outer, iseq_ref));
        let frame = Pin::from(frame.into_boxed_slice());
        let mut ep = EnvFrame::from_ref(&frame[65]);
        ep[EV_EP] = ep.enc();
        ep[EV_MFP] = match &outer {
            None => ep.enc(),
            Some(heap) => heap.mfp().enc(),
        };
        let mut lfp = ep.get_lfp();
        for i in &iseq_ref.lvar.kw {
            lfp[*i] = Value::uninitialized();
        }
        HeapCtxRef::new(HeapContext { frame, ep }).as_ep()
    }

    pub(crate) fn dup_frame(mut cur_ep: EnvFrame, outer: Option<EnvFrame>) -> Self {
        let local_len = cur_ep.flag_len();
        let f = cur_ep.frame().to_vec().into_boxed_slice();
        let frame = Pin::from(f);
        let mut ep = EnvFrame::from_ref(&frame[local_len + 2]);
        let ep_enc = ep.enc();
        ep[EV_EP] = ep_enc;
        cur_ep[EV_EP] = ep_enc;

        let mfp = match outer {
            None => ep.enc(),
            Some(outer) => outer.mfp().enc(),
        };
        ep[EV_MFP] = mfp;
        cur_ep[EV_MFP] = mfp;

        let outer = EnvFrame::encode(outer);
        ep[EV_OUTER] = outer;
        cur_ep[EV_OUTER] = outer;

        HeapCtxRef::new(HeapContext { frame, ep })
    }
}
