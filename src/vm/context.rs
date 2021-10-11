pub use crate::*;
use indexmap::IndexSet;
use std::ops::{Index, IndexMut};

const FLAG_OFFSET: usize = 0;
//const CFP_OFFSET: usize = 1;
const MFP_OFFSET: usize = 2;
const DFP_OFFSET: usize = 3;
//const PC_OFFSET: usize = 4;
//const HEAP_OFFSET: usize = 5;
const ISEQ_OFFSET: usize = 6;
const BLK_OFFSET: usize = 7;
const RUBY_FRAME_LEN: usize = 8;

#[derive(Clone, PartialEq)]
pub struct HeapContext {
    pub method_frame: Box<[Value]>,
    pub self_value: Value,
    /// Method context.
    pub method: Option<HeapCtxRef>,
    /// Outer context.
    pub lvar: Vec<Value>,
}

impl std::fmt::Debug for HeapContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let iseq = self.iseq();
        writeln!(
            f,
            "self:{:?} block:{:?} iseq_kind:{:?} opt:{:?} lvar:{:?}",
            self.self_value, self.method_frame[BLK_OFFSET], iseq.kind, iseq.opt_flag, iseq.lvar
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
        if let Some(b) = &self.block() {
            b.mark(alloc)
        };
        match self.outer() {
            Some(c) => c.mark(alloc),
            None => {}
        }
    }
}

impl HeapContext {
    pub fn flag(&self) -> Value {
        self.method_frame[FLAG_OFFSET]
    }

    pub fn flag_mut(&mut self) -> &mut Value {
        &mut self.method_frame[FLAG_OFFSET]
    }

    pub fn block(&self) -> Option<Block> {
        let val = self.method_frame[BLK_OFFSET];
        match val.as_fixnum() {
            None => Some(val.into()),
            Some(0) => None,
            Some(i) => Some(Block::decode(i)),
        }
    }

    pub fn iseq(&self) -> ISeqRef {
        ISeqRef::decode(self.method_frame[ISEQ_OFFSET].as_fnum())
    }

    pub fn set_iseq(&mut self, iseq: ISeqRef) {
        self.method_frame[ISEQ_OFFSET] = Value::fixnum(iseq.encode());
        self.lvar.resize(iseq.lvars, Value::nil());
    }

    pub fn outer(&self) -> Option<HeapCtxRef> {
        match self.method_frame[DFP_OFFSET].as_fnum() {
            0 => None,
            i => Some(HeapCtxRef::decode(i)),
        }
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
        let flag = Value::fixnum(0);
        let mut context = HeapContext {
            method_frame: Box::new([
                flag,             // Flag
                Value::fixnum(0), // prev_cfp: not used
                Value::fixnum(0), // mfp
                Value::fixnum(match &outer {
                    None => 0,
                    Some(h) => h.encode(),
                }), // dfp
                Value::fixnum(0), // pc: not used
                Value::fixnum(0), // ctx: not used
                Value::fixnum(iseq_ref.encode()), // iseq
                match block {
                    None => Value::fixnum(0),
                    Some(block) => block.encode(),
                }, // block
            ]),
            self_value,
            lvar: match lvars {
                None => vec![Value::nil(); lvar_num],
                Some(slice) => slice.to_vec(),
            },
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
            let iseq = c.iseq();
            for v in iseq.lvar.table() {
                vec.insert(*v);
            }
            ctx = c.outer();
        }
    }
}
