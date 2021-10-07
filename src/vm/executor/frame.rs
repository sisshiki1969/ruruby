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
const CTX_OFFSET: usize = 5;
const ISEQ_OFFSET: usize = 6;
const FRAME_LEN: usize = 7;

/// Control frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame(usize);

impl Frame {
    fn from(fp: usize) -> Option<Self> {
        if fp == 0 {
            None
        } else {
            Some(Frame(fp))
        }
    }
}

impl VM {
    fn frame_prev_cfp(&self, f: Frame) -> usize {
        self.exec_stack[f.0 + CFP_OFFSET].as_fixnum().unwrap() as usize
    }

    fn frame_mfp(&self, f: Frame) -> usize {
        self.exec_stack[f.0 + MFP_OFFSET].as_fixnum().unwrap() as usize
    }

    fn frame_dfp(&self, f: Frame) -> Option<Context> {
        let dfp = self.exec_stack[f.0 + DFP_OFFSET].as_fixnum().unwrap();
        if dfp == 0 {
            None
        } else if dfp < 0 {
            Some(Frame(-dfp as usize).into())
        } else {
            Some(HeapCtxRef::from_ptr((dfp << 3) as *const HeapContext as *mut _).into())
        }
    }

    fn frame_pc(&self, f: Frame) -> usize {
        self.exec_stack[f.0 + PC_OFFSET].as_fixnum().unwrap() as usize
    }

    fn frame_locals(&self, f: Frame) -> &[Value] {
        let lfp = f.0 - self.frame_local_len(f) - 1;
        self.slice(lfp, f.0)
    }

    /// Get context of `frame`.
    ///
    /// If `frame` is a native (Rust) frame, return None.
    pub(super) fn frame_heap(&self, frame: Frame) -> Option<HeapCtxRef> {
        assert!(frame.0 != 0);
        let ctx = self.exec_stack[frame.0 + CTX_OFFSET];
        match ctx.as_fixnum() {
            Some(i) => {
                let u = (i << 3) as u64;
                Some(HeapCtxRef::from_ptr(u as *const HeapContext as *mut _))
            }
            None => {
                assert!(ctx.is_nil());
                None
            }
        }
    }

    pub(super) fn frame_iseq(&self, frame: Frame) -> Option<ISeqRef> {
        let i = self.exec_stack[frame.0 + ISEQ_OFFSET].as_fixnum().unwrap();
        if i == 0 {
            None
        } else {
            let u = (i << 3) as u64;
            Some(ISeqRef::from_ptr(u as *const ISeqInfo as *mut _))
        }
    }

    pub(super) fn frame_self(&self, frame: Frame) -> Value {
        assert!(frame.0 != 0);
        self.exec_stack[frame.0 - 1]
    }

    fn frame_local_len(&self, frame: Frame) -> usize {
        (self.exec_stack[frame.0 + FLAG_OFFSET].as_fixnum().unwrap() as usize) >> 32
    }
}

impl VM {
    /// Get the caller frame of `frame`.
    pub(super) fn frame_caller(&self, frame: Frame) -> Option<Frame> {
        let cfp = self.frame_prev_cfp(frame);
        Frame::from(cfp)
    }

    /// Get the method frame of `frame`.
    fn frame_method_frame(&self, frame: Frame) -> Option<Frame> {
        let mfp = self.frame_mfp(frame);
        Frame::from(mfp)
    }

    /// Set the context of `frame` to `ctx`.
    pub(super) fn set_heap(&mut self, frame: Frame, ctx: HeapCtxRef) {
        let adr = ctx.id();
        assert!(adr & 0b111 == 0);
        let i = adr as i64 >> 3;
        self.exec_stack[frame.0 + CTX_OFFSET] = Value::integer(i)
    }
}

impl VM {
    /// Get current frame.
    pub(super) fn cur_frame(&self) -> Frame {
        Frame::from(self.cfp).unwrap()
    }

    /// Get current method frame.
    fn cur_method_frame(&self) -> Option<Frame> {
        self.frame_method_frame(self.cur_frame())
    }

    pub fn cur_caller_frame(&self) -> Option<Frame> {
        self.frame_caller(self.cur_frame())
    }

    pub fn cur_delegate(&self) -> Option<Value> {
        let method_context = self.get_method_context();
        match method_context.iseq_ref.params.delegate {
            Some(v) => {
                let delegate = method_context[v];
                if delegate.is_nil() {
                    None
                } else {
                    Some(delegate)
                }
            }
            None => None,
        }
    }

    pub fn caller_method_context(&self) -> HeapCtxRef {
        let frame = self.cur_caller_frame().unwrap();
        if let Some(f) = self.frame_method_frame(frame) {
            self.frame_heap(f).unwrap()
        } else {
            // In the case of the first invoked context of Fiber
            self.get_fiber_method_context()
        }
    }

    pub(super) fn get_method_context(&self) -> HeapCtxRef {
        if let Some(f) = self.cur_method_frame() {
            self.frame_heap(f).unwrap()
        } else {
            // In the case of the first invoked context of Fiber
            self.get_fiber_method_context()
        }
    }

    pub(super) fn get_method_iseq(&self) -> ISeqRef {
        if let Some(f) = self.cur_method_frame() {
            self.frame_iseq(f).unwrap()
        } else {
            // In the case of the first invoked context of Fiber
            self.get_fiber_method_context().iseq_ref
        }
    }

    pub(super) fn get_context_self(&self, outer: &Context) -> Value {
        match outer {
            Context::Frame(f) => self.frame_self(*f),
            Context::Heap(c) => c.self_value,
        }
    }

    pub fn get_context_heap(&self, outer: &Context) -> HeapCtxRef {
        match outer {
            Context::Frame(f) => self.frame_heap(*f).unwrap(),
            Context::Heap(c) => *c,
        }
    }

    pub(super) fn cur_iseq(&self) -> ISeqRef {
        self.frame_iseq(self.cur_frame()).unwrap()
    }

    pub(crate) fn caller_iseq(&self) -> ISeqRef {
        let c = self.cur_caller_frame().unwrap();
        self.frame_iseq(c).unwrap()
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
    pub(crate) fn prepare_block_args(&mut self, iseq: ISeqRef, args_pos: usize) {
        // if a single Array argument is given for the block requiring multiple formal parameters,
        // the arguments must be expanded.
        let req_len = iseq.params.req;
        let post_len = iseq.params.post;
        if iseq.is_block() && self.stack_len() - args_pos == 1 && req_len + post_len > 1 {
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
        self.frame_push_reg(0, 0, ISeqPos::from(0), false, None, None, None, 0)
    }

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
    ///
    ///  ### After
    ///~~~~text
    ///   lfp                            cfp                                              sp
    ///    v                              v                                                v
    /// +------+------+--+------+------+------+------+------+------+------+------+------+-----
    /// |  a0  |  a1  |..|  an  | self | flg* | cfp* | mfp  | dfp  |  pc* | ctx  | iseq |
    /// +------+------+--+------+------+------+------+------+------+------+------+------+-----
    ///  <-------- local frame --------> <-------------- control frame ---------------->
    ///~~~~
    ///
    /// - lfp*: prev lfp
    /// - cfp*: prev cfp
    /// - pc*:  prev pc
    /// - flag: flags
    /// - ctx: ContextRef (if native function, nil is stored.)
    /// - iseq: ISeqRef (if native function, nil is stored.)
    ///
    pub fn prepare_frame(
        &mut self,
        args_len: usize,
        use_value: bool,
        ctx: impl Into<Option<HeapCtxRef>>,
        outer: Option<Context>,
        iseq: impl Into<Option<ISeqRef>>,
    ) {
        let ctx = ctx.into();
        let iseq: Option<ISeqRef> = iseq.into();
        let prev_cfp = self.cfp;
        self.lfp = self.stack_len() - args_len - 1;
        self.cfp = self.stack_len();
        assert!(prev_cfp != 0);
        let mfp = if iseq.is_some() {
            if outer.is_none() {
                // In the case of Ruby method.
                self.cfp
            } else {
                // In the case of Ruby block.
                match self.frame_caller(Frame(prev_cfp)) {
                    None => 0,
                    Some(f) => self.frame_mfp(f),
                }
            }
        } else {
            // In the case of native method.
            self.frame_mfp(Frame(prev_cfp))
        };
        self.frame_push_reg(
            prev_cfp, mfp, self.pc, use_value, ctx, outer, iseq, args_len,
        );
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
            if let Some(ctx) = self.frame_heap(self.cur_frame()) {
                eprintln!("{:?}", *ctx);
                eprintln!("lvars: {:?}", ctx.iseq_ref.lvars);
                eprintln!("param: {:?}", ctx.iseq_ref.params);
            } else {
                eprintln!("None");
            }
        }
    }

    pub(super) fn unwind_frame(&mut self) {
        /*if let Some(mut heap) = self.frame_heap(self.cur_frame()) {
            heap.lvar = self.args().to_vec();
        }*/
        let (cfp, pc) = self.frame_fetch_reg();
        self.set_stack_len(self.lfp);
        self.cfp = cfp;
        self.pc = pc;
        assert!(cfp != 0);
        let args_len = (self.flag().as_fixnum().unwrap() as usize) >> 32;
        self.lfp = cfp - args_len - 1;
        #[cfg(feature = "trace")]
        if self.globals.startup_flag {
            eprintln!("unwind lfp:{} cfp:{}", self.lfp, self.cfp);
        }
    }

    pub(super) fn clear_stack(&mut self) {
        self.set_stack_len(self.cfp + FRAME_LEN);
    }

    fn frame_push_reg(
        &mut self,
        cfp: usize,
        mfp: usize,
        pc: ISeqPos,
        use_value: bool,
        ctx: Option<HeapCtxRef>,
        outer: Option<Context>,
        iseq: Option<ISeqRef>,
        args_len: usize,
    ) {
        self.stack_push(Value::integer(
            if use_value { 0 } else { 2 } | ((args_len as i64) << 32),
        ));
        self.stack_push(Value::integer(cfp as i64));
        self.stack_push(Value::integer(mfp as i64));
        self.stack_push(Value::integer(if let Some(outer) = outer {
            match outer {
                Context::Frame(f) => -(f.0 as i64),
                Context::Heap(h) => (h.id() >> 3) as i64,
            }
        } else {
            0
        }));
        self.stack_push(Value::integer(pc.into_usize() as i64));
        self.stack_push(if let Some(ctx) = ctx {
            let adr = ctx.id();
            assert!(adr & 0b111 == 0);
            let i = adr as i64 >> 3;
            Value::integer(i)
        } else {
            Value::nil()
        });
        self.stack_push(Value::integer(if let Some(iseq) = iseq {
            let adr = iseq.id();
            assert!(adr & 0b111 == 0);
            adr as i64 >> 3
        } else {
            0
        }));
    }

    fn frame_fetch_reg(&mut self) -> (usize, ISeqPos) {
        let f = Frame(self.cfp);
        (self.frame_prev_cfp(f), ISeqPos::from(self.frame_pc(f)))
    }

    ///
    /// Frame flags.
    ///
    /// 0 0 0 0_0 0 0 1
    ///         | | | |
    ///         | | | +-- always 1 (represents Value::integer)
    ///         | | +---- is_called (0: normaly invoked  1: vm_loop was called recursively)
    ///         | +------ discard_value (0: use return value  1: discard return value)
    ///         +-------- is_module_function (0: no 1:yes)
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

    /// Check module_function flag of the current frame.
    pub fn is_module_function(&self) -> bool {
        // TODO:This may cause panic in some code like:
        //
        // module m
        //   f = Fiber.new { def f; end }
        //   f.resume
        // end
        //
        let mfp = self.cur_method_frame().unwrap().0;
        self.exec_stack[mfp + FLAG_OFFSET].get() & 0b1000 != 0
    }

    /// Set module_function flag of the current frame to true.
    pub fn set_module_function(&mut self) {
        let mfp = self.cur_method_frame().unwrap().0;
        let f = &mut self.exec_stack[mfp + FLAG_OFFSET];
        *f = Value::from(f.get() | 0b1000);
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
        outer: impl Into<Option<Context>>,
        use_value: bool,
    ) -> Result<(), RubyError> {
        let self_value = self.stack_pop();
        let params = &iseq.params;
        let base = self.stack_len() - args.len();
        let outer = outer.into();
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
            if !args.kw_arg.is_nil() {
                self.stack_push(args.kw_arg);
            }
            (!args.kw_arg.is_nil(), false)
        } else {
            (false, !args.kw_arg.is_nil())
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
        let mut context = HeapCtxRef::new_heap(
            self_value,
            args.block.clone(),
            iseq,
            outer.as_ref().map(|ctx| self.move_outer_to_heap(ctx)),
        );
        self.prepare_frame(
            self.stack_len() - base - 1,
            use_value,
            Some(context),
            outer.map(|c| c.into()),
            iseq,
        );
        // Handling block paramter.
        if let Some(id) = iseq.lvar.block_param() {
            self.fill_block_argument(base, id, &args.block);
        }
        context.lvar = self.args().to_vec();

        #[cfg(feature = "trace")]
        self.dump_current_frame();
        Ok(())
    }

    /// Move outer execution contexts on the stack to the heap.
    pub fn move_outer_to_heap(&mut self, ctx: &Context) -> HeapCtxRef {
        match ctx {
            Context::Frame(f) => {
                let self_val = self.frame_self(*f);
                let iseq = self.frame_iseq(*f).unwrap();
                let outer = self.frame_dfp(*f);
                let outer = outer.map(|ctx| self.move_outer_to_heap(&ctx));
                let mut heap = HeapCtxRef::new_heap(self_val, None, iseq, outer);
                heap.lvar = self.frame_locals(*f).to_vec();
                self.set_heap(*f, heap);
                heap
            }
            Context::Heap(h) => return *h,
        }
    }
}
