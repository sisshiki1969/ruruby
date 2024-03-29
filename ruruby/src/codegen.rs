use std::path::PathBuf;

use crate::*;
use ruruby_parse::{BlockInfo, Node};
mod defined;
mod send;

/// Codegen State
#[derive(Debug, Clone)]
pub struct Codegen {
    method_stack: Vec<FnId>,
    loop_stack: Vec<LoopInfo>,
    context_stack: Vec<Context>,
    extern_context: Option<EnvFrame>,
    pub loc: Loc,
    pub source_info: SourceInfoRef,
}

// Public APIs
impl Codegen {
    pub(crate) fn gen_toplevel(
        globals: &mut Globals,
        kind: ContextKind,
        path: impl Into<PathBuf>,
        code: String,
        extern_context: Option<EnvFrame>,
    ) -> Result<FnId, RubyError> {
        let result =
            Parser::<EnvFrame>::parse_program(code, path, "Top", Option::<EnvFrame>::None)?;

        Self::new_iseq(globals, kind, result, extern_context)
    }

    pub(crate) fn gen_toplevel_binding(
        globals: &mut Globals,
        kind: ContextKind,
        path: impl Into<PathBuf>,
        code: String,
        binding_context: Option<EnvFrame>,
        extern_context: Option<EnvFrame>,
    ) -> Result<FnId, RubyError> {
        let path = path.into();
        let result =
            Parser::<EnvFrame>::parse_program_binding(code, path, binding_context, extern_context)?;
        Self::new_iseq(globals, kind, result, extern_context)
    }

    /// Generate ISeq for sym.to_proc.
    /// this function make iseq mostly equivalent to {|x| x.method}.
    pub(crate) fn gen_sym_to_proc_iseq(globals: &mut Globals, method: IdentId) -> FnId {
        let id = globals.methods.add(MethodInfo::default());
        let mut iseq = ISeq::new();
        let mut iseq_sourcemap = vec![];
        iseq.push(Inst::GET_LOCAL);
        iseq.push32(0);
        iseq_sourcemap.push((iseq.current(), Loc(0, 0)));
        iseq.push(Inst::OPT_SEND);
        iseq.push32(method.into());
        iseq.push16(0);
        iseq.push_method(None);
        iseq.push32(globals.methods.add_inline_cache_entry());
        iseq.gen_return();

        let info = MethodInfo::RubyFunc {
            iseq: ISeqRef::new(ISeqInfo::new_sym_to_proc(
                id,
                iseq,
                iseq_sourcemap,
                SourceInfoRef::new(SourceInfo {
                    path: std::path::PathBuf::from("(eval)"),
                    code: "".to_string(),
                }),
            )),
        };
        globals.methods.update(id, info);
        id
    }
}

impl Codegen {
    fn new(source_info: SourceInfoRef, extern_context: Option<EnvFrame>) -> Self {
        Codegen {
            method_stack: vec![],
            context_stack: vec![Context::new()],
            extern_context,
            loop_stack: vec![LoopInfo::new_top()],
            loc: Loc(0, 0),
            source_info,
        }
    }

    fn new_iseq(
        globals: &mut Globals,
        kind: ContextKind,
        result: ParseResult,
        extern_context: Option<EnvFrame>,
    ) -> Result<FnId, RubyError> {
        let mut codegen = Codegen::new(result.source_info, extern_context);
        let loc = result.node.loc;
        codegen.gen_iseq(
            globals,
            vec![],
            result.node,
            result.lvar_collector,
            true,
            kind,
            vec![],
            loc,
        )
    }

    fn context(&self) -> &Context {
        self.context_stack.last().unwrap()
    }

    fn context_mut(&mut self) -> &mut Context {
        self.context_stack.last_mut().unwrap()
    }

    fn push_jump_dest(&mut self, has_ensure: bool) {
        self.context_mut()
            .jump_dest
            .push(LocalJumpDest::new(has_ensure));
    }

    fn pop_jump_dest(&mut self) -> LocalJumpDest {
        self.context_mut().jump_dest.pop().unwrap()
    }

    fn push_ex_rescue(&mut self, body_start: ISeqPos, body_end: ISeqPos, dest: ISeqPos) {
        self.context_mut()
            .exception_table
            .push(ExceptionEntry::new_rescue(body_start, body_end, dest));
    }

    fn push_ex_continue(&mut self, body_start: ISeqPos, body_end: ISeqPos, dest: ISeqPos) {
        self.context_mut()
            .exception_table
            .push(ExceptionEntry::new_continue(body_start, body_end, dest));
    }
}

impl Codegen {
    /// Generate ISeq.
    fn gen_iseq(
        &mut self,
        globals: &mut Globals,
        param_list: Vec<FormalParam>,
        node: Node,
        lvar_collector: LvarCollector,
        use_value: bool,
        kind: ContextKind,
        forvars: Vec<IdentId>,
        iseq_loc: Loc,
    ) -> Result<FnId, RubyError> {
        let id = globals.methods.add(MethodInfo::default());
        let is_block = !kind.is_method();
        if !is_block {
            self.method_stack.push(id)
        }
        let save_loc = self.loc;
        let mut params = ISeqParams::default();
        let mut iseq = ISeq::new();

        self.context_stack
            .push(Context::from(lvar_collector.table.clone(), kind));
        for (lvar_id, param) in param_list.into_iter().enumerate() {
            match param.kind {
                ParamKind::Param(id) => {
                    params.param_ident.push(id);
                    params.req += 1;
                }
                ParamKind::Post(id) => {
                    params.param_ident.push(id);
                    params.post += 1;
                }
                ParamKind::Optional(id, default) => {
                    params.param_ident.push(id);
                    params.opt += 1;
                    self.gen_default_expr(globals, &mut iseq, id, *default)?;
                }
                ParamKind::Rest(id) => {
                    params.param_ident.push(id);
                    params.rest = Some(true);
                }
                ParamKind::RestDiscard => {
                    params.rest = Some(false);
                }
                ParamKind::Keyword(id, default) => {
                    params.param_ident.push(id);
                    params.keyword.insert(id, lvar_id.into());
                    if let Some(default) = default {
                        self.gen_default_expr(globals, &mut iseq, id, *default)?
                    }
                }
                ParamKind::KWRest(id) => {
                    params.param_ident.push(id);
                    params.kwrest = true;
                }
                ParamKind::Block(id) => {
                    params.param_ident.push(id);
                    params.block = true;
                }
                ParamKind::Delegate => {
                    params.delegate = Some(LvarId::from(params.req));
                    break;
                }
            }
        }

        forvars.iter().enumerate().for_each(|(i, id)| {
            iseq.push(Inst::GET_LOCAL);
            iseq.push32(i as u32);
            self.emit_set_local(&mut iseq, *id);
        });
        self.gen(globals, &mut iseq, node, use_value)?;
        let context = self.context_stack.pop().unwrap();

        let iseq_sourcemap = context.iseq_sourcemap;
        let exception_table = context.exception_table;
        iseq.gen_return();
        iseq.optimize();
        self.loc = save_loc;

        let info = MethodInfo::RubyFunc {
            iseq: ISeqRef::new(ISeqInfo::new(
                id,
                params,
                iseq,
                lvar_collector,
                exception_table,
                iseq_sourcemap,
                self.source_info.clone(),
                match kind {
                    ContextKind::Block => ISeqKind::Block,
                    ContextKind::Eval => ISeqKind::Other,
                    ContextKind::Class(name) => ISeqKind::Class(name),
                    ContextKind::Method(name) => ISeqKind::Method(name),
                },
                iseq_loc,
            )),
        };

        if !is_block {
            self.method_stack.pop();
        }
        globals.methods.update(id, info);
        #[cfg(feature = "emit-iseq")]
        {
            if globals.startup_flag {
                let iseq = globals.methods[id].as_iseq();
                eprintln!("-----------------------------------------");
                eprintln!("[{:?}] {:?}", id, *iseq);
                eprintln!(
                    "source: {}:{}",
                    iseq.source_info.get_file_name(),
                    iseq.source_info.get_lines(&iseq_loc)[0].no
                );
                eprintln!("params: {:?}", iseq.params);
                eprintln!("sourcemap: {:?}", iseq.iseq_sourcemap);
                eprintln!("exception: {:?}", iseq.exception_table);
                eprint!("local var: ");
                for (i, id) in iseq.lvar.table().iter().enumerate() {
                    eprint!("{}:{:?} ", i, id);
                }
                eprintln!();
                eprintln!("block: {:?}", iseq.lvar.block());
                let mut pc = ISeqPos::from(0);
                while pc.into_usize() < iseq.iseq.len() {
                    eprintln!("  {:05x} {}", pc.into_usize(), globals.inst_info(iseq, pc));
                    pc += Inst::inst_size(iseq.iseq[pc]);
                }
            }
        }
        Ok(id)
    }

    /// stack: val
    fn gen_assign(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        lhs: Node,
    ) -> Result<(), RubyError> {
        let lhs_loc = lhs.loc();
        match lhs.kind {
            NodeKind::Ident(id) | NodeKind::LocalVar(id) => self.emit_set_local(iseq, id),
            NodeKind::Const { id, toplevel: _ } => {
                iseq.gen_push_nil();
                iseq.gen_set_const(id);
            }
            NodeKind::InstanceVar(id) => iseq.gen_set_instance_var(id),
            NodeKind::GlobalVar(id) => iseq.gen_set_global_var(id),
            NodeKind::SpecialVar(id) => self.emit_set_special_var(iseq, id, lhs_loc)?,
            NodeKind::ClassVar(id) => self.emit_set_class_var(iseq, id),
            NodeKind::Scope(parent, id) => {
                self.gen(globals, iseq, *parent, true)?;
                iseq.gen_set_const(id);
            }
            NodeKind::Send {
                receiver, method, ..
            } => {
                // attr_setter
                let name = format!("{:?}=", method);
                let assign_id = IdentId::get_id_from_string(name);
                self.gen(globals, iseq, *receiver, true)?;
                iseq.gen_sinkn(1);
                //self.loc = lhs_loc;
                self.emit_opt_send(globals, iseq, assign_id, 1, None, false, lhs_loc);
                //iseq.gen_pop();
            }
            NodeKind::Index { base, index } => {
                self.gen(globals, iseq, *base, true)?;
                let index_len = index.len();
                if index_len == 1 && !index[0].is_splat() {
                    match index[0].is_imm_u32() {
                        Some(u) => {
                            iseq.gen_topn(1);
                            iseq.push(Inst::SET_IDX_I);
                            iseq.push32(u);
                            self.save_loc(iseq, lhs_loc);
                        }
                        None => {
                            self.gen(globals, iseq, index[0].clone(), true)?;
                            iseq.gen_topn(2);
                            iseq.gen_set_array_elem();
                            self.save_loc(iseq, lhs_loc);
                        }
                    }
                    return Ok(());
                } else {
                    let has_splat = self.gen_nodes_check_splat(globals, iseq, index)?;
                    iseq.gen_topn(index_len + 1);
                    //self.loc = lhs_loc;
                    self.gen_send_with_splat(
                        globals,
                        iseq,
                        IdentId::_INDEX_ASSIGN,
                        index_len + 1,
                        has_splat,
                        false,
                        lhs_loc,
                    );
                };
            }
            _ => {
                return Err(
                    self.error_syntax(format!("Unimplemented LHS form. {:#?}", lhs), lhs.loc())
                )
            }
        }
        Ok(())
    }

    fn gen_assign_val(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        rhs: Node,
        use_value: bool,
    ) -> Result<(), RubyError> {
        self.gen(globals, iseq, rhs, true)?;
        if use_value {
            iseq.gen_dup(1);
        };
        Ok(())
    }

    fn gen_assign2(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        lhs: Node,
        rhs: Node,
        use_value: bool,
    ) -> Result<(), RubyError> {
        let lhs_loc = lhs.loc();
        match lhs.kind {
            NodeKind::Ident(id) | NodeKind::LocalVar(id) => {
                self.gen_assign_val(globals, iseq, rhs, use_value)?;
                self.emit_set_local(iseq, id);
            }
            NodeKind::Const { id, toplevel: _ } => {
                self.gen_assign_val(globals, iseq, rhs, use_value)?;
                iseq.gen_push_nil();
                iseq.gen_set_const(id);
            }
            NodeKind::InstanceVar(id) => {
                self.gen_assign_val(globals, iseq, rhs, use_value)?;
                iseq.gen_set_instance_var(id)
            }
            NodeKind::ClassVar(id) => {
                self.gen_assign_val(globals, iseq, rhs, use_value)?;
                self.emit_set_class_var(iseq, id)
            }
            NodeKind::GlobalVar(id) => {
                self.gen_assign_val(globals, iseq, rhs, use_value)?;
                iseq.gen_set_global_var(id);
            }
            NodeKind::SpecialVar(id) => {
                self.gen_assign_val(globals, iseq, rhs, use_value)?;
                self.emit_set_special_var(iseq, id, lhs_loc)?;
            }
            NodeKind::Scope(parent, id) => {
                self.gen_assign_val(globals, iseq, rhs, use_value)?;
                self.gen(globals, iseq, *parent, true)?;
                iseq.gen_set_const(id);
            }
            NodeKind::Send {
                receiver, method, ..
            } => {
                let assign_id = IdentId::get_id_from_string(format!("{:?}=", method));
                self.gen(globals, iseq, *receiver, true)?;
                self.gen(globals, iseq, rhs, true)?;
                self.emit_opt_send(globals, iseq, assign_id, 1, None, use_value, lhs_loc);
            }
            NodeKind::Index { base, index } => {
                self.gen(globals, iseq, *base, true)?;
                let index_len = index.len();
                if index_len == 1 && !index[0].is_splat() {
                    match index[0].is_imm_u32() {
                        Some(u) => {
                            self.gen_assign_val(globals, iseq, rhs, use_value)?;
                            if use_value {
                                iseq.gen_sinkn(2);
                            }
                            iseq.push(Inst::SET_IDX_I);
                            iseq.push32(u);
                            self.save_loc(iseq, lhs_loc);
                            return Ok(());
                        }
                        None => {
                            self.gen(globals, iseq, index[0].clone(), true)?;
                            self.gen_assign_val(globals, iseq, rhs, use_value)?;
                            if use_value {
                                iseq.gen_sinkn(3);
                            }
                            iseq.gen_set_array_elem();
                            self.save_loc(iseq, lhs_loc);
                            return Ok(());
                        }
                    }
                }
                let has_splat = self.gen_nodes_check_splat(globals, iseq, index)?;
                self.gen_assign_val(globals, iseq, rhs, use_value)?;
                if use_value {
                    iseq.gen_sinkn(index_len + 2);
                }
                self.gen_send_with_splat(
                    globals,
                    iseq,
                    IdentId::_INDEX_ASSIGN,
                    index_len + 1,
                    has_splat,
                    false,
                    lhs_loc,
                );
            }
            _ => {
                return Err(
                    self.error_syntax(format!("Unimplemented LHS form. {:#?}", lhs), lhs_loc)
                )
            }
        }
        Ok(())
    }

    fn gen_comp_stmt(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        mut nodes: Vec<Node>,
        use_value: bool,
    ) -> Result<(), RubyError> {
        match nodes.len() {
            0 => {
                if use_value {
                    iseq.gen_push_nil()
                }
            }
            1 => {
                self.gen(globals, iseq, nodes.remove(0), use_value)?;
            }
            _ => {
                let last = nodes.remove(nodes.len() - 1);
                for node in nodes {
                    self.gen(globals, iseq, node, false)?;
                }
                self.gen(globals, iseq, last, use_value)?;
            }
        }
        Ok(())
    }

    fn get_block(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        block: Option<Box<Node>>,
    ) -> Result<(Option<FnId>, bool), RubyError> {
        let block = match block {
            Some(block) => match block.kind {
                // Block literal ({})
                NodeKind::Lambda(BlockInfo { params, body, lvar }) => {
                    self.loop_stack.push(LoopInfo::new_top());
                    let methodref = self.gen_iseq(
                        globals,
                        params,
                        *body,
                        lvar,
                        true,
                        ContextKind::Block,
                        vec![],
                        block.loc,
                    )?;
                    self.loop_stack.pop().unwrap();
                    (Some(methodref), false)
                }
                // Block parameter (&block)
                _ => {
                    self.gen(globals, iseq, *block, true)?;
                    (None, true)
                }
            },
            None => (None, false),
        };
        Ok(block)
    }

    fn gen_default_expr(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        id: IdentId,
        default: Node,
    ) -> Result<(), RubyError> {
        self.emit_check_local(iseq, id)?;
        let src1 = iseq.gen_jmp_if_f();
        self.gen(globals, iseq, default, true)?;
        self.emit_set_local(iseq, id);
        iseq.write_disp_from_cur(src1);
        Ok(())
    }

    /// Generate `cond` + JMP_IF_FALSE.
    ///
    /// If `cond` is "lhs cmp Integer" (cmp: == or != or <= ...), generate optimized instruction. (JMP_F_EQI etc..)
    fn gen_jmp_if_false(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        cond: Node,
    ) -> Result<ISeqPos, RubyError> {
        let pos = match cond.kind {
            NodeKind::BinOp(op, lhs, rhs) if op.is_cmp_op() => {
                self.gen(globals, iseq, *lhs, true)?;
                if let Some(i) = rhs.is_imm_i32() {
                    let inst = match op {
                        BinOp::Eq => Inst::JMP_F_EQI,
                        BinOp::Ne => Inst::JMP_F_NEI,
                        BinOp::Ge => Inst::JMP_F_GEI,
                        BinOp::Gt => Inst::JMP_F_GTI,
                        BinOp::Le => Inst::JMP_F_LEI,
                        BinOp::Lt => Inst::JMP_F_LTI,
                        _ => unreachable!(),
                    };
                    iseq.push(inst);
                    iseq.push32(i as u32);
                    iseq.push32(0);
                    self.save_loc(iseq, cond.loc);
                } else {
                    self.gen(globals, iseq, *rhs, true)?;
                    let inst = match op {
                        BinOp::Eq => Inst::JMP_F_EQ,
                        BinOp::Ne => Inst::JMP_F_NE,
                        BinOp::Ge => Inst::JMP_F_GE,
                        BinOp::Gt => Inst::JMP_F_GT,
                        BinOp::Le => Inst::JMP_F_LE,
                        BinOp::Lt => Inst::JMP_F_LT,
                        _ => unreachable!(),
                    };
                    iseq.push(inst);
                    iseq.push32(0);
                    self.save_loc(iseq, cond.loc);
                }
                iseq.current()
            }
            _ => {
                self.gen(globals, iseq, cond, true)?;
                iseq.gen_jmp_if_f()
            }
        };
        Ok(pos)
    }

    fn gen(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        node: Node,
        use_value: bool,
    ) -> Result<(), RubyError> {
        self.loc = node.loc();
        let node_loc = node.loc();
        if !use_value {
            match node.kind {
                NodeKind::Nil
                | NodeKind::Bool(_)
                | NodeKind::Integer(_)
                | NodeKind::Float(_)
                | NodeKind::Imaginary(_)
                | NodeKind::String(_)
                | NodeKind::Symbol(_)
                | NodeKind::SelfValue => return Ok(()),
                _ => {}
            }
        };
        match node.kind {
            NodeKind::Nil => iseq.gen_push_nil(),
            NodeKind::Bool(b) => iseq.gen_val(Value::bool(b)),
            NodeKind::Integer(num) => iseq.gen_integer(globals, num),
            NodeKind::Bignum(num) => iseq.gen_const_val(globals, Value::bignum(num)),
            NodeKind::Float(num) => iseq.gen_float(globals, num),
            NodeKind::Imaginary(r) => iseq.gen_complex(
                globals,
                match r {
                    NReal::Integer(i) => crate::value::Real::Integer(i),
                    NReal::Bignum(b) => crate::value::Real::Bignum(b),
                    NReal::Float(f) => crate::value::Real::Float(f),
                },
            ),
            NodeKind::String(s) => iseq.gen_string(globals, &s),
            NodeKind::Symbol(id) => iseq.gen_val(Value::symbol(id)),
            NodeKind::InterporatedString(nodes) => {
                let mut c = 0;
                for node in nodes {
                    match node.kind {
                        NodeKind::String(s) => {
                            if s.len() != 0 {
                                iseq.gen_string(globals, &s);
                                c += 1;
                            }
                        }
                        NodeKind::CompStmt(nodes) => {
                            self.gen_comp_stmt(globals, iseq, nodes, true)?;
                            iseq.push(Inst::TO_S);
                            c += 1;
                        }
                        NodeKind::GlobalVar(id) => {
                            iseq.gen_get_global_var(id);
                            iseq.push(Inst::TO_S);
                            c += 1;
                        }
                        _ => unimplemented!("Illegal arguments in Nodekind::InterporatedString."),
                    }
                }
                iseq.gen_concat(c);
                if !use_value {
                    iseq.gen_pop()
                };
            }
            NodeKind::RegExp(nodes, is_const) => {
                if is_const {
                    if use_value {
                        let val = self.const_regexp(globals, nodes, node_loc)?;
                        iseq.gen_const_val(globals, val);
                    }
                } else {
                    let nodes_len = nodes.len();
                    for node in nodes {
                        match node.kind {
                            NodeKind::String(s) => {
                                iseq.gen_string(globals, &s);
                            }
                            NodeKind::CompStmt(nodes) => {
                                self.gen_comp_stmt(globals, iseq, nodes, true)?;
                                iseq.push(Inst::TO_S);
                            }
                            _ => {
                                unimplemented!("Illegal arguments in Nodekind::InterporatedString.")
                            }
                        }
                    }
                    iseq.gen_concat(nodes_len);
                    let loc = self.loc;
                    iseq.gen_create_regexp();
                    self.save_loc(iseq, loc);
                    if !use_value {
                        iseq.gen_pop()
                    };
                }
            }
            NodeKind::SelfValue => {
                iseq.gen_push_self();
            }
            NodeKind::Range {
                start,
                end,
                exclude_end,
                is_const,
            } => {
                if is_const {
                    if use_value {
                        let start = self.const_expr(globals, *start)?;
                        let end = self.const_expr(globals, *end)?;
                        let val = Value::range(start, end, exclude_end);
                        iseq.gen_const_val(globals, val);
                    }
                } else {
                    iseq.gen_val(Value::bool(exclude_end));
                    self.gen(globals, iseq, *end, true)?;
                    self.gen(globals, iseq, *start, true)?;
                    iseq.push(Inst::CREATE_RANGE);
                    self.save_loc(iseq, node_loc);
                    if !use_value {
                        iseq.gen_pop()
                    };
                }
            }
            NodeKind::Array(nodes, is_const) => {
                if is_const {
                    if use_value {
                        if nodes.len() == 0 {
                            iseq.gen_create_array(0);
                        } else {
                            let val = self.const_array(globals, nodes)?;
                            iseq.gen_const_val(globals, val);
                        }
                    }
                } else {
                    let len = nodes.len();
                    for node in nodes {
                        self.gen(globals, iseq, node, true)?;
                    }
                    iseq.gen_create_array(len);
                    if !use_value {
                        iseq.gen_pop()
                    };
                }
            }
            NodeKind::Hash(key_value, is_const) => {
                if is_const {
                    if use_value {
                        if key_value.len() == 0 {
                            iseq.gen_create_hash(0);
                        } else {
                            let val = self.const_hash(globals, key_value)?;
                            iseq.gen_const_val(globals, val);
                        }
                    }
                } else {
                    let len = key_value.len();
                    for (k, v) in key_value {
                        self.gen(globals, iseq, k, true)?;
                        self.gen(globals, iseq, v, true)?;
                    }
                    iseq.gen_create_hash(len);
                    if !use_value {
                        iseq.gen_pop()
                    };
                }
            }
            NodeKind::Ident(id) => {
                iseq.gen_push_self();
                self.emit_opt_send(globals, iseq, id, 0, None, use_value, node_loc);
            }
            NodeKind::LocalVar(id) => {
                self.emit_get_local(iseq, id)?;
                if !use_value {
                    iseq.gen_pop()
                };
            }
            NodeKind::GlobalVar(id) => {
                iseq.gen_get_global_var(id);
                if !use_value {
                    iseq.gen_pop()
                };
            }
            NodeKind::SpecialVar(id) => {
                iseq.gen_get_special_var(id);
                if !use_value {
                    iseq.gen_pop()
                };
            }
            NodeKind::Const { id, toplevel } => {
                if toplevel {
                    self.emit_get_const_top(iseq, id);
                } else {
                    self.emit_get_const(globals, iseq, id);
                };
                if !use_value {
                    iseq.gen_pop()
                };
            }
            NodeKind::Scope(parent, id) => {
                self.gen(globals, iseq, *parent, true)?;
                self.emit_get_scope(iseq, id, node.loc);
                if !use_value {
                    iseq.gen_pop()
                };
            }
            NodeKind::InstanceVar(id) => {
                iseq.gen_get_instance_var(id);
                if !use_value {
                    iseq.gen_pop()
                };
            }
            NodeKind::ClassVar(id) => {
                self.emit_get_class_var(iseq, id);
                if !use_value {
                    iseq.gen_pop()
                };
            }
            NodeKind::BinOp(op, lhs, rhs) => {
                let loc = self.loc;
                macro_rules! binop {
                    ($inst:expr) => {{
                        self.gen(globals, iseq, *lhs, true)?;
                        self.gen(globals, iseq, *rhs, true)?;
                        iseq.push($inst);
                        self.save_loc(iseq, loc);
                    }};
                }
                macro_rules! binop_imm {
                    ($inst:expr, $inst_i:expr) => {
                        if let Some(i) = rhs.is_imm_i32() {
                            self.gen(globals, iseq, *lhs, true)?;
                            iseq.push($inst_i);
                            iseq.push32(i as u32);
                            self.save_loc(iseq, loc);
                        } else {
                            binop!($inst);
                        }
                    };
                }
                match op {
                    BinOp::Add => binop_imm!(Inst::ADD, Inst::ADDI),
                    BinOp::Sub => binop_imm!(Inst::SUB, Inst::SUBI),
                    BinOp::Mul => binop!(Inst::MUL),
                    BinOp::Div => binop!(Inst::DIV),
                    BinOp::Exp => binop!(Inst::POW),
                    BinOp::Rem => binop!(Inst::REM),
                    BinOp::Shr => binop!(Inst::SHR),
                    BinOp::Shl => binop!(Inst::SHL),
                    BinOp::BitOr => binop!(Inst::BOR),
                    BinOp::BitAnd => binop!(Inst::BAND),
                    BinOp::BitXor => binop!(Inst::BXOR),
                    BinOp::Eq => binop_imm!(Inst::EQ, Inst::EQI),
                    BinOp::Ne => binop_imm!(Inst::NE, Inst::NEI),
                    BinOp::TEq => binop!(Inst::TEQ),
                    BinOp::Match => {
                        let method = IdentId::get_id("=~");
                        self.gen(globals, iseq, *lhs, true)?;
                        self.gen(globals, iseq, *rhs, true)?;
                        self.emit_opt_send(globals, iseq, method, 1, None, use_value, loc);
                        return Ok(());
                    }
                    BinOp::Ge => binop_imm!(Inst::GE, Inst::GEI),
                    BinOp::Gt => binop_imm!(Inst::GT, Inst::GTI),
                    BinOp::Le => binop_imm!(Inst::LE, Inst::LEI),
                    BinOp::Lt => binop_imm!(Inst::LT, Inst::LTI),
                    BinOp::Cmp => binop!(Inst::CMP),
                    BinOp::LAnd => {
                        self.gen(globals, iseq, *lhs, true)?;
                        if use_value {
                            iseq.gen_dup(1);
                        }
                        let src = iseq.gen_jmp_if_f();
                        if use_value {
                            iseq.gen_pop();
                        }
                        self.gen(globals, iseq, *rhs, use_value)?;
                        iseq.write_disp_from_cur(src);
                        return Ok(());
                    }
                    BinOp::LOr => {
                        self.gen(globals, iseq, *lhs, true)?;
                        if use_value {
                            iseq.gen_dup(1);
                        }
                        let src = iseq.gen_jmp_if_t();
                        if use_value {
                            iseq.gen_pop();
                        }
                        self.gen(globals, iseq, *rhs, use_value)?;
                        iseq.write_disp_from_cur(src);
                        return Ok(());
                    }
                }
                if !use_value {
                    iseq.gen_pop()
                };
            }
            NodeKind::AssignOp(op, box lhs, box rhs) => match (&op, &lhs.kind, &rhs.kind) {
                _ => {
                    let rhs = Node::new_binop(op, lhs.clone(), rhs);
                    self.gen_assign2(globals, iseq, lhs, rhs, use_value)?;
                }
            },
            NodeKind::UnOp(op, lhs) => {
                self.gen(globals, iseq, *lhs, true)?;
                match op {
                    UnOp::BitNot => {
                        iseq.push(Inst::BNOT);
                        self.save_loc(iseq, node_loc);
                    }
                    UnOp::Not => {
                        iseq.push(Inst::NOT);
                        self.save_loc(iseq, node_loc);
                    }
                    UnOp::Neg => {
                        iseq.push(Inst::NEG);
                        self.save_loc(iseq, node_loc);
                    }
                    UnOp::Pos => {}
                }
                if !use_value {
                    iseq.gen_pop()
                };
            }
            NodeKind::Index { base, mut index } => {
                let loc = node_loc;
                let num_args = index.len();
                if num_args == 1 && !index[0].is_splat() {
                    self.gen(globals, iseq, *base, true)?;
                    match index[0].is_imm_u32() {
                        Some(u) => {
                            iseq.push(Inst::GET_IDX_I);
                            iseq.push32(u);
                            self.save_loc(iseq, loc);
                            if !use_value {
                                iseq.gen_pop()
                            };
                            return Ok(());
                        }
                        None => {
                            self.gen(globals, iseq, index.remove(0), true)?;
                            self.emit_get_array_elem(iseq, loc);
                            if !use_value {
                                iseq.gen_pop()
                            };
                            return Ok(());
                        }
                    }
                }
                self.gen(globals, iseq, *base, true)?;
                let has_splat = self.gen_nodes_check_splat(globals, iseq, index)?;
                self.gen_send_with_splat(
                    globals,
                    iseq,
                    IdentId::_INDEX,
                    num_args,
                    has_splat,
                    use_value,
                    loc,
                );
            }
            NodeKind::Splat(array) => {
                self.gen(globals, iseq, *array, true)?;
                iseq.gen_splat();
                if !use_value {
                    iseq.gen_pop()
                };
            }
            NodeKind::CompStmt(nodes) => self.gen_comp_stmt(globals, iseq, nodes, use_value)?,
            NodeKind::If { cond, then_, else_ } => {
                let src1 = self.gen_jmp_if_false(globals, iseq, *cond)?;
                self.gen(globals, iseq, *then_, use_value)?;
                if use_value {
                    let src2 = iseq.gen_jmp();
                    iseq.write_disp_from_cur(src1);
                    self.gen(globals, iseq, *else_, true)?;
                    iseq.write_disp_from_cur(src2);
                } else if else_.is_empty() {
                    iseq.write_disp_from_cur(src1);
                } else {
                    let src2 = iseq.gen_jmp();
                    iseq.write_disp_from_cur(src1);
                    self.gen(globals, iseq, *else_, false)?;
                    iseq.write_disp_from_cur(src2);
                }
            }
            NodeKind::For {
                param,
                iter,
                body: BlockInfo { params, body, lvar },
            } => {
                let loc = iter.loc;
                self.loop_stack.push(LoopInfo::new_top());
                let block = self.gen_iseq(
                    globals,
                    params.to_vec(),
                    *body,
                    lvar,
                    true,
                    ContextKind::Block,
                    param,
                    node.loc,
                )?;
                self.loop_stack.pop().unwrap();
                self.gen(globals, iseq, *iter, true)?;
                self.emit_opt_send(globals, iseq, IdentId::EACH, 0, Some(block), use_value, loc);
            }
            NodeKind::While {
                cond,
                body,
                cond_op,
            } => {
                self.loop_stack.push(LoopInfo::new_loop());

                let loop_start = iseq.current();
                let src = if cond_op {
                    self.gen_jmp_if_false(globals, iseq, *cond)?
                } else {
                    self.gen(globals, iseq, *cond, true)?;
                    iseq.gen_jmp_if_t()
                };
                self.gen(globals, iseq, *body, false)?;
                iseq.gen_jmp_back(loop_start);
                iseq.write_disp_from_cur(src);

                if use_value {
                    iseq.gen_push_nil();
                }
                let src = iseq.gen_jmp();
                for p in self.loop_stack.pop().unwrap().escape {
                    match p.kind {
                        EscapeKind::Break => {
                            iseq.write_disp_from_cur(p.pos);
                        }
                        EscapeKind::Next => iseq.write_disp(p.pos, loop_start),
                    }
                }
                if !use_value {
                    iseq.gen_pop();
                }

                iseq.write_disp_from_cur(src);
            }
            NodeKind::Begin {
                body,
                rescue,
                else_,
                ensure,
            } => {
                let mut ensure_dest = vec![];
                let body_start = iseq.current();
                self.push_jump_dest(ensure.is_some());
                self.gen(globals, iseq, *body, use_value)?;
                let jump_dest = self.pop_jump_dest();
                let body_end = iseq.current();
                let mut dest = None;
                let mut prev = None;

                if !rescue.is_empty() {
                    let else_dest = iseq.gen_jmp();
                    // Rescue clauses.
                    for RescueEntry {
                        exception_list,
                        assign,
                        body,
                    } in rescue
                    {
                        if dest.is_none() {
                            dest = Some(iseq.current())
                        };
                        if let Some(prev) = prev {
                            iseq.write_disp_from_cur(prev);
                        }
                        iseq.gen_dup(1);
                        if !exception_list.is_empty() {
                            let len = exception_list.len();
                            for ex in exception_list {
                                self.gen(globals, iseq, ex, true)?;
                            }
                            iseq.push(Inst::RESCUE);
                            iseq.push32(len as u32);
                        } else {
                            // When no error_type were given, use "StandardError".
                            self.emit_get_const_top(iseq, IdentId::get_id("StandardError"));
                            iseq.push(Inst::RESCUE);
                            iseq.push32(1);
                        }
                        prev = Some(iseq.gen_jmp_if_f());
                        // assign the error value.
                        match assign {
                            Some(assign) => self.gen_assign(globals, iseq, *assign)?,
                            None => iseq.gen_pop(),
                        }
                        self.gen(globals, iseq, *body, use_value)?;
                        ensure_dest.push(iseq.gen_jmp());
                    }
                    // When no rescue clause were matched
                    if let Some(prev) = prev {
                        iseq.write_disp_from_cur(prev);
                    }
                    if let Some(box ensure) = ensure.clone() {
                        self.gen(globals, iseq, ensure, false)?;
                    }
                    self.save_loc(iseq, node.loc);
                    iseq.push(Inst::THROW);
                    //iseq.gen_pop();
                    //if use_value {
                    //    iseq.gen_push_nil()
                    //};
                    //ensure_dest.push(iseq.gen_jmp());
                    iseq.write_disp_from_cur(else_dest);
                }
                // If no exception occured, execute else clause.
                if let Some(else_) = else_ {
                    if use_value {
                        iseq.gen_pop()
                    };
                    self.gen(globals, iseq, *else_, use_value)?
                };
                if !jump_dest.is_empty() {
                    // Ensure clause for exception return path.
                    let ensure_label = iseq.gen_jmp();
                    for dest in &jump_dest.return_entries {
                        iseq.write_disp_from_cur(*dest);
                    }
                    if !jump_dest.return_entries.is_empty() {
                        if let Some(box ensure) = ensure.clone() {
                            self.gen(globals, iseq, ensure, false)?;
                        }
                        self.save_loc(iseq, node.loc);
                        iseq.gen_return();
                    }
                    for dest in &jump_dest.mreturn_entries {
                        iseq.write_disp_from_cur(*dest);
                    }
                    if !jump_dest.mreturn_entries.is_empty() {
                        if let Some(box ensure) = ensure.clone() {
                            self.gen(globals, iseq, ensure, false)?;
                        }
                        self.save_loc(iseq, node.loc);
                        iseq.gen_method_return();
                    }
                    iseq.write_disp_from_cur(ensure_label);
                }
                // Ensure clause for noraml return path.
                for src in ensure_dest {
                    iseq.write_disp_from_cur(src);
                }
                if let Some(dest) = dest {
                    self.push_ex_rescue(body_start, body_end, dest);
                }
                // Ensure clause does not return value.
                if let Some(ensure) = ensure {
                    self.gen(globals, iseq, *ensure, false)?;
                }
            }
            NodeKind::Case { cond, when_, else_ } => {
                let mut end = vec![];
                match cond {
                    Some(cond) => {
                        self.gen(globals, iseq, *cond, true)?;
                        // This flag is set when when label of all branches are Integer.
                        let mut opt_flag = true;
                        // This flag is set when when label of all branches are Integer, Symbol, or String.
                        let mut opt_flag2 = true;
                        let mut opt_min = i64::max_value();
                        let mut opt_max = i64::min_value();
                        for branch in &when_ {
                            for elem in &branch.when {
                                match elem.kind {
                                    NodeKind::Integer(i) => {
                                        if i < opt_min {
                                            opt_min = i
                                        };
                                        if i > opt_max {
                                            opt_max = i
                                        };
                                    }
                                    NodeKind::Bignum(_)
                                    | NodeKind::Float(_)
                                    | NodeKind::Symbol(_)
                                    | NodeKind::String(_)
                                    | NodeKind::Bool(_)
                                    | NodeKind::Nil => opt_flag2 = false,
                                    _ => {
                                        opt_flag = false;
                                        break;
                                    }
                                }
                            }
                            if !opt_flag {
                                break;
                            }
                        }
                        if opt_flag && opt_flag2 {
                            //eprintln!("{} {}", opt_min, opt_max);
                            let map_id = globals.case_dispatch2.new_entry();
                            let start = iseq.gen_opt_case2(map_id);
                            self.save_cur_loc(iseq);
                            let mut map = FxHashMap::default();
                            for branch in when_ {
                                let disp = start - iseq.current();
                                for elem in &branch.when {
                                    let i = match &elem.kind {
                                        NodeKind::Integer(i) => *i,
                                        _ => unreachable!(),
                                    };
                                    map.insert(i, disp);
                                }
                                self.gen(globals, iseq, *branch.body, use_value)?;
                                end.push(iseq.gen_jmp());
                            }
                            let default_disp = start - iseq.current();
                            let case_map = globals.case_dispatch2.get_mut_entry(map_id);
                            let mut vec = vec![default_disp; (opt_max - opt_min + 1) as usize];
                            map.iter()
                                .for_each(|(i, disp)| vec[(*i - opt_min) as usize] = *disp);
                            *case_map = (opt_min, opt_max, vec);

                            iseq.write_disp_from_cur(start);
                        } else if opt_flag {
                            let map_id = globals.case_dispatch.new_entry();
                            let start = iseq.gen_opt_case(map_id);
                            self.save_cur_loc(iseq);
                            for branch in when_ {
                                let map = globals.case_dispatch.get_mut_entry(map_id);
                                let disp = start - iseq.current();
                                for elem in &branch.when {
                                    let k = match &elem.kind {
                                        NodeKind::Integer(i) => Value::integer(*i),
                                        NodeKind::Bignum(n) => Value::bignum(n.clone()),
                                        NodeKind::Float(f) => Value::float(*f),
                                        NodeKind::Symbol(sym) => Value::symbol(*sym),
                                        NodeKind::String(str) => Value::string(str),
                                        NodeKind::Bool(b) => Value::bool(*b),
                                        NodeKind::Nil => Value::nil(),
                                        _ => unreachable!(),
                                    };
                                    map.insert(HashKey(k), disp);
                                }
                                self.gen(globals, iseq, *branch.body, use_value)?;
                                end.push(iseq.gen_jmp());
                            }
                            iseq.write_disp_from_cur(start);
                        } else {
                            let mut next = None;
                            for branch in when_ {
                                let mut jmp_dest = vec![];
                                match next {
                                    Some(next) => {
                                        iseq.write_disp_from_cur(next);
                                    }
                                    None => {}
                                }
                                for elem in branch.when {
                                    iseq.gen_dup(1);
                                    let loc = elem.loc;
                                    self.gen(globals, iseq, elem, true)?;
                                    iseq.gen_sinkn(1);
                                    iseq.push(Inst::TEQ);
                                    self.save_loc(iseq, loc);
                                    jmp_dest.push(iseq.gen_jmp_if_t());
                                }
                                next = Some(iseq.gen_jmp());
                                for dest in jmp_dest {
                                    iseq.write_disp_from_cur(dest);
                                }
                                iseq.gen_pop();
                                self.gen(globals, iseq, *branch.body, use_value)?;
                                end.push(iseq.gen_jmp());
                            }
                            match next {
                                Some(next) => {
                                    iseq.write_disp_from_cur(next);
                                }
                                None => {}
                            }
                            iseq.gen_pop();
                        }
                        self.gen(globals, iseq, *else_, use_value)?;
                        for dest in end {
                            iseq.write_disp_from_cur(dest);
                        }
                    }
                    None => {
                        for branch in when_ {
                            let mut next = vec![];
                            for elem in branch.when {
                                let loc = elem.loc;
                                self.gen(globals, iseq, elem, true)?;
                                self.save_loc(iseq, loc);
                                next.push(iseq.gen_jmp_if_f());
                            }
                            //next = Some(iseq.gen_jmp());
                            self.gen(globals, iseq, *branch.body, use_value)?;
                            end.push(iseq.gen_jmp());
                            for dest in next {
                                iseq.write_disp_from_cur(dest);
                            }
                        }
                        self.gen(globals, iseq, *else_, use_value)?;
                        for dest in end {
                            iseq.write_disp_from_cur(dest);
                        }
                    }
                }
            }
            NodeKind::MulAssign(mut mlhs, mut mrhs) => {
                let lhs_len = mlhs.len();
                if lhs_len == 1 && mrhs.len() == 1 {
                    self.gen_assign2(globals, iseq, mlhs.remove(0), mrhs.remove(0), use_value)?;
                } else if mrhs.len() == 1 {
                    self.gen(globals, iseq, mrhs.remove(0), true)?;
                    if use_value {
                        iseq.gen_dup(1);
                    };
                    iseq.gen_take(lhs_len);

                    for lhs in mlhs.into_iter().rev() {
                        self.gen_assign(globals, iseq, lhs)?;
                    }
                } else {
                    // no splat. mlhs.len != 1
                    let mrhs_len = mrhs.len();
                    if !use_value {
                        for (i, node) in mrhs.into_iter().enumerate() {
                            self.gen(globals, iseq, node, i < mlhs.len())?;
                        }
                        if mlhs.len() > mrhs_len {
                            for _ in mrhs_len..mlhs.len() {
                                iseq.gen_push_nil();
                            }
                        }
                        for lhs in mlhs.into_iter().rev() {
                            self.gen_assign(globals, iseq, lhs)?;
                        }
                    } else {
                        let len = std::cmp::max(mlhs.len(), mrhs.len());
                        for rhs in mrhs {
                            self.gen(globals, iseq, rhs, true)?;
                        }
                        for _ in mrhs_len..len {
                            iseq.gen_push_nil();
                        }
                        iseq.gen_dup(len);
                        for _ in mlhs.len()..len {
                            iseq.gen_pop();
                        }
                        for lhs in mlhs.into_iter().rev() {
                            self.gen_assign(globals, iseq, lhs)?;
                        }
                        iseq.gen_create_array(len);
                    }
                }
            }
            NodeKind::Command(content) => {
                iseq.gen_push_self();
                self.gen(globals, iseq, *content, true)?;
                self.emit_opt_send(
                    globals,
                    iseq,
                    IdentId::get_id("`"),
                    1,
                    None,
                    use_value,
                    node_loc,
                );
            }
            NodeKind::Send {
                receiver,
                method,
                arglist,
                safe_nav,
            } => self.gen_send(
                globals, iseq, *receiver, method, arglist, safe_nav, use_value, node_loc,
            )?,
            NodeKind::Yield(arglist) => {
                let len = arglist.args.len();
                for arg in arglist.args {
                    self.gen(globals, iseq, arg, true)?;
                }
                self.emit_yield(iseq, len);
                if !use_value {
                    iseq.gen_pop();
                };
            }
            NodeKind::Super(arglist) => {
                iseq.gen_push_self();
                match arglist {
                    None => {
                        self.emit_super(iseq, 0, None, true);
                    }
                    Some(arglist) => {
                        let len = arglist.args.len();
                        for arg in arglist.args {
                            self.gen(globals, iseq, arg, true)?;
                        }
                        let (block, flag) = self.get_block(globals, iseq, arglist.block)?;
                        if flag {
                            unreachable!("Block param is not supported for super.")
                        }
                        self.emit_super(iseq, len, block, false);
                    }
                }

                if !use_value {
                    iseq.gen_pop();
                };
            }
            NodeKind::MethodDef(id, params, body, lvar) => {
                let method = self.gen_iseq(
                    globals,
                    params,
                    *body,
                    lvar,
                    true,
                    ContextKind::Method(Some(id)),
                    vec![],
                    node.loc,
                )?;
                iseq.push(Inst::DEF_METHOD);
                iseq.push32(id.into());
                iseq.push32(method.into());
                if use_value {
                    iseq.gen_val(Value::symbol(id));
                };
            }
            NodeKind::SingletonMethodDef(singleton, id, params, body, lvar) => {
                let method = self.gen_iseq(
                    globals,
                    params.into(),
                    *body,
                    lvar,
                    true,
                    ContextKind::Method(Some(id)),
                    vec![],
                    node.loc,
                )?;
                self.gen(globals, iseq, *singleton, true)?;
                iseq.push(Inst::DEF_SMETHOD);
                iseq.push32((id).into());
                iseq.push32(method.into());
                if use_value {
                    iseq.gen_val(Value::symbol(id));
                };
            }
            NodeKind::ClassDef {
                base,
                id,
                superclass,
                body,
                is_module,
                lvar,
            } => {
                let method = self.gen_iseq(
                    globals,
                    vec![],
                    *body,
                    lvar,
                    true,
                    ContextKind::Class(id),
                    vec![],
                    node.loc,
                )?;
                self.gen(globals, iseq, *superclass, true)?;
                self.gen(globals, iseq, *base, true)?;
                iseq.push(Inst::DEF_CLASS);
                iseq.push(if is_module { 1 } else { 0 });
                iseq.push32(id.into());
                iseq.push32(method.into());
                self.save_loc(iseq, node_loc);
                if !use_value {
                    iseq.gen_pop();
                };
            }
            NodeKind::SingletonClassDef {
                singleton,
                body,
                lvar,
            } => {
                let method = self.gen_iseq(
                    globals,
                    vec![],
                    *body,
                    lvar,
                    true,
                    ContextKind::Class(IdentId::get_id("Singleton")),
                    vec![],
                    node.loc,
                )?;
                self.gen(globals, iseq, *singleton, true)?;
                iseq.push(Inst::DEF_SCLASS);
                iseq.push32(method.into());
                self.save_loc(iseq, node_loc);
                if !use_value {
                    iseq.gen_pop();
                };
            }
            NodeKind::Return(val) => {
                self.gen(globals, iseq, *val, true)?;
                // Call ensure clauses.
                // Note ensure clause does not return any value.
                let is_block = self.context().kind == ContextKind::Block;
                if let Some(ex) = self.context_mut().jump_dest.last_mut() {
                    if ex.has_ensure {
                        let dest = iseq.gen_jmp();
                        if is_block {
                            ex.mreturn_entries.push(dest)
                        } else {
                            ex.return_entries.push(dest)
                        };
                        return Ok(());
                    }
                }
                self.save_loc(iseq, node.loc);
                if is_block {
                    iseq.gen_method_return();
                } else {
                    iseq.gen_return();
                }
            }
            NodeKind::Break(val) => {
                if self.loop_stack.last().unwrap().state == LoopState::Top {
                    //In the case of outer of loops
                    match self.context().kind {
                        ContextKind::Block => {
                            self.gen(globals, iseq, *val, true)?;
                            self.save_loc(iseq, node_loc);
                            iseq.gen_break();
                        }
                        ContextKind::Method(_) | ContextKind::Class(_) => {
                            return Err(
                                self.error_syntax("Invalid break.", node_loc.merge(self.loc))
                            );
                        }
                        ContextKind::Eval => {
                            return Err(self.error_syntax(
                                "Can't escape from eval with break.",
                                node_loc.merge(self.loc),
                            ));
                        }
                    }
                } else {
                    //In the case of inner of loops
                    self.gen(globals, iseq, *val, true)?;
                    let src = iseq.gen_jmp();
                    let x = self.loop_stack.last_mut().unwrap();
                    x.escape.push(EscapeInfo::new(src, EscapeKind::Break));
                }
            }
            NodeKind::Next(val) => {
                if self.loop_stack.last().unwrap().state == LoopState::Top {
                    //In the case of outer of loops
                    match self.context_stack.last().unwrap().kind {
                        ContextKind::Block => {
                            self.gen(globals, iseq, *val, true)?;
                            iseq.gen_return();
                        }
                        ContextKind::Method(_) | ContextKind::Class(_) => {
                            return Err(
                                self.error_syntax("Invalid next.", node_loc.merge(self.loc))
                            );
                        }
                        ContextKind::Eval => {
                            return Err(self.error_syntax(
                                "Can't escape from eval with next.",
                                node_loc.merge(self.loc),
                            ));
                        }
                    }
                } else {
                    //In the case of inner of loops
                    self.gen(globals, iseq, *val, use_value)?;
                    let src = iseq.gen_jmp();
                    let x = self.loop_stack.last_mut().unwrap();
                    x.escape.push(EscapeInfo::new(src, EscapeKind::Next));
                }
            }
            NodeKind::Lambda(BlockInfo { params, body, lvar }) => {
                self.loop_stack.push(LoopInfo::new_top());
                let method = self.gen_iseq(
                    globals,
                    params,
                    *body,
                    lvar,
                    true,
                    ContextKind::Block,
                    vec![],
                    node.loc,
                )?;
                self.loop_stack.pop().unwrap();
                iseq.push(Inst::CREATE_PROC);
                iseq.push32(method.into());
                if !use_value {
                    iseq.gen_pop()
                };
            }
            NodeKind::Defined(content) => {
                self.gen_defined(globals, iseq, *content)?;
                if !use_value {
                    iseq.gen_pop()
                };
            }
            NodeKind::AliasMethod(new, old) => {
                iseq.gen_push_self();
                self.gen(globals, iseq, *new, true)?;
                self.gen(globals, iseq, *old, true)?;
                self.emit_opt_send(
                    globals,
                    iseq,
                    IdentId::_ALIAS_METHOD,
                    2,
                    None,
                    use_value,
                    node_loc,
                );
            } //_ => unreachable!("Codegen: Unimplemented syntax. {:?}", node.kind),
        };
        Ok(())
    }
}

// Emit ISeq.
impl Codegen {
    fn emit_get_array_elem(&mut self, iseq: &mut ISeq, loc: Loc) {
        iseq.push(Inst::GET_INDEX);
        self.save_loc(iseq, loc);
    }

    fn emit_yield(&mut self, iseq: &mut ISeq, args_num: usize) {
        iseq.push(Inst::YIELD);
        iseq.push32(args_num as u32);
        self.save_cur_loc(iseq);
    }

    fn emit_super(&mut self, iseq: &mut ISeq, arg_num: usize, block: Option<FnId>, no_arg: bool) {
        iseq.push(Inst::SUPER);
        iseq.push16(arg_num as u32 as u16);
        iseq.push_method(block);
        iseq.push8(if no_arg { 1 } else { 0 });
        self.save_cur_loc(iseq);
    }

    fn emit_set_local(&mut self, iseq: &mut ISeq, id: IdentId) {
        let (outer, lvar_id) = match self.get_local_var(id) {
            Some((outer, id)) => (outer, id),
            None => unreachable!("CodeGen: Illegal LvarId in gen_set_local(). id:{:?}", id),
        };
        if outer == 0 {
            iseq.push(Inst::SET_LOCAL);
            iseq.push32(lvar_id.as_u32());
        } else {
            iseq.push(Inst::SET_DYNLOCAL);
            iseq.push32(lvar_id.as_u32());
            iseq.push32(outer);
        }
    }

    fn emit_get_local(&mut self, iseq: &mut ISeq, id: IdentId) -> Result<(), RubyError> {
        let (outer, lvar_id) = match self.get_local_var(id) {
            Some((outer, id)) => (outer, id),
            None => return Err(RubyError::name("undefined local variable.")),
        };
        if outer == 0 {
            iseq.push(Inst::GET_LOCAL);
            iseq.push32(lvar_id.as_u32());
        } else {
            iseq.push(Inst::GET_DYNLOCAL);
            iseq.push32(lvar_id.as_u32());
            iseq.push32(outer);
        }
        Ok(())
    }

    fn emit_check_local(&mut self, iseq: &mut ISeq, id: IdentId) -> Result<(), RubyError> {
        let (outer, lvar_id) = match self.get_local_var(id) {
            Some((outer, id)) => (outer, id),
            None => return Err(RubyError::name("undefined local variable.")),
        };
        iseq.push(Inst::CHECK_LOCAL);
        iseq.push32(lvar_id.as_u32());
        iseq.push32(outer);
        Ok(())
    }

    fn get_local_var(&mut self, id: IdentId) -> Option<(u32, LvarId)> {
        let mut idx = 0u32;
        for (i, context) in self.context_stack.iter().rev().enumerate() {
            match context.lvar_info.get_lvarid(id) {
                Some(id) => return Some((i as u32, id)),
                None => idx = i as u32,
            };
            if context.kind.is_method() {
                return None;
            }
        }
        let mut ep = self.extern_context?;
        loop {
            if let Some(id) = ep.iseq().lvar.table.get_lvarid(id) {
                return Some((idx as u32, id));
            };
            ep = ep.outer()?;
            idx += 1;
        }
    }

    fn emit_get_class_var(&mut self, iseq: &mut ISeq, id: IdentId) {
        iseq.push(Inst::GET_CVAR);
        iseq.push32(id.into());
        self.save_cur_loc(iseq);
    }

    fn emit_set_class_var(&mut self, iseq: &mut ISeq, id: IdentId) {
        iseq.push(Inst::SET_CVAR);
        iseq.push32(id.into());
        self.save_cur_loc(iseq);
    }

    fn emit_set_special_var(
        &mut self,
        iseq: &mut ISeq,
        id: usize,
        loc: Loc,
    ) -> Result<(), RubyError> {
        if id == 0 || id == 1 || id >= 100 {
            return Err(self.error_syntax("Can't set variable.", loc));
        }
        iseq.push(Inst::SET_SVAR);
        iseq.push32(id as u32);
        self.save_cur_loc(iseq);
        Ok(())
    }

    fn emit_get_const(&mut self, globals: &mut Globals, iseq: &mut ISeq, id: IdentId) {
        iseq.push(Inst::GET_CONST);
        iseq.push32(id.into());
        iseq.push32(globals.add_const_cache_entry());
        self.save_cur_loc(iseq);
    }

    fn emit_get_const_top(&mut self, iseq: &mut ISeq, id: IdentId) {
        iseq.push(Inst::GET_CONST_TOP);
        iseq.push32(id.into());
        self.save_cur_loc(iseq);
    }

    fn emit_get_scope(&mut self, iseq: &mut ISeq, id: IdentId, loc: Loc) {
        iseq.push(Inst::GET_SCOPE);
        iseq.push32(id.into());
        self.save_loc(iseq, loc);
    }
}

// Utility methods for Codegen
impl Codegen {
    fn save_loc(&mut self, iseq: &mut ISeq, loc: Loc) {
        self.context_stack
            .last_mut()
            .unwrap()
            .iseq_sourcemap
            .push((iseq.current(), loc));
    }

    fn save_cur_loc(&mut self, iseq: &mut ISeq) {
        self.save_loc(iseq, self.loc)
    }
}

impl Codegen {
    fn error_syntax(&self, msg: impl Into<String>, loc: Loc) -> RubyError {
        RubyError::new_parse_err(
            ParseErrKind::SyntaxError(msg.into()),
            self.source_info.clone(),
            loc,
        )
    }
}

impl Codegen {
    /// Evaluate constant expression and return the value.
    fn const_expr(&self, globals: &mut Globals, node: Node) -> Result<Value, RubyError> {
        let loc = node.loc();
        match node.kind {
            NodeKind::Bool(b) => Ok(Value::bool(b)),
            NodeKind::Integer(i) => Ok(Value::integer(i)),
            NodeKind::Bignum(n) => Ok(Value::bignum(n)),
            NodeKind::Float(f) => Ok(Value::float(f)),
            NodeKind::Nil => Ok(Value::nil()),
            NodeKind::Symbol(s) => Ok(Value::symbol(s)),
            NodeKind::String(s) => Ok(Value::string(s)),
            NodeKind::Hash(key_value, true) => self.const_hash(globals, key_value),
            NodeKind::Array(nodes, true) => self.const_array(globals, nodes),
            NodeKind::RegExp(nodes, true) => self.const_regexp(globals, nodes, loc),
            _ => unreachable!("const_expr(): not supported. {:?}", node.kind),
        }
    }

    fn const_hash(&self, globals: &mut Globals, key_value: Vec<(Node, Node)>) -> VMResult {
        let mut map = FxIndexMap::default();
        for (k, v) in key_value {
            map.insert(
                HashKey(self.const_expr(globals, k)?),
                self.const_expr(globals, v)?,
            );
        }
        Ok(Value::hash_from_map(map))
    }

    fn const_array(&self, globals: &mut Globals, nodes: Vec<Node>) -> VMResult {
        let mut ary = vec![];
        for n in nodes {
            ary.push(self.const_expr(globals, n)?)
        }
        Ok(Value::array_from(ary))
    }

    fn const_regexp(&self, globals: &mut Globals, nodes: Vec<Node>, loc: Loc) -> VMResult {
        let mut string = String::new();
        for node in nodes {
            match &node.kind {
                NodeKind::String(s) => string += s,
                _ => unreachable!(),
            }
        }
        match string.pop().unwrap() {
            'i' => string.insert_str(0, "(?mi)"),
            'm' => string.insert_str(0, "(?ms)"),
            'x' => string.insert_str(0, "(?mx)"),
            'o' => string.insert_str(0, "(?mo)"),
            '-' => string.insert_str(0, "(?m)"),
            _ => return Err(self.error_syntax("Illegal internal regexp expression.", loc)),
        };
        let re = match RegexpInfo::from_string(globals, &string) {
            Ok(re) => re,
            Err(err) => {
                return Err(self.error_syntax(
                    format!(
                        "Invalid string for a regular expression. {} {}",
                        string, err
                    ),
                    loc,
                ))
            }
        };
        Ok(Value::regexp(re))
    }
}

/// Infomation for loops.
#[derive(Debug, Clone, PartialEq)]
struct LoopInfo {
    state: LoopState,
    escape: Vec<EscapeInfo>,
}

impl LoopInfo {
    fn new_top() -> Self {
        LoopInfo {
            state: LoopState::Top,
            escape: vec![],
        }
    }

    fn new_loop() -> Self {
        LoopInfo {
            state: LoopState::Loop,
            escape: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum LoopState {
    /// in a loop
    Loop,
    /// top level (outside a loop)
    Top,
}

#[derive(Debug, Clone, PartialEq)]
struct EscapeInfo {
    pos: ISeqPos,
    kind: EscapeKind,
}

impl EscapeInfo {
    fn new(pos: ISeqPos, kind: EscapeKind) -> Self {
        EscapeInfo { pos, kind }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum EscapeKind {
    Break,
    Next,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    lvar_info: LvarTable,
    pub iseq_sourcemap: Vec<(ISeqPos, Loc)>,
    /// Unsolved destinations of local jumps.
    jump_dest: Vec<LocalJumpDest>,
    exception_table: Vec<ExceptionEntry>,
    kind: ContextKind,
}

impl Context {
    fn new() -> Self {
        Context {
            lvar_info: LvarTable::new(),
            iseq_sourcemap: vec![],
            jump_dest: vec![],
            exception_table: vec![],
            kind: ContextKind::Eval,
        }
    }

    fn from(lvar_info: LvarTable, kind: ContextKind) -> Self {
        Context {
            lvar_info,
            iseq_sourcemap: vec![],
            jump_dest: vec![],
            exception_table: vec![],
            kind,
        }
    }
}

/// Local jump destination entry.
/// i.e. return and method_return
#[derive(Debug, Clone, PartialEq)]
struct LocalJumpDest {
    has_ensure: bool,
    return_entries: Vec<ISeqPos>,
    mreturn_entries: Vec<ISeqPos>,
}

impl LocalJumpDest {
    fn new(has_ensure: bool) -> Self {
        LocalJumpDest {
            has_ensure,
            return_entries: vec![],
            mreturn_entries: vec![],
        }
    }

    fn is_empty(&self) -> bool {
        self.return_entries.is_empty() && self.mreturn_entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::*;

    #[test]
    fn codegen_usevalue() {
        let program = r#"
        a = 100
        true; 4; 3.2; "and"; :foo; self;
        1..3; [1,0]; {s:0}; a; $foo; nil; Object; @boo; false
        "#;
        assert_script(program);
    }

    #[test]
    fn codegen_invalid_break1() {
        assert_error(r#"eval("break")"#);
    }

    #[test]
    fn codegen_invalid_break2() {
        assert_error(r#"break"#);
    }

    #[test]
    fn codegen_invalid_break3() {
        assert_error("def foo; break; end");
    }

    #[test]
    fn codegen_invalid_next1() {
        assert_error(r#"eval("next")"#);
    }

    #[test]
    fn codegen_invalid_next2() {
        assert_error(r#"next"#);
    }

    #[test]
    fn codegen_invalid_next3() {
        assert_error("def foo; next; end");
    }

    #[test]
    fn codegen_error1() {
        assert_error(r#"a"#);
    }

    #[test]
    fn codegen_error2() {
        assert_error(r#"a + 3 = 100"#);
    }
}
