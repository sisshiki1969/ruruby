use super::vm_inst::*;
use crate::error::{ParseErrKind, RubyError, RuntimeErrKind};
use crate::parse::node::{BinOp, Node, NodeKind, UnOp};
use crate::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Codegen {
    // Codegen State
    //pub class_stack: Vec<IdentId>,
    method_stack: Vec<MethodRef>,
    loop_stack: Vec<LoopInfo>,
    context_stack: Vec<Context>,
    pub loc: Loc,
    pub source_info: SourceInfoRef,
}

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
    Loop,
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
    lvar_info: HashMap<IdentId, LvarId>,
    pub iseq_sourcemap: Vec<(ISeqPos, Loc)>,
    kind: ContextKind,
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
            lvar_info: HashMap::new(),
            iseq_sourcemap: vec![],
            kind: ContextKind::Eval,
        }
    }

    fn from(lvar_info: HashMap<IdentId, LvarId>, kind: ContextKind) -> Self {
        Context {
            lvar_info,
            iseq_sourcemap: vec![],
            kind,
        }
    }
}

pub type ISeq = Vec<u8>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ISeqPos(usize);

impl ISeqPos {
    pub fn from_usize(pos: usize) -> Self {
        ISeqPos(pos)
    }

    fn disp(&self, dist: ISeqPos) -> i32 {
        let dist = dist.0 as i64;
        (dist - (self.0 as i64)) as i32
    }
}

impl Codegen {
    pub fn new(source_info: SourceInfoRef) -> Self {
        Codegen {
            method_stack: vec![],
            context_stack: vec![Context::new()],
            loop_stack: vec![LoopInfo::new_top()],
            loc: Loc(0, 0),
            source_info,
        }
    }

    pub fn current(iseq: &ISeq) -> ISeqPos {
        ISeqPos::from_usize(iseq.len())
    }
}

// Codegen
impl Codegen {
    fn gen_push_nil(&mut self, iseq: &mut ISeq) {
        iseq.push(Inst::PUSH_NIL);
    }

    fn gen_push_self(&mut self, iseq: &mut ISeq) {
        iseq.push(Inst::PUSH_SELF);
    }

    fn gen_fixnum(&mut self, iseq: &mut ISeq, num: i64) {
        iseq.push(Inst::PUSH_FIXNUM);
        self.push64(iseq, num as u64);
    }

    fn gen_string(&mut self, globals: &mut Globals, iseq: &mut ISeq, s: &str) {
        iseq.push(Inst::PUSH_STRING);
        let id = globals.get_ident_id(s);
        self.push32(iseq, id.into());
    }

    fn gen_symbol(&mut self, iseq: &mut ISeq, id: IdentId) {
        iseq.push(Inst::PUSH_SYMBOL);
        self.push32(iseq, id.into());
    }

    fn gen_add(&mut self, iseq: &mut ISeq, globals: &mut Globals) {
        iseq.push(Inst::ADD);
        self.push32(iseq, globals.add_inline_cache_entry() as u32);
    }

    fn gen_addi(&mut self, iseq: &mut ISeq, i: i32) {
        iseq.push(Inst::ADDI);
        self.push32(iseq, i as u32);
    }

    fn gen_sub(&mut self, iseq: &mut ISeq, globals: &mut Globals) {
        iseq.push(Inst::SUB);
        self.push32(iseq, globals.add_inline_cache_entry() as u32);
    }

    fn gen_subi(&mut self, iseq: &mut ISeq, i: i32) {
        iseq.push(Inst::SUBI);
        self.push32(iseq, i as u32);
    }

    fn gen_create_array(&mut self, iseq: &mut ISeq, len: usize) {
        iseq.push(Inst::CREATE_ARRAY);
        self.push32(iseq, len as u32);
    }

    fn gen_create_hash(&mut self, iseq: &mut ISeq, len: usize) {
        iseq.push(Inst::CREATE_HASH);
        self.push32(iseq, len as u32);
    }

    fn gen_create_regexp(&mut self, iseq: &mut ISeq) {
        iseq.push(Inst::CREATE_REGEXP);
    }

    fn gen_get_array_elem(&mut self, iseq: &mut ISeq, num_args: usize) {
        iseq.push(Inst::GET_INDEX);
        self.push32(iseq, num_args as u32);
    }

    fn gen_set_array_elem(&mut self, iseq: &mut ISeq, num_args: usize) {
        iseq.push(Inst::SET_INDEX);
        self.push32(iseq, num_args as u32);
    }

    fn gen_splat(&mut self, iseq: &mut ISeq) {
        iseq.push(Inst::SPLAT);
    }

    fn gen_jmp_if_false(&mut self, iseq: &mut ISeq) -> ISeqPos {
        iseq.push(Inst::JMP_IF_FALSE);
        iseq.push(0);
        iseq.push(0);
        iseq.push(0);
        iseq.push(0);
        ISeqPos(iseq.len())
    }

    fn gen_jmp_back(&mut self, iseq: &mut ISeq, pos: ISeqPos) {
        let disp = Codegen::current(iseq).disp(pos) - 5;
        iseq.push(Inst::JMP);
        self.push32(iseq, disp as u32);
    }

    fn gen_jmp(&mut self, iseq: &mut ISeq) -> ISeqPos {
        iseq.push(Inst::JMP);
        iseq.push(0);
        iseq.push(0);
        iseq.push(0);
        iseq.push(0);
        ISeqPos(iseq.len())
    }

    fn gen_end(&mut self, iseq: &mut ISeq) {
        iseq.push(Inst::END);
    }

    fn gen_return(&mut self, iseq: &mut ISeq) {
        iseq.push(Inst::RETURN);
    }

    fn gen_method_return(&mut self, iseq: &mut ISeq) {
        iseq.push(Inst::MRETURN);
    }

    fn gen_opt_case(&mut self, iseq: &mut ISeq, map_id: u32) -> ISeqPos {
        iseq.push(Inst::OPT_CASE);
        self.push32(iseq, map_id);
        self.push32(iseq, 0);
        ISeqPos(iseq.len())
    }

    fn gen_set_local(&mut self, iseq: &mut ISeq, id: IdentId) {
        iseq.push(Inst::SET_LOCAL);
        let (outer, lvar_id) = match self.get_local_var(id) {
            Some((outer, id)) => (outer, id),
            None => panic!(format!(
                "CodeGen: Illegal LvarId in gen_set_local(). id:{:?}",
                id
            )),
        };
        self.push32(iseq, lvar_id.as_u32());
        self.push32(iseq, outer);
    }

    fn gen_get_local(&mut self, iseq: &mut ISeq, id: IdentId) -> Result<(), RubyError> {
        iseq.push(Inst::GET_LOCAL);
        let (outer, lvar_id) = match self.get_local_var(id) {
            Some((outer, id)) => (outer, id),
            None => return Err(self.error_name("undefined local variable.")),
        };
        self.push32(iseq, lvar_id.as_u32());
        self.push32(iseq, outer);
        Ok(())
    }

    fn gen_check_local(&mut self, iseq: &mut ISeq, id: IdentId) -> Result<(), RubyError> {
        iseq.push(Inst::CHECK_LOCAL);
        let (outer, lvar_id) = match self.get_local_var(id) {
            Some((outer, id)) => (outer, id),
            None => return Err(self.error_name("undefined local variable.")),
        };
        self.push32(iseq, lvar_id.as_u32());
        self.push32(iseq, outer);
        Ok(())
    }

    fn get_local_var(&mut self, id: IdentId) -> Option<(u32, LvarId)> {
        for (i, context) in self.context_stack.iter().rev().enumerate() {
            match context.lvar_info.get(&id) {
                Some(id) => return Some((i as u32, id.clone())),
                None => {}
            };
            if context.kind == ContextKind::Method {
                return None;
            }
        }
        None
    }

    fn gen_get_instance_var(&mut self, iseq: &mut ISeq, id: IdentId) {
        iseq.push(Inst::GET_INSTANCE_VAR);
        self.push32(iseq, id.into());
    }

    fn gen_set_instance_var(&mut self, iseq: &mut ISeq, id: IdentId) {
        iseq.push(Inst::SET_INSTANCE_VAR);
        self.push32(iseq, id.into());
    }

    fn gen_ivar_addi(&mut self, iseq: &mut ISeq, id: IdentId, val: u32, use_value: bool) {
        iseq.push(Inst::IVAR_ADDI);
        self.push32(iseq, id.into());
        self.push32(iseq, val);
        if use_value {
            self.gen_get_instance_var(iseq, id);
        }
    }

    fn gen_get_global_var(&mut self, iseq: &mut ISeq, id: IdentId) {
        iseq.push(Inst::GET_GLOBAL_VAR);
        self.push32(iseq, id.into());
    }

    fn gen_set_global_var(&mut self, iseq: &mut ISeq, id: IdentId) {
        iseq.push(Inst::SET_GLOBAL_VAR);
        self.push32(iseq, id.into());
    }

    fn gen_set_const(&mut self, iseq: &mut ISeq, id: IdentId) {
        iseq.push(Inst::SET_CONST);
        self.push32(iseq, id.into());
    }

    fn gen_get_const(&mut self, iseq: &mut ISeq, id: IdentId) {
        self.save_cur_loc(iseq);
        iseq.push(Inst::GET_CONST);
        self.push32(iseq, id.into());
    }

    fn gen_get_const_top(&mut self, iseq: &mut ISeq, id: IdentId) {
        self.save_cur_loc(iseq);
        iseq.push(Inst::GET_CONST_TOP);
        self.push32(iseq, id.into());
    }

    fn gen_get_scope(&mut self, iseq: &mut ISeq, id: IdentId) {
        self.save_cur_loc(iseq);
        iseq.push(Inst::GET_SCOPE);
        self.push32(iseq, id.into());
    }

    fn gen_send(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        method: IdentId,
        args_num: usize,
        flag: usize,
        block: Option<MethodRef>,
    ) {
        self.save_cur_loc(iseq);
        iseq.push(Inst::SEND);
        self.push32(iseq, method.into());
        self.push16(iseq, args_num as u32 as u16);
        self.push16(iseq, flag as u32 as u16);
        self.push32(iseq, globals.add_inline_cache_entry() as u32);
        self.push32(
            iseq,
            match block {
                Some(block) => block,
                None => MethodRef::from(0),
            }
            .into(),
        )
    }

    fn gen_send_self(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        method: IdentId,
        args_num: usize,
        flag: usize,
        block: Option<MethodRef>,
    ) {
        self.save_cur_loc(iseq);
        iseq.push(Inst::SEND_SELF);
        self.push32(iseq, method.into());
        self.push16(iseq, args_num as u32 as u16);
        self.push16(iseq, flag as u32 as u16);
        self.push32(iseq, globals.add_inline_cache_entry() as u32);
        self.push32(
            iseq,
            match block {
                Some(block) => block,
                None => MethodRef::from(0),
            }
            .into(),
        )
    }

    fn gen_assign(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        lhs: &Node,
    ) -> Result<(), RubyError> {
        match &lhs.kind {
            NodeKind::Ident(id) | NodeKind::LocalVar(id) => self.gen_set_local(iseq, *id),
            NodeKind::Const { id, toplevel: _ } => {
                self.gen_set_const(iseq, *id);
            }
            NodeKind::InstanceVar(id) => self.gen_set_instance_var(iseq, *id),
            NodeKind::GlobalVar(id) => self.gen_set_global_var(iseq, *id),
            NodeKind::Send {
                receiver, method, ..
            } => {
                let name = globals.get_ident_name(*method).to_string() + "=";
                let assign_id = globals.get_ident_id(name);
                self.gen(globals, iseq, &receiver, true)?;
                self.loc = lhs.loc();
                self.gen_send(globals, iseq, assign_id, 1, 0, None);
                self.gen_pop(iseq);
            }
            NodeKind::ArrayMember { array, index } => {
                self.gen(globals, iseq, array, true)?;
                for i in index {
                    self.gen(globals, iseq, i, true)?;
                }
                self.save_loc(iseq, lhs.loc());
                self.gen_set_array_elem(iseq, index.len());
            }
            _ => return Err(self.error_syntax(format!("Unimplemented LHS form."), lhs.loc())),
        }
        Ok(())
    }

    fn gen_pop(&mut self, iseq: &mut ISeq) {
        iseq.push(Inst::POP);
    }

    fn gen_dup(&mut self, iseq: &mut ISeq, len: usize) {
        iseq.push(Inst::DUP);
        self.push32(iseq, len as u32);
    }

    fn gen_take(&mut self, iseq: &mut ISeq, len: usize) {
        iseq.push(Inst::TAKE);
        self.push32(iseq, len as u32);
    }

    fn gen_concat(&mut self, iseq: &mut ISeq) {
        iseq.push(Inst::CONCAT_STRING);
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
                    self.gen_push_nil(iseq)
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

    fn write_disp_from_cur(&mut self, iseq: &mut ISeq, src: ISeqPos) {
        let dest = Codegen::current(iseq);
        self.write_disp(iseq, src, dest);
    }

    fn write_disp(&mut self, iseq: &mut ISeq, src: ISeqPos, dest: ISeqPos) {
        let num = src.disp(dest) as u32;
        iseq[src.0 - 4] = (num >> 0) as u8;
        iseq[src.0 - 3] = (num >> 8) as u8;
        iseq[src.0 - 2] = (num >> 16) as u8;
        iseq[src.0 - 1] = (num >> 24) as u8;
    }

    fn push16(&mut self, iseq: &mut ISeq, num: u16) {
        iseq.push(num as u8);
        iseq.push((num >> 8) as u8);
    }

    fn push32(&mut self, iseq: &mut ISeq, num: u32) {
        iseq.push(num as u8);
        iseq.push((num >> 8) as u8);
        iseq.push((num >> 16) as u8);
        iseq.push((num >> 24) as u8);
    }

    fn push64(&mut self, iseq: &mut ISeq, num: u64) {
        iseq.push(num as u8);
        iseq.push((num >> 8) as u8);
        iseq.push((num >> 16) as u8);
        iseq.push((num >> 24) as u8);
        iseq.push((num >> 32) as u8);
        iseq.push((num >> 40) as u8);
        iseq.push((num >> 48) as u8);
        iseq.push((num >> 56) as u8);
    }

    fn save_loc(&mut self, iseq: &mut ISeq, loc: Loc) {
        self.context_stack
            .last_mut()
            .unwrap()
            .iseq_sourcemap
            .push((ISeqPos(iseq.len()), loc));
    }

    fn save_cur_loc(&mut self, iseq: &mut ISeq) {
        self.save_loc(iseq, self.loc)
    }

    pub fn context_push(&mut self, lvar: LvarCollector) {
        self.context_stack
            .push(Context::from(lvar.clone_table(), ContextKind::Method));
    }

    /// Generate ISeq.
    pub fn gen_iseq(
        &mut self,
        globals: &mut Globals,
        params: &[Node],
        node: &Node,
        lvar_collector: &LvarCollector,
        use_value: bool,
        kind: ContextKind,
        name: Option<IdentId>,
    ) -> Result<MethodRef, RubyError> {
        let methodref = globals.new_method();
        let is_block = match kind {
            ContextKind::Method => false,
            _ => true,
        };
        if !is_block {
            self.method_stack.push(methodref)
        }
        let save_loc = self.loc;
        let mut req_params = 0;
        let mut opt_params = 0;
        let mut rest_param = false;
        let mut post_params = 0;
        let mut block_param = false;
        let mut min = 0;
        let mut max = 0;
        let mut unlimited_flag = false;
        let mut param_ident = vec![];
        let mut keyword_params = HashMap::new();
        let mut iseq = ISeq::new();

        self.context_stack
            .push(Context::from(lvar_collector.clone_table(), kind));
        for (lvar_id, param) in params.iter().enumerate() {
            match &param.kind {
                NodeKind::Param(id) => {
                    param_ident.push(*id);
                    req_params += 1;
                    min += 1;
                    max += 1;
                }
                NodeKind::PostParam(id) => {
                    param_ident.push(*id);
                    post_params += 1;
                    min += 1;
                    max += 1;
                }
                NodeKind::OptionalParam(id, default) => {
                    param_ident.push(*id);
                    opt_params += 1;
                    max += 1;
                    self.gen_check_local(&mut iseq, *id)?;
                    let src1 = self.gen_jmp_if_false(&mut iseq);
                    self.gen(globals, &mut iseq, default, true)?;
                    self.gen_set_local(&mut iseq, *id);
                    self.write_disp_from_cur(&mut iseq, src1);
                }
                NodeKind::RestParam(id) => {
                    param_ident.push(*id);
                    rest_param = true;
                    unlimited_flag = true;
                }
                NodeKind::KeywordParam(id, default) => {
                    param_ident.push(*id);
                    keyword_params.insert(*id, LvarId::from_usize(lvar_id));
                    match &**default {
                        Some(default) => {
                            self.gen_check_local(&mut iseq, *id)?;
                            let src1 = self.gen_jmp_if_false(&mut iseq);
                            self.gen(globals, &mut iseq, &default, true)?;
                            self.gen_set_local(&mut iseq, *id);
                            self.write_disp_from_cur(&mut iseq, src1);
                        }
                        None => {}
                    }
                }
                NodeKind::BlockParam(id) => {
                    param_ident.push(*id);
                    block_param = true;
                }
                _ => return Err(self.error_syntax("Parameters should be identifier.", param.loc)),
            }
        }

        self.gen(globals, &mut iseq, node, use_value)?;
        let context = self.context_stack.pop().unwrap();
        let iseq_sourcemap = context.iseq_sourcemap;
        self.gen_end(&mut iseq);
        self.loc = save_loc;

        let info = MethodInfo::RubyFunc {
            iseq: ISeqRef::new(ISeqInfo::new(
                methodref,
                req_params,
                opt_params,
                rest_param,
                post_params,
                block_param,
                if is_block { 0 } else { min },
                if is_block || unlimited_flag {
                    std::usize::MAX
                } else {
                    max
                },
                param_ident,
                keyword_params,
                iseq,
                lvar_collector.clone(),
                iseq_sourcemap,
                self.source_info,
                match kind {
                    ContextKind::Block => ISeqKind::Proc(*self.method_stack.last().unwrap()),
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
        globals.set_method(methodref, info);
        #[cfg(feature = "emit-iseq")]
        {
            let info = globals.get_method_info(methodref);
            let iseq = if let MethodInfo::RubyFunc { iseq } = info {
                iseq.clone()
            } else {
                panic!("CodeGen: Illegal methodref.")
            };
            eprintln!("-----------------------------------------");
            eprintln!("{:?}", methodref);
            for (k, v) in iseq.lvar.table() {
                eprint!("local var: {:?}:{}", v.as_u32(), globals.get_ident_name(*k));
            }
            eprintln!("");
            eprintln!("block: {:?}", iseq.lvar.block());
            let iseq = &iseq.iseq;
            let mut pc = 0;
            while iseq[pc] != Inst::END {
                eprintln!("  {:>05} {}", pc, inst_info(globals, &iseq, pc));
                pc += Inst::inst_size(iseq[pc]);
            }
            eprintln!("  {:>05} {}", pc, inst_info(globals, &iseq, pc));

            fn inst_info(globals: &mut Globals, iseq: &ISeq, pc: usize) -> String {
                match iseq[pc] {
                    Inst::END
                    | Inst::PUSH_NIL
                    | Inst::PUSH_TRUE
                    | Inst::PUSH_FALSE
                    | Inst::PUSH_SELF
                    | Inst::ADD
                    | Inst::SUB
                    | Inst::MUL
                    | Inst::DIV
                    | Inst::REM
                    | Inst::EQ
                    | Inst::NE
                    | Inst::GT
                    | Inst::GE
                    | Inst::NOT
                    | Inst::SHR
                    | Inst::SHL
                    | Inst::BIT_OR
                    | Inst::BIT_AND
                    | Inst::BIT_XOR
                    | Inst::BIT_NOT
                    | Inst::CONCAT_STRING
                    | Inst::CREATE_RANGE
                    | Inst::RETURN
                    | Inst::TO_S
                    | Inst::SPLAT
                    | Inst::POP => format!("{}", Inst::inst_name(iseq[pc])),
                    Inst::PUSH_STRING => format!("PUSH_STRING {}", read32(iseq, pc + 1) as i32),
                    Inst::PUSH_SYMBOL => format!("PUSH_SYMBOL {}", read32(iseq, pc + 1) as i32),
                    Inst::ADDI => format!("ADDI {}", read32(iseq, pc + 1) as i32),
                    Inst::SUBI => format!("SUBI {}", read32(iseq, pc + 1) as i32),
                    Inst::PUSH_FIXNUM => format!("PUSH_FIXNUM {}", read64(iseq, pc + 1) as i64),
                    Inst::PUSH_FLONUM => {
                        format!("PUSH_FLONUM {}", f64::from_bits(read64(iseq, pc + 1)))
                    }

                    Inst::JMP => format!("JMP {:>05}", pc as i32 + 5 + read32(iseq, pc + 1) as i32),
                    Inst::JMP_IF_FALSE => format!(
                        "JMP_IF_FALSE {:>05}",
                        pc as i32 + 5 + read32(iseq, pc + 1) as i32
                    ),
                    Inst::OPT_CASE => {
                        let val = Value::from(read64(iseq, pc + 1));
                        let info = val.as_hash().unwrap();
                        let map = match info.inner_mut() {
                            HashInfo::Map(map) => map,
                            _ => panic!(),
                        };
                        format!(
                            "OPT_CASE {:>05} {:?}",
                            pc as i32 + 13 + read32(iseq, pc + 9) as i32,
                            map,
                        )
                    }
                    Inst::SET_LOCAL => {
                        let frame = read32(iseq, pc + 5);
                        format!("SET_LOCAL outer:{} LvarId:{}", frame, read32(iseq, pc + 1))
                    }
                    Inst::GET_LOCAL => {
                        let frame = read32(iseq, pc + 5);
                        format!("GET_LOCAL outer:{} LvarId:{}", frame, read32(iseq, pc + 1))
                    }
                    Inst::CHECK_LOCAL => {
                        let frame = read32(iseq, pc + 5);
                        format!(
                            "CHECK_LOCAL outer:{} LvarId:{}",
                            frame,
                            read32(iseq, pc + 1)
                        )
                    }
                    Inst::GET_CONST => format!("GET_CONST '{}'", ident_name(globals, iseq, pc + 1)),
                    Inst::GET_CONST_TOP => {
                        format!("GET_CONST_TOP '{}'", ident_name(globals, iseq, pc + 1))
                    }
                    Inst::SET_CONST => format!("SET_CONST '{}'", ident_name(globals, iseq, pc + 1)),
                    Inst::GET_SCOPE => format!("GET_SCOPE '{}'", ident_name(globals, iseq, pc + 1)),
                    Inst::GET_INSTANCE_VAR => {
                        format!("GET_INST_VAR '@{}'", ident_name(globals, iseq, pc + 1))
                    }
                    Inst::SET_INSTANCE_VAR => {
                        format!("SET_INST_VAR '@{}'", ident_name(globals, iseq, pc + 1))
                    }
                    Inst::GET_INDEX => format!("GET_INDEX {} items", read32(iseq, pc + 1)),
                    Inst::SET_INDEX => format!("SET_INDEX {} items", read32(iseq, pc + 1)),
                    Inst::SEND => format!(
                        "SEND '{}' {} items",
                        ident_name(globals, iseq, pc + 1),
                        read32(iseq, pc + 5)
                    ),
                    Inst::SEND_SELF => format!(
                        "SEND_SELF '{}' {} items",
                        ident_name(globals, iseq, pc + 1),
                        read32(iseq, pc + 5)
                    ),

                    Inst::CREATE_ARRAY => format!("CREATE_ARRAY {} items", read32(iseq, pc + 1)),
                    Inst::CREATE_PROC => format!("CREATE_PROC method:{}", read32(iseq, pc + 1)),
                    Inst::CREATE_HASH => format!("CREATE_HASH {} items", read32(iseq, pc + 1)),
                    Inst::DUP => format!("DUP {}", read32(iseq, pc + 1)),
                    Inst::TAKE => format!("TAKE {}", read32(iseq, pc + 1)),
                    Inst::DEF_CLASS => format!(
                        "DEF_CLASS {} '{}' method:{}",
                        if read8(iseq, pc + 1) == 1 {
                            "module"
                        } else {
                            "class"
                        },
                        ident_name(globals, iseq, pc + 2),
                        read32(iseq, pc)
                    ),
                    Inst::DEF_METHOD => {
                        format!("DEF_METHOD '{}'", ident_name(globals, iseq, pc + 1))
                    }
                    Inst::DEF_SMETHOD => {
                        format!("DEF_SMETHOD '{}'", ident_name(globals, iseq, pc + 1))
                    }
                    _ => format!("undefined"),
                }
            }

            fn read64(iseq: &ISeq, pc: usize) -> u64 {
                let ptr = iseq[pc..pc + 1].as_ptr() as *const u64;
                unsafe { *ptr }
            }

            fn read32(iseq: &ISeq, pc: usize) -> u32 {
                let ptr = iseq[pc..pc + 1].as_ptr() as *const u32;
                unsafe { *ptr }
            }

            fn read8(iseq: &ISeq, pc: usize) -> u8 {
                iseq[pc]
            }

            fn ident_name(globals: &mut Globals, iseq: &ISeq, pc: usize) -> String {
                globals
                    .get_ident_name(IdentId::from(read32(iseq, pc)))
                    .to_owned()
            }
        }
        Ok(methodref)
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
                | NodeKind::String(_)
                | NodeKind::Symbol(_)
                | NodeKind::SelfValue => return Ok(()),
                _ => {}
            }
        };
        match &node.kind {
            NodeKind::Nil => self.gen_push_nil(iseq),
            NodeKind::Bool(b) => {
                if *b {
                    iseq.push(Inst::PUSH_TRUE)
                } else {
                    iseq.push(Inst::PUSH_FALSE)
                }
            }
            NodeKind::Integer(num) => {
                self.gen_fixnum(iseq, *num);
            }
            NodeKind::Float(num) => {
                iseq.push(Inst::PUSH_FLONUM);
                self.push64(iseq, f64::to_bits(*num));
            }
            NodeKind::String(s) => {
                self.gen_string(globals, iseq, s);
            }
            NodeKind::Symbol(id) => {
                self.gen_symbol(iseq, *id);
            }
            NodeKind::InterporatedString(nodes) => {
                self.gen_string(globals, iseq, &"".to_string());
                for node in nodes {
                    match &node.kind {
                        NodeKind::String(s) => {
                            self.gen_string(globals, iseq, &s);
                        }
                        NodeKind::CompStmt(nodes) => {
                            self.gen_comp_stmt(globals, iseq, nodes, true)?;
                            iseq.push(Inst::TO_S);
                        }
                        _ => unimplemented!("Illegal arguments in Nodekind::InterporatedString."),
                    }
                    self.gen_concat(iseq);
                }
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::RegExp(nodes) => {
                self.gen_string(globals, iseq, &"".to_string());
                for node in nodes {
                    match &node.kind {
                        NodeKind::String(s) => {
                            self.gen_string(globals, iseq, &s);
                        }
                        NodeKind::CompStmt(nodes) => {
                            self.gen_comp_stmt(globals, iseq, nodes, true)?;
                            iseq.push(Inst::TO_S);
                        }
                        _ => unimplemented!("Illegal arguments in Nodekind::InterporatedString."),
                    }
                    self.gen_concat(iseq);
                }
                let loc = self.loc;
                self.save_loc(iseq, loc);
                self.gen_create_regexp(iseq);
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::SelfValue => {
                self.gen_push_self(iseq);
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
            NodeKind::Array(nodes) => {
                let len = nodes.len();
                for node in nodes {
                    self.gen(globals, iseq, node, true)?;
                }
                self.gen_create_array(iseq, len);
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::Hash(key_value) => {
                let len = key_value.len();
                for (k, v) in key_value {
                    self.gen(globals, iseq, k, true)?;
                    self.gen(globals, iseq, v, true)?;
                }
                self.gen_create_hash(iseq, len);
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::Ident(id) => {
                self.gen_send_self(globals, iseq, *id, 0, 0, None);
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
                self.gen_get_global_var(iseq, *id);
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::Const { id, toplevel } => {
                if *toplevel {
                    self.gen_get_const_top(iseq, *id);
                } else {
                    self.gen_get_const(iseq, *id);
                };
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::Scope(parent, id) => {
                self.gen(globals, iseq, parent, true)?;
                self.gen_get_scope(iseq, *id);
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::InstanceVar(id) => {
                self.gen_get_instance_var(iseq, *id);
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::BinOp(op, lhs, rhs) => {
                let loc = self.loc;
                match op {
                    BinOp::Add => match (&lhs.kind, &rhs.kind) {
                        (_, NodeKind::Integer(i)) if *i as i32 as i64 == *i => {
                            self.gen(globals, iseq, lhs, true)?;
                            self.save_loc(iseq, loc);
                            self.gen_addi(iseq, *i as i32);
                        }
                        _ => {
                            self.gen(globals, iseq, lhs, true)?;
                            self.gen(globals, iseq, rhs, true)?;
                            self.save_loc(iseq, loc);
                            self.gen_add(iseq, globals);
                        }
                    },
                    BinOp::Sub => match rhs.kind {
                        NodeKind::Integer(i) if i as i32 as i64 == i => {
                            self.gen(globals, iseq, lhs, true)?;
                            self.save_loc(iseq, loc);
                            self.gen_subi(iseq, i as i32);
                        }
                        _ => {
                            self.gen(globals, iseq, lhs, true)?;
                            self.gen(globals, iseq, rhs, true)?;
                            self.save_loc(iseq, loc);
                            self.gen_sub(iseq, globals);
                        }
                    },
                    BinOp::Mul => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq, loc);
                        iseq.push(Inst::MUL);
                        self.push32(iseq, globals.add_inline_cache_entry() as u32);
                    }
                    BinOp::Div => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq, loc);
                        iseq.push(Inst::DIV);
                    }
                    BinOp::Exp => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq, loc);
                        iseq.push(Inst::POW);
                    }
                    BinOp::Rem => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq, loc);
                        iseq.push(Inst::REM);
                    }
                    BinOp::Shr => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq, loc);
                        iseq.push(Inst::SHR);
                    }
                    BinOp::Shl => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq, loc);
                        iseq.push(Inst::SHL);
                        self.push32(iseq, globals.add_inline_cache_entry() as u32);
                    }
                    BinOp::BitOr => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq, loc);
                        iseq.push(Inst::BIT_OR);
                    }
                    BinOp::BitAnd => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq, loc);
                        iseq.push(Inst::BIT_AND);
                    }
                    BinOp::BitXor => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq, loc);
                        iseq.push(Inst::BIT_XOR);
                    }
                    BinOp::Eq => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq, loc);
                        iseq.push(Inst::EQ);
                    }
                    BinOp::Ne => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq, loc);
                        iseq.push(Inst::NE);
                    }
                    BinOp::TEq => {
                        self.gen(globals, iseq, rhs, true)?;
                        self.gen(globals, iseq, lhs, true)?;
                        self.save_loc(iseq, loc);
                        iseq.push(Inst::TEQ);
                    }
                    BinOp::Match => {
                        let method = globals.get_ident_id("=~");
                        self.gen(globals, iseq, rhs, true)?;
                        self.gen(globals, iseq, lhs, true)?;
                        self.loc = loc;
                        self.gen_send(globals, iseq, method, 1, 0, None);
                    }
                    BinOp::Ge => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq, loc);
                        iseq.push(Inst::GE);
                    }
                    BinOp::Gt => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq, loc);
                        iseq.push(Inst::GT);
                    }
                    BinOp::Le => {
                        self.gen(globals, iseq, rhs, true)?;
                        self.gen(globals, iseq, lhs, true)?;
                        self.save_loc(iseq, loc);
                        iseq.push(Inst::GE);
                    }
                    BinOp::Lt => {
                        self.gen(globals, iseq, rhs, true)?;
                        self.gen(globals, iseq, lhs, true)?;
                        self.save_loc(iseq, loc);
                        iseq.push(Inst::GT);
                    }
                    BinOp::LAnd => {
                        self.gen(globals, iseq, lhs, true)?;
                        let src1 = self.gen_jmp_if_false(iseq);
                        self.gen(globals, iseq, rhs, true)?;
                        let src2 = self.gen_jmp(iseq);
                        self.write_disp_from_cur(iseq, src1);
                        iseq.push(Inst::PUSH_FALSE);
                        self.write_disp_from_cur(iseq, src2);
                    }
                    BinOp::LOr => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen_dup(iseq, 1);
                        let src1 = self.gen_jmp_if_false(iseq);
                        let src2 = self.gen_jmp(iseq);
                        self.write_disp_from_cur(iseq, src1);
                        self.gen_pop(iseq);
                        self.gen(globals, iseq, rhs, true)?;
                        self.write_disp_from_cur(iseq, src2);
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
                        iseq.push(Inst::BIT_NOT);
                    }
                    UnOp::Not => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.save_loc(iseq, node.loc());
                        iseq.push(Inst::NOT);
                    }
                }
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::ArrayMember { array, index } => {
                let loc = node.loc();
                self.gen(globals, iseq, array, true)?;
                let num_args = index.len();
                for i in index {
                    self.gen(globals, iseq, i, true)?;
                }
                self.save_loc(iseq, loc);
                self.gen_get_array_elem(iseq, num_args);
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::Splat(array) => {
                self.gen(globals, iseq, array, true)?;
                self.gen_splat(iseq);
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::CompStmt(nodes) => self.gen_comp_stmt(globals, iseq, nodes, use_value)?,
            NodeKind::If { cond, then_, else_ } => {
                self.gen(globals, iseq, &cond, true)?;
                let src1 = self.gen_jmp_if_false(iseq);
                self.gen(globals, iseq, &then_, use_value)?;
                let src2 = self.gen_jmp(iseq);
                self.write_disp_from_cur(iseq, src1);
                self.gen(globals, iseq, &else_, use_value)?;
                self.write_disp_from_cur(iseq, src2);
            }
            NodeKind::For { param, iter, body } => {
                let id = match param.kind {
                    NodeKind::Ident(id) | NodeKind::LocalVar(id) => id,
                    _ => return Err(self.error_syntax("Expected an identifier.", param.loc())),
                };
                self.loop_stack.push(LoopInfo::new_loop());
                let loop_continue;
                match &iter.kind {
                    NodeKind::Range {
                        start,
                        end,
                        exclude_end,
                    } => {
                        self.gen(globals, iseq, start, true)?;
                        self.gen_set_local(iseq, id);
                        let loop_start = Codegen::current(iseq);
                        self.gen(globals, iseq, end, true)?;
                        self.gen_get_local(iseq, id)?;
                        iseq.push(if *exclude_end { Inst::GT } else { Inst::GE });
                        let src = self.gen_jmp_if_false(iseq);
                        self.gen(globals, iseq, body, false)?;
                        loop_continue = Codegen::current(iseq);
                        self.gen_get_local(iseq, id)?;
                        self.gen_addi(iseq, 1);
                        self.gen_set_local(iseq, id);
                        self.gen_jmp_back(iseq, loop_start);
                        self.write_disp_from_cur(iseq, src);
                    }
                    _ => return Err(self.error_syntax("Expected Range.", iter.loc())),
                };

                if use_value {
                    self.gen(globals, iseq, iter, true)?;
                }
                let src = self.gen_jmp(iseq);
                for p in self.loop_stack.pop().unwrap().escape {
                    match p.kind {
                        EscapeKind::Break => {
                            self.write_disp_from_cur(iseq, p.pos);
                        }
                        EscapeKind::Next => self.write_disp(iseq, p.pos, loop_continue),
                    }
                }
                if !use_value {
                    self.gen_pop(iseq);
                }

                self.write_disp_from_cur(iseq, src);
            }
            NodeKind::While { cond, body } => {
                self.loop_stack.push(LoopInfo::new_loop());

                let loop_start = Codegen::current(iseq);
                self.gen(globals, iseq, cond, true)?;
                let src = self.gen_jmp_if_false(iseq);
                self.gen(globals, iseq, body, false)?;
                self.gen_jmp_back(iseq, loop_start);
                self.write_disp_from_cur(iseq, src);

                if use_value {
                    self.gen_push_nil(iseq);
                }
                let src = self.gen_jmp(iseq);
                for p in self.loop_stack.pop().unwrap().escape {
                    match p.kind {
                        EscapeKind::Break => {
                            self.write_disp_from_cur(iseq, p.pos);
                        }
                        EscapeKind::Next => self.write_disp(iseq, p.pos, loop_start),
                    }
                }
                if !use_value {
                    self.gen_pop(iseq);
                }

                self.write_disp_from_cur(iseq, src);
            }
            NodeKind::Begin {
                body,
                rescue: _,
                else_: _,
                ensure: _,
            } => {
                self.gen(globals, iseq, body, use_value)?;
            }
            NodeKind::Case { cond, when_, else_ } => {
                let mut end = vec![];
                self.gen(globals, iseq, cond, true)?;
                let mut opt_flag = true;
                for branch in when_ {
                    for elem in &branch.when {
                        match elem.kind {
                            NodeKind::Integer(_) => (),
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
                    let map_id = globals.new_case_dispatch_map();
                    self.save_cur_loc(iseq);
                    let start = self.gen_opt_case(iseq, map_id);
                    for branch in when_ {
                        let map = globals.get_mut_case_dispatch_map(map_id);
                        let disp = start.disp(Codegen::current(iseq)) as i32;
                        for elem in &branch.when {
                            let k = match elem.kind {
                                NodeKind::Integer(i) => Value::fixnum(i),
                                _ => unreachable!(),
                            };
                            map.insert(k, disp);
                        }
                        self.gen(globals, iseq, &branch.body, use_value)?;
                        end.push(self.gen_jmp(iseq));
                    }
                    self.write_disp_from_cur(iseq, start);
                } else {
                    let mut next = None;
                    for branch in when_ {
                        let mut jmp_dest = vec![];
                        match next {
                            Some(next) => {
                                self.write_disp_from_cur(iseq, next);
                            }
                            None => {}
                        }
                        for elem in &branch.when {
                            self.gen_dup(iseq, 1);
                            self.gen(globals, iseq, elem, true)?;
                            self.save_loc(iseq, elem.loc);
                            iseq.push(Inst::TEQ);
                            iseq.push(Inst::NOT);
                            jmp_dest.push(self.gen_jmp_if_false(iseq));
                        }
                        next = Some(self.gen_jmp(iseq));
                        for dest in jmp_dest {
                            self.write_disp_from_cur(iseq, dest);
                        }
                        self.gen_pop(iseq);
                        self.gen(globals, iseq, &branch.body, use_value)?;
                        end.push(self.gen_jmp(iseq));
                    }
                    match next {
                        Some(next) => {
                            self.write_disp_from_cur(iseq, next);
                        }
                        None => {}
                    }
                    self.gen_pop(iseq);
                }
                self.gen(globals, iseq, &else_, use_value)?;
                for dest in end {
                    self.write_disp_from_cur(iseq, dest);
                }
            }
            NodeKind::MulAssign(mlhs, mrhs) => {
                let lhs_len = mlhs.len();
                let rhs_len = mrhs.len();
                let splat_flag = mrhs.iter().any(|x| {
                    if let NodeKind::Splat(_) = x.kind {
                        true
                    } else {
                        false
                    }
                });
                if lhs_len == rhs_len && !splat_flag {
                    if lhs_len == 1 {
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
                                self.gen_ivar_addi(iseq, *id1, *i as i32 as u32, use_value);
                            }
                            _ => {
                                self.gen(globals, iseq, &mrhs[0], true)?;
                                if use_value {
                                    self.gen_dup(iseq, 1);
                                };
                                self.gen_assign(globals, iseq, &mlhs[0])?;
                            }
                        }
                    } else {
                        for rhs in mrhs.iter().rev() {
                            self.gen(globals, iseq, rhs, true)?;
                        }
                        if use_value {
                            self.gen_dup(iseq, rhs_len);
                        }
                        for lhs in mlhs {
                            self.gen_assign(globals, iseq, lhs)?;
                        }
                        if use_value && rhs_len != 1 {
                            self.gen_create_array(iseq, rhs_len);
                        };
                    }
                } else if lhs_len == 1 {
                    for rhs in mrhs.iter().rev() {
                        self.gen(globals, iseq, rhs, true)?;
                    }
                    self.gen_create_array(iseq, rhs_len);
                    if use_value {
                        self.gen_dup(iseq, 1);
                    };
                    self.gen_assign(globals, iseq, &mlhs[0])?;
                } else {
                    for rhs in mrhs.iter().rev() {
                        self.gen(globals, iseq, rhs, true)?;
                    }
                    if splat_flag || rhs_len != 1 {
                        self.gen_create_array(iseq, rhs_len);
                    }
                    if use_value {
                        self.gen_dup(iseq, 1);
                    };
                    self.gen_take(iseq, lhs_len);

                    for lhs in mlhs.iter().rev() {
                        self.gen_assign(globals, iseq, lhs)?;
                    }
                }
            }
            NodeKind::Send {
                receiver,
                method,
                send_args,
                ..
            } => {
                let loc = self.loc;
                for arg in &send_args.args {
                    self.gen(globals, iseq, arg, true)?;
                }
                let kw_flag = send_args.kw_args.len() != 0;
                if kw_flag {
                    for (id, default) in &send_args.kw_args {
                        self.gen_symbol(iseq, *id);
                        self.gen(globals, iseq, default, true)?;
                    }
                    self.gen_create_hash(iseq, send_args.kw_args.len());
                }
                let mut block_flag = false;
                let block_ref = match &send_args.block {
                    Some(block) => match &block.kind {
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
                        _ => {
                            self.gen(globals, iseq, block, true)?;
                            block_flag = true;
                            None
                        }
                    },
                    None => None,
                };
                if NodeKind::SelfValue == receiver.kind {
                    self.loc = loc;
                    self.gen_send_self(
                        globals,
                        iseq,
                        *method,
                        send_args.args.len(),
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
                        send_args.args.len(),
                        create_flag(kw_flag, block_flag),
                        block_ref,
                    );
                };
                if !use_value {
                    self.gen_pop(iseq)
                };

                /// Create flag for argument info.
                /// 0b0011
                ///     ||
                ///     |+- 1: keyword args exists. 0: no keyword args,
                ///     +-- 1: a block arg exists. 0: no block arg.
                fn create_flag(kw_flag: bool, block_flag: bool) -> usize {
                    (if kw_flag { 1usize } else { 0usize })
                        + (if block_flag { 2usize } else { 0usize })
                }
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
                self.push32(iseq, (*id).into());
                self.push32(iseq, methodref.into());
                if use_value {
                    self.gen_symbol(iseq, *id);
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
                self.push32(iseq, (*id).into());
                self.push32(iseq, methodref.into());
                if use_value {
                    self.gen_symbol(iseq, *id);
                };
            }
            NodeKind::ClassDef {
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
                self.save_loc(iseq, loc);
                iseq.push(Inst::DEF_CLASS);
                iseq.push(if *is_module { 1 } else { 0 });
                self.push32(iseq, (*id).into());
                self.push32(iseq, methodref.into());
                if !use_value {
                    self.gen_pop(iseq);
                };
            }
            NodeKind::Return(val) => {
                self.gen(globals, iseq, val, true)?;
                if self.context_stack.last().unwrap().kind == ContextKind::Block {
                    self.gen_method_return(iseq);
                } else {
                    self.gen_return(iseq);
                }
            }
            NodeKind::Break(val) => {
                let loc = node.loc();
                if self.loop_stack.last().unwrap().state == LoopState::Top {
                    //In the case of outer of loops
                    match self.context_stack.last().unwrap().kind {
                        ContextKind::Block => {
                            self.gen(globals, iseq, val, true)?;
                            self.gen_return(iseq);
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
                    let src = self.gen_jmp(iseq);
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
                            self.gen_end(iseq);
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
                    let src = self.gen_jmp(iseq);
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
                self.push32(iseq, methodref.into());
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            _ => {
                return Err(self.error_syntax(
                    format!("Codegen: Unimplemented syntax. {:?}", node.kind),
                    self.loc,
                ))
            }
        };
        Ok(())
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
        RubyError::new_runtime_err(RuntimeErrKind::Name(msg.into()), self.source_info, self.loc)
    }
}
