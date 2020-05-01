pub use crate::*;

const LVAR_ARRAY_SIZE: usize = 8;

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
}

pub type ContextRef = Ref<Context>;

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
        }
    }

    pub fn get_lvar(&self, id: LvarId) -> Value {
        let id = id.as_usize();
        if id < LVAR_ARRAY_SIZE {
            self.lvar_scope[id]
        } else {
            self.ext_lvar[id - LVAR_ARRAY_SIZE]
        }
    }

    pub fn set_lvar(&mut self, id: usize, val: Value) {
        if id < LVAR_ARRAY_SIZE {
            self.lvar_scope[id] = val;
        } else {
            self.ext_lvar[id - LVAR_ARRAY_SIZE] = val;
        }
    }

    pub fn get_mut_lvar(&mut self, id: LvarId) -> &mut Value {
        let id = id.as_usize();
        if id < LVAR_ARRAY_SIZE {
            &mut self.lvar_scope[id]
        } else {
            &mut self.ext_lvar[id - LVAR_ARRAY_SIZE]
        }
    }

    pub fn set_arguments(&mut self, globals: &Globals, args: &Args, kw_arg: Option<Value>) {
        let mut kw_len = if kw_arg.is_some() { 1 } else { 0 };
        let req_len = self.iseq_ref.req_params;
        let opt_len = self.iseq_ref.opt_params;
        let rest_len = if self.iseq_ref.rest_param { 1 } else { 0 };
        let post_len = self.iseq_ref.post_params;
        let post_pos = req_len + opt_len + rest_len;

        match self.iseq_ref.kind {
            ISeqKind::Proc(_) if args.len() == 1 && req_len + post_len > 1 => {
                match args[0].as_array() {
                    Some(ary) => {
                        let arg_len = ary.elements.len() + kw_len;
                        let args = &ary.elements;

                        if post_len != 0 {
                            // fill post_req params.
                            for i in 0..post_len - kw_len {
                                self.set_lvar(post_pos + i, args[arg_len - post_len + i]);
                            }
                            if kw_len == 1 {
                                // fill keyword params as a hash.
                                self.set_lvar(post_pos + post_len - 1, kw_arg.unwrap());
                                kw_len = 0;
                            }
                        }
                        let req_opt = std::cmp::min(opt_len + req_len, arg_len - post_len);
                        if req_opt != 0 {
                            // fill req and opt params.
                            for i in 0..req_opt - kw_len {
                                self.set_lvar(i, args[i]);
                            }
                            if kw_len == 1 {
                                // fill keyword params as a hash.
                                self.set_lvar(req_opt - 1, kw_arg.unwrap());
                                kw_len = 0;
                            }
                            if req_opt < req_len {
                                // fill rest req params with nil.
                                for i in req_opt..req_len {
                                    self.set_lvar(i, Value::nil());
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
                            self.set_lvar(req_len + opt_len, val);
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
                self.set_lvar(post_pos + i, args[arg_len - post_len + i]);
            }
            if kw_len == 1 {
                // fill keyword params as a hash.
                self.set_lvar(post_pos + post_len - 1, kw_arg.unwrap());
                kw_len = 0;
            }
        }
        let req_opt = std::cmp::min(opt_len + req_len, arg_len - post_len);
        if req_opt != 0 {
            // fill req and opt params.
            for i in 0..req_opt - kw_len {
                self.set_lvar(i, args[i]);
            }
            if kw_len == 1 {
                // fill keyword params as a hash.
                self.set_lvar(req_opt - 1, kw_arg.unwrap());
                kw_len = 0;
            }
            if req_opt < req_len {
                // fill rest req params with nil.
                for i in req_opt..req_len {
                    self.set_lvar(i, Value::nil());
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
            self.set_lvar(req_len + opt_len, val);
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
