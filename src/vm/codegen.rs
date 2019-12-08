use super::vm_inst::*;
use crate::error::{ParseErrKind, RubyError, RuntimeErrKind};
use crate::node::{BinOp, Node, NodeKind};
use crate::vm::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Codegen {
    // Codegen State
    //pub class_stack: Vec<IdentId>,
    pub loop_stack: Vec<Vec<(ISeqPos, EscapeKind)>>,
    pub context_stack: Vec<Context>,
    pub loc: Loc,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    lvar_info: HashMap<IdentId, LvarId>,
    pub iseq_sourcemap: Vec<(ISeqPos, Loc)>,
    kind: ContextKind,
}

#[derive(Debug, Clone, PartialEq)]
enum ContextKind {
    Method,
    Block,
}

impl Context {
    fn new() -> Self {
        Context {
            lvar_info: HashMap::new(),
            iseq_sourcemap: vec![],
            kind: ContextKind::Method,
        }
    }

    fn from(lvar_info: HashMap<IdentId, LvarId>, is_block: bool) -> Self {
        Context {
            lvar_info,
            iseq_sourcemap: vec![],
            kind: if is_block {
                ContextKind::Block
            } else {
                ContextKind::Method
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EscapeKind {
    Break,
    Next,
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
    pub fn new() -> Self {
        Codegen {
            context_stack: vec![Context::new()],
            //class_stack: vec![],
            loop_stack: vec![],
            loc: Loc(0, 0),
        }
    }

    pub fn set_context(&mut self, lvar_table: HashMap<IdentId, LvarId>) {
        self.context_stack = vec![Context::from(lvar_table, false)];
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

    fn gen_fixnum(&mut self, iseq: &mut ISeq, num: i64) {
        iseq.push(Inst::PUSH_FIXNUM);
        self.push64(iseq, num as u64);
    }

    fn gen_string(&mut self, globals: &mut Globals, iseq: &mut ISeq, s: &String) {
        iseq.push(Inst::PUSH_STRING);
        let id = globals.get_ident_id(s.clone());
        self.push32(iseq, id.into());
    }

    fn gen_symbol(&mut self, iseq: &mut ISeq, id: IdentId) {
        iseq.push(Inst::PUSH_SYMBOL);
        self.push32(iseq, id.into());
    }

    fn gen_add(&mut self, iseq: &mut ISeq) {
        self.save_loc(iseq);
        iseq.push(Inst::ADD);
    }

    fn gen_create_array(&mut self, iseq: &mut ISeq, len: usize) {
        iseq.push(Inst::CREATE_ARRAY);
        self.push32(iseq, len as u32);
    }

    fn gen_get_array_elem(&mut self, iseq: &mut ISeq, num_args: usize) {
        iseq.push(Inst::GET_ARRAY_ELEM);
        self.push32(iseq, num_args as u32);
    }

    fn gen_set_array_elem(&mut self, iseq: &mut ISeq, num_args: usize) {
        iseq.push(Inst::SET_ARRAY_ELEM);
        self.push32(iseq, num_args as u32);
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

    fn gen_set_local(&mut self, iseq: &mut ISeq, id: IdentId) {
        iseq.push(Inst::SET_LOCAL);
        let (outer, lvar_id) = match self.get_local_var(id) {
            Some((outer, id)) => (outer, id),
            None => panic!("CodeGen: Illegal LvarId in gen_set_local()."),
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

    fn get_local_var(&mut self, id: IdentId) -> Option<(u32, LvarId)> {
        let len = self.context_stack.len();
        for i in 0..len {
            let context = &self.context_stack[len - i - 1];
            match context.lvar_info.get(&id) {
                Some(id) => return Some((i as u32, id.clone())),
                None => {}
            };
            if context.kind != ContextKind::Block {
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

    fn gen_set_const(&mut self, iseq: &mut ISeq, id: IdentId) {
        iseq.push(Inst::SET_CONST);
        self.push32(iseq, id.into());
    }

    fn gen_get_const(&mut self, iseq: &mut ISeq, id: IdentId) {
        self.save_loc(iseq);
        iseq.push(Inst::GET_CONST);
        self.push32(iseq, id.into());
    }

    fn gen_send(&mut self, iseq: &mut ISeq, method: IdentId, args_num: usize) {
        self.save_loc(iseq);
        iseq.push(Inst::SEND);
        self.push32(iseq, method.into());
        self.push32(iseq, args_num as u32);
    }

    fn gen_send_self(&mut self, iseq: &mut ISeq, method: IdentId, args_num: usize) {
        self.save_loc(iseq);
        iseq.push(Inst::SEND_SELF);
        self.push32(iseq, method.into());
        self.push32(iseq, args_num as u32);
    }

    fn gen_assign(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        lhs: &Node,
    ) -> Result<(), RubyError> {
        match &lhs.kind {
            NodeKind::Ident(id) | NodeKind::LocalVar(id) => self.gen_set_local(iseq, *id),
            NodeKind::Const(id) => self.gen_set_const(iseq, *id),
            NodeKind::InstanceVar(id) => self.gen_set_instance_var(iseq, *id),
            NodeKind::Send {
                receiver, method, ..
            } => {
                let name = globals.get_ident_name(*method).clone() + "=";
                let assign_id = globals.get_ident_id(name);
                self.gen(globals, iseq, &receiver, true)?;
                self.loc = lhs.loc();
                self.gen_send(iseq, assign_id, 1);
                self.gen_pop(iseq);
            }
            NodeKind::ArrayMember { array, index } => {
                self.gen(globals, iseq, array, true)?;
                if index.len() != 1 {
                    return Err(self.error_syntax(format!("Unimplemented LHS form."), lhs.loc()));
                }
                self.gen(globals, iseq, &index[0], true)?;
                self.gen_set_array_elem(iseq, 1);
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

    fn gen_concat(&mut self, iseq: &mut ISeq) {
        iseq.push(Inst::CONCAT_STRING);
    }

    fn gen_comp_stmt(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        nodes: &Vec<Node>,
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
    fn save_loc(&mut self, iseq: &mut ISeq) {
        self.context_stack
            .last_mut()
            .unwrap()
            .iseq_sourcemap
            .push((ISeqPos(iseq.len()), self.loc));
    }

    /// Generate ISeq.

    pub fn gen_iseq(
        &mut self,
        globals: &mut Globals,
        params: &Vec<Node>,
        node: &Node,
        lvar_collector: &LvarCollector,
        use_value: bool,
        is_block: bool,
    ) -> Result<MethodRef, RubyError> {
        let save_loc = self.loc;
        let mut params_lvar = vec![];
        for param in params {
            match param.kind {
                NodeKind::Param(id) => {
                    let lvar = lvar_collector.table.get(&id).unwrap();
                    params_lvar.push(*lvar);
                }
                _ => return Err(self.error_syntax("Parameters should be identifier.", self.loc)),
            }
        }
        let mut iseq = ISeq::new();
        self.context_stack
            .push(Context::from(lvar_collector.table.clone(), is_block));
        self.gen(globals, &mut iseq, node, use_value)?;
        let context = self.context_stack.pop().unwrap();
        let iseq_sourcemap = context.iseq_sourcemap;
        iseq.push(Inst::END);
        self.loc = save_loc;

        let info = MethodInfo::RubyFunc {
            iseq: ISeqRef::new(ISeqInfo::new(
                params_lvar,
                iseq,
                lvar_collector.clone(),
                iseq_sourcemap,
            )),
        };
        let methodref = globals.add_method(info);
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
            let iseq = &iseq.iseq;
            let mut pc = 0;
            while iseq[pc] != Inst::END {
                eprintln!(
                    "  {:>05} {}",
                    pc,
                    inst_info(globals, &lvar_collector.table, &iseq, pc)
                );
                pc += Inst::inst_size(iseq[pc]);
            }
            eprintln!(
                "  {:>05} {}",
                pc,
                inst_info(globals, &lvar_collector.table, &iseq, pc)
            );

            fn inst_info(
                globals: &mut Globals,
                _lvar_table: &HashMap<IdentId, LvarId>,
                iseq: &ISeq,
                pc: usize,
            ) -> String {
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
                    | Inst::EQ
                    | Inst::NE
                    | Inst::GT
                    | Inst::GE
                    | Inst::SHR
                    | Inst::SHL
                    | Inst::BIT_OR
                    | Inst::BIT_AND
                    | Inst::BIT_XOR
                    | Inst::CONCAT_STRING
                    | Inst::CREATE_RANGE
                    | Inst::TO_S
                    | Inst::POP => format!("{}", Inst::inst_name(iseq[pc])),
                    Inst::PUSH_FIXNUM => format!("PUSH_FIXNUM {}", read64(iseq, pc + 1) as i64),
                    Inst::PUSH_FLONUM => format!("PUSH_FLONUM {}", unsafe {
                        std::mem::transmute::<u64, f64>(read64(iseq, pc + 1))
                    }),
                    Inst::PUSH_STRING => format!("PUSH_STRING "),
                    Inst::PUSH_SYMBOL => format!("PUSH_SYMBOL "),
                    Inst::JMP => format!("JMP {:>05}", pc as i32 + 5 + read32(iseq, pc + 1) as i32),
                    Inst::JMP_IF_FALSE => format!(
                        "JMP_IF_FALSE {:>05}",
                        pc as i32 + 5 + read32(iseq, pc + 1) as i32
                    ),
                    Inst::SET_LOCAL => {
                        let id = read32(iseq, pc + 1);
                        let frame = read32(iseq, pc + 5);
                        format!("SET_LOCAL {} '{}'", frame, id)
                    }
                    Inst::GET_LOCAL => {
                        let id = read32(iseq, pc + 1);
                        let frame = read32(iseq, pc + 5);
                        format!("GET_LOCAL {} '{}'", frame, id)
                    }
                    Inst::GET_CONST => format!("GET_CONST '{}'", ident_name(globals, iseq, pc + 1)),
                    Inst::SET_CONST => format!("SET_CONST '{}'", ident_name(globals, iseq, pc + 1)),
                    Inst::GET_INSTANCE_VAR => {
                        format!("GET_INST_VAR '@{}'", ident_name(globals, iseq, pc + 1))
                    }
                    Inst::SET_INSTANCE_VAR => {
                        format!("SET_INST_VAR '@{}'", ident_name(globals, iseq, pc + 1))
                    }
                    Inst::GET_ARRAY_ELEM => format!("GET_ARY_ELEM {} items", read32(iseq, pc + 1)),
                    Inst::SET_ARRAY_ELEM => format!("SET_ARY_ELEM {} items", read32(iseq, pc + 1)),
                    Inst::SEND => format!(
                        "SEND '{}' {} items",
                        ident_name(globals, iseq, pc + 1),
                        read32(iseq, pc + 5)
                    ),
                    Inst::CREATE_ARRAY => format!("CREATE_ARRAY {} items", read32(iseq, pc + 1)),
                    Inst::CREATE_PROC => format!("CREATE_PROC method:{}", read32(iseq, pc + 1)),
                    Inst::DUP => format!("DUP {}", read32(iseq, pc + 1)),
                    Inst::DEF_CLASS => format!("DEF_CLASS"),
                    Inst::DEF_METHOD => format!("DEF_METHOD"),
                    Inst::DEF_CLASS_METHOD => format!("DEF_CLASS_METHOD"),
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
                | NodeKind::Number(_)
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
            NodeKind::Number(num) => {
                self.gen_fixnum(iseq, *num);
            }
            NodeKind::Float(num) => {
                iseq.push(Inst::PUSH_FLONUM);
                unsafe { self.push64(iseq, std::mem::transmute(*num)) };
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
            NodeKind::SelfValue => {
                iseq.push(Inst::PUSH_SELF);
            }
            NodeKind::Range {
                start,
                end,
                exclude_end,
            } => {
                if *exclude_end {
                    iseq.push(Inst::PUSH_TRUE);
                } else {
                    iseq.push(Inst::PUSH_FALSE)
                };
                self.gen(globals, iseq, end, true)?;
                self.gen(globals, iseq, start, true)?;
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
            NodeKind::Ident(id) => {
                return Err(self.error_name(format!(
                    "Undefined local variable or method `{}'.",
                    globals.get_ident_name(*id)
                )));
            }
            NodeKind::LocalVar(id) => {
                self.gen_get_local(iseq, *id)?;
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::Const(id) => {
                self.gen_get_const(iseq, *id);
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
                    BinOp::Add => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.loc = loc;
                        self.gen(globals, iseq, rhs, true)?;
                        self.gen_add(iseq);
                    }
                    BinOp::Sub => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.loc = loc;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq);
                        iseq.push(Inst::SUB);
                    }
                    BinOp::Mul => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.loc = loc;
                        self.save_loc(iseq);
                        iseq.push(Inst::MUL);
                    }
                    BinOp::Div => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.loc = loc;
                        self.save_loc(iseq);
                        iseq.push(Inst::DIV);
                    }
                    BinOp::Shr => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        iseq.push(Inst::SHR);
                    }
                    BinOp::Shl => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq);
                        iseq.push(Inst::SHL);
                    }
                    BinOp::BitOr => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq);
                        iseq.push(Inst::BIT_OR);
                    }
                    BinOp::BitAnd => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq);
                        iseq.push(Inst::BIT_AND);
                    }
                    BinOp::BitXor => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq);
                        iseq.push(Inst::BIT_XOR);
                    }
                    BinOp::Eq => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq);
                        iseq.push(Inst::EQ);
                    }
                    BinOp::Ne => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq);
                        iseq.push(Inst::NE);
                    }
                    BinOp::Ge => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq);
                        iseq.push(Inst::GE);
                    }
                    BinOp::Gt => {
                        self.gen(globals, iseq, lhs, true)?;
                        self.gen(globals, iseq, rhs, true)?;
                        self.save_loc(iseq);
                        iseq.push(Inst::GT);
                    }
                    BinOp::Le => {
                        self.gen(globals, iseq, rhs, true)?;
                        self.gen(globals, iseq, lhs, true)?;
                        self.save_loc(iseq);
                        iseq.push(Inst::GE);
                    }
                    BinOp::Lt => {
                        self.gen(globals, iseq, rhs, true)?;
                        self.gen(globals, iseq, lhs, true)?;
                        self.save_loc(iseq);
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
                        let src1 = self.gen_jmp_if_false(iseq);
                        iseq.push(Inst::PUSH_TRUE);
                        let src2 = self.gen_jmp(iseq);
                        self.write_disp_from_cur(iseq, src1);
                        self.gen(globals, iseq, rhs, true)?;
                        self.write_disp_from_cur(iseq, src2);
                    }
                }
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::ArrayMember { array, index } => {
                // number of index elements must be 1 or 2 (ensured by parser).
                self.gen(globals, iseq, array, true)?;
                let num_args = index.len();
                for i in index {
                    self.gen(globals, iseq, i, true)?;
                }
                self.gen_get_array_elem(iseq, num_args);
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
                let (start, end, exclude) = match &iter.kind {
                    NodeKind::Range {
                        start,
                        end,
                        exclude_end,
                    } => (start, end, exclude_end),
                    _ => return Err(self.error_syntax("Expected Range.", iter.loc())),
                };
                self.loop_stack.push(vec![]);
                self.gen(globals, iseq, start, true)?;
                self.gen_set_local(iseq, id);
                let loop_start = Codegen::current(iseq);
                self.gen(globals, iseq, end, true)?;
                self.gen_get_local(iseq, id)?;
                iseq.push(if *exclude { Inst::GT } else { Inst::GE });
                let src = self.gen_jmp_if_false(iseq);
                self.gen(globals, iseq, body, false)?;
                let loop_continue = Codegen::current(iseq);
                self.gen_get_local(iseq, id)?;
                self.gen_fixnum(iseq, 1);
                self.gen_add(iseq);

                self.gen_set_local(iseq, id);

                self.gen_jmp_back(iseq, loop_start);
                self.write_disp_from_cur(iseq, src);
                if use_value {
                    self.gen(globals, iseq, iter, true)?;
                }
                for p in self.loop_stack.pop().unwrap() {
                    match p.1 {
                        EscapeKind::Break => {
                            self.write_disp_from_cur(iseq, p.0);
                        }
                        EscapeKind::Next => self.write_disp(iseq, p.0, loop_continue),
                    }
                }
            }
            NodeKind::Assign(lhs, rhs) => {
                self.gen(globals, iseq, rhs, true)?;
                if use_value {
                    self.gen_dup(iseq, 1);
                };
                self.gen_assign(globals, iseq, lhs)?;
            }
            NodeKind::MulAssign(mlhs, mrhs) => {
                let lhs_len = mlhs.len();
                let rhs_len = mrhs.len();
                for rhs in mrhs {
                    self.gen(globals, iseq, rhs, true)?;
                }
                if use_value {
                    self.gen_dup(iseq, rhs_len);
                };
                if rhs_len < lhs_len {
                    for _ in 0..lhs_len - rhs_len {
                        self.gen_push_nil(iseq);
                    }
                }
                if lhs_len < rhs_len {
                    for _ in 0..rhs_len - lhs_len {
                        self.gen_pop(iseq);
                    }
                }
                for lhs in mlhs.iter().rev() {
                    self.gen_assign(globals, iseq, lhs)?;
                }
                if use_value {
                    if rhs_len != 1 {
                        self.gen_create_array(iseq, rhs_len);
                    }
                }
            }
            NodeKind::Send {
                receiver,
                method,
                args,
                ..
            } => {
                let loc = self.loc;
                for arg in args {
                    self.gen(globals, iseq, arg, true)?;
                }
                if NodeKind::SelfValue == receiver.kind {
                    self.loc = loc;
                    self.gen_send_self(iseq, *method, args.len());
                } else {
                    self.gen(globals, iseq, receiver, true)?;
                    self.loc = loc;
                    self.gen_send(iseq, *method, args.len());
                };
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::MethodDef(id, params, body, lvar) => {
                let methodref = self.gen_iseq(globals, params, body, lvar, true, false)?;
                iseq.push(Inst::DEF_METHOD);
                self.push32(iseq, (*id).into());
                self.push32(iseq, methodref.into());
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::ClassMethodDef(id, params, body, lvar) => {
                let methodref = self.gen_iseq(globals, params, body, lvar, true, false)?;
                iseq.push(Inst::DEF_CLASS_METHOD);
                self.push32(iseq, (*id).into());
                self.push32(iseq, methodref.into());
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::ClassDef {
                id,
                superclass,
                body,
                lvar,
            } => {
                let methodref = self.gen_iseq(globals, &vec![], body, lvar, true, false)?;
                self.gen_get_const(iseq, *superclass);
                self.save_loc(iseq);
                iseq.push(Inst::DEF_CLASS);
                self.push32(iseq, (*id).into());
                self.push32(iseq, methodref.into());
                if !use_value {
                    self.gen_pop(iseq)
                };
            }
            NodeKind::Break => {
                if use_value {
                    self.gen_push_nil(iseq);
                };
                let src = self.gen_jmp(iseq);
                match self.loop_stack.last_mut() {
                    Some(x) => {
                        x.push((src, EscapeKind::Break));
                    }
                    None => {
                        return Err(
                            self.error_syntax("Can't escape from eval with break.", self.loc)
                        );
                    }
                }
            }
            NodeKind::Next => {
                if use_value {
                    self.gen_push_nil(iseq);
                }
                let src = self.gen_jmp(iseq);
                match self.loop_stack.last_mut() {
                    Some(x) => {
                        x.push((src, EscapeKind::Next));
                    }
                    None => {
                        return Err(
                            self.error_syntax("Can't escape from eval with next.", self.loc)
                        );
                    }
                }
            }
            NodeKind::Proc { params, body, lvar } => {
                let methodref = self.gen_iseq(globals, params, body, lvar, true, true)?;
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
        RubyError::new_parse_err(ParseErrKind::SyntaxError(msg.into()), loc)
    }
    pub fn error_name(&self, msg: impl Into<String>) -> RubyError {
        RubyError::new_runtime_err(RuntimeErrKind::Name(msg.into()), self.loc)
    }
}
