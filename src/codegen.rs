use crate::class::*;
use crate::error::{ParseErrKind, RubyError, RuntimeErrKind};
use crate::node::{BinOp, Node, NodeKind};
use crate::parser::{LvarCollector, LvarId};
use crate::util::{IdentId, IdentifierTable, Loc};
use crate::value::Value;
use crate::vm::{Inst, VMResult, VM};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Codegen {
    pub ident_table: IdentifierTable,
    pub method_table: MethodTable,
    // Codegen State
    pub class_stack: Vec<ClassRef>,
    pub loop_stack: Vec<Vec<(ISeqPos, EscapeKind)>>,
    pub lvar_table: HashMap<IdentId, LvarId>,
    pub loc: Loc,
    pub iseq_info: Vec<(ISeqPos, Loc)>,
}

pub type BuiltinFunc = fn(vm: &mut VM, receiver: Value, args: Vec<Value>) -> VMResult;

pub type MethodTable = HashMap<IdentId, MethodInfo>;

#[derive(Clone)]
pub enum MethodInfo {
    RubyFunc { params: Vec<LvarId>, iseq: ISeq },
    BuiltinFunc { name: String, func: BuiltinFunc },
}

impl std::fmt::Debug for MethodInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MethodInfo::RubyFunc { params, .. } => write!(f, "RubyFunc {:?}", params),
            MethodInfo::BuiltinFunc { name, .. } => write!(f, "BuiltinFunc {:?}", name),
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
            ident_table: IdentifierTable::new(),
            method_table: MethodTable::new(),
            lvar_table: match lvar_collector {
                Some(collector) => collector.table,
                None => HashMap::new(),
            },
            class_stack: vec![],
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
        self.push32(iseq, id.as_usize() as u32);
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

    fn gen_get_const(&mut self, iseq: &mut ISeq, id: IdentId) {
        iseq.push(Inst::GET_CONST);
        self.push32(iseq, id.as_usize() as u32);
    }

    fn gen_send(&mut self, iseq: &mut ISeq, method: IdentId, args_num: usize) {
        iseq.push(Inst::SEND);
        self.push32(iseq, method.as_usize() as u32);
        self.push32(iseq, args_num as u32);
    }

    fn gen_pop(&mut self, iseq: &mut ISeq) {
        iseq.push(Inst::POP);
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
    pub fn gen_iseq(&mut self, node: &Node) -> Result<ISeq, RubyError> {
        let mut iseq = ISeq::new();
        self.gen(&mut iseq, node)?;
        iseq.push(Inst::END);
        Ok(iseq)
    }

    pub fn gen_method_iseq(
        &mut self,
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
        self.gen(&mut iseq, node)?;
        std::mem::swap(&mut self.lvar_table, &mut new_lvar);
        iseq.push(Inst::END);
        //println!("{:?}", iseq);
        Ok(MethodInfo::RubyFunc {
            iseq,
            params: params_lvar,
        })
    }

    pub fn gen(&mut self, iseq: &mut ISeq, node: &Node) -> Result<(), RubyError> {
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
                iseq.push(Inst::PUSH_STRING);
                let id = self.ident_table.get_ident_id(s);
                self.push32(iseq, id.as_usize() as u32);
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
                self.gen(iseq, end)?;
                self.gen(iseq, start)?;
                self.save_loc(iseq);
                iseq.push(Inst::CREATE_RANGE);
            }
            NodeKind::Ident(id) => {
                self.save_loc(iseq);
                self.gen_get_local(iseq, *id)?;
            }
            NodeKind::Const(id) => {
                self.gen_get_const(iseq, *id);
            }
            NodeKind::BinOp(op, lhs, rhs) => match op {
                BinOp::Add => {
                    self.gen(iseq, lhs)?;
                    self.gen(iseq, rhs)?;
                    iseq.push(Inst::ADD);
                }
                BinOp::Sub => {
                    self.gen(iseq, lhs)?;
                    self.gen(iseq, rhs)?;
                    iseq.push(Inst::SUB);
                }
                BinOp::Mul => {
                    self.gen(iseq, lhs)?;
                    self.gen(iseq, rhs)?;
                    iseq.push(Inst::MUL);
                }
                BinOp::Div => {
                    self.gen(iseq, lhs)?;
                    self.gen(iseq, rhs)?;
                    iseq.push(Inst::DIV);
                }
                BinOp::Shr => {
                    self.gen(iseq, lhs)?;
                    self.gen(iseq, rhs)?;
                    iseq.push(Inst::SHR);
                }
                BinOp::Shl => {
                    self.gen(iseq, lhs)?;
                    self.gen(iseq, rhs)?;
                    iseq.push(Inst::SHL);
                }
                BinOp::BitOr => {
                    self.gen(iseq, lhs)?;
                    self.gen(iseq, rhs)?;
                    iseq.push(Inst::BIT_OR);
                }
                BinOp::BitAnd => {
                    self.gen(iseq, lhs)?;
                    self.gen(iseq, rhs)?;
                    iseq.push(Inst::BIT_AND);
                }
                BinOp::BitXor => {
                    self.gen(iseq, lhs)?;
                    self.gen(iseq, rhs)?;
                    iseq.push(Inst::BIT_XOR);
                }
                BinOp::Eq => {
                    self.gen(iseq, lhs)?;
                    self.gen(iseq, rhs)?;
                    iseq.push(Inst::EQ);
                }
                BinOp::Ne => {
                    self.gen(iseq, lhs)?;
                    self.gen(iseq, rhs)?;
                    iseq.push(Inst::NE);
                }
                BinOp::Ge => {
                    self.gen(iseq, lhs)?;
                    self.gen(iseq, rhs)?;
                    iseq.push(Inst::GE);
                }
                BinOp::Gt => {
                    self.gen(iseq, lhs)?;
                    self.gen(iseq, rhs)?;
                    iseq.push(Inst::GT);
                }
                BinOp::Le => {
                    self.gen(iseq, rhs)?;
                    self.gen(iseq, lhs)?;
                    iseq.push(Inst::GE);
                }
                BinOp::Lt => {
                    self.gen(iseq, rhs)?;
                    self.gen(iseq, lhs)?;
                    iseq.push(Inst::GT);
                }
                BinOp::LAnd => {
                    self.gen(iseq, lhs)?;
                    let src1 = self.gen_jmp_if_false(iseq);
                    self.gen(iseq, rhs)?;
                    let src2 = self.gen_jmp(iseq);
                    self.write_disp_from_cur(iseq, src1);
                    iseq.push(Inst::PUSH_FALSE);
                    self.write_disp_from_cur(iseq, src2);
                }
                BinOp::LOr => {
                    self.gen(iseq, lhs)?;
                    let src1 = self.gen_jmp_if_false(iseq);
                    iseq.push(Inst::PUSH_TRUE);
                    let src2 = self.gen_jmp(iseq);
                    self.write_disp_from_cur(iseq, src1);
                    self.gen(iseq, rhs)?;
                    self.write_disp_from_cur(iseq, src2);
                }
            },
            NodeKind::CompStmt(nodes) => match nodes.len() {
                0 => self.gen_push_nil(iseq),
                1 => self.gen(iseq, &nodes[0])?,
                _ => {
                    let mut flag = false;
                    for node in nodes {
                        if flag {
                            self.gen_pop(iseq);
                        } else {
                            flag = true;
                        };
                        self.gen(iseq, &node)?;
                    }
                }
            },
            NodeKind::If(cond_, then_, else_) => {
                self.gen(iseq, &cond_)?;
                let src1 = self.gen_jmp_if_false(iseq);
                self.gen(iseq, &then_)?;
                let src2 = self.gen_jmp(iseq);
                self.write_disp_from_cur(iseq, src1);
                self.gen(iseq, &else_)?;
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
                self.gen(iseq, start)?;
                self.gen_set_local(iseq, id);
                self.gen_pop(iseq);
                let loop_start = Codegen::current(iseq);
                self.gen(iseq, end)?;
                self.gen_get_local(iseq, id)?;
                iseq.push(if *exclude { Inst::GT } else { Inst::GE });
                let src = self.gen_jmp_if_false(iseq);
                self.gen(iseq, body)?;
                self.gen_pop(iseq);
                let loop_continue = Codegen::current(iseq);
                self.gen_get_local(iseq, id)?;
                self.gen_fixnum(iseq, 1);
                iseq.push(Inst::ADD);
                self.gen_set_local(iseq, id);
                self.gen_pop(iseq);

                self.gen_jmp_back(iseq, loop_start);
                self.write_disp_from_cur(iseq, src);
                self.gen(iseq, iter)?;
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
                self.gen(iseq, rhs)?;
                match lhs.kind {
                    NodeKind::Ident(id) => {
                        self.gen_set_local(iseq, id);
                    }
                    NodeKind::Const(id) => {
                        self.gen_set_const(iseq, id);
                    }
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
                    self.gen(iseq, arg)?;
                }
                self.gen(iseq, receiver)?;
                self.save_loc(iseq);
                self.gen_send(iseq, id, args.len());
            }
            NodeKind::MethodDecl(id, params, body, lvar_collector) => {
                let info = self.gen_method_iseq(params, body, lvar_collector)?;
                //if self.class_stack.len() == 1 {
                // A method defined in "top level" is registered to the global method table.
                self.method_table.insert(*id, info);
                //} else {
                /*
                // A method defined in a class definition is registered as a instance method of the class.
                let class = self.class_stack.last().unwrap();
                let class_info = self.class_table.get_mut(*class);
                class_info.instance_method.insert(*id, info);
                */
                //}
                self.gen_push_nil(iseq);
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
