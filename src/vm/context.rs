pub use crate::*;
use std::ops::{Index, IndexMut, Range};

const LVAR_ARRAY_SIZE: usize = 4;
const INITIAL_STACK_SIZE: usize = 256;

#[derive(Debug, Clone)]
pub struct ContextStack {
    buf: *mut Context,
    sp: usize,
}

impl ContextStack {
    /// Allocate new virtual stack.
    pub fn new() -> Self {
        use std::alloc::{alloc, Layout};
        let layout = Layout::from_size_align(
            INITIAL_STACK_SIZE * std::mem::size_of::<Context>(),
            INITIAL_STACK_SIZE,
        )
        .unwrap();
        let buf = unsafe { alloc(layout) as *mut Context };
        Self { buf, sp: 0 }
    }

    /// Push `context` to the virtual stack, and return a context handle.
    pub fn push(&mut self, context: Context) -> ContextRef {
        unsafe {
            if self.sp >= INITIAL_STACK_SIZE {
                panic!("stack overflow")
            };
            let ptr = self.buf.add(self.sp);
            std::ptr::write(ptr, context);
            self.sp += 1;
            ContextRef::from_ptr(ptr)
        }
    }

    /// Push `context` to the virtual stack, and return a context handle.
    pub fn push_with(
        &mut self,
        self_value: Value,
        block: Block,
        iseq: ISeqRef,
        outer: Option<ContextRef>,
    ) -> ContextRef {
        unsafe {
            if self.sp >= INITIAL_STACK_SIZE {
                panic!("stack overflow")
            };
            let ptr = self.buf.add(self.sp);
            std::ptr::write(ptr, Context::default());
            let lvar_num = iseq.lvars;
            if lvar_num > LVAR_ARRAY_SIZE {
                let v = vec![Value::uninitialized(); lvar_num - LVAR_ARRAY_SIZE];
                (*ptr).lvar_vec = v;
            };
            (*ptr).self_value = self_value;
            (*ptr).block = block;
            (*ptr).iseq_ref = Some(iseq);
            (*ptr).outer = outer;
            self.sp += 1;
            ContextRef::from_ptr(ptr)
        }
    }

    /// Pop `context` from the virtual stack.
    pub fn pop(&mut self, _context: ContextRef) {
        unsafe {
            if self.sp == 0 {
                return;
            }
            let ptr = self.buf.add(self.sp - 1);
            #[cfg(debug_assertions)]
            {
                let ctx = ContextRef::from_ptr(ptr);
                match ctx.on_stack {
                    CtxKind::Stack => {
                        assert_eq!(ctx, _context);
                    }
                    CtxKind::Dead(ctx) => {
                        assert_eq!(ctx, _context);
                    }
                    _ => unreachable!("CtxKind::Heap on the stack."),
                }
            }
            (*ptr).lvar_vec = Vec::new();
            self.sp -= 1;
        }
    }

    #[allow(dead_code)]
    #[cfg(not(tarpaulin_include))]
    pub fn dump(&self) {
        eprintln!("dump context stack");
        for i in 0..self.sp {
            eprint!("[{}]", i);
            ContextRef::from_ptr(unsafe { self.buf.add(self.sp - 1 - i) }).pp();
        }
    }
}

#[derive(Debug, Clone)]
pub struct Context {
    pub self_value: Value,
    pub block: Block,
    lvar_ary: [Value; LVAR_ARRAY_SIZE],
    lvar_vec: Vec<Value>,
    pub iseq_ref: Option<ISeqRef>,
    /// Context of outer scope.
    pub outer: Option<ContextRef>,
    /// Previous context.
    pub caller: Option<ContextRef>,
    pub on_stack: CtxKind,
    pub cur_pc: ISeqPos,
    pub prev_pc: ISeqPos,
    pub prev_stack_len: usize,
    pub called: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CtxKind {
    Heap,
    Stack,
    Dead(ContextRef),
}

pub type ContextRef = Ref<Context>;

impl Index<LvarId> for Context {
    type Output = Value;

    fn index(&self, index: LvarId) -> &Self::Output {
        &self[index.as_usize()]
    }
}

impl Index<usize> for Context {
    type Output = Value;

    fn index(&self, index: usize) -> &Self::Output {
        if index < LVAR_ARRAY_SIZE {
            &self.lvar_ary[index]
        } else {
            &self.lvar_vec[index - LVAR_ARRAY_SIZE]
        }
    }
}

impl IndexMut<LvarId> for Context {
    fn index_mut(&mut self, index: LvarId) -> &mut Self::Output {
        &mut self[index.as_usize()]
    }
}

impl IndexMut<usize> for Context {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index < LVAR_ARRAY_SIZE {
            &mut self.lvar_ary[index]
        } else {
            &mut self.lvar_vec[index - LVAR_ARRAY_SIZE]
        }
    }
}

impl Into<ContextRef> for &Context {
    fn into(self) -> ContextRef {
        Ref::from_ref(self)
    }
}

impl GC for ContextRef {
    fn mark(&self, alloc: &mut Allocator) {
        self.self_value.mark(alloc);
        match self.iseq_ref {
            Some(iseq_ref) => {
                for i in 0..iseq_ref.lvars {
                    self[i].mark(alloc);
                }
            }
            None => {}
        }
        self.block.mark(alloc);
        match self.outer {
            Some(c) => c.mark(alloc),
            None => {}
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Context {
            self_value: Value::uninitialized(),
            block: Block::None,
            lvar_ary: [Value::uninitialized(); LVAR_ARRAY_SIZE],
            lvar_vec: Vec::new(),
            iseq_ref: None,
            outer: None,
            caller: None,
            on_stack: CtxKind::Stack,
            cur_pc: ISeqPos::from(0),
            prev_pc: ISeqPos::from(0),
            prev_stack_len: 0,
            called: true,
        }
    }
}

impl Context {
    fn new(self_value: Value, block: Block, iseq_ref: ISeqRef, outer: Option<ContextRef>) -> Self {
        let lvar_num = iseq_ref.lvars;
        let lvar_vec = if lvar_num > LVAR_ARRAY_SIZE {
            vec![Value::uninitialized(); lvar_num - LVAR_ARRAY_SIZE]
        } else {
            Vec::new()
        };
        Context {
            self_value,
            block,
            lvar_ary: [Value::uninitialized(); LVAR_ARRAY_SIZE],
            lvar_vec,
            iseq_ref: Some(iseq_ref),
            outer,
            caller: None,
            on_stack: CtxKind::Stack,
            cur_pc: ISeqPos::from(0),
            prev_pc: ISeqPos::from(0),
            prev_stack_len: 0,
            called: true,
        }
    }

    fn new_native() -> Self {
        Context {
            self_value: Value::nil(),
            block: Block::None,
            lvar_ary: [Value::uninitialized(); LVAR_ARRAY_SIZE],
            lvar_vec: vec![],
            iseq_ref: None,
            outer: None,
            caller: None,
            on_stack: CtxKind::Stack,
            cur_pc: ISeqPos::from(0),
            prev_pc: ISeqPos::from(0),
            prev_stack_len: 0,
            called: true,
        }
    }

    pub fn on_heap(&self) -> bool {
        self.on_stack == CtxKind::Heap
    }

    pub fn alive(&self) -> bool {
        match self.on_stack {
            CtxKind::Dead(_) => false,
            _ => true,
        }
    }

    pub fn is_method(&self) -> bool {
        self.iseq_ref.unwrap().is_method()
    }

    #[allow(dead_code)]
    #[cfg(not(tarpaulin_include))]
    pub fn dump(&self) {
        eprintln!(
            "{:?} context:{:?} outer:{:?}",
            self.on_stack, self as *const Context, self.outer
        );
        match self.iseq_ref {
            Some(iseq_ref) => {
                eprintln!("  iseq: {:?}", *iseq_ref);
                for i in 0..iseq_ref.lvars {
                    let id = i.into();
                    let (k, _) = iseq_ref
                        .lvar
                        .table()
                        .iter()
                        .find(|(_, v)| **v == id)
                        .unwrap();
                    eprintln!("  self: {:#?}", self.self_value);
                    eprintln!("  lvar({}): {:?} {:#?}", id.as_u32(), k, self[id]);
                }
            }
            None => {}
        }
    }

    #[cfg(not(tarpaulin_include))]
    pub fn pp(&self) {
        println!(
            "{:?} context:{:?} outer:{:?}",
            self.on_stack, self as *const Context, self.outer
        );
    }

    fn copy_from_slice(&mut self, index: usize, slice: &[Value]) {
        let len = slice.len();
        if index + len <= LVAR_ARRAY_SIZE {
            self.lvar_ary[index..index + len].copy_from_slice(slice);
        } else if index >= LVAR_ARRAY_SIZE {
            self.lvar_vec[index - LVAR_ARRAY_SIZE..index + len - LVAR_ARRAY_SIZE]
                .copy_from_slice(slice)
        } else {
            self.lvar_ary[index..LVAR_ARRAY_SIZE]
                .copy_from_slice(&slice[..LVAR_ARRAY_SIZE - index]);
            self.lvar_vec[0..index + len - LVAR_ARRAY_SIZE]
                .copy_from_slice(&slice[LVAR_ARRAY_SIZE - index..])
        }
    }

    pub fn copy_from_slice0(&mut self, slice: &[Value]) {
        let len = slice.len();
        if len <= LVAR_ARRAY_SIZE {
            self.lvar_ary[0..len].copy_from_slice(slice);
        } else {
            self.lvar_ary[0..LVAR_ARRAY_SIZE].copy_from_slice(&slice[..LVAR_ARRAY_SIZE]);
            self.lvar_vec[0..len - LVAR_ARRAY_SIZE].copy_from_slice(&slice[LVAR_ARRAY_SIZE..])
        }
    }

    fn fill(&mut self, range: Range<usize>, val: Value) {
        for i in range {
            self[i] = val;
        }
    }

    fn set_arguments(&mut self, args: &Args, kw_arg: Value) {
        let iseq = self.iseq_ref.unwrap();
        let req_len = iseq.params.req;
        let post_len = iseq.params.post;
        if iseq.is_block() && args.len() == 1 && req_len + post_len > 1 {
            if let Some(ary) = args[0].as_array() {
                self.fill_arguments(&ary.elements, &iseq.params, kw_arg);
                return;
            }
        }

        self.fill_arguments(args, &iseq.params, kw_arg);
    }

    pub fn fill_arguments(&mut self, args: &[Value], params: &ISeqParams, kw_arg: Value) {
        let args_len = args.len();
        let mut kw_len = if kw_arg.is_nil() { 0 } else { 1 };
        let req_len = params.req;
        let rest_len = if params.rest == Some(true) { 1 } else { 0 };
        let post_len = params.post;
        let arg_len = args_len + kw_len - post_len;
        let optreq_len = req_len + params.opt;
        if post_len != 0 {
            // fill post_req params.
            let post_pos = optreq_len + rest_len;
            self.copy_from_slice(post_pos, &args[arg_len..args_len]);
            if kw_len == 1 {
                // fill keyword params as a hash.
                self[post_pos + post_len - 1] = kw_arg;
                kw_len = 0;
            }
        }
        let req_opt = std::cmp::min(optreq_len, arg_len);
        if req_opt != 0 {
            // fill req and opt params.
            self.copy_from_slice0(&args[0..req_opt - kw_len]);
            if kw_len == 1 {
                // fill keyword params as a hash.
                self[req_opt - 1] = kw_arg;
                kw_len = 0;
            }
            if req_opt < req_len {
                // fill rest req params with nil.
                self.fill(req_opt..req_len, Value::nil());
            }
        }
        if rest_len == 1 {
            let ary = if optreq_len >= arg_len {
                vec![]
            } else {
                let mut v = args[optreq_len..arg_len - kw_len].to_vec();
                if kw_len == 1 {
                    v.push(kw_arg);
                }
                v
            };
            self[optreq_len] = Value::array_from(ary);
        }
    }

    pub fn fill_arguments_opt(&mut self, args: &[Value], req_len: usize) {
        let args_len = args.len();
        if req_len <= args_len {
            // fill req params.
            self.copy_from_slice0(&args[0..req_len]);
        } else {
            // fill req params.
            self.copy_from_slice0(args);
            // fill the remaining req params with nil.
            self.fill(args_len..req_len, Value::nil());
        }
    }

    fn from_args_opt_block(&mut self, params: &ISeqParams, args: &Args) {
        let args_len = args.len();
        let req_len = params.req;
        if args_len == 1 && req_len > 1 {
            if let Some(ary) = args[0].as_array() {
                // if a single array argument is given for the block with multiple formal parameters,
                // the arguments must be expanded.
                self.fill_arguments_opt(&ary.elements, req_len);
                return;
            };
        }

        self.fill_arguments_opt(args, req_len);
    }
}

impl ContextRef {
    pub fn new_heap(
        self_value: Value,
        block: Block,
        iseq_ref: ISeqRef,
        outer: Option<ContextRef>,
    ) -> Self {
        let mut context = Context::new(self_value, block, iseq_ref, outer);
        context.on_stack = CtxKind::Heap;
        ContextRef::new(context)
    }

    pub fn new_native(vm: &mut VM) -> Self {
        let context = Context::new_native();
        vm.new_stack_context(context)
    }

    pub fn get_current(self) -> Self {
        match self.on_stack {
            CtxKind::Dead(c) => c,
            _ => self,
        }
    }

    pub fn from_args(
        vm: &mut VM,
        self_value: Value,
        iseq: ISeqRef,
        args: &Args,
        outer: Option<ContextRef>,
    ) -> Result<Self, RubyError> {
        if iseq.opt_flag {
            if !args.kw_arg.is_nil() {
                return Err(RubyError::argument("Undefined keyword."));
            };

            if iseq.is_block() {
                let mut context =
                    vm.new_stack_context_with(self_value, args.block.clone(), iseq, outer);
                context.from_args_opt_block(&iseq.params, args);
                return Ok(context);
            } else {
                let req_len = iseq.params.req;
                args.check_args_num(req_len)?;
                let mut context =
                    vm.new_stack_context_with(self_value, args.block.clone(), iseq, outer);
                context.copy_from_slice0(args);
                return Ok(context);
            }
        }
        let params = &iseq.params;
        let mut keyword_flag = false;
        let kw = if params.keyword.is_empty() && !params.kwrest {
            // if no keyword param nor kwrest param exists in formal parameters,
            // make Hash.
            args.kw_arg
        } else {
            keyword_flag = !args.kw_arg.is_nil();
            Value::nil()
        };
        if !iseq.is_block() {
            let min = params.req + params.post;
            let kw = if kw.is_nil() { 0 } else { 1 };
            if params.rest.is_some() {
                if min > kw {
                    args.check_args_min(min - kw)?;
                }
            } else {
                args.check_args_range_ofs(kw, min, min + params.opt)?;
            }
        }

        let mut context = vm.new_stack_context_with(self_value, args.block.clone(), iseq, outer);
        context.set_arguments(args, kw);
        if params.kwrest || keyword_flag {
            let mut kwrest = FxIndexMap::default();
            if keyword_flag {
                let keyword = args.kw_arg.as_hash().unwrap();
                for (k, v) in keyword.iter() {
                    let id = k.as_symbol().unwrap();
                    match params.keyword.get(&id) {
                        Some(lvar) => {
                            context[*lvar] = v;
                        }
                        None => {
                            if params.kwrest {
                                kwrest.insert(HashKey(k), v);
                            } else {
                                return Err(RubyError::argument("Undefined keyword."));
                            }
                        }
                    };
                }
            };
            if let Some(id) = iseq.lvar.kwrest_param() {
                context[id] = Value::hash_from_map(kwrest);
            }
        };
        if let Some(id) = iseq.lvar.block_param() {
            context[id] = match &args.block {
                Block::Block(method, ctx) => {
                    let proc_context = vm.create_block_context(*method, *ctx);
                    Value::procobj(proc_context)
                }
                Block::Proc(proc) => *proc,
                Block::None => Value::nil(),
            }
        }
        Ok(context)
    }

    /// Move a context on the stack to the heap.
    pub fn move_to_heap(mut self) -> ContextRef {
        if self.on_heap() {
            return self;
        }
        assert!(self.alive());
        let mut heap = self.dup();
        heap.on_stack = CtxKind::Heap;
        self.on_stack = CtxKind::Dead(heap);

        match heap.outer {
            Some(c) => {
                if c.on_stack == CtxKind::Stack {
                    let c_heap = c.move_to_heap();
                    heap.outer = Some(c_heap);
                }
            }
            None => {}
        }
        heap
    }

    #[cfg(not(tarpaulin_include))]
    pub fn adjust_lvar_size(&mut self) {
        let len = self.iseq_ref.unwrap().lvars;
        if LVAR_ARRAY_SIZE != len {
            //panic!();
        }
    }
}
