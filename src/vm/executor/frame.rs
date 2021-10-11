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

const FLAG_OFFSET: usize = 0;
const CFP_OFFSET: usize = 1;
const MFP_OFFSET: usize = 2;
const DFP_OFFSET: usize = 3;
const PC_OFFSET: usize = 4;
const HEAP_OFFSET: usize = 5;
const ISEQ_OFFSET: usize = 6;
const BLK_OFFSET: usize = 7;
const NATIVE_FRAME_LEN: usize = 2;
const RUBY_FRAME_LEN: usize = 8;

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

    pub fn encode(&self) -> i64 {
        -(self.0 as i64)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MethodFrame(*mut Value);

impl MethodFrame {
    fn encode(self) -> Value {
        Value::from((self.0 as u64) | 0b1)
    }

    fn decode(v: Value) -> Self {
        Self((v.get() & (-2i64 as u64)) as *mut _)
    }

    fn iseq(self) -> ISeqRef {
        unsafe {
            let v = *self.0.add(ISEQ_OFFSET);
            ISeqRef::decode(v.as_fnum())
        }
    }

    fn block(self) -> Block {
        unsafe {
            let v = *self.0.add(BLK_OFFSET);
            Block::decode(v.as_fnum())
        }
    }
}

impl VM {
    fn new_mfp_heap(&self) -> MethodFrame {
        MethodFrame(Box::into_raw(Box::new([Value::nil(); 5])) as *mut _)
    }

    fn new_mfp_from_stack(&self, f: Frame) -> MethodFrame {
        MethodFrame(std::ptr::addr_of!(self.exec_stack[f.0]) as *mut _)
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

    fn frame_method_context(&self, f: Frame) -> Context {
        //assert!(self.frame_is_ruby_func(f));
        let mfp = self.exec_stack[f.0 + MFP_OFFSET].as_fnum();
        if mfp == 0 {
            unreachable!()
        } else if mfp < 0 {
            Frame(-mfp as usize).into()
        } else {
            HeapCtxRef::decode(mfp).into()
        }
    }

    fn frame_method_context_encode(&self, f: Frame) -> i64 {
        //assert!(self.frame_is_ruby_func(f));
        let mfp = self.exec_stack[f.0 + MFP_OFFSET].as_fnum();
        if mfp == 0 {
            unreachable!()
        } else if mfp < 0 {
            Frame(-mfp as usize).encode()
        } else {
            mfp
        }
    }

    pub fn frame_dfp(&self, f: Frame) -> Option<Context> {
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

    pub fn frame_outer(&self, f: Frame) -> Frame {
        let dfp = self.exec_stack[f.0 + DFP_OFFSET].as_fnum();
        assert!(dfp < 0);
        Frame(-dfp as usize)
    }

    pub fn cur_frame_pc(&self) -> ISeqPos {
        //assert!(self.is_ruby_func());
        ISeqPos::from(self.exec_stack[self.cfp + PC_OFFSET].as_fnum() as u64 as u32 as usize)
    }

    pub fn cur_frame_pc_set(&mut self, pc: ISeqPos) {
        //assert!(self.is_ruby_func());
        let pc_ptr = &mut self.exec_stack[self.cfp + PC_OFFSET];
        *pc_ptr = Value::fixnum(pc.into_usize() as i64);
    }

    pub fn frame_locals(&self, f: Frame) -> &[Value] {
        let lfp = f.0 - self.frame_local_len(f) - 1;
        self.slice(lfp, f.0 - 1)
    }

    pub fn frame_mut_locals(&mut self, f: Frame) -> &mut [Value] {
        let lfp = f.0 - self.frame_local_len(f) - 1;
        self.slice_mut(lfp, f.0 - 1)
    }

    pub fn frame_context(&self, frame: Frame) -> Context {
        assert!(self.frame_is_ruby_func(frame));
        match self.frame_heap(frame) {
            Some(h) => h.into(),
            None => frame.into(),
        }
    }

    pub fn outer_context(&self, context: Context) -> Option<Context> {
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
        match val.as_fixnum() {
            None => Some(val.into()),
            Some(0) => None,
            Some(i) => Some(Block::decode(i)),
        }
    }

    fn frame_local_len(&self, frame: Frame) -> usize {
        (self.exec_stack[frame.0 + FLAG_OFFSET].as_fnum() as usize) >> 32
    }

    pub fn frame_is_ruby_func(&self, frame: Frame) -> bool {
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
    pub(super) fn set_heap(&mut self, frame: Frame, heap: HeapCtxRef) {
        self.exec_stack[frame.0 + HEAP_OFFSET] = Value::fixnum(heap.encode());
    }
}

impl VM {
    /// Get current frame.
    pub(super) fn cur_frame(&self) -> Frame {
        Frame::from(self.cfp).unwrap()
    }

    /// Get current method frame.
    fn cur_method_context(&self) -> Context {
        self.frame_method_context(self.cur_frame())
    }

    pub fn cur_outer_frame(&self) -> Frame {
        let mut frame = self.frame_caller(self.cur_frame());
        while let Some(f) = frame {
            if self.frame_is_ruby_func(f) {
                return f;
            }
            frame = self.frame_caller(f);
        }
        unreachable!("no caller frame");
    }

    pub fn cur_delegate(&self) -> Option<Value> {
        let delegate = match self.cur_method_context() {
            Context::Frame(f) => {
                let v = self.frame_iseq(f).params.delegate?;
                self.frame_locals(f)[*v]
            }
            Context::Heap(h) => {
                let v = h.iseq().params.delegate?;
                h[v]
            }
        };
        if delegate.is_nil() {
            None
        } else {
            Some(delegate)
        }
    }

    pub fn caller_method_block(&self) -> Option<Block> {
        let frame = self.cur_outer_frame();
        match self.frame_method_context(frame) {
            Context::Frame(f) => self.frame_block(f),
            Context::Heap(h) => h.block(),
        }
    }

    pub fn caller_method_iseq(&self) -> ISeqRef {
        let frame = self.cur_outer_frame();
        match self.frame_method_context(frame) {
            Context::Frame(f) => self.frame_iseq(f),
            Context::Heap(h) => h.iseq(),
        }
    }

    pub(super) fn get_method_block(&self) -> Option<Block> {
        match self.cur_method_context() {
            Context::Frame(f) => self.frame_block(f),
            Context::Heap(h) => h.block(),
        }
    }

    pub(super) fn get_method_iseq(&self) -> ISeqRef {
        match self.cur_method_context() {
            Context::Frame(f) => self.frame_iseq(f),
            Context::Heap(h) => h.iseq(),
        }
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

    pub fn init_frame(&mut self) {
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
    ///  <----- args_len ------>
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
    pub fn prepare_frame(
        &mut self,
        args_len: usize,
        use_value: bool,
        ctx: impl Into<Option<HeapCtxRef>>,
        outer: Option<Context>,
        iseq: ISeqRef,
        block: Option<&Block>,
    ) {
        self.save_next_pc();
        let ctx = ctx.into();
        let prev_cfp = self.cfp;
        self.lfp = self.stack_len() - args_len - 1;
        self.cfp = self.stack_len();
        assert!(prev_cfp != 0);
        let (mfp, outer) = match &outer {
            // In the case of Ruby method.
            None => (self.cur_frame().encode(), 0),
            // In the case of Ruby block.
            Some(outer) => match outer {
                Context::Frame(f) => (self.frame_method_context_encode(*f), f.encode()),
                Context::Heap(h) => (h.method_context().encode(), h.encode()),
            },
        };
        self.push_control_frame(prev_cfp, mfp, use_value, ctx, outer, iseq, args_len, block);
        self.pc = ISeqPos::from(0);
        #[cfg(feature = "perf-method")]
        MethodRepo::inc_counter(iseq.method);
        #[cfg(any(feature = "trace", feature = "trace-func"))]
        if self.globals.startup_flag {
            let ch = if self.is_called() { "+++" } else { "---" };
            eprintln!(
                "{}> {:?} {:?} {:?}",
                ch, iseq.method, iseq.kind, iseq.source_info.path
            );
        }
        //self.check_integrity();
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
    pub fn prepare_native_frame(&mut self, args_len: usize, use_value: bool) {
        self.save_next_pc();
        let prev_cfp = self.cfp;
        self.lfp = self.stack_len() - args_len - 1;
        self.cfp = self.stack_len();
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

    #[cfg(feature = "trace")]
    pub fn dump_current_frame(&self) {
        if self.globals.startup_flag && self.is_ruby_func() {
            eprintln!("lfp:{} cfp:{}", self.lfp, self.cfp,);
            eprintln!("LOCALS---------------------------------------------");
            for i in self.lfp..self.cfp {
                eprint!("[{:?}] ", self.exec_stack[i]);
            }
            eprintln!("\nCUR CTX------------------------------------------");
            if let Some(ctx) = self.frame_heap(self.cur_frame()) {
                eprintln!("{:?}", *ctx);
                eprintln!("lvars: {:?}", ctx.iseq().lvars);
                eprintln!("param: {:?}", ctx.iseq().params);
            } else {
                eprintln!("None");
            }
        }
    }

    pub(super) fn unwind_frame(&mut self) {
        let cfp = self.frame_prev_cfp(self.cur_frame());
        assert!(cfp != 0);
        self.set_stack_len(self.lfp);
        self.cfp = cfp;
        if self.is_ruby_func() {
            self.pc = ISeqPos::from((self.exec_stack[cfp + PC_OFFSET].as_fnum() as usize) >> 32);
        }

        let args_len = (self.flag().as_fnum() as usize) >> 32;
        self.lfp = cfp - args_len - 1;
        #[cfg(feature = "trace")]
        if self.globals.startup_flag {
            eprintln!("unwind lfp:{} cfp:{}", self.lfp, self.cfp);
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
        mfp: i64,
        use_value: bool,
        ctx: Option<HeapCtxRef>,
        outer: i64,
        iseq: ISeqRef,
        args_len: usize,
        block: Option<&Block>,
    ) {
        self.stack_append(&[
            Value::fixnum(if use_value { 0 } else { 2 } | ((args_len as i64) << 32)),
            Value::fixnum(prev_cfp as i64),
            Value::fixnum(mfp),
            Value::fixnum(outer),
            Value::fixnum(0),
            Value::fixnum(ctx.map_or(0, |ctx| ctx.encode())),
            Value::fixnum(iseq.encode()),
            match block {
                None => Value::fixnum(0),
                Some(block) => block.encode(),
            },
        ]);
        self.set_ruby_func()
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
    fn flag(&self) -> Value {
        let cfp = self.cfp;
        self.exec_stack[cfp + FLAG_OFFSET]
    }

    fn flag_mut(&mut self) -> &mut Value {
        let cfp = self.cfp;
        &mut self.exec_stack[cfp + FLAG_OFFSET]
    }

    pub fn is_called(&self) -> bool {
        self.flag().get() & 0b0010 != 0
    }

    pub fn set_called(&mut self) {
        let f = self.flag_mut();
        *f = Value::from(f.get() | 0b0010);
    }

    pub fn discard_val(&self) -> bool {
        self.flag().get() & 0b0100 != 0
    }

    pub fn set_discard_val(&mut self) {
        let f = self.flag_mut();
        *f = Value::from(f.get() | 0b0100);
    }

    pub fn is_ruby_func(&self) -> bool {
        self.flag().get() & 0b1000_0000 != 0
    }

    pub fn set_ruby_func(&mut self) {
        let f = self.flag_mut();
        *f = Value::from(f.get() | 0b1000_0000);
    }

    /// Check module_function flag of the current frame.
    pub fn is_module_function(&self) -> bool {
        match self.cur_method_context() {
            Context::Frame(mfp) => self.exec_stack[mfp.0 + FLAG_OFFSET].get() & 0b1000 != 0,
            Context::Heap(h) => h.flag().get() & 0b1000 != 0,
        }
    }

    /// Set module_function flag of the caller frame to true.
    pub fn set_module_function(&mut self) {
        match self.frame_method_context(self.cur_outer_frame()) {
            Context::Frame(mfp) => {
                let f = &mut self.exec_stack[mfp.0 + FLAG_OFFSET];
                *f = Value::from(f.get() | 0b1000);
            }
            Context::Heap(mut h) => {
                let f = h.flag_mut();
                *f = Value::from(f.get() | 0b1000);
            }
        };
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

        #[cfg(feature = "trace")]
        self.dump_current_frame();
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

        #[cfg(feature = "trace")]
        self.dump_current_frame();
        Ok(())
    }

    /// Move outer execution contexts on the stack to the heap.
    pub fn move_frame_to_heap(&mut self, f: Frame) -> HeapCtxRef {
        if let Some(h) = self.frame_heap(f) {
            return h;
        }
        let self_val = self.frame_self(f);
        let iseq = self.frame_iseq(f);
        let outer = self.frame_dfp(f);
        let outer = match outer {
            Some(Context::Frame(f)) => Some(self.move_frame_to_heap(f)),
            Some(Context::Heap(h)) => Some(h),
            None => None,
        };
        let block = self.frame_block(f);
        let heap = HeapCtxRef::new_heap(self_val, block, iseq, outer, Some(self.frame_locals(f)));
        self.set_heap(f, heap);
        heap
    }
}
