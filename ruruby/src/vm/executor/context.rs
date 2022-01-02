pub use crate::*;
use indexmap::IndexSet;
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
            write!(f, "[{:?}] ", ep[i])?;
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
    fn local_len(&self) -> usize {
        self.as_ep().flag_len()
    }

    fn set_local_len(&mut self, new_len: usize) {
        let mut ep = self.as_ep();
        ep[EV_FLAG] = Value::from((ep.flag() & 0xffff_ffff) | (new_len as u64) << 32);
    }

    pub(crate) fn self_val(&self) -> Value {
        self.frame[self.local_len() + 1]
    }

    pub fn as_ep(&self) -> EnvFrame {
        self.ep
    }

    pub(crate) fn set_iseq(&mut self, iseq: ISeqRef) {
        let local_len = self.local_len();
        let mut f = self.frame[0..local_len + 1].to_vec();
        let self_val = self.self_val();
        f.resize(iseq.lvars + 1, Value::nil());
        f.push(self_val);
        f.extend_from_slice(&self.frame[local_len + 2..]);
        self.frame = Pin::from(f.into_boxed_slice());
        let local_len = iseq.lvars;
        let mut ep = EnvFrame::from_ref(&self.frame[local_len + 2]);
        self.ep = ep;
        self.set_local_len(local_len);
        ep[EV_ISEQ] = Value::fixnum(iseq.encode());
    }
}

impl HeapCtxRef {
    pub fn new_heap(self_value: Value, iseq_ref: ISeqRef, outer: Option<EnvFrame>) -> Self {
        let local_len = iseq_ref.lvars;
        let mut frame = vec![Value::nil(); local_len + 1];
        frame[0] = self_value;
        frame.push(self_value);
        frame.extend_from_slice(&VM::control_frame(
            ControlFrame::default(),
            //StackPtr::default(),
            VM::ruby_flag(true, local_len),
        ));
        frame.extend_from_slice(&VM::heap_env_frame(outer, iseq_ref));
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
        //assert_eq!(lfp, ep.get_lfp());
        HeapCtxRef::new(HeapContext { frame, ep })
    }

    pub(crate) fn new_from_frame(mut cur_ep: EnvFrame, outer: Option<EnvFrame>) -> Self {
        let self_value = cur_ep.self_value();
        let frame = cur_ep.frame();
        let local_len = cur_ep.flag_len();
        let mut f = vec![self_value];
        f.extend_from_slice(frame);
        let frame = Pin::from(f.into_boxed_slice());
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

        //let lfp = LocalFrame::from_ref(&frame[1]);
        //assert_eq!(lfp, ep.get_lfp());
        HeapCtxRef::new(HeapContext { frame, ep })
    }

    pub(crate) fn enumerate_local_vars(&self, vec: &mut IndexSet<IdentId>) {
        let mut ep = Some(self.as_ep());
        while let Some(e) = ep {
            let iseq = e.iseq();
            for v in iseq.lvar.table() {
                vec.insert(*v);
            }
            ep = e.outer();
        }
    }
}
