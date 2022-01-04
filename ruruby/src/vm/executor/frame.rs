use super::*;
pub use heap::HeapCtxRef;
pub use ruby_stack::*;
use std::ops::IndexMut;
pub mod arg_handler;
mod heap;
pub mod ruby_stack;

const EV_PREV_CFP: isize = 0;
const EV_EP: isize = 1;
const EV_FLAG: isize = 2;

const EV_MFP: isize = 3;
const EV_OUTER: isize = 4;
const EV_PC: isize = 5;
const EV_ISEQ: isize = 6;
const EV_BLK: isize = 7;

pub(super) const CONT_FRAME_LEN: usize = 3;
pub(super) const RUBY_FRAME_LEN: usize = 5;

const FLG_NONE: u64 = 0b0000_0000;
const FLG_DISCARD: u64 = 0b0000_0100;
const FLG_MOD_FUNC: u64 = 0b0000_1000;
const FLG_IS_RUBY: u64 = 0b1000_0000;

/// Control frame on the RubyStack.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame(pub u32);

pub(crate) trait CF: Copy + Index<isize, Output = Value> + IndexMut<isize> {
    fn as_ptr(self) -> *mut Value;

    fn from_ptr(p: *mut Value) -> Self;

    #[inline(always)]
    fn as_sp(&self) -> StackPtr {
        StackPtr::from(self.as_ptr())
    }

    #[inline(always)]
    fn self_value(&self) -> Value {
        self[-1]
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
    fn flag(&self) -> u64 {
        self[EV_FLAG].get()
    }

    #[inline(always)]
    fn flag_len(&self) -> usize {
        (self.flag() as usize) >> 32
    }

    #[inline(always)]
    fn is_ruby_func(&self) -> bool {
        self.flag() & 0b1000_0000 != 0
    }

    #[inline(always)]
    fn ep(&self) -> EnvFrame {
        EnvFrame::decode(self[EV_EP]).unwrap()
    }
}

#[macro_export]
macro_rules! impl_ptr_ops {
    ($ty:ident) => {
        impl std::default::Default for $ty {
            #[inline(always)]
            fn default() -> Self {
                Self(std::ptr::null_mut())
            }
        }

        impl Index<isize> for $ty {
            type Output = Value;
            #[inline(always)]
            fn index(&self, index: isize) -> &Self::Output {
                unsafe { &*self.0.offset(index) }
            }
        }

        impl IndexMut<isize> for $ty {
            #[inline(always)]
            fn index_mut(&mut self, index: isize) -> &mut Self::Output {
                unsafe { &mut *self.0.offset(index) }
            }
        }
    };
}

macro_rules! impl_cf {
    ($ty:ident) => {
        impl CF for $ty {
            #[inline(always)]
            fn as_ptr(self) -> *mut Value {
                self.0
            }

            #[inline(always)]
            fn from_ptr(p: *mut Value) -> Self {
                Self(p)
            }
        }
    };
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

impl_ptr_ops!(ControlFrame);
impl_cf!(ControlFrame);

impl ControlFrame {
    #[inline(always)]
    fn as_ep(self) -> EnvFrame {
        EnvFrame(self.0)
    }

    #[inline(always)]
    pub fn pc(&self) -> ISeqPos {
        ISeqPos::from(self[EV_PC].as_fnum() as usize)
    }

    /// Get the previous frame of `cfp`.
    #[inline(always)]
    pub(super) fn prev(&self) -> Option<ControlFrame> {
        let v = self[EV_PREV_CFP];
        Self::dec(v).map(|p| Self(p))
    }

    pub(super) fn get_prev_sp(&self) -> StackPtr {
        self.as_sp() - self.flag_len() - 2
    }
}

///
/// Environment frame
///
/// Wrapped raw pointer which points to an environment frame.
/// You can obtain or alter various information like outer frame, mfp, lfp, and the number of local variables
/// in the frame through `EnvFrame`.
///
/// - `EnvFrame` may points to either the execution stack or a heap.
/// - `EnvFrame` must be a Ruby environment frame.
///
#[derive(Clone, Copy, PartialEq)]
pub struct EnvFrame(*mut Value);

impl_ptr_ops!(EnvFrame);
impl_cf!(EnvFrame);

impl std::fmt::Debug for EnvFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_ruby_func() {
            writeln!(
                f,
                "EnvFrame({:?}) mfp({:?}) outer({:?})",
                self.as_ptr(),
                self.mfp().as_ptr(),
                self.outer().map(|e| e.as_ptr()),
            )?;
            let iseq = self.iseq();
            write!(f, "--Ruby {:?} ", *iseq)?;
            let lvar = iseq.lvar.table();
            assert_eq!(iseq.lvars, self.flag_len());
            for (i, v) in self.locals().iter().enumerate() {
                write!(f, "{:?}:[{:?}] ", lvar[i], v)?;
            }
        } else {
            write!(f, "EnvFrame({:?}) Native ", self.as_ptr())?;
            for (i, v) in self.locals().iter().enumerate() {
                write!(f, "{}:[{:?}] ", i, *v)?;
            }
        }
        writeln!(f)
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

impl GC<RValue> for EnvFrame {
    fn mark(&self, alloc: &mut Allocator<RValue>) {
        self.locals().iter().for_each(|v| v.mark(alloc));
        if self.is_ruby_func() {
            if let Some(d) = self.outer() {
                d.mark(alloc)
            }
        }
    }
}

impl EnvFrame {
    fn from_ref(r: &Value) -> Self {
        Self(r as *const _ as *mut _)
    }

    fn decode(v: Value) -> Option<Self> {
        Self::dec(v).map(|p| Self(p))
    }

    fn encode(opt: Option<Self>) -> Value {
        opt.unwrap_or_default().enc()
    }

    fn mfp(&self) -> EnvFrame {
        debug_assert!(self.is_ruby_func());
        EnvFrame(EnvFrame::dec(self[EV_MFP]).unwrap())
    }

    #[inline(always)]
    pub(crate) fn outer(&self) -> Option<EnvFrame> {
        debug_assert!(self.is_ruby_func());
        EnvFrame::decode(self[EV_OUTER])
    }

    #[inline(always)]
    pub fn iseq(self) -> ISeqRef {
        debug_assert!(self.is_ruby_func());
        let i = self[EV_ISEQ].as_fnum();
        assert!(i != 0);
        ISeqRef::decode(i)
    }

    fn block(&self) -> Option<Block> {
        debug_assert!(self.is_ruby_func());
        Block::decode(self[EV_BLK])
    }

    fn is_module_function(self) -> bool {
        debug_assert!(self.is_ruby_func());
        self.flag() & FLG_MOD_FUNC != 0
    }

    fn set_module_function(mut self) {
        debug_assert!(self.is_ruby_func());
        self[EV_FLAG] = Value::from(self.flag() | FLG_MOD_FUNC);
    }

    pub(super) fn get_lfp(&self) -> LocalFrame {
        (self.as_sp() - self.flag_len() - 1).as_lfp()
    }

    pub(super) fn locals(&self) -> &[Value] {
        let len = self.flag_len();
        let lfp = (self.as_sp() - len - 1).as_lfp();
        unsafe { std::slice::from_raw_parts(lfp.0, len) }
    }

    fn frame(&self) -> &[Value] {
        debug_assert!(self.is_ruby_func());
        let len = self.flag_len();
        let top = self.as_sp() - len - 2;
        unsafe {
            std::slice::from_raw_parts(top.as_ptr(), len + 2 + CONT_FRAME_LEN + RUBY_FRAME_LEN)
        }
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

impl_ptr_ops!(LocalFrame);

impl LocalFrame {
    #[inline(always)]
    pub(super) fn from_ptr(r: *const Value) -> Self {
        Self(r as *mut _)
    }

    #[inline(always)]
    pub(super) fn as_sp(&self) -> StackPtr {
        StackPtr::from(self.0)
    }
}

impl Index<LvarId> for LocalFrame {
    type Output = Value;
    #[inline(always)]
    fn index(&self, index: LvarId) -> &Self::Output {
        &self[index.as_usize() as isize]
    }
}

impl IndexMut<LvarId> for LocalFrame {
    #[inline(always)]
    fn index_mut(&mut self, index: LvarId) -> &mut Self::Output {
        &mut self[index.as_usize() as isize]
    }
}

impl Index<std::ops::Range<usize>> for LocalFrame {
    type Output = [Value];
    #[inline(always)]
    fn index(&self, range: std::ops::Range<usize>) -> &Self::Output {
        unsafe { std::slice::from_raw_parts(self.0.add(range.start), range.len()) }
    }
}

impl VM {
    /// Get the index of `cfp`.
    fn cfp_index(&self, cfp: ControlFrame) -> u32 {
        unsafe {
            let ptr = self.stack.as_mut_ptr();
            let offset = cfp.0.offset_from(ptr);
            assert!(offset >= 0);
            offset as usize as u32
        }
    }

    #[cfg(feature = "trace-func")]
    /// Get the index of `sp`.
    fn sp_index(&self, sp: StackPtr) -> isize {
        unsafe {
            let ptr = self.stack.as_mut_ptr();
            sp.as_ptr().offset_from(ptr)
        }
    }

    #[inline(always)]
    pub(super) fn cfp_from_frame(&self, f: Frame) -> ControlFrame {
        let p = unsafe { self.stack.as_mut_ptr().add(f.0 as usize) };
        ControlFrame(p)
    }

    fn cfp_is_zero(&self, f: ControlFrame) -> bool {
        let ptr = self.stack.as_mut_ptr();
        f.0 == ptr
    }
}

impl VM {
    /// Get current frame.
    #[inline(always)]
    pub(super) fn cur_frame(&self) -> Frame {
        Frame(self.cfp_index(self.cfp))
    }

    /// Get current method frame.
    fn cur_mfp(&self) -> EnvFrame {
        self.cfp.ep().mfp()
    }

    pub(super) fn cur_delegate(&self) -> Option<Value> {
        let lvar_id = self.cur_mfp().iseq().params.delegate?;
        let delegate = self.lfp[lvar_id];
        if delegate.is_nil() {
            None
        } else {
            Some(delegate)
        }
    }

    pub(crate) fn caller_cfp(&self) -> ControlFrame {
        let mut cfp = self.cfp.prev();
        while let Some(f) = cfp {
            if f.is_ruby_func() {
                return f;
            }
            cfp = f.prev();
        }
        unreachable!("no caller frame");
    }

    #[inline(always)]
    pub(crate) fn caller_frame(&self) -> Frame {
        let cfp = self.caller_cfp();
        let i = self.cfp_index(cfp);
        Frame(i)
    }

    pub(crate) fn caller_method_block(&self) -> Option<Block> {
        self.caller_cfp().ep().mfp().block()
    }

    pub(crate) fn caller_method_iseq(&self) -> ISeqRef {
        self.caller_cfp().ep().mfp().iseq()
    }

    pub(super) fn get_method_block(&self) -> Option<Block> {
        self.cur_mfp().block()
    }

    pub(super) fn get_method_iseq(&self) -> ISeqRef {
        self.cur_mfp().iseq()
    }

    pub(crate) fn caller_iseq(&self) -> ISeqRef {
        self.caller_cfp().ep().iseq()
    }
}

impl VM {
    pub(crate) fn init_frame(&mut self) {
        self.stack_push(Value::nil());
        self.cfp = self.sp().as_cfp();
        self.push_control_frame(
            ControlFrame::default(),
            self.cfp.as_ep(),
            VM::native_flag(0),
        );
    }

    pub(crate) fn push_block_frame_from_heap(&mut self, ctx: HeapCtxRef) {
        let ep = ctx.as_ep();
        self.stack_push(ctx.self_val());
        self.push_block_frame(
            self.sp() - 1,
            true,
            Some(ep),
            ep.outer(),
            ep.iseq(),
            0,
            ep.get_lfp(),
        );
    }

    fn push_method_frame(
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
        let ep = self.cfp.as_ep();
        debug_assert!(!self.cfp_is_zero(prev_cfp));
        let flag = VM::ruby_flag(use_value, local_len);
        self.push_control_frame(prev_cfp, ep, flag);
        self.stack
            .extend_from_slice(&method_env_frame(ep, iseq, block));

        self.iseq = iseq;
        self.pc = iseq.iseq.as_ptr();
        self.lfp = lfp;
        //assert_eq!(prev_sp, self.cfp.get_prev_sp());
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
            eprintln!(">>> new method frame");
            self.dump_frame();
        }
    }

    fn push_block_frame(
        &mut self,
        prev_sp: StackPtr,
        use_value: bool,
        heap: Option<EnvFrame>,
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
        debug_assert!(!self.cfp_is_zero(prev_cfp));
        let mfp = match &outer {
            // In the case of Ruby method.
            None => self.cfp.as_ep(),
            // In the case of Ruby block.
            Some(outer) => outer.mfp(),
        };
        let flag = VM::ruby_flag(use_value, local_len);
        let ep = match heap {
            Some(heap) => heap,
            None => self.cfp.as_ep(),
        };
        self.push_control_frame(prev_cfp, ep, flag);
        self.stack
            .extend_from_slice(&block_env_frame(mfp, outer, iseq));

        self.iseq = iseq;
        self.pc = iseq.iseq.as_ptr();
        self.lfp = lfp;
        //assert_eq!(prev_sp, self.cfp.get_prev_sp());
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
            eprintln!(">>> new block frame");
            self.dump_frame();
        }
    }

    pub(super) fn push_native_frame(&mut self, args_len: usize) {
        let prev_cfp = self.cfp;
        let prev_sp = self.sp() - args_len - 1;
        self.stack_push(prev_sp[0]);
        self.cfp = self.sp().as_cfp();
        self.lfp = (prev_sp + 1).as_lfp();
        self.push_control_frame(prev_cfp, self.cfp.as_ep(), VM::native_flag(args_len));
        #[cfg(feature = "trace-func")]
        if self.globals.startup_flag {
            eprintln!(">>> new native frame");
            self.dump_frame();
        }
    }

    pub(super) fn unwind_frame(&mut self) {
        let cfp = self.cfp.prev().unwrap();
        self.stack.sp = self.cfp.get_prev_sp();
        self.cfp = cfp;
        let ep = cfp.ep();
        self.lfp = ep.get_lfp();
        if self.is_ruby_func() {
            self.iseq = ep.iseq();
            self.set_pc(cfp.pc());
        }
        #[cfg(feature = "trace-func")]
        if self.globals.startup_flag {
            eprintln!("<<< unwind frame");
            self.dump_frame();
        }
    }

    pub(super) fn unwind_native_frame(&mut self, cfp: ControlFrame) {
        self.stack.sp = self.cfp.get_prev_sp();
        self.cfp = cfp;
        self.lfp = cfp.ep().get_lfp();
        #[cfg(feature = "trace-func")]
        if self.globals.startup_flag {
            eprintln!("<<< unwind frame");
            self.dump_frame();
        }
    }

    pub fn save_next_pc(&mut self) {
        if self.is_ruby_func() {
            let pc = self.pc_offset();
            self.cfp[EV_PC] = Value::fixnum(pc.into_usize() as i64);
        }
    }

    pub(super) fn clear_stack(&mut self) {
        self.stack.sp = self.cfp.as_sp()
            + CONT_FRAME_LEN
            + if self.is_ruby_func() {
                RUBY_FRAME_LEN
            } else {
                0
            };
    }

    fn push_control_frame(&mut self, prev_cfp: ControlFrame, ep: EnvFrame, flag: u64) {
        let f = control_frame(prev_cfp, ep, flag);
        self.stack.extend_from_slice(&f);
    }
}

fn control_frame(prev_cfp: ControlFrame, ep: EnvFrame, flag: u64) -> [Value; CONT_FRAME_LEN] {
    [prev_cfp.enc(), ep.enc(), Value::from(flag)]
}

fn method_env_frame(ep: EnvFrame, iseq: ISeqRef, block: &Option<Block>) -> [Value; RUBY_FRAME_LEN] {
    [
        ep.enc(),
        EnvFrame::encode(None),
        Value::fixnum(0),
        Value::fixnum(iseq.encode()),
        match block {
            None => Value::fixnum(0),
            Some(block) => block.encode(),
        },
    ]
}

fn block_env_frame(
    mfp: EnvFrame,
    outer: Option<EnvFrame>,
    iseq: ISeqRef,
) -> [Value; RUBY_FRAME_LEN] {
    [
        mfp.enc(),
        EnvFrame::encode(outer),
        Value::fixnum(0),
        Value::fixnum(iseq.encode()),
        Value::fixnum(0),
    ]
}

fn heap_env_frame(outer: Option<EnvFrame>, iseq: ISeqRef) -> [Value; RUBY_FRAME_LEN] {
    [
        ControlFrame::default().enc(),
        EnvFrame::encode(outer),
        Value::fixnum(0),
        Value::fixnum(iseq.encode()),
        Value::fixnum(0),
    ]
}

impl VM {
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
    pub(super) fn discard_val(&self) -> bool {
        self.cfp[EV_FLAG].get() & FLG_DISCARD != 0
    }

    #[inline(always)]
    pub(super) fn is_ruby_func(&self) -> bool {
        self.cfp[EV_FLAG].get() & FLG_IS_RUBY != 0
    }

    fn ruby_flag(use_value: bool, local_len: usize) -> u64 {
        ((local_len as u64) << 32)
            | 1
            | FLG_IS_RUBY
            | (if use_value { FLG_NONE } else { FLG_DISCARD })
    }

    fn native_flag(args_len: usize) -> u64 {
        ((args_len as u64) << 32) | 1
    }

    /// Check module_function flag of the current frame.
    pub(crate) fn is_module_function(&self) -> bool {
        self.cur_mfp().is_module_function()
    }

    /// Set module_function flag of the caller frame to true.
    pub(crate) fn set_module_function(&mut self) {
        self.caller_cfp().ep().mfp().set_module_function();
    }
}

impl VM {
    /// Move outer execution contexts on the stack to the heap.
    pub(crate) fn move_cfp_to_heap(&mut self, cfp: ControlFrame) -> EnvFrame {
        let ep = cfp.ep();
        self.move_ep_to_heap(ep)
    }

    /// Move outer execution contexts on the stack to the heap.
    fn move_ep_to_heap(&mut self, ep: EnvFrame) -> EnvFrame {
        if self.check_boundary(ep.as_ptr()).is_none() {
            return ep;
        }
        let outer = ep.outer().map(|d| self.move_ep_to_heap(d));
        let heap_ep = HeapCtxRef::dup_frame(ep, outer).as_ep();

        if self.cfp.as_ptr() == ep.as_ptr() {
            self.lfp = heap_ep.get_lfp();
        }
        #[cfg(feature = "trace-func")]
        eprintln!("stack {:?} => heap {:?}", ep.as_ptr(), heap_ep.as_ptr());
        heap_ep
    }
}

impl VM {
    #[cfg(feature = "trace-func")]
    pub(crate) fn dump_frame(&self) {
        if !self.globals.startup_flag {
            return;
        }
        let cfp = self.cfp;
        eprintln!("STACK---------------------------------------------------------------");
        eprintln!("  VM:{:?}", VMRef::from_ref(&self));
        eprintln!("CONTROL FRAME-------------------------------------------------------");
        eprintln!("  self: [{:?}]", cfp.self_value());
        eprintln!(
            "  cfp:{:?} prev_cfp:{:?} prev_sp:{} lfp:{}",
            self.cfp_index(cfp),
            match cfp.prev() {
                Some(cfp) => self.cfp_index(cfp),
                None => 0,
            },
            {
                let prev_sp = cfp.get_prev_sp();
                self.sp_index(prev_sp)
            },
            {
                if let Some(offset) = self.check_boundary(self.lfp.0) {
                    format!("stack({})", offset)
                } else {
                    format!("heap({:?})", self.lfp)
                }
            },
        );
        self.dump_stack(cfp);
        eprintln!("--------------------------------------------------------------------");
    }

    #[cfg(feature = "trace-func")]
    fn dump_stack(&self, cfp: ControlFrame) {
        let mut cfp = Some(cfp);
        let mut i = 1;
        while let Some(f) = cfp {
            let ep = f.ep();
            eprint!(
                "{}:{}({:?})",
                i,
                if self.check_boundary(ep.as_ptr()).is_some() {
                    "STACK"
                } else {
                    "HEAP "
                },
                f.as_ptr()
            );
            i += 1;
            eprint!(" {:?}", ep);
            cfp = f.prev();
        }
    }
}
