use super::*;

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

    /// Get caller frame.
    pub(super) fn caller_frame(&self) -> Frame {
        self.get_caller_frame(self.cur_frame())
    }

    /// Get the caller frame of `frame`.
    pub(super) fn get_caller_frame(&self, frame: Frame) -> Frame {
        assert!(frame.0 != 0);
        let cfp = self.exec_stack[frame.0 + 1].as_fixnum().unwrap() as usize;
        Frame(cfp)
    }

    /// Get context of `frame`.
    ///
    /// If `frame` is a native (Rust) frame, return None.
    pub(super) fn get_context(&self, frame: Frame) -> Option<ContextRef> {
        let ctx = self.exec_stack[frame.0 + 4];
        match ctx.as_fixnum() {
            Some(i) => {
                let u = (i << 3) as u64;
                Some(ContextRef::from_ptr(u as *const Context as *mut _).get_current())
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
        self.exec_stack[frame.0 + 4] = Value::integer(i)
    }

    pub(super) fn get_iseq(&self, frame: Frame) -> Option<ISeqRef> {
        let ctx = self.exec_stack[frame.0 + 5];
        match ctx.as_fixnum() {
            Some(i) => {
                let u = (i << 3) as u64;
                Some(ISeqRef::from_ptr(u as *const ISeqInfo as *mut _))
            }
            None => {
                assert!(ctx.is_nil());
                None
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
    ///   lfp                            cfp                                       sp
    ///    v                              v                                         v
    /// +------+------+--+------+------+------+------+------+------+------+------+---
    /// |  a0  |  a1  |..|  an  | self | lfp* | cfp* |  pc* | flag | ctx  | iseq |
    /// +------+------+--+------+------+------+------+------+------+------+------+---
    ///  <-------- local frame --------> <----------- control frame ------------>
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
        let iseq = iseq.into();
        let prev_lfp = self.lfp;
        let prev_cfp = self.cfp;
        self.lfp = self.stack_len() - args_len - 1;
        self.cfp = self.stack_len();
        self.frame_push_reg(prev_lfp, prev_cfp, self.pc, use_value, ctx, iseq);
    }

    pub(super) fn unwind_frame(&mut self) {
        let (lfp, cfp, pc) = self.frame_fetch_reg();
        self.set_stack_len(self.lfp);
        self.lfp = lfp;
        self.cfp = cfp;
        self.pc = pc;
    }

    pub(super) fn clear_stack(&mut self) {
        self.set_stack_len(self.cfp + 6);
    }

    fn frame_push_reg(
        &mut self,
        lfp: usize,
        cfp: usize,
        pc: ISeqPos,
        use_value: bool,
        ctx: Option<ContextRef>,
        iseq: Option<ISeqRef>,
    ) {
        self.stack_push(Value::integer(lfp as i64));
        self.stack_push(Value::integer(cfp as i64));
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
            self.exec_stack[cfp].as_fixnum().unwrap() as usize,
            self.exec_stack[cfp + 1].as_fixnum().unwrap() as usize,
            ISeqPos::from(self.exec_stack[cfp + 2].as_fixnum().unwrap() as usize),
        )
    }

    fn flag(&self) -> Value {
        let cfp = self.cfp;
        self.exec_stack[cfp + 3]
    }

    fn flag_mut(&mut self) -> &mut Value {
        let cfp = self.cfp;
        &mut self.exec_stack[cfp + 3]
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
