use super::*;
use std::ops::IndexMut;

pub const CFP_OFFSET: usize = 0;
pub const LFP_OFFSET: usize = 1;
pub const FLAG_OFFSET: usize = 2;
pub const MFP_OFFSET: usize = 3;
pub const DFP_OFFSET: usize = 4;
pub const PC_OFFSET: usize = 5;
pub const HEAP_OFFSET: usize = 6;
pub const ISEQ_OFFSET: usize = 7;
pub const BLK_OFFSET: usize = 8;
pub const NATIVE_FRAME_LEN: usize = 3;
pub const RUBY_FRAME_LEN: usize = 9;

/// Control frame on the RubyStack.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame(pub usize);

impl Frame {
    #[inline(always)]
    fn from(fp: usize) -> Option<Self> {
        if fp == 0 {
            None
        } else {
            Some(Frame(fp))
        }
    }
}

pub(crate) trait CF: Copy {
    fn as_ptr(self) -> *mut Value;

    fn from_ptr(p: *mut Value) -> Self;

    fn local_len(&self) -> usize;

    #[inline(always)]
    fn self_value(&self) -> Value {
        unsafe { *self.as_ptr().sub(1) }
    }

    #[inline(always)]
    fn lfp(&self) -> LocalFrame {
        let v = unsafe { *self.as_ptr().add(LFP_OFFSET) };
        LocalFrame::decode(v)
    }

    #[inline(always)]
    fn enc(self) -> Value {
        Value::from((self.as_ptr() as u64) | 0b1)
    }

    #[inline(always)]
    fn dec(v: Value) -> *mut Value {
        (v.get() & (-2i64 as u64)) as *mut _
    }

    #[inline(always)]
    fn mfp(&self) -> ControlFrame {
        let v = unsafe { *self.as_ptr().add(MFP_OFFSET) };
        ControlFrame(ControlFrame::dec(v))
    }

    #[inline(always)]
    fn flag(&self) -> Value {
        unsafe { *self.as_ptr().add(FLAG_OFFSET) }
    }

    #[inline(always)]
    fn is_ruby_func(&self) -> bool {
        self.flag().get() & 0b1000_0000 != 0
    }

    #[inline(always)]
    fn dfp(&self) -> Option<DynamicFrame> {
        debug_assert!(self.is_ruby_func());
        let v = unsafe { *self.as_ptr().add(DFP_OFFSET) };
        DynamicFrame::decode(v)
    }

    #[inline(always)]
    fn heap(&self) -> Option<HeapCtxRef> {
        debug_assert!(self.is_ruby_func());
        let ctx = unsafe { *self.as_ptr().add(HEAP_OFFSET) };
        match ctx.as_fnum() {
            0 => None,
            i => Some(HeapCtxRef::decode(i)),
        }
    }

    #[inline(always)]
    fn iseq(self) -> ISeqRef {
        debug_assert!(self.is_ruby_func());
        unsafe {
            let v = *self.as_ptr().add(ISEQ_OFFSET);
            ISeqRef::decode(v.as_fnum())
        }
    }

    /// Set the context of `frame` to `ctx`.
    fn set_heap(self, heap: HeapCtxRef) {
        let dfp = heap.as_dfp();
        unsafe {
            *self.as_ptr().add(HEAP_OFFSET) = Value::fixnum(heap.encode());
            *self.as_ptr().add(MFP_OFFSET) = dfp.mfp().encode();
            *self.as_ptr().add(LFP_OFFSET) = dfp.lfp().encode();
            *self.as_ptr().add(DFP_OFFSET) = DynamicFrame::encode(dfp.dfp());
        }
    }

    fn frame(&self) -> &[Value] {
        debug_assert!(self.heap().is_none());
        debug_assert!(self.is_ruby_func());
        let lfp = self.lfp();
        unsafe {
            let len = self.as_ptr().offset_from(lfp.0);
            assert!(len > 0);
            std::slice::from_raw_parts(lfp.0, len as usize + RUBY_FRAME_LEN)
        }
    }

    #[inline(always)]
    fn locals(&self) -> &[Value] {
        let lfp = self.lfp();
        let len = self.local_len() + 1;
        unsafe { std::slice::from_raw_parts(lfp.0, len) }
    }
}

///
/// Control frame
///
/// Wrapped raw pointer which points to a certain point within `RubyStack`.
/// You can obtain or alter various information like cfp, lfp, and the number of local variables
/// in the frame through `ControlFrame`.
///
/// There is some assumptions for using Control Frame safely.
///
/// - The address which is pointed by `ControlFrame` must be on the execution stack.
/// - `ControlFrame` may be Ruby func or native func.
///
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControlFrame(*mut Value);

impl std::ops::Sub<StackPtr> for ControlFrame {
    type Output = usize;
    #[inline(always)]
    fn sub(self, other: StackPtr) -> usize {
        unsafe {
            let offset = self.0.offset_from(other.0);
            assert!(offset >= 0);
            offset as usize
        }
    }
}

impl std::default::Default for ControlFrame {
    #[inline(always)]
    fn default() -> Self {
        Self(std::ptr::null_mut())
    }
}

impl CF for ControlFrame {
    #[inline(always)]
    fn as_ptr(self) -> *mut Value {
        self.0
    }

    #[inline(always)]
    fn from_ptr(p: *mut Value) -> Self {
        Self(p)
    }

    #[inline(always)]
    fn local_len(&self) -> usize {
        (self.flag().as_fnum() as usize) >> 32
    }
}

impl ControlFrame {
    #[inline(always)]
    pub(super) fn from_ref(r: &[Value]) -> Self {
        Self(r.as_ptr() as *mut _)
    }

    #[inline(always)]
    pub(crate) fn as_dfp(self) -> DynamicFrame {
        //assert!(self.is_ruby_func());
        DynamicFrame(self.0)
    }

    #[inline(always)]
    pub(super) fn decode(v: Value) -> Self {
        Self(Self::dec(v))
    }

    #[inline(always)]
    pub(super) fn encode(self) -> Value {
        self.enc()
    }

    #[inline(always)]
    pub(super) fn pc(&self) -> ISeqPos {
        ISeqPos::from(unsafe { (*self.0.add(PC_OFFSET)).as_fnum() as usize })
    }

    #[inline(always)]
    pub(super) fn set_pc(&mut self, pc: usize) {
        unsafe {
            *self.0.add(PC_OFFSET) = Value::fixnum(pc as i64);
        }
    }

    #[inline(always)]
    pub(super) fn block(self) -> Option<Block> {
        unsafe {
            let v = *self.0.add(BLK_OFFSET);
            Block::decode(v)
        }
    }

    #[inline(always)]
    fn flag_mut(&mut self) -> &mut Value {
        unsafe { &mut *self.0.add(FLAG_OFFSET) }
    }

    #[inline(always)]
    fn is_module_function(self) -> bool {
        self.flag().get() & 0b1000 != 0
    }

    #[inline(always)]
    fn set_module_function(mut self) {
        *self.flag_mut() = Value::from(self.flag().get() | 0b1000);
    }
}

///
/// Dynamic frame
///
/// Wrapped raw pointer which points to a control frame on the stack or heap.
/// You can obtain or alter various information like cfp, lfp, and the number of local variables
/// in the frame through `DynamicFrame`.
///
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicFrame(*mut Value);

impl CF for DynamicFrame {
    #[inline(always)]
    fn as_ptr(self) -> *mut Value {
        self.0
    }

    #[inline(always)]
    fn from_ptr(p: *mut Value) -> Self {
        Self(p)
    }

    #[inline(always)]
    fn local_len(&self) -> usize {
        self.iseq().lvars
    }
}

impl std::default::Default for DynamicFrame {
    #[inline(always)]
    fn default() -> Self {
        Self(std::ptr::null_mut())
    }
}

impl GC for DynamicFrame {
    fn mark(&self, alloc: &mut Allocator) {
        self.locals().iter().for_each(|v| v.mark(alloc));
        if let Some(d) = self.dfp() {
            d.mark(alloc)
        }
    }
}

impl DynamicFrame {
    #[inline(always)]
    pub(super) fn from_ref(r: &[Value]) -> Self {
        Self(r.as_ptr() as *mut _)
    }

    #[inline(always)]
    pub(super) fn decode(v: Value) -> Option<Self> {
        let ptr = Self::dec(v);
        if ptr.is_null() {
            None
        } else {
            Some(Self(ptr))
        }
    }

    #[inline(always)]
    pub(super) fn encode(opt: Option<Self>) -> Value {
        match opt {
            Some(d) => d.enc(),
            None => Self::default().enc(),
        }
    }

    #[inline(always)]
    pub(crate) fn outer(&self) -> Option<DynamicFrame> {
        let v = unsafe { *self.0.add(DFP_OFFSET) };
        DynamicFrame::decode(v)
    }
}

///
/// Local frame
///
/// Wrapped raw pointer which points to a local variables area on the stack or heap.
/// You can handle local variables of the frame.
///
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalFrame(pub(super) *mut Value);

impl std::default::Default for LocalFrame {
    #[inline(always)]
    fn default() -> Self {
        LocalFrame(std::ptr::null_mut())
    }
}

impl LocalFrame {
    #[inline(always)]
    pub(super) fn from_ref(r: &[Value]) -> Self {
        Self(r.as_ptr() as *mut _)
    }

    #[inline(always)]
    pub(crate) fn as_ptr(self) -> *mut Value {
        self.0
    }

    #[inline(always)]
    pub(super) fn encode(self) -> Value {
        Value::from((self.0 as u64) | 0b1)
    }

    #[inline(always)]
    pub(super) fn decode(v: Value) -> Self {
        Self((v.get() & (-2i64 as u64)) as *mut _)
    }
}

impl Index<LvarId> for LocalFrame {
    type Output = Value;
    #[inline(always)]
    fn index(&self, index: LvarId) -> &Self::Output {
        &self[index.as_usize()]
    }
}

impl IndexMut<LvarId> for LocalFrame {
    #[inline(always)]
    fn index_mut(&mut self, index: LvarId) -> &mut Self::Output {
        unsafe { &mut *self.0.add(index.into()) }
    }
}

impl Index<usize> for LocalFrame {
    type Output = Value;
    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        unsafe { &*self.0.add(index) }
    }
}

impl Index<std::ops::Range<usize>> for LocalFrame {
    type Output = [Value];
    #[inline(always)]
    fn index(&self, range: std::ops::Range<usize>) -> &Self::Output {
        unsafe { std::slice::from_raw_parts(self.0.add(range.start), range.end - range.start) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StackPtr(*mut Value);

impl std::default::Default for StackPtr {
    #[inline(always)]
    fn default() -> Self {
        StackPtr(std::ptr::null_mut())
    }
}

impl std::ops::Add<usize> for StackPtr {
    type Output = Self;
    #[inline(always)]
    fn add(self, other: usize) -> Self {
        Self(unsafe { self.0.add(other) })
    }
}

impl std::ops::Sub<usize> for StackPtr {
    type Output = Self;
    #[inline(always)]
    fn sub(self, other: usize) -> Self {
        Self(unsafe { self.0.sub(other) })
    }
}

impl StackPtr {
    #[inline(always)]
    pub(super) fn as_ptr(self) -> *mut Value {
        self.0
    }

    #[inline(always)]
    pub(super) fn from(ptr: *mut Value) -> Self {
        Self(ptr)
    }

    #[inline(always)]
    pub(crate) fn as_cfp(self) -> ControlFrame {
        ControlFrame::from_ptr(self.0)
    }
}

impl VM {
    pub(super) fn cfp_index(&self, cfp: ControlFrame) -> usize {
        unsafe {
            let ptr = self.exec_stack.as_ptr() as *mut Value;
            let offset = cfp.0.offset_from(ptr);
            assert!(offset >= 0);
            offset as usize
        }
    }

    pub(super) fn cfp(&self) -> usize {
        self.cfp_index(self.cfp)
    }

    #[inline(always)]
    pub(crate) fn cfp_from_frame(&self, f: Frame) -> ControlFrame {
        unsafe {
            let ptr = self.exec_stack.as_ptr();
            ControlFrame(ptr.add(f.0) as *mut _)
        }
    }

    #[inline(always)]
    pub(crate) fn dfp_from_frame(&self, f: Frame) -> DynamicFrame {
        unsafe {
            let ptr = self.exec_stack.as_ptr();
            DynamicFrame(ptr.add(f.0) as *mut _)
        }
    }

    pub(super) fn cfp_is_zero(&self, f: ControlFrame) -> bool {
        let ptr = self.exec_stack.as_ptr() as *mut Value;
        f.0 == ptr
    }

    #[inline(always)]
    pub(super) fn prev_cfp(&self, cfp: ControlFrame) -> Option<ControlFrame> {
        let v = unsafe { *cfp.0.add(CFP_OFFSET) };
        let prev_cfp = ControlFrame::decode(v);
        if self.cfp_is_zero(prev_cfp) {
            None
        } else {
            Some(prev_cfp)
        }
    }

    #[inline(always)]
    fn lfp_from_sp(&self, local_len: usize) -> LocalFrame {
        LocalFrame((self.sp() - local_len - 1).0)
    }

    #[inline(always)]
    pub(super) fn prev_sp(&self) -> StackPtr {
        let local_len = self.cfp.local_len();
        let cfp = StackPtr(self.cfp.0);
        cfp - local_len - 1
    }
}

impl VM {
    pub(super) fn frame_self(&self, frame: Frame) -> Value {
        assert!(frame.0 != 0);
        self.exec_stack[frame.0 - 1]
    }
}

impl VM {
    /// Get current frame.
    #[inline(always)]
    pub(crate) fn cur_frame(&self) -> Frame {
        Frame::from(self.cfp()).unwrap()
    }

    /// Get current method frame.
    fn cur_mfp(&self) -> ControlFrame {
        self.cfp.mfp()
    }

    pub(crate) fn cur_outer_cfp(&self) -> ControlFrame {
        let mut cfp = self.prev_cfp(self.cfp);
        while let Some(f) = cfp {
            if f.is_ruby_func() {
                return f;
            }
            cfp = self.prev_cfp(f);
        }
        unreachable!("no caller frame");
    }

    pub(crate) fn cur_outer_frame(&self) -> Frame {
        let cfp = self.cur_outer_cfp();
        Frame(self.cfp_index(cfp))
    }

    pub(crate) fn cur_delegate(&self) -> Option<Value> {
        let lvar_id = self.cur_mfp().iseq().params.delegate?;
        let delegate = self.lfp[lvar_id];
        if delegate.is_nil() {
            None
        } else {
            Some(delegate)
        }
    }

    pub(crate) fn caller_method_block(&self) -> Option<Block> {
        self.cur_outer_cfp().mfp().block()
    }

    pub(crate) fn caller_method_iseq(&self) -> ISeqRef {
        self.cur_outer_cfp().mfp().iseq()
    }

    pub(super) fn get_method_block(&self) -> Option<Block> {
        self.cur_mfp().block()
    }

    pub(super) fn get_method_iseq(&self) -> ISeqRef {
        self.cur_mfp().iseq()
    }

    #[inline(always)]
    pub(super) fn cur_iseq(&self) -> ISeqRef {
        self.cfp.iseq()
    }

    pub(crate) fn caller_iseq(&self) -> ISeqRef {
        self.cur_outer_cfp().iseq()
    }

    pub(super) fn cur_source_info(&self) -> SourceInfoRef {
        self.cur_iseq().source_info.clone()
    }

    pub(super) fn get_loc(&self) -> Loc {
        let iseq = self.cur_iseq();
        let cur_pc = self.pc_offset();
        match iseq
            .iseq_sourcemap
            .iter()
            .find(|x| x.0.into_usize() == cur_pc)
        {
            Some((_, loc)) => *loc,
            None => {
                panic!("Bad sourcemap. pc={:?} {:?}", self.pc, iseq.iseq_sourcemap);
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
        self.cfp = self.cfp_from_frame(Frame(1));
        self.push_native_control_frame(self.cfp_from_frame(Frame(0)), LocalFrame::default(), 0);
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
    fn prepare_frame(
        &mut self,
        local_len: usize,
        use_value: bool,
        outer: Option<DynamicFrame>,
        iseq: ISeqRef,
        block: Option<&Block>,
    ) {
        self.push_control_frame(
            use_value,
            None,
            outer,
            iseq,
            local_len,
            block,
            self.lfp_from_sp(local_len),
        );
    }

    fn prepare_block_frame(
        &mut self,
        local_len: usize,
        use_value: bool,
        outer: Option<DynamicFrame>,
        iseq: ISeqRef,
    ) {
        self.push_control_frame(
            use_value,
            None,
            outer,
            iseq,
            local_len,
            None,
            self.lfp_from_sp(local_len),
        );
    }

    fn prepare_method_frame(
        &mut self,
        local_len: usize,
        use_value: bool,
        iseq: ISeqRef,
        block: Option<&Block>,
    ) {
        self.push_control_frame(
            use_value,
            None,
            None,
            iseq,
            local_len,
            block,
            self.lfp_from_sp(local_len),
        );
    }

    pub(crate) fn prepare_frame_from_heap(&mut self, ctx: HeapCtxRef) {
        let outer = ctx.outer();
        let iseq = ctx.iseq();
        self.push_control_frame(true, Some(ctx), outer, iseq, 0, None, ctx.lfp());
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
    ///   lfp                            cfp                 sp
    ///    v                              v                   v
    /// +------+------+--+------+------+------+------+-----+---
    /// |  a0  |  a1  |..|  an  | self | cfp* | lfp  | flg |
    /// +------+------+--+------+------+------+------+-----+---
    ///  <-------- local frame --------> <---------->
    ///                               native control frame
    ///~~~~
    ///
    /// - cfp*: prev cfp
    /// - lfp: lfp
    /// - flg: flags
    ///
    pub(crate) fn prepare_native_frame(&mut self, args_len: usize) {
        self.save_next_pc();
        let prev_cfp = self.cfp;
        self.cfp = self.sp().as_cfp();
        self.lfp = self.lfp_from_sp(args_len);
        self.push_native_control_frame(prev_cfp, self.lfp, args_len)
    }

    fn save_next_pc(&mut self) {
        if self.is_ruby_func() {
            let pc = self.pc_offset();
            self.cfp.set_pc(pc);
        }
    }

    pub(super) fn unwind_frame(&mut self) {
        let cfp = self.prev_cfp(self.cfp).unwrap();
        self.exec_stack.sp = self.prev_sp();
        self.cfp = cfp;
        self.lfp = cfp.lfp();
        if self.is_ruby_func() {
            self.set_pc(cfp.pc());
        }
        #[cfg(feature = "trace-func")]
        self.dump_frame(self.cfp);
    }

    pub(super) fn clear_stack(&mut self) {
        self.set_stack_len(
            self.cfp()
                + if self.is_ruby_func() {
                    RUBY_FRAME_LEN
                } else {
                    NATIVE_FRAME_LEN
                },
        );
    }

    #[inline(always)]
    fn push_control_frame(
        &mut self,
        use_value: bool,
        ctx: Option<HeapCtxRef>,
        outer: Option<DynamicFrame>,
        iseq: ISeqRef,
        local_len: usize,
        block: Option<&Block>,
        lfp: LocalFrame,
    ) {
        self.save_next_pc();
        let prev_cfp = self.cfp;
        self.cfp = self.sp().as_cfp();
        debug_assert!(!self.cfp_is_zero(prev_cfp));
        let mfp = match &outer {
            // In the case of Ruby method.
            None => self.cfp,
            // In the case of Ruby block.
            Some(outer) => outer.mfp(),
        };
        let flag = VM::ruby_flag(use_value, local_len);

        self.stack_push(prev_cfp.encode());
        self.stack_push(lfp.encode());
        self.stack_push(Value::fixnum(flag));
        self.stack_push(mfp.encode());
        self.stack_push(DynamicFrame::encode(outer));
        self.stack_push(Value::fixnum(0));
        self.stack_push(Value::fixnum(ctx.map_or(0, |ctx| ctx.encode())));
        self.stack_push(Value::fixnum(iseq.encode()));
        self.stack_push(match block {
            None => Value::fixnum(0),
            Some(block) => block.encode(),
        });

        self.pc = ISeqPtr::from_iseq(&iseq.iseq);
        self.lfp = lfp;
        #[cfg(feature = "perf-method")]
        self.globals.methods.inc_counter(iseq.method);
        #[cfg(feature = "trace")]
        {
            let ch = /*if self.is_called() {*/ "+++" /* } else { "---" }*/;
            eprintln!(
                "{}> {:?} {:?} {:?}",
                ch, iseq.method, iseq.kind, iseq.source_info.path
            );
        }
        #[cfg(feature = "trace-func")]
        {
            self.dump_frame(self.cfp);
        }
    }

    pub(super) fn control_frame(
        flag: i64,
        prev_cfp: ControlFrame,
        mfp: ControlFrame,
        ctx: Option<HeapCtxRef>,
        outer: Option<DynamicFrame>,
        iseq: ISeqRef,
        block: Option<&Block>,
        lfp: LocalFrame,
    ) -> [Value; RUBY_FRAME_LEN] {
        [
            prev_cfp.encode(),
            lfp.encode(),
            Value::fixnum(flag),
            mfp.encode(),
            DynamicFrame::encode(outer),
            Value::fixnum(0),
            Value::fixnum(ctx.map_or(0, |ctx| ctx.encode())),
            Value::fixnum(iseq.encode()),
            match block {
                None => Value::fixnum(0),
                Some(block) => block.encode(),
            },
        ]
    }

    fn push_native_control_frame(
        &mut self,
        prev_cfp: ControlFrame,
        lfp: LocalFrame,
        args_len: usize,
    ) {
        self.stack_push(prev_cfp.encode());
        self.stack_push(lfp.encode());
        self.stack_push(Value::fixnum((args_len as i64) << 32));
    }

    ///
    /// Frame flags.
    ///
    /// 0 0 0 0_0 0 0 1
    /// |       | | | |
    /// |       | | | +-- always 1 (represents Value::integer)
    /// |       | | +----
    /// |       | +------ discard_value (0: use return value  1: discard return value)
    /// |       +-------- is_module_function (0: no 1:yes)
    /// +---------------- 1: Ruby func  0: native func
    ///
    #[inline(always)]
    fn flag(&self) -> Value {
        unsafe { *self.cfp.0.add(FLAG_OFFSET) }
    }

    #[inline(always)]
    pub(crate) fn discard_val(&self) -> bool {
        self.flag().get() & 0b0100 != 0
    }

    #[inline(always)]
    pub(crate) fn is_ruby_func(&self) -> bool {
        self.flag().get() & 0b1000_0000 != 0
    }

    #[inline(always)]
    pub(crate) fn ruby_flag(use_value: bool, local_len: usize) -> i64 {
        (if use_value { 0b100_0000 } else { 0b100_0010 }) | ((local_len as i64) << 32)
    }

    /// Check module_function flag of the current frame.
    pub(crate) fn is_module_function(&self) -> bool {
        self.cur_mfp().is_module_function()
    }

    /// Set module_function flag of the caller frame to true.
    pub(crate) fn set_module_function(&mut self) {
        self.cur_outer_cfp().mfp().set_module_function();
    }
}

impl VM {
    fn fill_positional_arguments(&mut self, base: usize, iseq: ISeqRef) {
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
            self.exec_stack.resize(base + lvars);
        } else {
            self.exec_stack.resize(base + lvars);
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

    fn fill_keyword_arguments(
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

    fn fill_block_argument(&mut self, base: usize, id: LvarId, block: &Option<Block>) {
        self.exec_stack[base + id.as_usize()] = block
            .as_ref()
            .map_or(Value::nil(), |block| self.create_proc(block));
    }
}

impl VM {
    pub(crate) fn push_frame(
        &mut self,
        iseq: ISeqRef,
        args: &Args2,
        outer: Option<DynamicFrame>,
        use_value: bool,
        is_method: bool,
    ) -> Result<(), RubyError> {
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
        if is_method {
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

    pub(super) fn push_block_frame_fast(
        &mut self,
        iseq: ISeqRef,
        args: &Args2,
        outer: Option<DynamicFrame>,
        use_value: bool,
    ) -> Result<(), RubyError> {
        let self_value = self.stack_pop();
        let base = self.stack_len() - args.len();
        let lvars = iseq.lvars;
        self.prepare_block_args(iseq, base);
        let args_len = self.stack_len() - base;
        let req_len = iseq.params.req;
        if req_len < args_len {
            self.set_stack_len(base + req_len);
        }

        self.exec_stack.resize(base + lvars);

        self.stack_push(self_value);
        self.prepare_block_frame(self.stack_len() - base - 1, use_value, outer, iseq);
        Ok(())
    }

    pub(crate) fn push_method_frame_fast(
        &mut self,
        iseq: ISeqRef,
        args: &Args2,
        use_value: bool,
    ) -> Result<(), RubyError> {
        let self_value = self.stack_pop();
        let min = iseq.params.req;
        let len = args.len();
        if len != min {
            return Err(RubyError::argument_wrong(len, min));
        }
        let local_len = iseq.lvars;
        self.exec_stack.grow(local_len - len);
        self.stack_push(self_value);
        self.prepare_method_frame(local_len, use_value, iseq, args.block.as_ref());
        Ok(())
    }

    /// Move outer execution contexts on the stack to the heap.
    pub(crate) fn move_frame_to_heap(&mut self, f: Frame) -> DynamicFrame {
        let dfp = self.dfp_from_frame(f);
        self.move_dfp_to_heap(dfp)
    }

    /// Move outer execution contexts on the stack to the heap.
    fn move_dfp_to_heap(&mut self, dfp: DynamicFrame) -> DynamicFrame {
        if let Some(h) = dfp.heap() {
            return h.as_dfp();
        }
        if !self.check_boundary(dfp.lfp().as_ptr()) {
            return dfp;
        }
        let outer = dfp.dfp().map(|d| self.move_dfp_to_heap(d));
        let local_len = dfp.local_len();
        let heap = HeapCtxRef::new_from_frame(dfp.frame(), outer, local_len);
        dfp.set_heap(heap);
        if self.cfp.as_ptr() == dfp.as_ptr() {
            self.lfp = dfp.lfp();
        }
        heap.as_dfp()
    }
}

impl VM {
    #[cfg(feature = "trace-func")]
    pub(crate) fn dump_frame(&self, cfp: ControlFrame) {
        if !self.globals.startup_flag {
            return;
        }
        eprintln!("STACK---------------------------------------------");
        eprintln!("{:?}", self.exec_stack);
        eprintln!("self: [{:?}]", cfp.self_value());
        eprintln!(
            "cfp:{:?} prev_cfp:{:?} lfp:{:?} prev_len:{:?}",
            self.cfp,
            self.prev_cfp(self.cfp),
            self.lfp,
            self.prev_sp(),
        );
        if cfp.is_ruby_func() {
            if let Some(offset) = self.check_within_stack(self.lfp) {
                eprintln!("LFP is on the stack: {}", offset);
            }
            let iseq = cfp.iseq();
            let lvar = iseq.lvar.table();
            let local_len = iseq.lvars;
            let lfp = cfp.lfp();
            for i in 0..local_len {
                eprint!("{:?}:[{:?}] ", lvar[i], lfp[i]);
            }
            eprintln!();
            /*if let Some(ctx) = self.frame_heap(f) {
                eprintln!("HEAP----------------------------------------------");
                eprintln!("self: [{:?}]", ctx.self_val());
                let iseq = ctx.iseq();
                let lvar = iseq.lvar.table();
                let local_len = iseq.lvars;
                let lfp = ctx.lfp();
                for i in 0..local_len {
                    eprint!("{:?}:[{:?}] ", lvar[i], lfp[i]);
                }
                eprintln!("");
            }*/
        } else {
            for v in &cfp.lfp()[0..cfp.local_len()] {
                eprint!("[{:?}] ", *v);
            }
        }
        eprintln!("--------------------------------------------------");
    }
}
