use super::*;

//
//  Stack handling
//
//  before frame preparation
//
//   lfp                            cfp                                                                  sp
//    v                              v                           <------ new local frame ----->           v
// +------+------+--+------+------+------+------+------+------+--------+------+------+--+------+------+------------------------
// |  a0  |  a1  |..|  an  | self | lfp2 | cfp2 | mfp1 |  pc2 |  ....  |  b0  |  b1  |..|  bn  | self |
// +------+------+--+------+------+------+------+------+------+--------+------+------+--+------+------+------------------------
//  <------- local frame --------> <-- control frame ->
//
//
//  after frame preparation
//
//   lfp1                           cfp1                          lfp                            cfp                                 sp
//    v                              v                             v                              v                                   v
// +------+------+--+------+------+------+------+------+------+--------+------+------+--+------+------+------+------+------+------+---
// |  a0  |  a1  |..|  an  | self | lfp2 | cfp2 | mfp1 |  pc2 |  ....  |  b0  |  b1  |..|  bn  | self | lfp1 | cfp1 | mfp  |  pc1 |
// +------+------+--+------+------+------+------+------+------+--------+------+------+--+------+------+------+------+------+------+---
//                                                               <------- local frame --------> <-- control frame ->
//
//  after execution
//
//   lfp                            cfp                                   sp
//    v                              v                                     v
// +------+------+--+------+------+------+------+------+------+--------+-------------------------------------------------------
// |  a0  |  a1  |..|  an  | self | lfp2 | cfp2 | mfp1 |  pc2 |  ....  |
// +------+------+--+------+------+------+------+------+------+--------+-------------------------------------------------------
//

const LFP_OFFSET: usize = 0;
const CFP_OFFSET: usize = 1;
const MFP_OFFSET: usize = 2;
const PC_OFFSET: usize = 3;
const FLAG_OFFSET: usize = 4;
const CTX_OFFSET: usize = 5;
const ISEQ_OFFSET: usize = 6;
const CFP_LEN: usize = 7;

/// Control frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame(usize);

impl Frame {
    pub(super) fn is_end(&self) -> bool {
        self.0 == 0
    }
}

impl VM {
    /// Get current frame.
    pub(super) fn cur_frame(&self) -> Frame {
        Frame(self.cfp)
    }

    /// Get current method frame.
    fn method_frame(&self) -> Frame {
        Frame(self.exec_stack[self.cfp + MFP_OFFSET].as_fixnum().unwrap() as usize)
    }

    pub fn caller_method_context(&self) -> ContextRef {
        let frame = self.get_caller_frame(self.cur_frame());
        assert!(frame.0 != 0);
        let f = Frame(self.exec_stack[frame.0 + MFP_OFFSET].as_fixnum().unwrap() as usize);
        if f.is_end() {
            // In the case of the first invoked context of Fiber
            self.get_context(self.cur_frame()).unwrap().method_context()
        } else {
            self.get_context(f).unwrap()
        }
    }

    /// Get the caller frame of `frame`.
    pub(super) fn get_caller_frame(&self, frame: Frame) -> Frame {
        assert!(frame.0 != 0);
        let cfp = self.exec_stack[frame.0 + CFP_OFFSET].as_fixnum().unwrap() as usize;
        Frame(cfp)
    }

    pub(super) fn get_method_context(&self) -> ContextRef {
        let f = self.method_frame();
        if f.is_end() {
            // In the case of the first invoked context of Fiber
            self.get_context(self.cur_frame()).unwrap().method_context()
        } else {
            self.get_context(f).unwrap()
        }
    }

    pub(super) fn get_method_iseq(&self) -> ISeqRef {
        let f = self.method_frame();
        if f.is_end() {
            // In the case of the first invoked context of Fiber
            self.get_context(self.cur_frame())
                .unwrap()
                .method_context()
                .iseq_ref
        } else {
            self.get_iseq(f).unwrap()
        }
    }

    /// Get context of `frame`.
    ///
    /// If `frame` is a native (Rust) frame, return None.
    pub(super) fn get_context(&self, frame: Frame) -> Option<ContextRef> {
        assert!(frame.0 != 0);
        let ctx = self.exec_stack[frame.0 + CTX_OFFSET];
        match ctx.as_fixnum() {
            Some(i) => {
                let u = (i << 3) as u64;
                Some(ContextRef::from_ptr(u as *const Context as *mut _))
            }
            None => {
                assert!(ctx.is_nil());
                None
            }
        }
    }

    /// Set the context of `frame` to `ctx`.
    pub(super) fn set_context(&mut self, frame: Frame, ctx: ContextRef) {
        let adr = ctx.id();
        assert!(adr & 0b111 == 0);
        let i = adr as i64 >> 3;
        self.exec_stack[frame.0 + CTX_OFFSET] = Value::integer(i)
    }

    pub(super) fn get_iseq(&self, frame: Frame) -> Option<ISeqRef> {
        let iseq = self.exec_stack[frame.0 + ISEQ_OFFSET];
        match iseq.as_fixnum() {
            Some(i) => {
                let u = (i << 3) as u64;
                Some(ISeqRef::from_ptr(u as *const ISeqInfo as *mut _))
            }
            None => {
                assert!(iseq.is_nil());
                None
            }
        }
    }

    pub(super) fn get_self(&self, frame: Frame) -> Value {
        assert!(frame.0 != 0);
        self.exec_stack[frame.0 - 1]
    }

    pub(super) fn cur_iseq(&self) -> ISeqRef {
        self.get_iseq(self.cur_frame()).unwrap()
    }

    pub(crate) fn caller_iseq(&self) -> ISeqRef {
        let c = self.get_caller_frame(self.cur_frame());
        self.get_iseq(c).unwrap()
    }

    pub(super) fn cur_source_info(&self) -> SourceInfoRef {
        self.cur_iseq().source_info.clone()
    }

    pub(super) fn get_loc(&self) -> Loc {
        let iseq = self.cur_iseq();
        let pc = self.cur_context().cur_pc;
        match iseq.iseq_sourcemap.iter().find(|x| x.0 == pc) {
            Some((_, loc)) => *loc,
            None => {
                eprintln!("Bad sourcemap. pc={:?} {:?}", pc, iseq.iseq_sourcemap);
                Loc(0, 0)
            }
        }
    }
}

impl VM {
    // Handling call frame

    /// Prepare control frame on the top of stack.
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
    ///  ### After
    ///~~~~text
    ///   lfp                            cfp                                              sp
    ///    v                              v                                                v
    /// +------+------+--+------+------+------+------+------+------+------+------+------+-----
    /// |  a0  |  a1  |..|  an  | self | lfp* | cfp* | mfp  |  pc* | flag | ctx  | iseq |
    /// +------+------+--+------+------+------+------+------+------+------+------+------+-----
    ///  <-------- local frame --------> <-------------- control frame ---------------->
    ///~~~~
    /// - lfp*: prev lfp
    /// - cfp*: prev cfp
    /// - pc*:  prev pc
    /// - flag: flags
    /// - ctx: ContextRef (if native function, nil is stored.)
    /// - iseq: ISeqRef (if native function, nil is stored.)
    pub fn prepare_frame(
        &mut self,
        args_len: usize,
        use_value: bool,
        ctx: impl Into<Option<ContextRef>>,
        iseq: impl Into<Option<ISeqRef>>,
    ) {
        let ctx = ctx.into();
        let iseq: Option<ISeqRef> = iseq.into();
        let prev_lfp = self.lfp;
        let prev_cfp = self.cfp;
        self.lfp = self.stack_len() - args_len - 1;

        if let Some(iseq) = iseq {
            // if a single Array argument is given for the block requiring multiple formal parameters,
            // the arguments must be expanded.
            let req_len = iseq.params.req;
            let post_len = iseq.params.post;
            if iseq.is_block() && args_len == 1 && req_len + post_len > 1 {
                if let Some(ary) = self.exec_stack[self.lfp].as_array() {
                    let self_val = self.stack_pop();
                    self.stack_pop();
                    self.exec_stack.extend_from_slice(&ary.elements);
                    self.stack_push(self_val);
                }
            }
        }

        self.cfp = self.stack_len();
        let mfp = if iseq.is_some() {
            if ctx.unwrap().outer.is_none() {
                // In the case of Ruby method.
                self.cfp
            } else if prev_cfp == 0 {
                // This only occurs in newly invoked Fiber.
                0
            } else {
                // In the case of Ruby block.
                let c = self.get_caller_frame(Frame(prev_cfp));
                if c.is_end() {
                    0
                } else {
                    self.exec_stack[c.0 + MFP_OFFSET].as_fixnum().unwrap() as usize
                }
            }
        } else {
            // In the case of native method.
            if prev_cfp == 0 {
                // This only occurs in newly invoked Fiber.
                0
            } else {
                self.exec_stack[prev_cfp + MFP_OFFSET].as_fixnum().unwrap() as usize
            }
        };
        self.frame_push_reg(prev_lfp, prev_cfp, mfp, self.pc, use_value, ctx, iseq);
        if let Some(_iseq) = iseq {
            self.pc = ISeqPos::from(0);
            #[cfg(feature = "perf-method")]
            MethodRepo::inc_counter(_iseq.method);
            #[cfg(any(feature = "trace", feature = "trace-func"))]
            if self.globals.startup_flag {
                let ch = if self.is_called() { "+++" } else { "---" };
                eprintln!(
                    "{}> {:?} {:?} {:?}",
                    ch, _iseq.method, _iseq.kind, _iseq.source_info.path
                );
            }
        }
    }

    #[cfg(feature = "trace")]
    pub fn dump_current_frame(&self) {
        if self.globals.startup_flag {
            eprintln!("lfp:{} cfp:{}", self.lfp, self.cfp,);
            eprintln!("LOCALS---------------------------------------------");
            for i in self.lfp..self.cfp {
                eprint!("[{:?}] ", self.exec_stack[i]);
            }
            eprintln!("\nCUR CTX------------------------------------------");
            if let Some(ctx) = self.get_context(self.cur_frame()) {
                eprintln!("{:?}", *ctx);
                eprintln!("METHOD CTX---------------------------------------");
                let m = self.method_frame();
                eprintln!("mfp: {:?}", m);
                eprintln!("{:?}", *self.get_method_context());
            } else {
                eprintln!("None");
            }
        }
    }

    pub(super) fn unwind_frame(&mut self) {
        let (lfp, cfp, pc) = self.frame_fetch_reg();
        self.set_stack_len(self.lfp);
        self.lfp = lfp;
        self.cfp = cfp;
        self.pc = pc;
        #[cfg(feature = "trace")]
        if self.globals.startup_flag {
            eprintln!("unwind lfp:{} cfp:{}", self.lfp, self.cfp);
        }
    }

    pub(super) fn clear_stack(&mut self) {
        self.set_stack_len(self.cfp + CFP_LEN);
    }

    fn frame_push_reg(
        &mut self,
        lfp: usize,
        cfp: usize,
        mfp: usize,
        pc: ISeqPos,
        use_value: bool,
        ctx: Option<ContextRef>,
        iseq: Option<ISeqRef>,
    ) {
        self.stack_push(Value::integer(lfp as i64));
        self.stack_push(Value::integer(cfp as i64));
        self.stack_push(Value::integer(mfp as i64));
        self.stack_push(Value::integer(pc.into_usize() as i64));
        self.stack_push(Value::integer(if use_value { 0 } else { 2 }));
        self.stack_push(match ctx {
            Some(ctx) => {
                let adr = ctx.id();
                assert!(adr & 0b111 == 0);
                let i = adr as i64 >> 3;
                Value::integer(i)
            }
            None => Value::nil(),
        });
        self.stack_push(match iseq {
            Some(iseq) => {
                let adr = iseq.id();
                assert!(adr & 0b111 == 0);
                let i = adr as i64 >> 3;
                Value::integer(i)
            }
            None => Value::nil(),
        });
    }

    fn frame_fetch_reg(&mut self) -> (usize, usize, ISeqPos) {
        let cfp = self.cfp;
        (
            self.exec_stack[cfp + LFP_OFFSET].as_fixnum().unwrap() as usize,
            self.exec_stack[cfp + CFP_OFFSET].as_fixnum().unwrap() as usize,
            ISeqPos::from(self.exec_stack[cfp + PC_OFFSET].as_fixnum().unwrap() as usize),
        )
    }

    //
    // Frame flags.
    //
    // 0 0 0 0_0 0 0 1
    //           | | |
    //           | | +-- always 1 (represents Value::integer)
    //           | +---- is_called (0: normaly invoked  1: vm_loop was called recursively)
    //           +------ discard_value (0: use return value  1: discard return value)
    //
    fn flag(&self) -> Value {
        let cfp = self.cfp;
        self.exec_stack[cfp + FLAG_OFFSET]
    }

    fn flag_mut(&mut self) -> &mut Value {
        let cfp = self.cfp;
        &mut self.exec_stack[cfp + FLAG_OFFSET]
    }

    pub fn is_called(&self) -> bool {
        self.flag().get() & 0b010 != 0
    }

    pub fn set_called(&mut self) {
        let f = self.flag_mut();
        *f = Value::from(f.get() | 0b010);
    }

    pub fn discard_val(&self) -> bool {
        self.flag().get() & 0b100 != 0
    }

    pub fn set_discard_val(&mut self) {
        let f = self.flag_mut();
        *f = Value::from(f.get() | 0b100);
    }
}
