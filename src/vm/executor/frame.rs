use super::*;

//
//  Stack handling
//
//  before frame preparation
//
//   lfp                            cfp                                                                 sp
//    v                              v                           <------ new local frame ----->          v
// +------+------+--+------+------+------+------+------+------+--------+------+------+--+------+------+------------------------
// |  a0  |  a1  |..|  an  | self | flg1 | cfp2 | mfp1 |  pc2 |  ....  |  b0  |  b1  |..|  bn  | self |
// +------+------+--+------+------+------+------+------+------+--------+------+------+--+------+------+------------------------
//  <------- local frame --------> <-- control frame ->
//
//
//  after frame preparation
//
//   lfp1                           cfp1                                 lfp                            cfp                            sp
//    v                              v                                    v                              v                              v
// +------+------+--+------+------+------+------+------+------+--------+------+------+--+------+------+------+------+------+------+---
// |  a0  |  a1  |..|  an  | self | flg1 | cfp2 | mfp1 |  pc2 |  ....  |  b0  |  b1  |..|  bn  | self | flg  | cfp1 | mfp  |  pc1 |
// +------+------+--+------+------+------+------+------+------+--------+------+------+--+------+------+------+------+------+------+---
//                                                                      <------- local frame --------> <------- control frame -------
//
//  after execution
//
//   lfp                            cfp                                   sp
//    v                              v                                     v
// +------+------+--+------+------+------+------+------+------+--------+-------------------------------------------------------
// |  a0  |  a1  |..|  an  | self | flg1 | cfp2 | mfp1 |  pc2 |  ....  |
// +------+------+--+------+------+------+------+------+------+--------+-------------------------------------------------------
//

pub const FLAG_OFFSET: usize = 0;
pub const CFP_OFFSET: usize = 1;
pub const MFP_OFFSET: usize = 2;
pub const DFP_OFFSET: usize = 3;
pub const PC_OFFSET: usize = 4;
pub const HEAP_OFFSET: usize = 5;
pub const ISEQ_OFFSET: usize = 6;
pub const BLK_OFFSET: usize = 7;
pub const LFP_OFFSET: usize = 8;
pub const NATIVE_FRAME_LEN: usize = 2;
pub const RUBY_FRAME_LEN: usize = 9;

/// Control frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame(pub usize);

impl Frame {
    fn from(fp: usize) -> Option<Self> {
        if fp == 0 {
            None
        } else {
            Some(Frame(fp))
        }
    }

    pub(crate) fn encode(&self) -> i64 {
        -(self.0 as i64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MethodFrame(*mut Value);

impl MethodFrame {
    pub(crate) fn from_ref(r: &[Value]) -> Self {
        Self(r.as_ptr() as *mut _)
    }

    pub(crate) fn encode(self) -> Value {
        Value::from((self.0 as u64) | 0b1)
    }

    pub(crate) fn decode(v: Value) -> Self {
        Self((v.get() & (-2i64 as u64)) as *mut _)
    }

    pub(crate) fn outer(&self) -> Option<HeapCtxRef> {
        unsafe {
            match (*self.0.add(DFP_OFFSET)).as_fnum() {
                0 => None,
                i => Some(HeapCtxRef::decode(i)),
            }
        }
    }

    pub(crate) fn iseq(self) -> ISeqRef {
        unsafe {
            let v = *self.0.add(ISEQ_OFFSET);
            ISeqRef::decode(v.as_fnum())
        }
    }

    pub(crate) fn block(self) -> Option<Block> {
        unsafe {
            let v = *self.0.add(BLK_OFFSET);
            Block::decode(v)
        }
    }

    fn is_module_function(self) -> bool {
        unsafe { (*self.0.add(FLAG_OFFSET)).get() & 0b1000 != 0 }
    }

    fn set_module_function(self) {
        unsafe {
            let p = self.0.add(FLAG_OFFSET);
            std::ptr::write(p, Value::from((*p).get() | 0b1000));
        }
    }

    #[cfg(not(tarpaulin_include))]
    #[allow(dead_code)]
    fn dump(&self) {
        unsafe {
            eprintln!("FLAG:{:?}", *self.0.add(FLAG_OFFSET));
            eprintln!("CFP: {:?}", *self.0.add(CFP_OFFSET));
            eprintln!("MFP: {:?}", *self.0.add(MFP_OFFSET));
            eprintln!("DFP: {:?}", *self.0.add(DFP_OFFSET));
            eprintln!("PC:  {:?}", *self.0.add(PC_OFFSET));
            eprintln!("CTX: {:?}", *self.0.add(HEAP_OFFSET));
            eprintln!("ISEQ:{:?}", *self.0.add(ISEQ_OFFSET));
            eprintln!("BLK: {:?}", *self.0.add(BLK_OFFSET));
            eprintln!("LFP: {:?}", *self.0.add(LFP_OFFSET));
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalFrame(pub(super) *mut Value);

impl std::default::Default for LocalFrame {
    fn default() -> Self {
        LocalFrame(std::ptr::null_mut())
    }
}

impl LocalFrame {
    pub(crate) fn from_ref(r: &[Value]) -> Self {
        Self(r.as_ptr() as *mut _)
    }

    pub(crate) fn encode(self) -> Value {
        Value::from((self.0 as u64) | 0b1)
    }

    pub(crate) fn decode(v: Value) -> Self {
        Self((v.get() & (-2i64 as u64)) as *mut _)
    }

    pub(crate) fn get(self, i: LvarId) -> Value {
        unsafe { *self.0.add(*i) }
    }

    pub(crate) fn set(self, i: LvarId, val: Value) {
        unsafe { *self.0.add(*i) = val }
    }
}

impl VM {
    fn new_mfp_from_stack(&mut self, f: Frame) -> MethodFrame {
        unsafe {
            let ptr = self.exec_stack.as_mut_ptr();
            MethodFrame(ptr.add(f.0) as *mut _)
        }
    }

    fn new_lfp_from_stack(&mut self, index: usize) -> LocalFrame {
        unsafe {
            let ptr = self.exec_stack.as_mut_ptr();
            LocalFrame(ptr.add(index) as *mut _)
        }
    }
}

impl VM {
    /*fn check_integrity(&self) {
        let mut cfp = Some(self.cur_frame());
        eprintln!("check integrity");
        eprintln!("----------------------------");
        while let Some(f) = cfp {
            eprintln!(
                "  frame:{:?} iseq:{:?} self:{:?} outer:{:?}",
                f,
                self.frame_iseq(f),
                self.frame_self(f),
                self.frame_dfp(f),
            );
            if let Some(c) = self.frame_heap(f) {
                assert!(self.frame_iseq(f) == c.iseq_ref);
                assert!(self.frame_self(f).id() == c.self_value.id());
                assert!(self.frame_dfp(f) == c.outer.map(|h| h.into()));
                eprintln!("  heap:{:?} OK", c,);
            }
            cfp = self.frame_caller(f);
            eprintln!("----------------------------");
        }
    }*/

    fn frame_prev_cfp(&self, f: Frame) -> usize {
        self.exec_stack[f.0 + CFP_OFFSET].as_fnum() as usize
    }

    fn frame_mfp(&self, f: Frame) -> MethodFrame {
        MethodFrame::decode(self.exec_stack[f.0 + MFP_OFFSET])
    }

    pub(crate) fn frame_lfp(&self, f: Frame) -> LocalFrame {
        LocalFrame::decode(self.exec_stack[f.0 + LFP_OFFSET])
    }

    fn frame_mfp_encode(&self, f: Frame) -> Value {
        self.exec_stack[f.0 + MFP_OFFSET]
    }

    pub(crate) fn frame_dfp(&self, f: Frame) -> Option<Context> {
        assert!(self.frame_is_ruby_func(f));
        let dfp = self.exec_stack[f.0 + DFP_OFFSET].as_fnum();
        if dfp == 0 {
            None
        } else if dfp < 0 {
            Some(Frame(-dfp as usize).into())
        } else {
            Some(HeapCtxRef::from_ptr((dfp << 3) as *const HeapContext as *mut _).into())
        }
    }

    pub(crate) fn cur_frame_pc(&self) -> ISeqPos {
        //assert!(self.is_ruby_func());
        ISeqPos::from(self.exec_stack[self.cfp + PC_OFFSET].as_fnum() as u64 as u32 as usize)
    }

    pub(crate) fn cur_frame_pc_set(&mut self, pc: ISeqPos) {
        //assert!(self.is_ruby_func());
        let pc_ptr = &mut self.exec_stack[self.cfp + PC_OFFSET];
        *pc_ptr = Value::fixnum(pc.into_usize() as i64);
    }

    pub(crate) fn frame_locals(&self, f: Frame) -> &[Value] {
        let lfp = f.0 - self.frame_local_len(f) - 1;
        &self.exec_stack[lfp..f.0 - 1]
    }

    pub(crate) fn frame_context(&self, frame: Frame) -> Context {
        assert!(self.frame_is_ruby_func(frame));
        match self.frame_heap(frame) {
            Some(h) => h.into(),
            None => frame.into(),
        }
    }

    pub(crate) fn outer_context(&self, context: Context) -> Option<Context> {
        match context {
            Context::Frame(frame) => match self.frame_heap(frame) {
                Some(h) => h.outer().map(|h| h.into()),
                None => self.frame_dfp(frame),
            },
            Context::Heap(h) => h.outer().map(|h| h.into()),
        }
    }

    /// Get context of `frame`.
    ///
    /// If `frame` is a native (Rust) frame, return None.
    pub(super) fn frame_heap(&self, frame: Frame) -> Option<HeapCtxRef> {
        assert!(self.frame_is_ruby_func(frame));
        assert!(frame.0 != 0);
        let ctx = self.exec_stack[frame.0 + HEAP_OFFSET];
        match ctx.as_fnum() {
            0 => None,
            i => Some(HeapCtxRef::decode(i)),
        }
    }

    pub(super) fn frame_iseq(&self, frame: Frame) -> ISeqRef {
        assert!(self.frame_is_ruby_func(frame));
        let i = self.exec_stack[frame.0 + ISEQ_OFFSET].as_fnum();
        assert!(i != 0);
        ISeqRef::decode(i)
    }

    pub(super) fn frame_self(&self, frame: Frame) -> Value {
        assert!(frame.0 != 0);
        self.exec_stack[frame.0 - 1]
    }

    fn frame_block(&self, frame: Frame) -> Option<Block> {
        let val = self.exec_stack[frame.0 + BLK_OFFSET];
        Block::decode(val)
    }

    fn frame_local_len(&self, frame: Frame) -> usize {
        (self.exec_stack[frame.0 + FLAG_OFFSET].as_fnum() as usize) >> 32
    }

    pub(crate) fn frame_is_ruby_func(&self, frame: Frame) -> bool {
        (self.exec_stack[frame.0 + FLAG_OFFSET].get() & 0b1000_0000) != 0
    }
}

impl VM {
    /// Get the caller frame of `frame`.
    pub(super) fn frame_caller(&self, frame: Frame) -> Option<Frame> {
        let cfp = self.frame_prev_cfp(frame);
        Frame::from(cfp)
    }

    /// Set the context of `frame` to `ctx`.
    pub(crate) fn set_heap(&mut self, frame: Frame, heap: HeapCtxRef) {
        self.exec_stack[frame.0 + HEAP_OFFSET] = Value::fixnum(heap.encode());
        self.exec_stack[frame.0 + MFP_OFFSET] = heap.as_mfp().encode();
        self.exec_stack[frame.0 + LFP_OFFSET] = heap.as_lfp().encode();
    }
}

impl VM {
    /// Get current frame.
    pub(crate) fn cur_frame(&self) -> Frame {
        Frame::from(self.cfp).unwrap()
    }

    /// Get current method frame.
    fn cur_mfp(&self) -> MethodFrame {
        self.frame_mfp(self.cur_frame())
    }

    pub(crate) fn cur_outer_frame(&self) -> Frame {
        let mut frame = self.frame_caller(self.cur_frame());
        while let Some(f) = frame {
            if self.frame_is_ruby_func(f) {
                return f;
            }
            frame = self.frame_caller(f);
        }
        unreachable!("no caller frame");
    }

    pub(crate) fn cur_delegate(&self) -> Option<Value> {
        let lvar_id = self.cur_mfp().iseq().params.delegate?;
        let delegate = self.lfp.get(lvar_id);
        if delegate.is_nil() {
            None
        } else {
            Some(delegate)
        }
    }

    pub(crate) fn caller_method_block(&self) -> Option<Block> {
        let frame = self.cur_outer_frame();
        self.frame_mfp(frame).block()
    }

    pub(crate) fn caller_method_iseq(&self) -> ISeqRef {
        let frame = self.cur_outer_frame();
        self.frame_mfp(frame).iseq()
    }

    pub(super) fn get_method_block(&self) -> Option<Block> {
        self.cur_mfp().block()
    }

    pub(super) fn get_method_iseq(&self) -> ISeqRef {
        self.cur_mfp().iseq()
    }

    pub(super) fn cur_iseq(&self) -> ISeqRef {
        self.frame_iseq(self.cur_frame())
    }

    pub(crate) fn caller_iseq(&self) -> ISeqRef {
        let c = self.cur_outer_frame();
        self.frame_iseq(c)
    }

    pub(super) fn cur_source_info(&self) -> SourceInfoRef {
        self.cur_iseq().source_info.clone()
    }

    pub(super) fn get_loc(&self) -> Loc {
        let iseq = self.cur_iseq();
        let pc = self.cur_frame_pc();
        match iseq.iseq_sourcemap.iter().find(|x| x.0 == pc) {
            Some((_, loc)) => *loc,
            None => {
                panic!(
                    "Bad sourcemap. pc={:?} cur_pc={:?} {:?}",
                    self.pc, pc, iseq.iseq_sourcemap
                );
            }
        }
    }
}

impl VM {
    fn prepare_block_args(&mut self, iseq: ISeqRef, args_pos: usize) {
        // if a single Array argument is given for the block requiring multiple formal parameters,
        // the arguments must be expanded.
        let req_len = iseq.params.req;
        let post_len = iseq.params.post;
        if self.stack_len() - args_pos == 1 && req_len + post_len > 1 {
            if let Some(ary) = self.exec_stack[args_pos].as_array() {
                self.stack_pop();
                self.exec_stack.extend_from_slice(&ary.elements);
            }
        }
    }

    // Handling call frame

    pub(crate) fn init_frame(&mut self) {
        self.stack_push(Value::nil());
        self.cfp = 1;
        self.push_native_control_frame(0, 0, false);
    }

    /// Prepare ruby control frame on the top of stack.
    ///
    ///  ### Before
    ///~~~~text
    ///                                  sp
    ///                                   v
    /// +------+------+:-+------+------+------+------+------+--------
    /// |  a0  |  a1  |..|  an  | self |
    /// +------+------+--+------+------+------+------+------+--------
    ///  <----- local_len ------>
    ///~~~~
    ///
    ///  ### After
    ///~~~~text
    ///   lfp                            cfp                                              sp
    ///    v                              v                                                v
    /// +------+------+--+------+------+------+------+------+------+------+------+------+-----
    /// |  a0  |  a1  |..|  an  | self | flg  | cfp* | mfp  | dfp  |  pc* | ctx  | iseq |
    /// +------+------+--+------+------+------+------+------+------+------+------+------+-----
    ///  <-------- local frame --------> <-------------- control frame ---------------->
    ///~~~~
    ///
    /// - flg: flags
    /// - cfp*: prev cfp
    /// - mfp*: mfp
    /// - dfp*: dfp
    /// - pc*:  pc
    /// - ctx: ContextRef
    /// - iseq: ISeqRef
    /// - blk: Option<Block> the block passed to the method.
    ///
    pub(crate) fn prepare_frame(
        &mut self,
        local_len: usize,
        use_value: bool,
        ctx: impl Into<Option<HeapCtxRef>>,
        outer: Option<Context>,
        iseq: ISeqRef,
        block: Option<&Block>,
    ) {
        self.save_next_pc();
        let ctx = ctx.into();
        let prev_cfp = self.cfp;
        self.prev_len = self.stack_len() - local_len - 1;
        self.cfp = self.stack_len();
        assert!(prev_cfp != 0);
        let (mfp, outer) = match &outer {
            // In the case of Ruby method.
            None => (self.new_mfp_from_stack(self.cur_frame()).encode(), 0),
            // In the case of Ruby block.
            Some(outer) => match outer {
                Context::Frame(f) => (self.frame_mfp_encode(*f), f.encode()),
                Context::Heap(h) => (h.method().encode(), h.encode()),
            },
        };
        let lfp = match ctx {
            None => self.new_lfp_from_stack(self.prev_len),
            Some(h) => h.lfp(),
        };
        self.push_control_frame(
            prev_cfp, mfp, use_value, ctx, outer, iseq, local_len, block, lfp,
        );
        self.pc = ISeqPos::from(0);
        self.lfp = lfp;
        #[cfg(feature = "perf-method")]
        self.globals.methods.inc_counter(iseq.method);
        #[cfg(any(feature = "trace", feature = "trace-func"))]
        if self.globals.startup_flag {
            let ch = if self.is_called() { "+++" } else { "---" };
            eprintln!(
                "{}> {:?} {:?} {:?}",
                ch, iseq.method, iseq.kind, iseq.source_info.path
            );
        }
        #[cfg(feature = "trace-func")]
        self.dump_frame(self.cur_frame());
    }

    /// Prepare native control frame on the top of stack.
    ///
    ///  ### Before
    ///~~~~text
    ///                                  sp
    ///                                   v
    /// +------+------+:-+------+------+------+------+------+--------
    /// |  a0  |  a1  |..|  an  | self |
    /// +------+------+--+------+------+------+------+------+--------
    ///  <----- args_len ------>
    ///~~~~
    ///
    ///  ### After
    ///~~~~text
    ///   lfp                            cfp           sp
    ///    v                              v             v
    /// +------+------+--+------+------+------+------+-----
    /// |  a0  |  a1  |..|  an  | self | flg  | cfp* |
    /// +------+------+--+------+------+------+------+-----
    ///  <-------- local frame --------> <---------->
    ///                               native control frame
    ///~~~~
    ///
    /// - flg: flags
    /// - cfp*: prev cfp
    ///
    pub(crate) fn prepare_native_frame(&mut self, args_len: usize, use_value: bool) {
        self.save_next_pc();
        let prev_cfp = self.cfp;
        self.prev_len = self.stack_len() - args_len - 1;
        self.cfp = self.stack_len();
        self.lfp = self.new_lfp_from_stack(self.prev_len);
        self.push_native_control_frame(prev_cfp, args_len, use_value)
        //self.check_integrity();
    }

    fn save_next_pc(&mut self) {
        if self.is_ruby_func() {
            self.exec_stack[self.cfp + PC_OFFSET] = Value::fixnum(
                (self.exec_stack[self.cfp + PC_OFFSET].as_fnum() as u64 as u32 as u64
                    | ((self.pc.into_usize() as u64) << 32)) as i64,
            );
        }
    }

    #[cfg(feature = "trace-func")]
    pub(crate) fn dump_frame(&self, f: Frame) {
        if !self.globals.startup_flag {
            return;
        }
        eprintln!("STACK---------------------------------------------");
        eprintln!("{:?}", self.exec_stack);
        eprintln!("self: [{:?}]", self.frame_self(f));
        if self.frame_is_ruby_func(f) {
            eprintln!(
                "cfp:{:?} lfp:{:?} prev_len:{:?}",
                self.cfp, self.lfp, self.prev_len,
            );
            if let Some(offset) = self.check_within_stack(self.lfp) {
                eprintln!("LFP is on the stack: {}", offset);
            }
            let iseq = self.frame_iseq(f);
            let lvar = iseq.lvar.table();
            let local_len = iseq.lvars;
            let lfp = self.frame_lfp(f);
            for i in 0..local_len {
                eprint!("{:?}:[{:?}] ", lvar[i], lfp.get(LvarId::from(i)));
            }
            eprintln!("");
            if let Some(ctx) = self.frame_heap(f) {
                eprintln!("HEAP----------------------------------------------");
                eprintln!("self: [{:?}]", ctx.self_val());
                let iseq = ctx.iseq();
                let lvar = iseq.lvar.table();
                let local_len = iseq.lvars;
                let lfp = ctx.lfp();
                for i in 0..local_len {
                    eprint!("{:?}:[{:?}] ", lvar[i], lfp.get(LvarId::from(i)));
                }
                eprintln!("");
            }
            eprintln!("--------------------------------------------------");
        } else {
            eprintln!("cfp:{:?} prev_len:{:?}", self.cfp, self.prev_len,);
            for v in self.frame_locals(f) {
                eprint!("[{:?}] ", *v);
            }
        }
    }

    pub(super) fn unwind_frame(&mut self) {
        let cfp = self.frame_prev_cfp(self.cur_frame());
        assert!(cfp != 0);
        self.set_stack_len(self.prev_len);
        self.cfp = cfp;
        if self.is_ruby_func() {
            self.lfp = self.frame_lfp(self.cur_frame());
            self.pc = ISeqPos::from((self.exec_stack[cfp + PC_OFFSET].as_fnum() as usize) >> 32);
        }

        let local_len = (self.flag().as_fnum() as usize) >> 32;
        self.prev_len = cfp - local_len - 1;
        if self.globals.startup_flag {
            #[cfg(feature = "trace")]
            eprintln!("unwind lfp:{} cfp:{}", self.prev_len, self.cfp);
            #[cfg(feature = "trace-func")]
            self.dump_frame(self.cur_frame());
        }
    }

    pub(super) fn clear_stack(&mut self) {
        self.set_stack_len(
            self.cfp
                + if self.is_ruby_func() {
                    RUBY_FRAME_LEN
                } else {
                    NATIVE_FRAME_LEN
                },
        );
    }

    fn push_control_frame(
        &mut self,
        prev_cfp: usize,
        mfp: Value,
        use_value: bool,
        ctx: Option<HeapCtxRef>,
        outer: i64,
        iseq: ISeqRef,
        args_len: usize,
        block: Option<&Block>,
        lfp: LocalFrame,
    ) {
        let flag = if use_value { 0 } else { 2 } | ((args_len as i64) << 32);
        self.stack_append(&VM::control_frame(
            flag, prev_cfp, mfp, ctx, outer, iseq, block, lfp,
        ));
        self.set_ruby_func()
    }

    pub(super) fn control_frame(
        flag: i64,
        prev_cfp: usize,
        mfp: Value,
        ctx: Option<HeapCtxRef>,
        outer: i64,
        iseq: ISeqRef,
        block: Option<&Block>,
        lfp: LocalFrame,
    ) -> [Value; RUBY_FRAME_LEN] {
        [
            Value::fixnum(flag),
            Value::fixnum(prev_cfp as i64),
            mfp,
            Value::fixnum(outer),
            Value::fixnum(0),
            Value::fixnum(ctx.map_or(0, |ctx| ctx.encode())),
            Value::fixnum(iseq.encode()),
            match block {
                None => Value::fixnum(0),
                Some(block) => block.encode(),
            },
            lfp.encode(),
        ]
    }

    fn push_native_control_frame(&mut self, prev_cfp: usize, args_len: usize, use_value: bool) {
        self.stack_append(&[
            Value::fixnum(if use_value { 0 } else { 2 } | ((args_len as i64) << 32)),
            Value::fixnum(prev_cfp as i64),
        ]);
    }

    ///
    /// Frame flags.
    ///
    /// 0 0 0 0_0 0 0 1
    /// |       | | | |
    /// |       | | | +-- always 1 (represents Value::integer)
    /// |       | | +---- is_called (0: normaly invoked  1: vm_loop was called recursively)
    /// |       | +------ discard_value (0: use return value  1: discard return value)
    /// |       +-------- is_module_function (0: no 1:yes)
    /// +---------------- 1: Ruby func  0: native func
    ///
    pub(crate) fn frame_flag(&self, f: Frame) -> Value {
        self.exec_stack[f.0 + FLAG_OFFSET]
    }

    fn flag(&self) -> Value {
        let cfp = self.cfp;
        self.exec_stack[cfp + FLAG_OFFSET]
    }

    fn flag_mut(&mut self) -> &mut Value {
        let cfp = self.cfp;
        &mut self.exec_stack[cfp + FLAG_OFFSET]
    }

    pub(crate) fn is_called(&self) -> bool {
        self.flag().get() & 0b0010 != 0
    }

    pub(crate) fn set_called(&mut self) {
        let f = self.flag_mut();
        *f = Value::from(f.get() | 0b0010);
    }

    pub(crate) fn discard_val(&self) -> bool {
        self.flag().get() & 0b0100 != 0
    }

    pub(crate) fn is_ruby_func(&self) -> bool {
        self.flag().get() & 0b1000_0000 != 0
    }

    pub(crate) fn set_ruby_func(&mut self) {
        let f = self.flag_mut();
        *f = Value::from(f.get() | 0b1000_0000);
    }

    /// Check module_function flag of the current frame.
    pub(crate) fn is_module_function(&self) -> bool {
        self.cur_mfp().is_module_function()
    }

    /// Set module_function flag of the caller frame to true.
    pub(crate) fn set_module_function(&mut self) {
        self.frame_mfp(self.cur_outer_frame()).set_module_function();
    }
}

impl VM {
    pub(crate) fn fill_positional_arguments(&mut self, base: usize, iseq: ISeqRef) {
        let params = &iseq.params;
        let lvars = iseq.lvars;
        let args_len = self.stack_len() - base;
        let req_len = params.req;
        let rest_len = if params.rest == Some(true) { 1 } else { 0 };
        let post_len = params.post;
        let no_post_len = args_len - post_len;
        let optreq_len = req_len + params.opt;

        if optreq_len < no_post_len {
            if let Some(delegate) = params.delegate {
                let v = self.stack_slice(base, optreq_len..no_post_len).to_vec();
                self.exec_stack[base + delegate.as_usize()] = Value::array_from(v);
            }
            if rest_len == 1 {
                let ary = self.stack_slice(base, optreq_len..no_post_len).to_vec();
                self.exec_stack[base + optreq_len] = Value::array_from(ary);
            }
            // fill post_req params.
            self.stack_copy_within(base, no_post_len..args_len, optreq_len + rest_len);
            self.set_stack_len(
                base + optreq_len
                    + rest_len
                    + post_len
                    + if params.delegate.is_some() { 1 } else { 0 },
            );
            self.exec_stack.resize(base + lvars, Value::nil());
        } else {
            self.exec_stack.resize(base + lvars, Value::nil());
            // fill post_req params.
            self.stack_copy_within(base, no_post_len..args_len, optreq_len + rest_len);
            if no_post_len < req_len {
                // fill rest req params with nil.
                self.stack_fill(base, no_post_len..req_len, Value::nil());
                // fill rest opt params with uninitialized.
                self.stack_fill(base, req_len..optreq_len, Value::uninitialized());
            } else {
                // fill rest opt params with uninitialized.
                self.stack_fill(base, no_post_len..optreq_len, Value::uninitialized());
            }
            if rest_len == 1 {
                self.exec_stack[base + optreq_len] = Value::array_from(vec![]);
            }
        }

        iseq.lvar
            .kw
            .iter()
            .for_each(|id| self.exec_stack[base + id.as_usize()] = Value::uninitialized());
    }

    pub(crate) fn fill_keyword_arguments(
        &mut self,
        base: usize,
        iseq: ISeqRef,
        kw_arg: Value,
        ordinary_kwarg: bool,
    ) -> Result<(), RubyError> {
        let mut kwrest = FxIndexMap::default();
        if ordinary_kwarg {
            let keyword = kw_arg.as_hash().unwrap();
            for (k, v) in keyword.iter() {
                let id = k.as_symbol().unwrap();
                match iseq.params.keyword.get(&id) {
                    Some(lvar) => self.exec_stack[base + lvar.as_usize()] = v,
                    None => {
                        if iseq.params.kwrest {
                            kwrest.insert(HashKey(k), v);
                        } else {
                            return Err(RubyError::argument("Undefined keyword."));
                        }
                    }
                };
            }
        };
        if let Some(id) = iseq.lvar.kwrest_param() {
            self.exec_stack[base + id.as_usize()] = Value::hash_from_map(kwrest);
        }
        Ok(())
    }

    pub(crate) fn fill_block_argument(&mut self, base: usize, id: LvarId, block: &Option<Block>) {
        self.exec_stack[base + id.as_usize()] = block
            .as_ref()
            .map_or(Value::nil(), |block| self.create_proc(&block));
    }
}

impl VM {
    pub fn push_frame(
        &mut self,
        iseq: ISeqRef,
        args: &Args2,
        outer: Option<Context>,
        use_value: bool,
    ) -> Result<(), RubyError> {
        if iseq.opt_flag {
            return self.push_frame_fast(iseq, args, outer, use_value, args.block.as_ref());
        }
        let self_value = self.stack_pop();
        let base = self.stack_len() - args.len();
        let params = &iseq.params;
        let kw_flag = !args.kw_arg.is_nil();
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
            if kw_flag {
                self.stack_push(args.kw_arg);
            }
            (kw_flag, false)
        } else {
            (false, kw_flag)
        };
        if !iseq.is_block() {
            params.check_arity(positional_kwarg, args)?;
        } else {
            self.prepare_block_args(iseq, base);
        }
        self.fill_positional_arguments(base, iseq);
        // Handling keyword arguments and a keyword rest paramter.
        if params.kwrest || ordinary_kwarg {
            self.fill_keyword_arguments(base, iseq, args.kw_arg, ordinary_kwarg)?;
        };
        self.stack_push(self_value);
        self.prepare_frame(
            self.stack_len() - base - 1,
            use_value,
            None,
            outer,
            iseq,
            args.block.as_ref(),
        );
        // Handling block paramter.
        if let Some(id) = iseq.lvar.block_param() {
            self.fill_block_argument(base, id, &args.block);
        }
        Ok(())
    }

    fn push_frame_fast(
        &mut self,
        iseq: ISeqRef,
        args: &Args2,
        outer: Option<Context>,
        use_value: bool,
        block: Option<&Block>,
    ) -> Result<(), RubyError> {
        let self_value = self.stack_pop();
        let base = self.stack_len() - args.len();
        let lvars = iseq.lvars;
        if !iseq.is_block() {
            let min = iseq.params.req;
            let len = args.len();
            if len != min {
                return Err(RubyError::argument_wrong(len, min));
            }
        } else {
            self.prepare_block_args(iseq, base);
            let args_len = self.stack_len() - base;
            let req_len = iseq.params.req;
            if req_len < args_len {
                self.set_stack_len(base + req_len);
            }
        }

        self.exec_stack.resize(base + lvars, Value::nil());

        self.stack_push(self_value);
        self.prepare_frame(
            self.stack_len() - base - 1,
            use_value,
            None,
            outer,
            iseq,
            block,
        );
        Ok(())
    }

    /// Move outer execution contexts on the stack to the heap.
    pub fn move_frame_to_heap(&mut self, f: Frame) -> HeapCtxRef {
        if let Some(h) = self.frame_heap(f) {
            return h;
        }
        let flag = self.frame_flag(f).as_fnum();
        let self_val = self.frame_self(f);
        let iseq = self.frame_iseq(f);
        let outer = self.frame_dfp(f);
        let outer = match outer {
            Some(Context::Frame(f)) => Some(self.move_frame_to_heap(f)),
            Some(Context::Heap(h)) => Some(h),
            None => None,
        };
        let block = self.frame_block(f);
        let heap = HeapCtxRef::new_heap(
            flag,
            self_val,
            block,
            iseq,
            outer,
            Some(self.frame_locals(f)),
        );
        self.set_heap(f, heap);
        if self.cur_frame() == f {
            self.lfp = self.frame_lfp(f);
        }
        heap
    }
}
