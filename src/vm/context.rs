pub use crate::*;

const LVAR_ARRAY_SIZE: usize = 16;

#[derive(Debug, Clone)]
pub struct Context {
    pub is_fiber: bool,
    pub self_value: Value,
    pub block: Option<MethodRef>,
    lvar_scope: [Value; LVAR_ARRAY_SIZE],
    ext_lvar: Vec<Value>,
    pub iseq_ref: ISeqRef,
    pub pc: usize,
    pub outer: Option<ContextRef>,
    pub on_stack: bool,
    pub stack_len: usize,
    pub kind: ISeqKind,
}

pub type ContextRef = Ref<Context>;

impl std::ops::Index<LvarId> for Context {
    type Output = Value;

    fn index(&self, index: LvarId) -> &Self::Output {
        let i = index.as_usize();
        if i < LVAR_ARRAY_SIZE {
            &self.lvar_scope[i]
        } else {
            &self.ext_lvar[i - LVAR_ARRAY_SIZE]
        }
    }
}

impl std::ops::Index<usize> for Context {
    type Output = Value;

    fn index(&self, index: usize) -> &Self::Output {
        if index < LVAR_ARRAY_SIZE {
            &self.lvar_scope[index]
        } else {
            &self.ext_lvar[index - LVAR_ARRAY_SIZE]
        }
    }
}

impl std::ops::IndexMut<LvarId> for Context {
    fn index_mut(&mut self, index: LvarId) -> &mut Self::Output {
        let i = index.as_usize();
        if i < LVAR_ARRAY_SIZE {
            &mut self.lvar_scope[i]
        } else {
            &mut self.ext_lvar[i - LVAR_ARRAY_SIZE]
        }
    }
}

impl std::ops::IndexMut<usize> for Context {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index < LVAR_ARRAY_SIZE {
            &mut self.lvar_scope[index]
        } else {
            &mut self.ext_lvar[index - LVAR_ARRAY_SIZE]
        }
    }
}

impl Context {
    pub fn new(
        self_value: Value,
        block: Option<MethodRef>,
        iseq_ref: ISeqRef,
        outer: Option<ContextRef>,
    ) -> Self {
        let lvar_num = iseq_ref.lvars;
        let ext_lvar = if lvar_num > LVAR_ARRAY_SIZE {
            vec![Value::uninitialized(); lvar_num - LVAR_ARRAY_SIZE]
        } else {
            Vec::new()
        };
        Context {
            is_fiber: false,
            self_value,
            block,
            lvar_scope: [Value::uninitialized(); LVAR_ARRAY_SIZE],
            ext_lvar,
            iseq_ref,
            pc: 0,
            outer,
            on_stack: true,
            stack_len: 0,
            kind: iseq_ref.kind.clone(),
        }
    }

    pub fn from_args(
        vm: &mut VM,
        self_value: Value,
        iseq: ISeqRef,
        args: &Args,
        outer: Option<ContextRef>,
    ) -> Result<Self, RubyError> {
        let mut context = Context::new(self_value, args.block, iseq, outer);
        let kw = if iseq.params.keyword_params.is_empty() {
            args.kw_arg
        } else {
            None
        };
        vm.check_args_range(
            args.len() + if kw.is_some() { 1 } else { 0 },
            iseq.params.min_params,
            iseq.params.max_params,
        )?;
        context.set_arguments(&vm.globals, args, kw);
        match args.kw_arg {
            Some(kw_arg) if kw.is_none() => {
                let keyword = kw_arg.as_hash().unwrap();
                for (k, v) in keyword.iter() {
                    let id = k.as_symbol().unwrap();
                    match iseq.params.keyword_params.get(&id) {
                        Some(lvar) => {
                            context[*lvar] = v;
                        }
                        None => return Err(vm.error_argument("Undefined keyword.")),
                    };
                }
            }
            _ => {}
        };
        if let Some(id) = iseq.lvar.block_param() {
            context[id] = match args.block {
                Some(block) => {
                    let proc_context = vm.create_block_context(block)?;
                    Value::procobj(&vm.globals, proc_context)
                }
                None => Value::nil(),
            }
        }
        Ok(context)
    }

    fn set_arguments(&mut self, globals: &Globals, args: &Args, kw_arg: Option<Value>) {
        let mut kw_len = if kw_arg.is_some() { 1 } else { 0 };
        let params = &self.iseq_ref.params;
        let req_len = params.req_params;
        let opt_len = params.opt_params;
        let rest_len = if params.rest_param { 1 } else { 0 };
        let post_len = params.post_params;
        let post_pos = req_len + opt_len + rest_len;

        match self.kind {
            ISeqKind::Block(_) if args.len() == 1 && req_len + post_len > 1 => {
                match args[0].as_array() {
                    Some(ary) => {
                        let arg_len = ary.elements.len() + kw_len;
                        let args = &ary.elements;

                        if post_len != 0 {
                            // fill post_req params.
                            for i in 0..post_len - kw_len {
                                self[post_pos + i] = args[arg_len - post_len + i];
                            }
                            if kw_len == 1 {
                                // fill keyword params as a hash.
                                self[post_pos + post_len - 1] = kw_arg.unwrap();
                                kw_len = 0;
                            }
                        }
                        let req_opt = std::cmp::min(opt_len + req_len, arg_len - post_len);
                        if req_opt != 0 {
                            // fill req and opt params.
                            for i in 0..req_opt - kw_len {
                                self[i] = args[i];
                            }
                            if kw_len == 1 {
                                // fill keyword params as a hash.
                                self[req_opt - 1] = kw_arg.unwrap();
                                kw_len = 0;
                            }
                            if req_opt < req_len {
                                // fill rest req params with nil.
                                for i in req_opt..req_len {
                                    self[i] = Value::nil();
                                }
                            }
                        }
                        if rest_len == 1 {
                            let ary = if req_len + opt_len + post_len >= arg_len {
                                vec![]
                            } else {
                                let mut v =
                                    args[req_len + opt_len..arg_len - post_len - kw_len].to_vec();
                                if kw_len == 1 {
                                    v.push(kw_arg.unwrap());
                                }
                                v
                            };
                            let val = Value::array_from(globals, ary);
                            self[req_len + opt_len] = val;
                        }
                        return;
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        let arg_len = args.len() + kw_len;
        if post_len != 0 {
            // fill post_req params.
            for i in 0..post_len - kw_len {
                self[post_pos + i] = args[arg_len - post_len + i];
            }
            if kw_len == 1 {
                // fill keyword params as a hash.
                self[post_pos + post_len - 1] = kw_arg.unwrap();
                kw_len = 0;
            }
        }
        let req_opt = std::cmp::min(opt_len + req_len, arg_len - post_len);
        if req_opt != 0 {
            // fill req and opt params.
            for i in 0..req_opt - kw_len {
                self[i] = args[i];
            }
            if kw_len == 1 {
                // fill keyword params as a hash.
                self[req_opt - 1] = kw_arg.unwrap();
                kw_len = 0;
            }
            if req_opt < req_len {
                // fill rest req params with nil.
                for i in req_opt..req_len {
                    self[i] = Value::nil();
                }
            }
        }
        if rest_len == 1 {
            let ary = if req_len + opt_len + post_len >= arg_len {
                vec![]
            } else {
                let mut v = args[req_len + opt_len..arg_len - post_len - kw_len].to_vec();
                if kw_len == 1 {
                    v.push(kw_arg.unwrap());
                }
                v
            };
            let val = Value::array_from(globals, ary);
            self[req_len + opt_len] = val;
        }
    }
}

impl ContextRef {
    pub fn from(
        self_value: Value,
        block: Option<MethodRef>,
        iseq_ref: ISeqRef,
        outer: Option<ContextRef>,
    ) -> Self {
        let mut context = Context::new(self_value, block, iseq_ref, outer);
        context.on_stack = false;
        ContextRef::new(context)
    }

    pub fn from_local(info: &Context) -> Self {
        Ref::from_ref(info)
    }

    pub fn adjust_lvar_size(&mut self) {
        let len = self.iseq_ref.lvars;
        if LVAR_ARRAY_SIZE < len {
            for _ in 0..len - LVAR_ARRAY_SIZE {
                self.ext_lvar.push(Value::nil());
            }
        }
    }
}
