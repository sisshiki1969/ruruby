use crate::error::{ParseErrKind, RubyError, RuntimeErrKind};
use crate::node::{BinOp, Node, NodeKind};
use crate::vm::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Codegen {
    // Codegen State
    //pub class_stack: Vec<IdentId>,
    pub loop_stack: Vec<Vec<(ISeqPos, EscapeKind)>>,
    pub lvar_table: HashMap<IdentId, LvarId>,
    pub loc: Loc,
    pub iseq_info: Vec<(ISeqPos, Loc)>,
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
    fn from_usize(pos: usize) -> Self {
        ISeqPos(pos)
    }

    fn disp(&self, dist: ISeqPos) -> i32 {
        let dist = dist.0 as i64;
        (dist - (self.0 as i64)) as i32
    }
}

impl Codegen {
    pub fn new(lvar_collector: Option<LvarCollector>) -> Self {
        Codegen {
            lvar_table: match lvar_collector {
                Some(collector) => collector.table,
                None => HashMap::new(),
            },
            //class_stack: vec![],
            loop_stack: vec![],
            loc: Loc(0, 0),
            iseq_info: vec![],
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

    fn gen_fixnum(&mut self, iseq: &mut ISeq, num: i64) {
        iseq.push(Inst::PUSH_FIXNUM);
        self.push64(iseq, num as u64);
    }

    fn gen_string(&mut self, globals: &mut Globals, iseq: &mut ISeq, s: &String) {
        iseq.push(Inst::PUSH_STRING);
        let id = globals.get_ident_id(&s);
        self.push32(iseq, id.into());
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
        let lvar_id = self.lvar_table.get(&id).unwrap().as_usize();
        self.push32(iseq, lvar_id as u32);
    }

    fn gen_set_const(&mut self, iseq: &mut ISeq, id: IdentId) {
        iseq.push(Inst::SET_CONST);
        self.push32(iseq, id.into());
    }

    fn gen_get_local(&mut self, iseq: &mut ISeq, id: IdentId) -> Result<(), RubyError> {
        iseq.push(Inst::GET_LOCAL);
        let lvar_id = match self.lvar_table.get(&id) {
            Some(x) => x,
            None => return Err(self.error_name("undefined local variable.")),
        }
        .as_usize();
        self.push32(iseq, lvar_id as u32);
        Ok(())
    }

    fn gen_get_instance_var(&mut self, iseq: &mut ISeq, id: IdentId) {
        iseq.push(Inst::GET_INSTANCE_VAR);
        self.push32(iseq, id.into());
    }

    fn gen_set_instance_var(&mut self, iseq: &mut ISeq, id: IdentId) {
        iseq.push(Inst::SET_INSTANCE_VAR);
        self.push32(iseq, id.into());
    }

    fn gen_get_const(&mut self, iseq: &mut ISeq, id: IdentId) {
        iseq.push(Inst::GET_CONST);
        self.push32(iseq, id.into());
    }

    fn gen_send(&mut self, iseq: &mut ISeq, method: IdentId, args_num: usize) {
        iseq.push(Inst::SEND);
        self.push32(iseq, method.into());
        self.push32(iseq, args_num as u32);
    }

    fn gen_pop(&mut self, iseq: &mut ISeq) {
        iseq.push(Inst::POP);
    }

    fn gen_concat(&mut self, iseq: &mut ISeq) {
        iseq.push(Inst::CONCAT_STRING);
    }

    fn gen_comp_stmt(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        nodes: &Vec<Node>,
    ) -> Result<(), RubyError> {
        match nodes.len() {
            0 => self.gen_push_nil(iseq),
            1 => self.gen(globals, iseq, &nodes[0])?,
            _ => {
                let mut flag = false;
                for node in nodes {
                    if flag {
                        self.gen_pop(iseq);
                    } else {
                        flag = true;
                    };
                    self.gen(globals, iseq, &node)?;
                }
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
        iseq[src.0 - 4] = (num >> 24) as u8;
        iseq[src.0 - 3] = (num >> 16) as u8;
        iseq[src.0 - 2] = (num >> 8) as u8;
        iseq[src.0 - 1] = num as u8;
    }

    fn push32(&mut self, iseq: &mut ISeq, num: u32) {
        iseq.push((num >> 24) as u8);
        iseq.push((num >> 16) as u8);
        iseq.push((num >> 8) as u8);
        iseq.push(num as u8);
    }

    fn push64(&mut self, iseq: &mut ISeq, num: u64) {
        iseq.push((num >> 56) as u8);
        iseq.push((num >> 48) as u8);
        iseq.push((num >> 40) as u8);
        iseq.push((num >> 32) as u8);
        iseq.push((num >> 24) as u8);
        iseq.push((num >> 16) as u8);
        iseq.push((num >> 8) as u8);
        iseq.push(num as u8);
    }
    fn save_loc(&mut self, iseq: &mut ISeq) {
        self.iseq_info.push((ISeqPos(iseq.len()), self.loc));
    }

    /// Generate ISeq.
    pub fn gen_iseq(&mut self, globals: &mut Globals, node: &Node) -> Result<ISeqRef, RubyError> {
        let mut iseq = ISeq::new();
        self.gen(globals, &mut iseq, node)?;
        iseq.push(Inst::END);
        Ok(ISeqRef::new(iseq))
    }

    pub fn gen_method_iseq(
        &mut self,
        globals: &mut Globals,
        params: &Vec<Node>,
        node: &Node,
        lvar_collector: &LvarCollector,
    ) -> Result<MethodInfo, RubyError> {
        //println!("PARAMS: {:?}", params);
        //println!("LVARS: {:?}", lvar_collector);
        let mut params_lvar = vec![];
        for param in params {
            match param.kind {
                NodeKind::Param(id) => {
                    //println!("param IdentId:{:?}", id);
                    let lvar = lvar_collector.table.get(&id).unwrap();
                    //println!("param LvarId:{:?}", lvar);
                    params_lvar.push(*lvar);
                }
                _ => return Err(self.error_syntax("Parameters should be identifier.", self.loc)),
            }
        }
        let mut iseq = ISeq::new();
        let mut new_lvar = lvar_collector.table.clone();
        std::mem::swap(&mut self.lvar_table, &mut new_lvar);
        self.gen(globals, &mut iseq, node)?;
        std::mem::swap(&mut self.lvar_table, &mut new_lvar);
        iseq.push(Inst::END);
        let lvars = lvar_collector.table.len();
        //println!("{:?}", iseq);
        Ok(MethodInfo::RubyFunc {
            iseq: ISeqRef::new(iseq),
            params: params_lvar,
            lvars,
        })
    }

    pub fn gen(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        node: &Node,
    ) -> Result<(), RubyError> {
        self.loc = node.loc();
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
            NodeKind::InterporatedString(nodes) => {
                self.gen_string(globals, iseq, &"".to_string());
                for node in nodes {
                    match &node.kind {
                        NodeKind::String(s) => {
                            self.gen_string(globals, iseq, &s);
                        }
                        NodeKind::CompStmt(nodes) => {
                            self.gen_comp_stmt(globals, iseq, nodes)?;
                            iseq.push(Inst::TO_S);
                        }
                        _ => unimplemented!("Illegal arguments in Nodekind::InterporatedString."),
                    }
                    self.gen_concat(iseq);
                }
            }
            NodeKind::SelfValue => {
                iseq.push(Inst::PUSH_SELF);
            }
            NodeKind::Range(start, end, exclude) => {
                if *exclude {
                    iseq.push(Inst::PUSH_TRUE);
                } else {
                    iseq.push(Inst::PUSH_FALSE)
                };
                self.gen(globals, iseq, end)?;
                self.gen(globals, iseq, start)?;
                self.save_loc(iseq);
                iseq.push(Inst::CREATE_RANGE);
            }
            NodeKind::Ident(id) => {
                self.save_loc(iseq);
                self.gen_get_local(iseq, *id)?;
            }
            NodeKind::Const(id) => self.gen_get_const(iseq, *id),
            NodeKind::InstanceVar(id) => self.gen_get_instance_var(iseq, *id),
            NodeKind::BinOp(op, lhs, rhs) => match op {
                BinOp::Add => {
                    self.gen(globals, iseq, lhs)?;
                    self.gen(globals, iseq, rhs)?;
                    iseq.push(Inst::ADD);
                }
                BinOp::Sub => {
                    self.gen(globals, iseq, lhs)?;
                    self.gen(globals, iseq, rhs)?;
                    iseq.push(Inst::SUB);
                }
                BinOp::Mul => {
                    self.gen(globals, iseq, lhs)?;
                    self.gen(globals, iseq, rhs)?;
                    iseq.push(Inst::MUL);
                }
                BinOp::Div => {
                    self.gen(globals, iseq, lhs)?;
                    self.gen(globals, iseq, rhs)?;
                    iseq.push(Inst::DIV);
                }
                BinOp::Shr => {
                    self.gen(globals, iseq, lhs)?;
                    self.gen(globals, iseq, rhs)?;
                    iseq.push(Inst::SHR);
                }
                BinOp::Shl => {
                    self.gen(globals, iseq, lhs)?;
                    self.gen(globals, iseq, rhs)?;
                    iseq.push(Inst::SHL);
                }
                BinOp::BitOr => {
                    self.gen(globals, iseq, lhs)?;
                    self.gen(globals, iseq, rhs)?;
                    iseq.push(Inst::BIT_OR);
                }
                BinOp::BitAnd => {
                    self.gen(globals, iseq, lhs)?;
                    self.gen(globals, iseq, rhs)?;
                    iseq.push(Inst::BIT_AND);
                }
                BinOp::BitXor => {
                    self.gen(globals, iseq, lhs)?;
                    self.gen(globals, iseq, rhs)?;
                    iseq.push(Inst::BIT_XOR);
                }
                BinOp::Eq => {
                    self.gen(globals, iseq, lhs)?;
                    self.gen(globals, iseq, rhs)?;
                    iseq.push(Inst::EQ);
                }
                BinOp::Ne => {
                    self.gen(globals, iseq, lhs)?;
                    self.gen(globals, iseq, rhs)?;
                    iseq.push(Inst::NE);
                }
                BinOp::Ge => {
                    self.gen(globals, iseq, lhs)?;
                    self.gen(globals, iseq, rhs)?;
                    iseq.push(Inst::GE);
                }
                BinOp::Gt => {
                    self.gen(globals, iseq, lhs)?;
                    self.gen(globals, iseq, rhs)?;
                    iseq.push(Inst::GT);
                }
                BinOp::Le => {
                    self.gen(globals, iseq, rhs)?;
                    self.gen(globals, iseq, lhs)?;
                    iseq.push(Inst::GE);
                }
                BinOp::Lt => {
                    self.gen(globals, iseq, rhs)?;
                    self.gen(globals, iseq, lhs)?;
                    iseq.push(Inst::GT);
                }
                BinOp::LAnd => {
                    self.gen(globals, iseq, lhs)?;
                    let src1 = self.gen_jmp_if_false(iseq);
                    self.gen(globals, iseq, rhs)?;
                    let src2 = self.gen_jmp(iseq);
                    self.write_disp_from_cur(iseq, src1);
                    iseq.push(Inst::PUSH_FALSE);
                    self.write_disp_from_cur(iseq, src2);
                }
                BinOp::LOr => {
                    self.gen(globals, iseq, lhs)?;
                    let src1 = self.gen_jmp_if_false(iseq);
                    iseq.push(Inst::PUSH_TRUE);
                    let src2 = self.gen_jmp(iseq);
                    self.write_disp_from_cur(iseq, src1);
                    self.gen(globals, iseq, rhs)?;
                    self.write_disp_from_cur(iseq, src2);
                }
            },
            NodeKind::CompStmt(nodes) => self.gen_comp_stmt(globals, iseq, nodes)?,
            NodeKind::If(cond_, then_, else_) => {
                self.gen(globals, iseq, &cond_)?;
                let src1 = self.gen_jmp_if_false(iseq);
                self.gen(globals, iseq, &then_)?;
                let src2 = self.gen_jmp(iseq);
                self.write_disp_from_cur(iseq, src1);
                self.gen(globals, iseq, &else_)?;
                self.write_disp_from_cur(iseq, src2);
            }
            NodeKind::For(id, iter, body) => {
                let id = match id.kind {
                    NodeKind::Ident(id) => id,
                    _ => return Err(self.error_syntax("Expected an identifier.", id.loc())),
                };
                let (start, end, exclude) = match &iter.kind {
                    NodeKind::Range(start, end, exclude) => (start, end, exclude),
                    _ => return Err(self.error_syntax("Expected Range.", iter.loc())),
                };
                self.loop_stack.push(vec![]);
                self.gen(globals, iseq, start)?;
                self.gen_set_local(iseq, id);
                self.gen_pop(iseq);
                let loop_start = Codegen::current(iseq);
                self.gen(globals, iseq, end)?;
                self.gen_get_local(iseq, id)?;
                iseq.push(if *exclude { Inst::GT } else { Inst::GE });
                let src = self.gen_jmp_if_false(iseq);
                self.gen(globals, iseq, body)?;
                self.gen_pop(iseq);
                let loop_continue = Codegen::current(iseq);
                self.gen_get_local(iseq, id)?;
                self.gen_fixnum(iseq, 1);
                iseq.push(Inst::ADD);
                self.gen_set_local(iseq, id);
                self.gen_pop(iseq);

                self.gen_jmp_back(iseq, loop_start);
                self.write_disp_from_cur(iseq, src);
                self.gen(globals, iseq, iter)?;
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
                self.gen(globals, iseq, rhs)?;
                match lhs.kind {
                    NodeKind::Ident(id) => self.gen_set_local(iseq, id),
                    NodeKind::Const(id) => self.gen_set_const(iseq, id),
                    NodeKind::InstanceVar(id) => self.gen_set_instance_var(iseq, id),
                    _ => (),
                }
            }
            NodeKind::Send(receiver, method, args) => {
                let id = match method.kind {
                    NodeKind::Ident(id) => id,
                    _ => {
                        return Err(self.error_syntax(format!("Expected identifier."), method.loc()))
                    }
                };
                for arg in args.iter().rev() {
                    self.gen(globals, iseq, arg)?;
                }
                self.gen(globals, iseq, receiver)?;
                self.save_loc(iseq);
                self.gen_send(iseq, id, args.len());
            }
            NodeKind::MethodDef(id, params, body, lvar) => {
                let info = self.gen_method_iseq(globals, params, body, lvar)?;
                let methodref = globals.add_method(info);
                iseq.push(Inst::DEF_METHOD);
                self.push32(iseq, (*id).into());
                self.push32(iseq, methodref.into());
            }
            NodeKind::ClassMethodDef(id, params, body, lvar) => {
                let info = self.gen_method_iseq(globals, params, body, lvar)?;
                let methodref = globals.add_method(info);
                iseq.push(Inst::DEF_CLASS_METHOD);
                self.push32(iseq, (*id).into());
                self.push32(iseq, methodref.into());
            }
            NodeKind::ClassDef(id, node, lvar) => {
                let info = self.gen_method_iseq(globals, &vec![], node, lvar)?;
                let methodref = globals.add_method(info);
                iseq.push(Inst::DEF_CLASS);
                self.push32(iseq, (*id).into());
                self.push32(iseq, methodref.into());
            }
            NodeKind::Break => {
                self.gen_push_nil(iseq);
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
                self.gen_push_nil(iseq);
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
            _ => unimplemented!("{:?}", node.kind),
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
