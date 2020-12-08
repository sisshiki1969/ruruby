use super::vm_inst::*;
use crate::error::{ParseErrKind, RubyError};
use crate::parse::node::{BinOp, FormalParam, Node, NodeKind, ParamKind, UnOp};
use crate::parse::parser::RescueEntry;
use crate::*;

#[derive(Debug, Clone)]
pub struct Codegen {
    // Codegen State
    method_stack: Vec<MethodRef>,
    loop_stack: Vec<LoopInfo>,
    context_stack: Vec<Context>,
    extern_context: Option<ContextRef>,
    pub loc: Loc,
    pub source_info: SourceInfoRef,
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
    lvar_info: FxHashMap<IdentId, LvarId>,
    pub iseq_sourcemap: Vec<(ISeqPos, Loc)>,
    /// Unsolved destinations of local jumps.
    jump_dest: Vec<LocalJumpDest>,
    exception_table: Vec<ExceptionEntry>,
    kind: ContextKind,
}

#[derive(Debug, Clone, PartialEq)]
struct LocalJumpDest {
    return_entries: Vec<ISeqPos>,
    mreturn_entries: Vec<ISeqPos>,
}

impl LocalJumpDest {
    fn new() -> Self {
        LocalJumpDest {
            return_entries: vec![],
            mreturn_entries: vec![],
        }
    }

    fn is_empty(&self) -> bool {
        self.return_entries.is_empty() && self.mreturn_entries.is_empty()
    }
}

#[derive(Clone, PartialEq)]
pub struct ExceptionEntry {
    /// start position in ISeq.
    pub start: ISeqPos,
    /// end position in ISeq.
    pub end: ISeqPos,
    pub dest: ISeqPos,
}

use std::fmt;

impl fmt::Debug for ExceptionEntry {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_fmt(format_args!(
            "ExceptionEntry ({}, {}) => {}",
            self.start.to_usize(),
            self.end.to_usize(),
            self.dest.to_usize(),
        ))
    }
}

impl ExceptionEntry {
    fn new(start: usize, end: usize, dest: ISeqPos) -> Self {
        Self {
            start: ISeqPos::from(start),
            end: ISeqPos::from(end),
            dest,
        }
    }

    pub fn include(&self, pc: usize) -> bool {
        self.start.to_usize() <= pc && pc < self.end.to_usize()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContextKind {
    Method,
    Block,
    Eval,
}

impl Context {
    fn new() -> Self {
        Context {
            lvar_info: FxHashMap::default(),
            iseq_sourcemap: vec![],
            jump_dest: vec![],
            exception_table: vec![],
            kind: ContextKind::Eval,
        }
    }

    fn from(lvar_info: FxHashMap<IdentId, LvarId>, kind: ContextKind) -> Self {
        Context {
            lvar_info,
            iseq_sourcemap: vec![],
            jump_dest: vec![],
            exception_table: vec![],
            kind,
        }
    }
}

impl Codegen {
    pub fn new(source_info: SourceInfoRef) -> Self {
        Codegen {
            method_stack: vec![],
            context_stack: vec![Context::new()],
            extern_context: None,
            loop_stack: vec![LoopInfo::new_top()],
            loc: Loc(0, 0),
            source_info,
        }
    }

    pub fn context(&self) -> &Context {
        self.context_stack.last().unwrap()
    }

    pub fn context_mut(&mut self) -> &mut Context {
        self.context_stack.last_mut().unwrap()
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

    pub fn set_external_context(&mut self, context: ContextRef) {
        self.extern_context = Some(context);
    }
}

impl Codegen {
    fn gen_get_array_elem(&mut self, iseq: &mut ISeq, loc: Loc) {
        self.save_loc(iseq, loc);
        iseq.push(Inst::GET_INDEX);
    }

    fn gen_yield(&mut self, iseq: &mut ISeq, args_num: usize) {
        self.save_cur_loc(iseq);
        iseq.push(Inst::YIELD);
        iseq.push32(args_num as u32);
    }

    fn gen_set_local(&mut self, iseq: &mut ISeq, id: IdentId) {
        let (outer, lvar_id) = match self.get_local_var(id) {
            Some((outer, id)) => (outer, id),
            None => panic!(format!(
                "CodeGen: Illegal LvarId in gen_set_local(). id:{:?}",
                id
            )),
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

    fn gen_get_local(&mut self, iseq: &mut ISeq, id: IdentId) -> Result<(), RubyError> {
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

    fn gen_check_local(&mut self, iseq: &mut ISeq, id: IdentId) -> Result<(), RubyError> {
        let (outer, lvar_id) = match self.get_local_var(id) {
            Some((outer, id)) => (outer, id),
            None => return Err(RubyError::name("undefined local variable.")),
        };
        iseq.push(Inst::CHECK_LOCAL);
        iseq.push32(lvar_id.as_u32());
        iseq.push32(outer);
        Ok(())
    }

    fn gen_lvar_addi(
        &mut self,
        iseq: &mut ISeq,
        id: IdentId,
        val: i32,
        use_value: bool,
    ) -> Result<(), RubyError> {
        let (outer, lvar_id) = match self.get_local_var(id) {
            Some((outer, id)) => (outer, id),
            None => return Err(RubyError::name("undefined local variable.")),
        };
        if outer == 0 {
            let loc = self.loc;
            self.save_loc(iseq, loc);
            iseq.push(Inst::LVAR_ADDI);
            iseq.push32(lvar_id.as_u32());
            iseq.push32(val as u32);
            if use_value {
                self.gen_get_local(iseq, id)?;
            }
        } else {
            iseq.push(Inst::GET_DYNLOCAL);
            iseq.push32(lvar_id.as_u32());
            iseq.push32(outer);
            let loc = self.loc;
            self.save_loc(iseq, loc);
            iseq.push(Inst::ADDI);
            iseq.push32(val as u32);
            if use_value {
                self.gen_dup(iseq, 1);
            }
            iseq.push(Inst::SET_DYNLOCAL);
            iseq.push32(lvar_id.as_u32());
            iseq.push32(outer);
        }

        Ok(())
    }

    fn get_local_var(&mut self, id: IdentId) -> Option<(u32, LvarId)> {
        let mut idx = 0u32;
        for (i, context) in self.context_stack.iter().rev().enumerate() {
            match context.lvar_info.get(&id) {
                Some(id) => return Some((i as u32, *id)),
                None => idx = i as u32,
            };
            if context.kind == ContextKind::Method {
                return None;
            }
        }
        let mut ctx = match self.extern_context {
            Some(ctx) => ctx,
            None => return None,
        };
        loop {
            match ctx.iseq_ref.unwrap().lvar.get(&id) {
                Some(id) => return Some((idx as u32, *id)),
                None => {}
            };
            ctx = match ctx.outer {
                Some(ctx) => ctx,
                None => return None,
            };
            idx += 1;
        }
    }

    fn gen_get_class_var(&mut self, iseq: &mut ISeq, id: IdentId) {
        self.save_cur_loc(iseq);
        iseq.push(Inst::GET_CVAR);
        iseq.push32(id.into());
    }

    fn gen_set_class_var(&mut self, iseq: &mut ISeq, id: IdentId) {
        self.save_cur_loc(iseq);
        iseq.push(Inst::SET_CVAR);
        iseq.push32(id.into());
    }

    fn gen_get_const(&mut self, globals: &mut Globals, iseq: &mut ISeq, id: IdentId) {
        self.save_cur_loc(iseq);
        iseq.push(Inst::GET_CONST);
        iseq.push32(id.into());
        iseq.push32(globals.add_const_cache_entry());
    }

    fn gen_get_const_top(&mut self, iseq: &mut ISeq, id: IdentId) {
        self.save_cur_loc(iseq);
        iseq.push(Inst::GET_CONST_TOP);
        iseq.push32(id.into());
    }

    fn gen_get_scope(&mut self, iseq: &mut ISeq, id: IdentId, loc: Loc) {
        self.save_loc(iseq, loc);
        iseq.push(Inst::GET_SCOPE);
        iseq.push32(id.into());
    }

    fn gen_send(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        method: IdentId,
        args_num: usize,
        kw_rest_num: usize,
        flag: u8,
        block: Option<MethodRef>,
    ) {
        self.save_cur_loc(iseq);
        iseq.push(Inst::SEND);
        iseq.push32(method.into());
        iseq.push16(args_num as u32 as u16);
        iseq.push8(kw_rest_num as u32 as u16 as u8);
        iseq.push8(flag);
        iseq.push64(match block {
            Some(block) => block.id(),
            None => 0,
        });
        iseq.push32(globals.add_inline_cache_entry());
    }

    fn gen_send_self(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        method: IdentId,
        args_num: usize,
        kw_rest_num: usize,
        flag: u8,
        block: Option<MethodRef>,
    ) {
        self.save_cur_loc(iseq);
        iseq.push(Inst::SEND_SELF);
        iseq.push32(method.into());
        iseq.push16(args_num as u32 as u16);
        iseq.push8(kw_rest_num as u32 as u16 as u8);
        iseq.push8(flag);
        iseq.push64(match block {
            Some(block) => block.id(),
            None => 0,
        });
        iseq.push32(globals.add_inline_cache_entry());
    }

    fn gen_opt_send(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        method: IdentId,
        args_num: usize,
        block: Option<MethodRef>,
        use_value: bool,
    ) {
        self.save_cur_loc(iseq);
        if block.is_none() {
            if use_value {
                iseq.push(Inst::OPT_SEND);
            } else {
                iseq.push(Inst::OPT_NSEND)
            }
        } else {
            if use_value {
                iseq.push(Inst::OPT_SEND_BLK);
            } else {
                iseq.push(Inst::OPT_NSEND_BLK);
            }
        }

        iseq.push32(method.into());
        iseq.push16(args_num as u32 as u16);
        if let Some(block) = block {
            iseq.push64(block.id());
        }
        iseq.push32(globals.add_inline_cache_entry());
    }

    fn gen_opt_send_self(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        method: IdentId,
        args_num: usize,
        block: Option<MethodRef>,
        use_value: bool,
    ) {
        self.save_cur_loc(iseq);
        if block.is_none() {
            if use_value {
                iseq.push(Inst::OPT_SEND_SELF);
            } else {
                iseq.push(Inst::OPT_NSEND_SELF)
            }
        } else {
            if use_value {
                iseq.push(Inst::OPT_SEND_SELF_BLK);
            } else {
                iseq.push(Inst::OPT_NSEND_SELF_BLK);
            }
        }
        iseq.push32(method.into());
        iseq.push16(args_num as u32 as u16);
        if let Some(block) = block {
            iseq.push64(block.id());
        }
        iseq.push32(globals.add_inline_cache_entry());
    }

    /// stack: val
    fn gen_assign(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        lhs: &Node,
    ) -> Result<(), RubyError> {
        match &lhs.kind {
            NodeKind::Ident(id) | NodeKind::LocalVar(id) => self.gen_set_local(iseq, *id),
            NodeKind::Const { id, toplevel: _ } => {
                iseq.gen_push_nil();
                iseq.gen_set_const(*id);
            }
            NodeKind::InstanceVar(id) => iseq.gen_set_instance_var(*id),
            NodeKind::GlobalVar(id) => iseq.gen_set_global_var(*id),
            NodeKind::ClassVar(id) => self.gen_set_class_var(iseq, *id),
            NodeKind::Scope(parent, id) => {
                self.gen(globals, iseq, parent, true)?;
                iseq.gen_set_const(*id);
            }
            NodeKind::Send {
                receiver, method, ..
            } => {
                let name = format!("{:?}=", method);
                let assign_id = IdentId::get_id(name);
                self.gen(globals, iseq, &receiver, true)?;
                self.loc = lhs.loc();
                self.gen_opt_send(globals, iseq, assign_id, 1, None, false);
                //self.gen_pop(iseq);
            }
            NodeKind::Index { base, index } => {
                self.gen(globals, iseq, base, true)?;
                if index.len() == 1 && !index[0].is_splat() {
                    match index[0].is_imm_u32() {
                        Some(u) => {
                            self.save_loc(iseq, lhs.loc());
                            iseq.push(Inst::SET_IDX_I);
                            iseq.push32(u);
                            return Ok(());
                        }
                        None => {
                            self.gen(globals, iseq, &index[0], true)?;
                            self.gen_topn(iseq, 2);
                            self.save_loc(iseq, lhs.loc());
                            iseq.gen_set_array_elem();
                            return Ok(());
                        }
                    }
                }
                for i in index {
                    self.gen(globals, iseq, i, true)?;
                }
                self.gen_topn(iseq, index.len() + 1);
                self.gen_topn(iseq, index.len() + 1);
                self.loc = lhs.loc();
                self.gen_send(
                    globals,
                    iseq,
                    IdentId::_INDEX_ASSIGN,
                    index.len() + 1,
                    0,
                    0,
                    None,
                );
                self.gen_pop(iseq);
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
        rhs: &Node,
        use_value: bool,
    ) -> Result<(), RubyError> {
        self.gen(globals, iseq, rhs, true)?;
        if use_value {
            self.gen_dup(iseq, 1);
        };
        Ok(())
    }

    fn gen_assign2(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        lhs: &Node,
        rhs: &Node,
        use_value: bool,
    ) -> Result<(), RubyError> {
        match &lhs.kind {
            NodeKind::Ident(id) | NodeKind::LocalVar(id) => {
                self.gen_assign_val(globals, iseq, rhs, use_value)?;
                self.gen_set_local(iseq, *id);
            }
            NodeKind::Const { id, toplevel: _ } => {
                self.gen_assign_val(globals, iseq, rhs, use_value)?;
                iseq.gen_push_nil();
                iseq.gen_set_const(*id);
            }
            NodeKind::InstanceVar(id) => {
                self.gen_assign_val(globals, iseq, rhs, use_value)?;
                iseq.gen_set_instance_var(*id)
            }
            NodeKind::ClassVar(id) => {
                self.gen_assign_val(globals, iseq, rhs, use_value)?;
                self.gen_set_class_var(iseq, *id)
            }
            NodeKind::GlobalVar(id) => {
                self.gen_assign_val(globals, iseq, rhs, use_value)?;
                iseq.gen_set_global_var(*id);
            }
            NodeKind::Scope(parent, id) => {
                self.gen_assign_val(globals, iseq, rhs, use_value)?;
                self.gen(globals, iseq, parent, true)?;
                iseq.gen_set_const(*id);
            }
            NodeKind::Send {
                receiver, method, ..
            } => {
                let name = format!("{:?}=", method);
                let assign_id = IdentId::get_id(name);
                self.gen_assign_val(globals, iseq, rhs, use_value)?;
                self.gen(globals, iseq, &receiver, true)?;
                self.loc = lhs.loc();
                self.gen_opt_send(globals, iseq, assign_id, 1, None, false);
                //self.gen_pop(iseq);
            }
            NodeKind::Index { base, index } => {
                self.gen(globals, iseq, base, true)?;
                if index.len() == 1 && !index[0].is_splat() {
                    match index[0].is_imm_u32() {
                        Some(u) => {
                            self.gen_assign_val(globals, iseq, rhs, use_value)?;
                            if use_value {
                                self.gen_topn(iseq, 2);
                            } else {
                                self.gen_topn(iseq, 1);
                            }
                            self.save_loc(iseq, lhs.loc());
                            iseq.push(Inst::SET_IDX_I);
                            iseq.push32(u);
                            return Ok(());
                        }
                        None => {
                            self.gen(globals, iseq, &index[0], true)?;
                            self.gen_assign_val(globals, iseq, rhs, use_value)?;
                            if use_value {
                                self.gen_sinkn(iseq, 3);
                            }
                            self.save_loc(iseq, lhs.loc());
                            iseq.gen_set_array_elem();
                            return Ok(());
                        }
                    }
                }
                for i in index {
                    self.gen(globals, iseq, i, true)?;
                }
                self.gen_assign_val(globals, iseq, rhs, use_value)?;
                if use_value {
                    self.gen_sinkn(iseq, index.len() + 2);
                }
                self.gen_topn(iseq, index.len() + 1);
                self.loc = lhs.loc();
                self.gen_send(
                    globals,
                    iseq,
                    IdentId::_INDEX_ASSIGN,
                    index.len() + 1,
                    0,
                    0,
                    None,
                );
                self.gen_pop(iseq);
            }
            _ => {
                return Err(
                    self.error_syntax(format!("Unimplemented LHS form. {:#?}", lhs), lhs.loc())
                )
            }
        }
        Ok(())
    }

    fn gen_pop(&mut self, iseq: &mut ISeq) {
        iseq.push(Inst::POP);
    }

    fn gen_dup(&mut self, iseq: &mut ISeq, len: usize) {
        iseq.push(Inst::DUP);
        iseq.push32(len as u32);
    }

    fn gen_sinkn(&mut self, iseq: &mut ISeq, len: usize) {
        iseq.push(Inst::SINKN);
        iseq.push32(len as u32);
    }

    fn gen_topn(&mut self, iseq: &mut ISeq, len: usize) {
        iseq.push(Inst::TOPN);
        iseq.push32(len as u32);
    }

    fn gen_take(&mut self, iseq: &mut ISeq, len: usize) {
        iseq.push(Inst::TAKE);
        iseq.push32(len as u32);
    }

    fn gen_concat(&mut self, iseq: &mut ISeq, len: usize) {
        iseq.push(Inst::CONCAT_STRING);
        iseq.push32(len as u32);
    }

    fn gen_comp_stmt(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        nodes: &[Node],
        use_value: bool,
    ) -> Result<(), RubyError> {
        match nodes.len() {
            0 => {
                if use_value {
                    iseq.gen_push_nil()
                }
            }
            1 => {
                self.gen(globals, iseq, &nodes[0], use_value)?;
            }
            _ => {
                for i in 0..nodes.len() - 1 {
                    self.gen(globals, iseq, &nodes[i], false)?;
                }
                self.gen(globals, iseq, &nodes[nodes.len() - 1], use_value)?;
            }
        }
        Ok(())
    }

    /// Generate ISeq.
    pub fn gen_iseq(
        &mut self,
        globals: &mut Globals,
        param_list: &[FormalParam],
        node: &Node,
        lvar_collector: &LvarCollector,
        use_value: bool,
        kind: ContextKind,
        name: Option<IdentId>,
    ) -> Result<MethodRef, RubyError> {
        let mut methodref = MethodRef::new(MethodInfo::default());
        let is_block = !(kind == ContextKind::Method);
        if !is_block {
            self.method_stack.push(methodref)
        }
        let save_loc = self.loc;
        let mut params = ISeqParams::default();
        let mut iseq = ISeq::new();

        self.context_stack
            .push(Context::from(lvar_collector.clone_table(), kind));
        for (lvar_id, param) in param_list.iter().enumerate() {
            match &param.kind {
                ParamKind::Param(id) => {
                    params.param_ident.push(*id);
                    params.req += 1;
                }
                ParamKind::Post(id) => {
                    params.param_ident.push(*id);
                    params.post += 1;
                }
                ParamKind::Optional(id, default) => {
                    params.param_ident.push(*id);
                    params.opt += 1;
                    self.gen_default_expr(globals, &mut iseq, *id, default)?;
                }
                ParamKind::Rest(id) => {
                    params.param_ident.push(*id);
                    params.rest = true;
                }
                ParamKind::Keyword(id, default) => {
                    params.param_ident.push(*id);
                    params.keyword.insert(*id, LvarId::from_usize(lvar_id));
                    if let Some(default) = default {
                        self.gen_default_expr(globals, &mut iseq, *id, default)?
                    }
                }
                ParamKind::KWRest(id) => {
                    params.param_ident.push(*id);
                    params.kwrest = true;
                }
                ParamKind::Block(id) => {
                    params.param_ident.push(*id);
                    params.block = true;
                }
            }
        }

        self.gen(globals, &mut iseq, node, use_value)?;
        let context = self.context_stack.pop().unwrap();
        let iseq_sourcemap = context.iseq_sourcemap;
        let exception_table = context.exception_table;
        iseq.gen_return();
        self.loc = save_loc;

        let info = MethodInfo::RubyFunc {
            iseq: ISeqRef::new(ISeqInfo::new(
                methodref,
                name,
                params,
                iseq,
                lvar_collector.clone(),
                exception_table,
                iseq_sourcemap,
                self.source_info,
                match kind {
                    ContextKind::Block => ISeqKind::Block,
                    ContextKind::Eval => ISeqKind::Other,
                    ContextKind::Method => {
                        if name.is_some() {
                            ISeqKind::Method(name.unwrap())
                        } else {
                            ISeqKind::Other
                        }
                    }
                },
            )),
        };

        if !is_block {
            self.method_stack.pop();
        }
        *methodref = info;
        #[cfg(feature = "emit-iseq")]
        {
            let iseq = methodref.as_iseq();
            println!("-----------------------------------------");
            let method_name = match iseq.name {
                Some(id) => format!("{:?}", id),
                None => "<unnamed>".to_string(),
            };
            println!(
                "{} {:?} opt_flag:{:?}",
                method_name, methodref, iseq.opt_flag
            );
            print!("local var: ");
            for (k, v) in iseq.lvar.table() {
                print!("{}:{:?} ", v.as_u32(), k);
            }
            println!("");
            println!("block: {:?}", iseq.lvar.block());
            let mut pc = 0;
            while pc < iseq.iseq.len() {
                println!("  {:05x} {}", pc, Inst::inst_info(globals, iseq, pc));
                pc += Inst::inst_size(iseq.iseq[pc]);
            }
        }
        Ok(methodref)
    }

    fn gen_default_expr(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        id: IdentId,
        default: &Node,
    ) -> Result<(), RubyError> {
        self.gen_check_local(iseq, id)?;
        let src1 = iseq.gen_jmp_if_f();
        self.gen(globals, iseq, default, true)?;
        self.gen_set_local(iseq, id);
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
        cond: &Node,
    ) -> Result<ISeqPos, RubyError> {
        let pos = match &cond.kind {
            NodeKind::BinOp(op, lhs, rhs) if op.is_cmp_op() => match rhs.is_imm_i32() {
                Some(i) => {
                    self.gen(globals, iseq, lhs, true)?;
                    let inst = match op {
                        BinOp::Eq => Inst::JMP_F_EQI,
                        BinOp::Ne => Inst::JMP_F_NEI,
                        BinOp::Ge => Inst::JMP_F_GEI,
                        BinOp::Gt => Inst::JMP_F_GTI,
                        BinOp::Le => Inst::JMP_F_LEI,
                        BinOp::Lt => Inst::JMP_F_LTI,
                        _ => unreachable!(),
                    };
                    self.save_loc(iseq, cond.loc);
                    iseq.push(inst);
                    iseq.push32(i as u32);
                    iseq.push32(0);
                    iseq.current()
                }
                None => {
                    self.gen(globals, iseq, lhs, true)?;
                    self.gen(globals, iseq, rhs, true)?;
                    self.save_loc(iseq, cond.loc);
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
                    iseq.current()
                }
            },
            _ => {
                self.gen(globals, iseq, &cond, true)?;
                iseq.gen_jmp_if_f()
            }
        };
        Ok(pos)
    }

    pub fn gen(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        node: &Node,
        use_value: bool,
    ) -> Result<(), RubyError> {
        self.loc = node.loc();
        if !use_value {
            match &node.kind {
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
        match &node.kind {
            NodeKind::Nil => iseq.gen_push_nil(),
            NodeKind::Bool(b) => {
                if *b {
                    iseq.push(Inst::PUSH_TRUE)
                } else {
                    iseq.push(Inst::PUSH_FALSE)
                }
            }
            NodeKind::Integer(num) => {
                iseq.gen_fixnum(*num);
            }
            NodeKind::Float(num) => {
                iseq.push(Inst::PUSH_FLONUM);
                iseq.push64(f64::to_bits(*num));
            }
            NodeKind::Imaginary(r) => {
                iseq.gen_complex(globals, *r);
            }
            NodeKind::String(s) => {
                iseq.gen_string(globals, s);
            }
            NodeKind::Symbol(id) => {
                iseq.gen_symbol(*id);
            }
            NodeKind::InterporatedString(nodes) => {
                let mut c = 0;
                for node in nodes {
                    match &node.kind {
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
                            iseq.gen_get_global_var(*id);
                            iseq.push(Inst::TO_S);
                            c += 1;
                        }
                        _ => unimplemented!("Illegal arguments in Nodekind::InterporatedString."),
                    }
                }
                self.gen_concat(iseq, c);
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::RegExp(nodes, is_const) => {
                if *is_const {
                    if use_value {
                        let val = self.const_expr(globals, node)?;
                        let id = globals.const_values.insert(val);
                        iseq.gen_const_val(id);
                    }
                } else {
                    for node in nodes {
                        match &node.kind {
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
                    self.gen_concat(iseq, nodes.len());
                    let loc = self.loc;
                    self.save_loc(iseq, loc);
                    iseq.gen_create_regexp();
                    if !use_value {
                        self.gen_pop(iseq)
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
            } => {
                let loc = node.loc();
                if *exclude_end {
                    iseq.push(Inst::PUSH_TRUE);
                } else {
                    iseq.push(Inst::PUSH_FALSE)
                };
                self.gen(globals, iseq, end, true)?;
                self.gen(globals, iseq, start, true)?;
                self.save_loc(iseq, loc);
                iseq.push(Inst::CREATE_RANGE);
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::Array(nodes, is_const) => {
                if *is_const {
                    if use_value {
                        if nodes.len() == 0 {
                            iseq.gen_create_array(0);
                        } else {
                            let val = self.const_expr(globals, node)?;
                            let id = globals.const_values.insert(val);
                            iseq.gen_const_val(id);
                        }
                    }
                } else {
                    let len = nodes.len();
                    for node in nodes {
                        self.gen(globals, iseq, node, true)?;
                    }
                    iseq.gen_create_array(len);
                    if !use_value {
                        self.gen_pop(iseq)
                    };
                }
            }
            NodeKind::Hash(key_value, is_const) => {
                if *is_const {
                    if use_value {
                        if key_value.len() == 0 {
                            iseq.gen_create_hash(0);
                        } else {
                            let val = self.const_expr(globals, node)?;
                            let id = globals.const_values.insert(val);
                            iseq.gen_const_val(id);
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
                        self.gen_pop(iseq)
                    };
                }
            }
            NodeKind::Ident(id) => {
                self.gen_send_self(globals, iseq, *id, 0, 0, 0, None);
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::LocalVar(id) => {
                self.gen_get_local(iseq, *id)?;
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::GlobalVar(id) => {
                iseq.gen_get_global_var(*id);
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::Const { id, toplevel } => {
                if *toplevel {
                    self.gen_get_const_top(iseq, *id);
                } else {
                    self.gen_get_const(globals, iseq, *id);
                };
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::Scope(parent, id) => {
                self.gen(globals, iseq, parent, true)?;
                self.gen_get_scope(iseq, *id, node.loc);
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::InstanceVar(id) => {
                iseq.gen_get_instance_var(*id);
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::ClassVar(id) => {
                self.gen_get_class_var(iseq, *id);
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::BinOp(op, lhs, rhs) => {
                let loc = self.loc;
                macro_rules! binop {
                    ($inst:expr) => {{
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq, loc);
                        iseq.push($inst);
                    }};
                }
                macro_rules! binop_imm {
                    ($inst:expr, $inst_i:expr) => {
                        match &rhs.kind {
                            NodeKind::Integer(i) if *i as i32 as i64 == *i => {
                                self.gen(globals, iseq, lhs, true)?;
                                self.save_loc(iseq, loc);
                                iseq.push($inst_i);
                                iseq.push32(*i as i32 as u32);
                            }
                            _ => {
                                self.gen(globals, iseq, lhs, true)?;
                                self.gen(globals, iseq, rhs, true)?;
                                self.save_loc(iseq, loc);
                                iseq.push($inst);
                            }
                        }
                    };
                }
                match op {
                    BinOp::Add => match (&lhs.kind, &rhs.kind) {
                        (_, NodeKind::Integer(i)) if *i as i32 as i64 == *i => {
                            self.gen(globals, iseq, lhs, true)?;
                            self.save_loc(iseq, loc);
                            iseq.push(Inst::ADDI);
                            iseq.push32(*i as u32);
                        }
                        _ => {
                            self.gen(globals, iseq, lhs, true)?;
                            self.gen(globals, iseq, rhs, true)?;
                            self.save_loc(iseq, loc);
                            iseq.push(Inst::ADD);
                        }
                    },
                    BinOp::Sub => match rhs.kind {
                        NodeKind::Integer(i) if i as i32 as i64 == i => {
                            self.gen(globals, iseq, lhs, true)?;
                            self.save_loc(iseq, loc);
                            iseq.gen_subi(i as i32);
                        }
                        _ => {
                            self.gen(globals, iseq, lhs, true)?;
                            self.gen(globals, iseq, rhs, true)?;
                            self.save_loc(iseq, loc);
                            iseq.push(Inst::SUB);
                        }
                    },
                    BinOp::Mul => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq, loc);
                        iseq.push(Inst::MUL);
                    }
                    BinOp::Div => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq, loc);
                        iseq.push(Inst::DIV);
                    }
                    BinOp::Exp => binop!(Inst::POW),
                    BinOp::Rem => binop!(Inst::REM),
                    BinOp::Shr => binop!(Inst::SHR),
                    BinOp::Shl => binop!(Inst::SHL),
                    BinOp::BitOr => binop_imm!(Inst::BOR, Inst::B_ORI),
                    BinOp::BitAnd => binop_imm!(Inst::BAND, Inst::B_ANDI),
                    BinOp::BitXor => binop!(Inst::BXOR),
                    BinOp::Eq => binop_imm!(Inst::EQ, Inst::EQI),
                    BinOp::Ne => binop_imm!(Inst::NE, Inst::NEI),
                    BinOp::TEq => binop!(Inst::TEQ),
                    BinOp::Match => {
                        let method = IdentId::get_id("=~");
                        self.gen(globals, iseq, rhs, true)?;
                        self.gen(globals, iseq, lhs, true)?;
                        self.loc = loc;
                        self.gen_opt_send(globals, iseq, method, 1, None, use_value);
                        return Ok(());
                    }
                    BinOp::Ge => binop_imm!(Inst::GE, Inst::GEI),
                    BinOp::Gt => binop_imm!(Inst::GT, Inst::GTI),
                    BinOp::Le => binop_imm!(Inst::LE, Inst::LEI),
                    BinOp::Lt => binop_imm!(Inst::LT, Inst::LTI),
                    BinOp::Cmp => binop!(Inst::CMP),
                    BinOp::LAnd => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen_dup(iseq, 1);
                        let src = iseq.gen_jmp_if_f();
                        self.gen_pop(iseq);
                        self.gen(globals, iseq, rhs, true)?;
                        iseq.write_disp_from_cur(src);
                    }
                    BinOp::LOr => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen_dup(iseq, 1);
                        let src = iseq.gen_jmp_if_t();
                        self.gen_pop(iseq);
                        self.gen(globals, iseq, rhs, true)?;
                        iseq.write_disp_from_cur(src);
                    }
                }
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::UnOp(op, lhs) => {
                match op {
                    UnOp::BitNot => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.save_loc(iseq, node.loc());
                        iseq.push(Inst::BNOT);
                    }
                    UnOp::Not => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.save_loc(iseq, node.loc());
                        iseq.push(Inst::NOT);
                    }
                    UnOp::Neg => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.save_loc(iseq, node.loc());
                        iseq.push(Inst::NEG);
                    }
                    UnOp::Pos => {
                        self.gen(globals, iseq, lhs, true)?;
                    }
                }
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::Index { base, index } => {
                let loc = node.loc();
                let num_args = index.len();
                if num_args == 1 && !index[0].is_splat() {
                    self.gen(globals, iseq, base, true)?;
                    match index[0].is_imm_u32() {
                        Some(u) => {
                            self.save_loc(iseq, loc);
                            iseq.push(Inst::GET_IDX_I);
                            iseq.push32(u);
                            if !use_value {
                                self.gen_pop(iseq)
                            };
                            return Ok(());
                        }
                        None => {
                            self.gen(globals, iseq, &index[0], true)?;
                            self.gen_get_array_elem(iseq, loc);
                            if !use_value {
                                self.gen_pop(iseq)
                            };
                            return Ok(());
                        }
                    }
                }
                for i in index {
                    self.gen(globals, iseq, i, true)?;
                }
                self.gen(globals, iseq, base, true)?;
                self.loc = loc;
                self.gen_send(globals, iseq, IdentId::_INDEX, num_args, 0, 0, None);
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::Splat(array) => {
                self.gen(globals, iseq, array, true)?;
                iseq.gen_splat();
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::CompStmt(nodes) => self.gen_comp_stmt(globals, iseq, nodes, use_value)?,
            NodeKind::If { cond, then_, else_ } => {
                let src1 = self.gen_jmp_if_false(globals, iseq, cond)?;
                self.gen(globals, iseq, &then_, use_value)?;
                if use_value {
                    let src2 = iseq.gen_jmp();
                    iseq.write_disp_from_cur(src1);
                    self.gen(globals, iseq, &else_, true)?;
                    iseq.write_disp_from_cur(src2);
                } else {
                    if else_.is_empty() {
                        iseq.write_disp_from_cur(src1);
                    } else {
                        let src2 = iseq.gen_jmp();
                        iseq.write_disp_from_cur(src1);
                        self.gen(globals, iseq, &else_, false)?;
                        iseq.write_disp_from_cur(src2);
                    }
                }
            }
            NodeKind::For { param, iter, body } => {
                let _id = match param.kind {
                    NodeKind::Ident(id) | NodeKind::LocalVar(id) => id,
                    _ => return Err(self.error_syntax("Expected an identifier.", param.loc())),
                };

                let block = match &body.kind {
                    NodeKind::Proc { params, body, lvar } => {
                        self.loop_stack.push(LoopInfo::new_top());
                        let methodref = self.gen_iseq(
                            globals,
                            params,
                            body,
                            lvar,
                            true,
                            ContextKind::Block,
                            None,
                        )?;
                        self.loop_stack.pop().unwrap();
                        methodref
                    }
                    // Block parameter (&block)
                    _ => unreachable!(),
                };
                self.gen(globals, iseq, iter, true)?;
                self.gen_send(globals, iseq, IdentId::EACH, 0, 0, 0, Some(block));
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::While {
                cond,
                body,
                cond_op,
            } => {
                self.loop_stack.push(LoopInfo::new_loop());

                let loop_start = iseq.current();
                let src = if *cond_op {
                    self.gen_jmp_if_false(globals, iseq, cond)?
                } else {
                    self.gen(globals, iseq, cond, true)?;
                    iseq.gen_jmp_if_t()
                };
                self.gen(globals, iseq, body, false)?;
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
                    self.gen_pop(iseq);
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
                let body_start = iseq.len();
                self.context_mut().jump_dest.push(LocalJumpDest::new());
                self.gen(globals, iseq, body, use_value)?;
                let body_end = iseq.len();
                let mut dest = None;
                let mut prev = None;
                let else_dest = iseq.gen_jmp();

                if !rescue.is_empty() {
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
                        if !exception_list.is_empty() {
                            self.gen_dup(iseq, 1);
                            let ex = &exception_list[0];
                            self.gen(globals, iseq, ex, true)?;
                            self.gen_sinkn(iseq, 1);
                            self.save_loc(iseq, ex.loc);
                            iseq.push(Inst::TEQ);
                            prev = Some(iseq.gen_jmp_if_f());
                        } else {
                            prev = None;
                        }
                        // assign the error value.
                        match assign {
                            Some(assign) => self.gen_assign(globals, iseq, assign)?,
                            None => self.gen_pop(iseq),
                        }
                        self.gen(globals, iseq, body, use_value)?;
                        ensure_dest.push(iseq.gen_jmp());
                    }
                    if let Some(prev) = prev {
                        iseq.write_disp_from_cur(prev);
                    }
                    self.gen_pop(iseq);
                    ensure_dest.push(iseq.gen_jmp());
                }
                // Else clause.
                iseq.write_disp_from_cur(else_dest);
                if let Some(else_) = else_ {
                    if use_value {
                        self.gen_pop(iseq)
                    };
                    self.gen(globals, iseq, else_, use_value)?
                };
                if !self.context().jump_dest.last().unwrap().is_empty() {
                    let ensure_label = iseq.gen_jmp();
                    let jump_dest = self.context_mut().jump_dest.pop().unwrap();
                    for dest in &jump_dest.return_entries {
                        iseq.write_disp_from_cur(*dest);
                    }
                    if !jump_dest.return_entries.is_empty() {
                        if let Some(ensure) = ensure {
                            self.gen(globals, iseq, ensure, false)?;
                        }
                        iseq.gen_return();
                    }
                    for dest in &jump_dest.mreturn_entries {
                        iseq.write_disp_from_cur(*dest);
                    }
                    if !jump_dest.mreturn_entries.is_empty() {
                        if let Some(ensure) = ensure {
                            self.gen(globals, iseq, ensure, false)?;
                        }
                        iseq.gen_method_return();
                    }
                    iseq.write_disp_from_cur(ensure_label);
                }
                // Ensure clause.
                for src in ensure_dest {
                    iseq.write_disp_from_cur(src);
                }
                if let Some(dest) = dest {
                    self.context_mut()
                        .exception_table
                        .push(ExceptionEntry::new(body_start, body_end, dest));
                }
                // Ensure clause does not return value.
                if let Some(ensure) = ensure {
                    self.gen(globals, iseq, ensure, false)?;
                }
            }
            NodeKind::Case { cond, when_, else_ } => {
                let mut end = vec![];
                match cond {
                    Some(cond) => {
                        self.gen(globals, iseq, cond, true)?;
                        let mut opt_flag = true;
                        for branch in when_ {
                            for elem in &branch.when {
                                match elem.kind {
                                    NodeKind::Integer(_) => (),
                                    NodeKind::Symbol(_) => (),
                                    NodeKind::String(_) => (),
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
                        if opt_flag {
                            let map_id = globals.case_dispatch.new_entry();
                            self.save_cur_loc(iseq);
                            let start = iseq.gen_opt_case(map_id);
                            for branch in when_ {
                                let map = globals.case_dispatch.get_mut_entry(map_id);
                                let disp = start.disp(iseq.current()) as i32;
                                for elem in &branch.when {
                                    let k = match &elem.kind {
                                        NodeKind::Integer(i) => Value::integer(*i),
                                        NodeKind::Symbol(sym) => Value::symbol(*sym),
                                        NodeKind::String(str) => Value::string(str),
                                        _ => unreachable!(),
                                    };
                                    map.insert(k, disp);
                                }
                                self.gen(globals, iseq, &branch.body, use_value)?;
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
                                for elem in &branch.when {
                                    self.gen_dup(iseq, 1);
                                    self.gen(globals, iseq, elem, true)?;
                                    self.gen_sinkn(iseq, 1);
                                    self.save_loc(iseq, elem.loc);
                                    iseq.push(Inst::TEQ);
                                    jmp_dest.push(iseq.gen_jmp_if_t());
                                }
                                next = Some(iseq.gen_jmp());
                                for dest in jmp_dest {
                                    iseq.write_disp_from_cur(dest);
                                }
                                self.gen_pop(iseq);
                                self.gen(globals, iseq, &branch.body, use_value)?;
                                end.push(iseq.gen_jmp());
                            }
                            match next {
                                Some(next) => {
                                    iseq.write_disp_from_cur(next);
                                }
                                None => {}
                            }
                            self.gen_pop(iseq);
                        }
                        self.gen(globals, iseq, &else_, use_value)?;
                        for dest in end {
                            iseq.write_disp_from_cur(dest);
                        }
                    }
                    None => {
                        for branch in when_ {
                            let mut next = vec![];
                            for elem in &branch.when {
                                self.gen(globals, iseq, elem, true)?;
                                self.save_loc(iseq, elem.loc);
                                next.push(iseq.gen_jmp_if_f());
                            }
                            //next = Some(iseq.gen_jmp());
                            self.gen(globals, iseq, &branch.body, use_value)?;
                            end.push(iseq.gen_jmp());
                            for dest in next {
                                iseq.write_disp_from_cur(dest);
                            }
                        }
                        self.gen(globals, iseq, &else_, use_value)?;
                        for dest in end {
                            iseq.write_disp_from_cur(dest);
                        }
                    }
                }
            }
            NodeKind::MulAssign(mlhs, mrhs) => {
                let lhs_len = mlhs.len();
                if lhs_len == 1 && mrhs.len() == 1 {
                    match (&mlhs[0].kind, &mrhs[0].kind) {
                        (
                            NodeKind::InstanceVar(id1),
                            NodeKind::BinOp(
                                BinOp::Add,
                                box Node {
                                    kind: NodeKind::InstanceVar(id2),
                                    ..
                                },
                                box Node {
                                    kind: NodeKind::Integer(i),
                                    ..
                                },
                            ),
                        ) if *id1 == *id2 && *i as i32 as i64 == *i => {
                            let loc = mlhs[0].loc.merge(mrhs[0].loc);
                            self.save_loc(iseq, loc);
                            iseq.gen_ivar_addi(*id1, *i as i32 as u32, use_value);
                        }
                        (
                            NodeKind::LocalVar(id1),
                            NodeKind::BinOp(
                                BinOp::Add,
                                box Node {
                                    kind: NodeKind::LocalVar(id2),
                                    ..
                                },
                                box Node {
                                    kind: NodeKind::Integer(i),
                                    ..
                                },
                            ),
                        ) if *id1 == *id2 && *i as i32 as i64 == *i => {
                            let loc = mlhs[0].loc.merge(mrhs[0].loc);
                            self.save_loc(iseq, loc);
                            self.gen_lvar_addi(iseq, *id1, *i as i32, use_value)?;
                        }
                        _ => {
                            self.gen_assign2(globals, iseq, &mlhs[0], &mrhs[0], use_value)?;
                        }
                    }
                } else if mrhs.len() == 1 {
                    self.gen(globals, iseq, &mrhs[0], true)?;
                    if use_value {
                        self.gen_dup(iseq, 1);
                    };
                    self.gen_take(iseq, lhs_len);

                    for lhs in mlhs.iter().rev() {
                        self.gen_assign(globals, iseq, lhs)?;
                    }
                } else {
                    // no splat. mlhs.len != 1
                    if !use_value {
                        for (i, node) in mrhs.iter().enumerate() {
                            self.gen(globals, iseq, node, i < mlhs.len())?;
                        }
                        if mlhs.len() > mrhs.len() {
                            for _ in 0..(mlhs.len() - mrhs.len()) {
                                iseq.gen_push_nil();
                            }
                        }
                        for lhs in mlhs.iter().rev() {
                            self.gen_assign(globals, iseq, lhs)?;
                        }
                    } else {
                        let len = std::cmp::max(mlhs.len(), mrhs.len());
                        for rhs in mrhs {
                            self.gen(globals, iseq, rhs, true)?;
                        }
                        for _ in 0..(len - mrhs.len()) {
                            iseq.gen_push_nil();
                        }
                        self.gen_dup(iseq, len);
                        for _ in 0..(len - mlhs.len()) {
                            self.gen_pop(iseq);
                        }
                        for lhs in mlhs.iter().rev() {
                            self.gen_assign(globals, iseq, lhs)?;
                        }
                        iseq.gen_create_array(len);
                    }
                }
            }
            NodeKind::Command(content) => {
                self.gen(globals, iseq, content, true)?;
                self.gen_opt_send_self(globals, iseq, IdentId::get_id("`"), 1, None, use_value);
            }
            NodeKind::Send {
                receiver,
                method,
                arglist,
                safe_nav,
                ..
            } => {
                let loc = self.loc;
                let mut no_splat_flag = true;
                // push positional args.
                for arg in &arglist.args {
                    if let NodeKind::Splat(_) = arg.kind {
                        no_splat_flag = false;
                    };
                    self.gen(globals, iseq, arg, true)?;
                }
                // push keword args as a Hash.
                let kw_flag = arglist.kw_args.len() != 0;
                if kw_flag {
                    for (id, default) in &arglist.kw_args {
                        iseq.gen_symbol(*id);
                        self.gen(globals, iseq, default, true)?;
                    }
                    iseq.gen_create_hash(arglist.kw_args.len());
                }
                // push keyword rest args.
                for arg in &arglist.kw_rest {
                    self.gen(globals, iseq, arg, true)?;
                }
                let mut block_flag = false;
                let block_ref = match &arglist.block {
                    Some(block) => match &block.kind {
                        // Block literal ({})
                        NodeKind::Proc { params, body, lvar } => {
                            self.loop_stack.push(LoopInfo::new_top());
                            let methodref = self.gen_iseq(
                                globals,
                                params,
                                body,
                                lvar,
                                true,
                                ContextKind::Block,
                                None,
                            )?;
                            self.loop_stack.pop().unwrap();
                            Some(methodref)
                        }
                        // Block parameter (&block)
                        _ => {
                            self.gen(globals, iseq, block, true)?;
                            block_flag = true;
                            None
                        }
                    },
                    None => None,
                };
                // If the method call without block nor keyword/block/splat/double splat arguments, gen OPT_SEND.
                if !block_flag
                    && !kw_flag
                    //&& block_ref.is_none()
                    && no_splat_flag
                    && arglist.kw_rest.is_empty()
                {
                    if NodeKind::SelfValue == receiver.kind {
                        self.loc = loc;
                        self.gen_opt_send_self(
                            globals,
                            iseq,
                            *method,
                            arglist.args.len(),
                            block_ref,
                            use_value,
                        );
                        return Ok(());
                    } else {
                        self.gen(globals, iseq, receiver, true)?;
                        if *safe_nav {
                            self.gen_dup(iseq, 1);
                            iseq.gen_push_nil();
                            iseq.push(Inst::NE);
                            let src = iseq.gen_jmp_if_f();
                            self.loc = loc;
                            self.gen_opt_send(
                                globals,
                                iseq,
                                *method,
                                arglist.args.len(),
                                block_ref,
                                use_value,
                            );
                            iseq.write_disp_from_cur(src);
                            return Ok(());
                        } else {
                            self.loc = loc;
                            self.gen_opt_send(
                                globals,
                                iseq,
                                *method,
                                arglist.args.len(),
                                block_ref,
                                use_value,
                            );
                            return Ok(());
                        }
                    }
                } else {
                    if NodeKind::SelfValue == receiver.kind {
                        self.loc = loc;
                        self.gen_send_self(
                            globals,
                            iseq,
                            *method,
                            arglist.args.len(),
                            arglist.kw_rest.len(),
                            create_flag(kw_flag, block_flag),
                            block_ref,
                        );
                    } else {
                        self.gen(globals, iseq, receiver, true)?;
                        self.loc = loc;
                        self.gen_send(
                            globals,
                            iseq,
                            *method,
                            arglist.args.len(),
                            arglist.kw_rest.len(),
                            create_flag(kw_flag, block_flag),
                            block_ref,
                        );
                    }
                };
                if !use_value {
                    self.gen_pop(iseq)
                };

                /// Create flag for argument info.
                /// 0b0000_0011
                ///          ||
                ///          |+- 1: keyword args exists. 0: no keyword args,
                ///          +-- 1: a block arg exists. 0: no block arg.
                fn create_flag(kw_flag: bool, block_flag: bool) -> u8 {
                    (if kw_flag { 1 } else { 0 }) + (if block_flag { 2 } else { 0 })
                }
            }
            NodeKind::Yield(arglist) => {
                //let loc = self.loc;
                for arg in &arglist.args {
                    self.gen(globals, iseq, arg, true)?;
                }
                self.gen_yield(iseq, arglist.args.len());
                if !use_value {
                    self.gen_pop(iseq);
                };
            }
            NodeKind::MethodDef(id, params, body, lvar) => {
                let methodref = self.gen_iseq(
                    globals,
                    params,
                    body,
                    lvar,
                    true,
                    ContextKind::Method,
                    Some(*id),
                )?;
                iseq.push(Inst::DEF_METHOD);
                iseq.push32((*id).into());
                iseq.push64(methodref.id());
                if use_value {
                    iseq.gen_symbol(*id);
                };
            }
            NodeKind::SingletonMethodDef(singleton, id, params, body, lvar) => {
                let methodref = self.gen_iseq(
                    globals,
                    params,
                    body,
                    lvar,
                    true,
                    ContextKind::Method,
                    Some(*id),
                )?;
                self.gen(globals, iseq, singleton, true)?;
                iseq.push(Inst::DEF_SMETHOD);
                iseq.push32((*id).into());
                iseq.push64(methodref.id());
                if use_value {
                    iseq.gen_symbol(*id);
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
                let loc = node.loc();
                let methodref = self.gen_iseq(
                    globals,
                    &vec![],
                    body,
                    lvar,
                    true,
                    ContextKind::Method,
                    None,
                )?;
                self.gen(globals, iseq, superclass, true)?;
                self.gen(globals, iseq, base, true)?;
                self.save_loc(iseq, loc);
                iseq.push(Inst::DEF_CLASS);
                iseq.push(if *is_module { 1 } else { 0 });
                iseq.push32((*id).into());
                iseq.push64(methodref.id());
                if !use_value {
                    self.gen_pop(iseq);
                };
            }
            NodeKind::SingletonClassDef {
                singleton,
                body,
                lvar,
            } => {
                let loc = node.loc();
                let methodref = self.gen_iseq(
                    globals,
                    &vec![],
                    body,
                    lvar,
                    true,
                    ContextKind::Method,
                    None,
                )?;
                self.gen(globals, iseq, singleton, true)?;
                self.save_loc(iseq, loc);
                iseq.push(Inst::DEF_SCLASS);
                iseq.push64(methodref.id());
                if !use_value {
                    self.gen_pop(iseq);
                };
            }
            NodeKind::Return(val) => {
                self.gen(globals, iseq, val, true)?;
                // Call ensure clauses.
                // Note ensure clause does not return any value.
                let is_block = self.context().kind == ContextKind::Block;
                if let Some(ex) = self.context_mut().jump_dest.last_mut() {
                    let dest = iseq.gen_jmp();
                    if is_block {
                        ex.mreturn_entries.push(dest)
                    } else {
                        ex.return_entries.push(dest)
                    };
                } else {
                    self.save_loc(iseq, node.loc);
                    if is_block {
                        iseq.gen_method_return();
                    } else {
                        iseq.gen_return();
                    }
                };
            }
            NodeKind::Break(val) => {
                let loc = node.loc();
                if self.loop_stack.last().unwrap().state == LoopState::Top {
                    //In the case of outer of loops
                    match self.context().kind {
                        ContextKind::Block => {
                            self.gen(globals, iseq, val, true)?;
                            self.save_loc(iseq, loc);
                            iseq.gen_break();
                        }
                        ContextKind::Method => {
                            return Err(self.error_syntax("Invalid break.", loc.merge(self.loc)));
                        }
                        ContextKind::Eval => {
                            return Err(self.error_syntax(
                                "Can't escape from eval with break.",
                                loc.merge(self.loc),
                            ));
                        }
                    }
                } else {
                    //In the case of inner of loops
                    self.gen(globals, iseq, val, true)?;
                    let src = iseq.gen_jmp();
                    let x = self.loop_stack.last_mut().unwrap();
                    x.escape.push(EscapeInfo::new(src, EscapeKind::Break));
                }
            }
            NodeKind::Next(val) => {
                let loc = node.loc();
                if self.loop_stack.last().unwrap().state == LoopState::Top {
                    //In the case of outer of loops
                    match self.context_stack.last().unwrap().kind {
                        ContextKind::Block => {
                            self.gen(globals, iseq, val, true)?;
                            iseq.gen_return();
                        }
                        ContextKind::Method => {
                            return Err(self.error_syntax("Invalid next.", loc.merge(self.loc)));
                        }
                        ContextKind::Eval => {
                            return Err(self.error_syntax(
                                "Can't escape from eval with next.",
                                loc.merge(self.loc),
                            ));
                        }
                    }
                } else {
                    //In the case of inner of loops
                    self.gen(globals, iseq, val, use_value)?;
                    let src = iseq.gen_jmp();
                    let x = self.loop_stack.last_mut().unwrap();
                    x.escape.push(EscapeInfo::new(src, EscapeKind::Next));
                }
            }
            NodeKind::Proc { params, body, lvar } => {
                self.loop_stack.push(LoopInfo::new_top());
                let methodref =
                    self.gen_iseq(globals, params, body, lvar, true, ContextKind::Block, None)?;
                self.loop_stack.pop().unwrap();
                iseq.push(Inst::CREATE_PROC);
                iseq.push64(methodref.id());
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::Defined(content) => {
                let mut nil_labels = vec![];
                self.check_defined(content, iseq, &mut nil_labels);
                if use_value {
                    iseq.gen_string(globals, content.test_defined());
                    let end = iseq.gen_jmp();
                    nil_labels
                        .iter()
                        .for_each(|label| iseq.write_disp_from_cur(*label));
                    iseq.gen_push_nil();
                    iseq.write_disp_from_cur(end);
                }
            }
            NodeKind::AliasMethod(new, old) => {
                iseq.gen_symbol(*new);
                iseq.gen_symbol(*old);
                self.gen_opt_send_self(globals, iseq, IdentId::_ALIAS_METHOD, 2, None, use_value);
            }
            _ => unreachable!("Codegen: Unimplemented syntax. {:?}", node.kind),
        };
        Ok(())
    }
}

impl Codegen {
    pub fn check_defined(&mut self, node: &Node, iseq: &mut ISeq, labels: &mut Vec<ISeqPos>) {
        match &node.kind {
            NodeKind::LocalVar(id) | NodeKind::Ident(id) => {
                match self.get_local_var(*id) {
                    Some((outer, lvar_id)) => {
                        iseq.push(Inst::CHECK_LOCAL);
                        iseq.push32(lvar_id.as_u32());
                        iseq.push32(outer);
                    }
                    None => {
                        iseq.push(Inst::PUSH_TRUE);
                    }
                };

                labels.push(iseq.gen_jmp_if_t());
            }
            NodeKind::GlobalVar(id) => {
                iseq.push(Inst::CHECK_GVAR);
                iseq.push32((*id).into());
                labels.push(iseq.gen_jmp_if_t());
            }
            NodeKind::InstanceVar(id) => {
                iseq.push(Inst::CHECK_IVAR);
                iseq.push32((*id).into());
                labels.push(iseq.gen_jmp_if_t());
            }
            NodeKind::Const { .. } | NodeKind::Scope(..) => {}
            NodeKind::Array(elems, ..) => elems
                .iter()
                .for_each(|n| self.check_defined(n, iseq, labels)),
            NodeKind::BinOp(_, lhs, rhs) => {
                self.check_defined(lhs, iseq, labels);
                self.check_defined(rhs, iseq, labels);
            }
            NodeKind::UnOp(_, node) => self.check_defined(node, iseq, labels),
            NodeKind::Index { base, index } => {
                self.check_defined(base, iseq, labels);
                index
                    .iter()
                    .for_each(|i| self.check_defined(i, iseq, labels));
            }
            NodeKind::Send {
                receiver, arglist, ..
            } => {
                self.check_defined(receiver, iseq, labels);
                arglist
                    .args
                    .iter()
                    .for_each(|n| self.check_defined(n, iseq, labels));
            }
            _ => {}
        }
    }
}

impl Codegen {
    pub fn error_syntax(&self, msg: impl Into<String>, loc: Loc) -> RubyError {
        RubyError::new_parse_err(
            ParseErrKind::SyntaxError(msg.into()),
            self.source_info,
            0,
            loc,
        )
    }
    pub fn error_name(&self, msg: impl Into<String>) -> RubyError {
        RubyError::new_parse_err(
            ParseErrKind::Name(msg.into()),
            self.source_info,
            0,
            self.loc,
        )
    }
}

impl Codegen {
    /// Construct and return value of constant expression.
    fn const_expr(&self, globals: &mut Globals, node: &Node) -> Result<Value, RubyError> {
        match &node.kind {
            NodeKind::Bool(b) => Ok(Value::bool(*b)),
            NodeKind::Integer(i) => Ok(Value::integer(*i)),
            NodeKind::Float(f) => Ok(Value::float(*f)),
            NodeKind::Nil => Ok(Value::nil()),
            NodeKind::Symbol(s) => Ok(Value::symbol(*s)),
            NodeKind::String(s) => Ok(Value::string(s)),
            NodeKind::Hash(key_value, true) => {
                let mut map = FxHashMap::default();
                for (k, v) in key_value {
                    map.insert(
                        HashKey(self.const_expr(globals, k)?),
                        self.const_expr(globals, v)?,
                    );
                }
                Ok(Value::hash_from_map(map))
            }
            NodeKind::Array(nodes, true) => {
                let mut ary = vec![];
                for n in nodes {
                    ary.push(self.const_expr(globals, n)?)
                }
                Ok(Value::array_from(ary))
            }
            NodeKind::RegExp(nodes, true) => {
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
                    _ => {
                        return Err(
                            self.error_syntax("Illegal internal regexp expression.", node.loc)
                        )
                    }
                };
                let re = match RegexpInfo::from_string(globals, &string) {
                    Ok(re) => re,
                    Err(_) => {
                        return Err(self.error_syntax(
                            format!("Invalid string for a regular expression. {}", string),
                            node.loc,
                        ))
                    }
                };
                Ok(Value::regexp(re))
            }
            _ => unreachable!("const_expr(): not supported. {:?}", node.kind),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::test::*;

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
    fn codegen_invalid_break() {
        assert_error(r#"eval("break")"#);
        assert_error(r#"break"#);
        assert_error("def foo; break; end");
    }

    #[test]
    fn codegen_invalid_next() {
        assert_error(r#"eval("next")"#);
        assert_error(r#"next"#);
        assert_error("def foo; next; end");
    }

    #[test]
    fn codegen_error() {
        assert_error(r#"a"#);
        assert_error(r#"a + 3 = 100"#);
    }
}
