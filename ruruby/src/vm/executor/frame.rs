use super::*;
use std::ops::IndexMut;

pub const EV_PREV_CFP: usize = 0;
pub const EV_PREV_SP: usize = 1;
pub const EV_FLAG: usize = 2;

pub const EV_LFP: usize = 3;
pub const EV_MFP: usize = 4;
pub const EV_OUTER: usize = 5;
pub const EV_PC: usize = 6;
pub const EV_EP: usize = 7;
pub const EV_ISEQ: usize = 8;
pub const EV_BLK: usize = 9;

pub const NATIVE_FRAME_LEN: usize = 3;
pub const RUBY_FRAME_LEN: usize = 10;

/// Control frame on the RubyStack.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame(pub u32);

pub(crate) trait CF: Copy + Index<usize, Output = Value> + IndexMut<usize> {
    fn as_ptr(self) -> *mut Value;

    fn from_ptr(p: *mut Value) -> Self;

    fn local_len(&self) -> usize;

    #[inline(always)]
    fn as_sp(&self) -> StackPtr {
        StackPtr::from(self.as_ptr())
    }

    #[inline(always)]
    fn self_value(&self) -> Value {
        unsafe { *self.as_ptr().sub(1) }
    }

    #[inline(always)]
    fn lfp(&self) -> LocalFrame {
        LocalFrame::decode(self[EV_LFP])
    }

    #[inline(always)]
    fn prev_sp(&self) -> StackPtr {
        StackPtr::decode(self[EV_PREV_SP])
    }

    #[inline(always)]
    fn enc(self) -> Value {
        Value::from((self.as_ptr() as u64) | 0b1)
    }

    #[inline(always)]
    fn dec(v: Value) -> Option<*mut Value> {
        let p = (v.get() & (-2i64 as u64)) as *mut Value;
        if p.is_null() {
            None
        } else {
            Some(p)
        }
    }

    #[inline(always)]
    fn mfp(&self) -> EnvFrame {
        EnvFrame(EnvFrame::dec(self[EV_MFP]).unwrap())
    }

    #[inline(always)]
    fn flag(&self) -> u64 {
        self[EV_FLAG].get()
    }

    #[inline(always)]
    fn is_ruby_func(&self) -> bool {
        self.flag() & 0b1000_0000 != 0
    }

    #[inline(always)]
    fn outer(&self) -> Option<EnvFrame> {
        debug_assert!(self.is_ruby_func());
        let v = self[EV_OUTER];
        EnvFrame::decode(v)
    }

    #[inline(always)]
    fn heap(&self) -> Option<EnvFrame> {
        debug_assert!(self.is_ruby_func());
        EnvFrame::decode(self[EV_EP])
    }

    #[inline(always)]
    fn iseq(self) -> ISeqRef {
        debug_assert!(self.is_ruby_func());
        let v = self[EV_ISEQ];
        ISeqRef::decode(v.as_fnum())
    }

    /// Set the context of `frame` to `ctx`.
    fn set_heap(mut self, heap: HeapCtxRef) {
        let ep = heap.as_ep();
        self[EV_EP] = EnvFrame::encode(Some(ep));
        self[EV_MFP] = ep.mfp().enc();
        self[EV_LFP] = ep.lfp().encode();
        self[EV_OUTER] = EnvFrame::encode(ep.outer());
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

    fn locals(&self) -> &[Value] {
        if self.is_ruby_func() {
            let lfp = self.lfp();
            let len = self.iseq().lvars + 1;
            unsafe { std::slice::from_raw_parts(lfp.0, len) }
        } else {
            let prev_sp = self.prev_sp();
            let len = (self.as_sp() - prev_sp) as usize;
            unsafe { std::slice::from_raw_parts(prev_sp.as_ptr(), len) }
        }
    }
}

///
/// Control frame
///
/// Wrapped raw pointer which points to a control frame.
///
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
            let offset = self.0.offset_from(other.as_ptr());
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
        (self.flag() as usize) >> 32
    }
}

impl Index<usize> for ControlFrame {
    type Output = Value;
    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        unsafe { &*self.0.add(index) }
    }
}

impl IndexMut<usize> for ControlFrame {
    #[inline(always)]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe { &mut *self.0.add(index) }
    }
}

impl ControlFrame {
    #[inline(always)]
    pub(crate) fn as_ep(self) -> EnvFrame {
        EnvFrame(self.0)
    }

    #[inline(always)]
    pub(super) fn decode(v: Value) -> Option<Self> {
        Self::dec(v).map(|p| Self(p))
    }

    #[inline(always)]
    pub(super) fn pc(&self) -> ISeqPos {
        ISeqPos::from(self[EV_PC].as_fnum() as usize)
    }
}

///
/// Environment frame
///
/// Wrapped raw pointer which points to an environment frame.
/// You can obtain or alter various information like outer frame, mfp, lfp, and the number of local variables
/// in the frame through `EnvFrame`.
///
/// `EnvFrame` may points to either the execution stack or a heap.
///
#[derive(Clone, Copy, PartialEq)]
pub struct EnvFrame(*mut Value);

impl std::fmt::Debug for EnvFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_ruby_func() {
            let iseq = self.iseq();
            write!(f, "Ruby {:?}  ", *iseq)?;
            let lvar = iseq.lvar.table();
            let local_len = iseq.lvars;
            let lfp = self.lfp();
            for i in 0..local_len {
                write!(f, "{:?}:[{:?}] ", lvar[i], lfp[i])?;
            }
            writeln!(f)?;
        } else {
            write!(f, "Native ")?;
            let local_len = (self.as_sp() - self.prev_sp() - 1) as usize;
            let lfp = self.prev_sp().as_lfp();
            for i in 0..local_len {
                write!(f, "[{:?}] ", lfp[i])?;
            }
        }
        Ok(())
    }
}

impl ruruby_parse::parser::LocalsContext for EnvFrame {
    fn outer(&self) -> Option<Self> {
        self.outer()
    }

    fn get_lvarid(&self, id: IdentId) -> Option<LvarId> {
        self.iseq().lvar.table.get_lvarid(id)
    }

    fn lvar_collector(&self) -> LvarCollector {
        self.iseq().lvar.clone()
    }
}

impl CF for EnvFrame {
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

impl std::default::Default for EnvFrame {
    #[inline(always)]
    fn default() -> Self {
        Self(std::ptr::null_mut())
    }
}

impl Index<usize> for EnvFrame {
    type Output = Value;
    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        unsafe { &*self.0.add(index) }
    }
}

impl IndexMut<usize> for EnvFrame {
    #[inline(always)]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        unsafe { &mut *self.0.add(index) }
    }
}

impl GC<RValue> for EnvFrame {
    fn mark(&self, alloc: &mut Allocator<RValue>) {
        self.locals().iter().for_each(|v| v.mark(alloc));
        if let Some(d) = self.outer() {
            d.mark(alloc)
        }
    }
}

impl EnvFrame {
    #[inline(always)]
    pub(super) fn from_ref(r: &Value) -> Self {
        Self(r as *const _ as *mut _)
    }

    #[inline(always)]
    pub(super) fn decode(v: Value) -> Option<Self> {
        Self::dec(v).map(|p| Self(p))
    }

    #[inline(always)]
    pub(super) fn encode(opt: Option<Self>) -> Value {
        match opt {
            Some(d) => d.enc(),
            None => Self::default().enc(),
        }
    }

    #[inline(always)]
    pub(crate) fn outer(&self) -> Option<EnvFrame> {
        let v = self[EV_OUTER];
        EnvFrame::decode(v)
    }

    #[inline(always)]
    pub(super) fn block(self) -> Option<Block> {
        let v = self[EV_BLK];
        Block::decode(v)
    }

    #[inline(always)]
    fn is_module_function(self) -> bool {
        self.flag() & 0b1000 != 0
    }

    #[inline(always)]
    fn set_module_function(mut self) {
        self[EV_FLAG] = Value::from(self.flag() | 0b1000);
    }
}

///
/// Local frame
///
/// Wrapped raw pointer which points to a local variables area on the stack or heap.
/// You can handle local variables of the frame.
///
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalFrame(*mut Value);

impl std::default::Default for LocalFrame {
    #[inline(always)]
    fn default() -> Self {
        LocalFrame(std::ptr::null_mut())
    }
}

impl LocalFrame {
    #[inline(always)]
    pub(super) fn from_ref(r: &Value) -> Self {
        Self(r as *const _ as *mut _)
    }

    #[inline(always)]
    pub(super) fn from_ptr(r: *const Value) -> Self {
        Self(r as *mut _)
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

impl VM {
    /// Get the index of `cfp`.
    pub(super) fn cfp_index(&self, cfp: ControlFrame) -> u32 {
        unsafe {
            let ptr = self.stack.as_mut_ptr();
            let offset = cfp.0.offset_from(ptr);
            assert!(offset >= 0);
            offset as usize as u32
        }
    }

    #[inline(always)]
    pub(crate) fn cfp_from_frame(&self, f: Frame) -> ControlFrame {
        let p = unsafe { self.stack.as_mut_ptr().add(f.0 as usize) };
        ControlFrame(p)
    }

    #[inline(always)]
    pub(crate) fn ep_from_frame(&self, f: Frame) -> EnvFrame {
        let p = unsafe { self.stack.as_mut_ptr().add(f.0 as usize) };
        EnvFrame(p)
    }

    pub(super) fn cfp_is_zero(&self, f: ControlFrame) -> bool {
        let ptr = self.stack.as_mut_ptr();
        f.0 == ptr
    }

    /// Get the previous frame of `cfp`.
    #[inline(always)]
    pub(super) fn prev_cfp(&self, cfp: ControlFrame) -> Option<ControlFrame> {
        let v = cfp[EV_PREV_CFP];
        ControlFrame::decode(v)
    }

    #[inline(always)]
    pub fn prev_sp(&self) -> StackPtr {
        self.cfp.prev_sp()
    }
}

impl VM {
    /// Get current frame.
    #[inline(always)]
    pub(crate) fn cur_frame(&self) -> Frame {
        Frame(self.cfp_index(self.cfp))
    }

    /// Get current method frame.
    fn cur_mfp(&self) -> EnvFrame {
        self.cfp.mfp()
    }

    pub(crate) fn caller_cfp(&self) -> ControlFrame {
        let mut cfp = self.prev_cfp(self.cfp);
        while let Some(f) = cfp {
            if f.is_ruby_func() {
                return f;
            }
            cfp = self.prev_cfp(f);
        }
        unreachable!("no caller frame");
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
        self.caller_cfp().mfp().block()
    }

    pub(crate) fn caller_method_iseq(&self) -> ISeqRef {
        self.caller_cfp().mfp().iseq()
    }

    pub(super) fn get_method_block(&self) -> Option<Block> {
        self.cur_mfp().block()
    }

    pub(super) fn get_method_iseq(&self) -> ISeqRef {
        self.cur_mfp().iseq()
    }

    pub(crate) fn caller_iseq(&self) -> ISeqRef {
        self.caller_cfp().iseq()
    }

    pub(super) fn cur_source_info(&self) -> SourceInfoRef {
        self.iseq.source_info.clone()
    }

    pub(super) fn get_loc(&self) -> Loc {
        let cur_pc = self.pc_offset();
        match self
            .iseq
            .iseq_sourcemap
            .iter()
            .find(|x| x.0.into_usize() == cur_pc)
        {
            Some((_, loc)) => *loc,
            None => {
                panic!(
                    "Bad sourcemap. pc={:?} {:?}",
                    self.pc, self.iseq.iseq_sourcemap
                );
            }
        }
    }
}

impl VM {
    fn prepare_block_args(&mut self, base: StackPtr, iseq: ISeqRef) {
        // if a single Array argument is given for the block requiring multiple formal parameters,
        // the arguments must be expanded.
        if self.sp() - base == 1 && iseq.mularg_flag {
            if let Some(ary) = base[0].as_array() {
                self.stack.pop();
                self.stack.extend_from_slice(&**ary);
            }
        }
    }

    // Handling call frame

    pub(crate) fn init_frame(&mut self) {
        self.stack_push(Value::nil());
        self.cfp = self.cfp_from_frame(Frame(1));
        self.push_native_control_frame(ControlFrame::default(), self.sp(), 0);
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
    pub(crate) fn push_block_frame_from_heap(&mut self, ctx: HeapCtxRef) {
        let ep = ctx.as_ep();
        self.stack_push(ctx.self_val());
        self.prepare_block_frame(
            self.sp() - 1,
            true,
            Some(ctx),
            ep.outer(),
            ep.iseq(),
            0,
            ep.lfp(),
        );
    }

    fn prepare_method_frame(
        &mut self,
        use_value: bool,
        iseq: ISeqRef,
        local_len: usize,
        block: &Option<Block>,
    ) {
        let prev_sp = self.sp() - local_len - 1;
        let receiver = prev_sp[0];
        let lfp = (prev_sp + 1).as_lfp();
        self.save_next_pc();
        let prev_cfp = self.cfp;
        self.stack_push(receiver);
        self.cfp = self.sp().as_cfp();
        self.iseq = iseq;
        debug_assert!(!self.cfp_is_zero(prev_cfp));
        let mfp = self.cfp;
        let flag = VM::ruby_flag(use_value, local_len);
        self.extend_method_frame(flag, prev_cfp, prev_sp, mfp, iseq, block);

        self.pc = iseq.iseq.as_ptr();
        self.lfp = lfp;
        #[cfg(feature = "perf-method")]
        self.globals.methods.inc_counter(iseq.method);
        #[cfg(feature = "trace")]
        if self.globals.startup_flag {
            let ch = /*if self.is_called() {*/ "+++" /* } else { "---" }*/;
            eprintln!(
                "{}> {:?} {:?} {:?}",
                ch, iseq.method, iseq.kind, iseq.source_info.path
            );
        }
        #[cfg(feature = "trace-func")]
        if self.globals.startup_flag {
            eprintln!(">>> new frame");
            self.dump_frame(self.cfp);
        }
    }

    #[inline(always)]
    fn prepare_block_frame(
        &mut self,
        prev_sp: StackPtr,
        use_value: bool,
        ctx: Option<HeapCtxRef>,
        outer: Option<EnvFrame>,
        iseq: ISeqRef,
        local_len: usize,
        lfp: LocalFrame,
    ) {
        let receiver = prev_sp[0];
        self.save_next_pc();
        let prev_cfp = self.cfp;
        self.stack_push(receiver);
        self.cfp = self.sp().as_cfp();
        self.iseq = iseq;
        debug_assert!(!self.cfp_is_zero(prev_cfp));
        let mfp = match &outer {
            // In the case of Ruby method.
            None => self.cfp.as_ep(),
            // In the case of Ruby block.
            Some(outer) => outer.mfp(),
        };
        let flag = VM::ruby_flag(use_value, local_len);

        self.extend_block_frame(flag, prev_cfp, prev_sp, mfp, ctx, outer, iseq, lfp);

        self.pc = iseq.iseq.as_ptr();
        self.lfp = lfp;
        #[cfg(feature = "perf-method")]
        self.globals.methods.inc_counter(iseq.method);
        #[cfg(feature = "trace")]
        if self.globals.startup_flag {
            let ch = /*if self.is_called() {*/ "+++" /* } else { "---" }*/;
            eprintln!(
                "{}> {:?} {:?} {:?}",
                ch, iseq.method, iseq.kind, iseq.source_info.path
            );
        }
        #[cfg(feature = "trace-func")]
        if self.globals.startup_flag {
            eprintln!(">>> new frame");
            self.dump_frame(self.cfp);
        }
    }

    fn extend_method_frame(
        &mut self,
        flag: u64,
        prev_cfp: ControlFrame,
        prev_sp: StackPtr,
        mfp: ControlFrame,
        iseq: ISeqRef,
        block: &Option<Block>,
    ) {
        self.stack.push(prev_cfp.enc());
        self.stack.push(prev_sp.enc());
        self.stack.push(Value::from(flag));
        self.stack.push((prev_sp + 1).as_lfp().encode());
        self.stack.push(mfp.enc());
        self.stack.push(EnvFrame::encode(None));
        self.stack.push(Value::fixnum(0));
        self.stack.push(EnvFrame::encode(None));
        self.stack.push(Value::fixnum(iseq.encode()));
        self.stack.push(match block {
            None => Value::fixnum(0),
            Some(block) => block.encode(),
        });
    }

    fn extend_block_frame(
        &mut self,
        flag: u64,
        prev_cfp: ControlFrame,
        prev_sp: StackPtr,
        mfp: EnvFrame,
        ctx: Option<HeapCtxRef>,
        outer: Option<EnvFrame>,
        iseq: ISeqRef,
        lfp: LocalFrame,
    ) {
        self.stack.push(prev_cfp.enc());
        self.stack.push(prev_sp.enc());
        self.stack.push(Value::from(flag));
        self.stack.push(lfp.encode());
        self.stack.push(mfp.enc());
        self.stack.push(EnvFrame::encode(outer));
        self.stack.push(Value::fixnum(0));
        self.stack.push(EnvFrame::encode(ctx.map(|c| c.as_ep())));
        self.stack.push(Value::fixnum(iseq.encode()));
        self.stack.push(Value::fixnum(0));
    }

    pub(super) fn heap_control_frame(
        outer: Option<EnvFrame>,
        iseq: ISeqRef,
    ) -> [Value; RUBY_FRAME_LEN] {
        [
            ControlFrame::default().enc(),
            Value::fixnum(0),
            Value::from(VM::ruby_flag(true, 0)),
            LocalFrame::default().encode(),
            ControlFrame::default().enc(),
            EnvFrame::encode(outer),
            Value::fixnum(0),
            EnvFrame::encode(None),
            Value::fixnum(iseq.encode()),
            Value::fixnum(0),
        ]
    }

    /// Prepare native control frame on the top of stack.
    ///
    ///  ### Before
    ///~~~~text
    ///                                  sp
    ///                                   v
    /// +------+------+------+:-+------+------+------+------+------+--------
    /// | self |  a0  |  a1  |..|  an  |
    /// +------+------+------+--+------+------+------+------+------+--------
    ///         <----- args_len ------>
    ///~~~~
    ///
    ///  ### After
    ///~~~~text
    ///          lfp                            cfp                 sp
    ///           v                              v                   v
    /// +------+------+------+--+------+------+------+------+-----+---
    /// | self |  a0  |  a1  |..|  an  | self | cfp* | lfp  | flg |
    /// +------+------+------+--+------+------+------+------+-----+---
    ///  <-------- local frame -------->      <------------------->
    ///                                       native control frame
    ///~~~~
    ///
    /// - cfp*: prev cfp
    /// - lfp: lfp
    /// - flg: flags
    ///
    pub(crate) fn prepare_native_frame(&mut self, args_len: usize) {
        let prev_cfp = self.cfp;
        let prev_sp = self.sp() - args_len - 1;
        let receiver = prev_sp[0];
        self.stack_push(receiver);
        self.cfp = self.sp().as_cfp();
        self.lfp = (prev_sp + 1).as_lfp();
        self.push_native_control_frame(prev_cfp, prev_sp, args_len)
    }

    fn save_next_pc(&mut self) {
        if self.is_ruby_func() {
            let pc = self.pc_offset();
            self.cfp[EV_PC] = Value::fixnum(pc as i64);
        }
    }

    pub(super) fn unwind_frame(&mut self) {
        let cfp = self.prev_cfp(self.cfp).unwrap();
        self.stack.sp = self.prev_sp();
        self.cfp = cfp;
        if self.is_ruby_func() {
            self.lfp = cfp.lfp();
            self.iseq = self.cfp.iseq();
            self.set_pc(self.cfp.pc());
        } else {
            self.lfp = (cfp.prev_sp() + 1).as_lfp();
        }
        #[cfg(feature = "trace-func")]
        if self.globals.startup_flag {
            eprintln!("<<< unwind frame");
            self.dump_frame(self.cfp);
        }
    }

    pub(super) fn clear_stack(&mut self) {
        self.stack.sp = self.cfp.as_sp()
            + if self.is_ruby_func() {
                RUBY_FRAME_LEN
            } else {
                NATIVE_FRAME_LEN
            };
    }

    fn push_native_control_frame(
        &mut self,
        prev_cfp: ControlFrame,
        prev_sp: StackPtr,
        args_len: usize,
    ) {
        self.stack_push(prev_cfp.enc());
        self.stack_push(prev_sp.enc());
        self.stack_push(Value::from(((args_len as u64) << 32) | 1u64));
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
    pub(crate) fn discard_val(&self) -> bool {
        self.cfp[EV_FLAG].get() & 0b0100 != 0
    }

    #[inline(always)]
    pub(crate) fn is_ruby_func(&self) -> bool {
        self.cfp[EV_FLAG].get() & 0b1000_0000 != 0
    }

    #[inline(always)]
    pub(crate) fn ruby_flag(use_value: bool, local_len: usize) -> u64 {
        (if use_value { 0b1000_0001 } else { 0b1000_0101 }) | ((local_len as u64) << 32)
    }

    /// Check module_function flag of the current frame.
    pub(crate) fn is_module_function(&self) -> bool {
        self.cur_mfp().is_module_function()
    }

    /// Set module_function flag of the caller frame to true.
    pub(crate) fn set_module_function(&mut self) {
        self.caller_cfp().mfp().set_module_function();
    }
}

impl VM {
    fn fill_positional_arguments(&mut self, mut base: StackPtr, iseq: ISeqRef) {
        //let mut base = self.exec_stack.bottom() + base;
        let params = &iseq.params;
        let lvars = iseq.lvars;
        let args_len = (self.sp() - base) as usize;
        let req_len = params.req;
        let rest_len = if params.rest == Some(true) { 1 } else { 0 };
        let post_len = params.post;
        let no_post_len = args_len - post_len;
        let optreq_len = req_len + params.opt;

        if optreq_len < no_post_len {
            if let Some(delegate) = params.delegate {
                let v = base[optreq_len..no_post_len].to_vec();
                base[delegate.as_usize() as isize] = Value::array_from(v);
            }
            if rest_len == 1 {
                let ary = base[optreq_len..no_post_len].to_vec();
                base[optreq_len as isize] = Value::array_from(ary);
            }
            // fill post_req params.
            RubyStack::stack_copy_within(base, no_post_len..args_len, optreq_len + rest_len);
            self.stack.sp = base
                + optreq_len
                + rest_len
                + post_len
                + if params.delegate.is_some() { 1 } else { 0 };
            self.stack.resize_to(base + lvars);
        } else {
            self.stack.resize_to(base + lvars);
            // fill post_req params.
            RubyStack::stack_copy_within(base, no_post_len..args_len, optreq_len + rest_len);
            if no_post_len < req_len {
                // fill rest req params with nil.
                base[no_post_len..req_len].fill(Value::nil());
                // fill rest opt params with uninitialized.
                base[req_len..optreq_len].fill(Value::uninitialized());
            } else {
                // fill rest opt params with uninitialized.
                base[no_post_len..optreq_len].fill(Value::uninitialized());
            }
            if rest_len == 1 {
                base[(optreq_len) as isize] = Value::array_from(vec![]);
            }
        }

        iseq.lvar
            .kw
            .iter()
            .for_each(|id| base[(id.as_usize()) as isize] = Value::uninitialized());
    }

    fn fill_keyword_arguments(
        &mut self,
        mut base: StackPtr,
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
                    Some(lvar) => base[lvar.as_usize() as isize] = v,
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
            base[id.as_usize() as isize] = Value::hash_from_map(kwrest);
        }
        Ok(())
    }

    fn fill_block_argument(&mut self, mut base: StackPtr, id: LvarId, block: &Option<Block>) {
        base[id.as_usize() as isize] = block
            .as_ref()
            .map_or(Value::nil(), |block| self.create_proc(block));
    }
}

impl VM {
    pub(crate) fn push_block_frame_slow(
        &mut self,
        iseq: ISeqRef,
        args: &Args2,
        outer: Option<EnvFrame>,
        use_value: bool,
    ) -> Result<(), RubyError> {
        let base = self.sp() - args.len();
        let params = &iseq.params;
        let kw_flag = !args.kw_arg.is_nil();
        let (_positional_kwarg, ordinary_kwarg) = if params.keyword.is_empty() && !params.kwrest {
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

        self.prepare_block_args(base, iseq);
        self.fill_positional_arguments(base, iseq);
        // Handling keyword arguments and a keyword rest paramter.
        if params.kwrest || ordinary_kwarg {
            self.fill_keyword_arguments(base, iseq, args.kw_arg, ordinary_kwarg)?;
        };
        let local_len = (self.sp() - base) as usize;
        self.prepare_block_frame(
            base - 1,
            use_value,
            None,
            outer,
            iseq,
            local_len,
            base.as_lfp(),
        );

        // Handling block paramter.
        if let Some(id) = iseq.lvar.block_param() {
            self.fill_block_argument(base, id, &args.block);
        }
        Ok(())
    }

    pub(crate) fn push_method_frame_slow(
        &mut self,
        iseq: ISeqRef,
        args: &Args2,
        use_value: bool,
    ) -> InvokeResult {
        let base_ptr = self.sp() - args.len();
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
        params.check_arity(positional_kwarg, args)?;
        self.fill_positional_arguments(base_ptr, iseq);
        // Handling keyword arguments and a keyword rest paramter.
        if params.kwrest || ordinary_kwarg {
            self.fill_keyword_arguments(base_ptr, iseq, args.kw_arg, ordinary_kwarg)?;
        };
        let local_len = (self.sp() - base_ptr) as usize;
        self.prepare_method_frame(use_value, iseq, local_len, &args.block);

        // Handling block paramter.
        if let Some(id) = iseq.lvar.block_param() {
            self.fill_block_argument(base_ptr, id, &args.block);
        }
        Ok(VMResKind::Invoke)
    }

    pub(super) fn push_block_frame_fast(
        &mut self,
        iseq: ISeqRef,
        args: &Args2,
        outer: Option<EnvFrame>,
        use_value: bool,
    ) {
        let base = self.sp() - args.len();
        let lvars = iseq.lvars;
        self.prepare_block_args(base, iseq);
        let args_len = (self.sp() - base) as usize;
        let req_len = iseq.params.req;
        if req_len < args_len {
            self.stack.sp = base + req_len;
        }

        self.stack.resize_to(base + lvars);

        let local_len = (self.sp() - base) as usize;
        self.prepare_block_frame(
            base - 1,
            use_value,
            None,
            outer,
            iseq,
            local_len,
            base.as_lfp(),
        );
    }

    pub(crate) fn push_method_frame_fast(
        &mut self,
        iseq: ISeqRef,
        args: &Args2,
        use_value: bool,
    ) -> InvokeResult {
        let min = iseq.params.req;
        let len = args.len();
        if len != min {
            return Err(RubyError::argument_wrong(len, min));
        }
        let local_len = iseq.lvars;
        self.stack.grow(local_len - len);
        self.prepare_method_frame(use_value, iseq, local_len, &args.block);
        Ok(VMResKind::Invoke)
    }

    /// Move outer execution contexts on the stack to the heap.
    pub(crate) fn move_ep_to_heap(&mut self, ep: EnvFrame) -> EnvFrame {
        if let Some(e) = ep.heap() {
            return e;
        }
        if !self.check_boundary(ep.lfp()) {
            return ep;
        }
        let outer = ep.outer().map(|d| self.move_ep_to_heap(d));
        let local_len = ep.local_len();
        let heap = HeapCtxRef::new_from_frame(ep.self_value(), ep.frame(), outer, local_len);
        ep.set_heap(heap);
        if self.cfp.as_ptr() == ep.as_ptr() {
            self.lfp = ep.lfp();
        }
        heap.as_ep()
    }
}

impl VM {
    #[cfg(feature = "trace-func")]
    pub(crate) fn dump_frame(&self, cfp: ControlFrame) {
        if !self.globals.startup_flag {
            return;
        }
        eprintln!("STACK---------------------------------------------------------------");
        eprintln!("  VM:{:?}", VMRef::from_ref(&self));
        eprintln!("  {:?}", self.stack);
        eprintln!("FRAME---------------------------------------------------------------");
        eprintln!("  self: [{:?}]", cfp.self_value());
        eprintln!(
            "  cfp:{:?} prev_cfp:{:?} lfp:{} prev_len:{}",
            self.cfp_index(self.cfp),
            match self.prev_cfp(self.cfp) {
                Some(cfp) => self.cfp_index(cfp),
                None => 0,
            },
            {
                if let Some(offset) = self.check_within_stack(self.lfp) {
                    format!("stack({})", offset)
                } else {
                    format!("heap({:?})", self.lfp)
                }
            },
            {
                let prev_sp = self.prev_sp().as_cfp();
                self.cfp_index(prev_sp)
            },
        );
        if cfp.is_ruby_func() {
            let iseq = cfp.iseq();
            eprint!("  Ruby {:?}  ", *iseq);
            let lvar = iseq.lvar.table();
            let local_len = iseq.lvars;
            let lfp = cfp.lfp();
            eprint!("  ");
            for i in 0..local_len {
                eprint!("{:?}:[{:?}] ", lvar[i], lfp[i]);
            }
            eprintln!();
            let mut dfp = cfp.as_ep();
            while let Some(d) = dfp.outer() {
                eprintln!("  {:?}", d);
                dfp = d;
            }
        } else {
            eprint!("  Native ");
            for v in &cfp.prev_sp().as_lfp()[0..cfp.local_len()] {
                eprint!("[{:?}] ", *v);
            }
            eprintln!();
        }
        eprintln!("--------------------------------------------------------------------");
    }
}
