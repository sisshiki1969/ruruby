use crate::class::*;
use crate::eval::{RuntimeErrKind, RuntimeError};
use crate::instance::*;
use crate::node::*;
use crate::util::*;
use crate::value::*;
use std::collections::HashMap;

pub type ValueTable = HashMap<IdentId, Value>;
pub type BuiltinFunc = fn(eval: &mut VM, receiver: Value, args: Vec<Value>) -> EvalResult;
pub type ISeq = Vec<u8>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ISeqPos(usize);

impl ISeqPos {
    fn disp(&self, dist: ISeqPos) -> i32 {
        let dist = dist.0 as i64;
        (dist - (self.0 as i64)) as i32
    }
}

#[derive(Clone)]
pub enum MethodInfo {
    RubyFunc { params: Vec<Node>, body: Box<Node> },
    BuiltinFunc { name: String, func: BuiltinFunc },
}

impl std::fmt::Debug for MethodInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MethodInfo::RubyFunc { params, body } => write!(f, "RubyFunc {:?} {:?}", params, body),
            MethodInfo::BuiltinFunc { name, .. } => write!(f, "BuiltinFunc {:?}", name),
        }
    }
}

pub type MethodTable = HashMap<IdentId, MethodInfo>;

#[derive(Debug, Clone, PartialEq)]
pub struct LocalScope {
    lvar_table: ValueTable,
}

impl LocalScope {
    pub fn new() -> Self {
        LocalScope {
            lvar_table: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EscapeKind {
    Break,
    Next,
}

pub type EvalResult = Result<Value, RuntimeError>;

#[derive(Debug, Clone)]
pub struct VM {
    // Global info
    pub source_info: SourceInfo,
    pub ident_table: IdentifierTable,
    pub class_table: GlobalClassTable,
    pub instance_table: GlobalInstanceTable,
    pub method_table: MethodTable,
    pub const_table: ValueTable,
    // Codegen State
    pub class_stack: Vec<ClassRef>,
    pub loop_stack: Vec<Vec<(ISeqPos, EscapeKind)>>,
    pub loc: Loc,
    // VM state
    pub iseq: ISeq,
    pub scope_stack: Vec<LocalScope>,
}

pub struct Inst;
impl Inst {
    const END: u8 = 0;
    const PUSH_FIXNUM: u8 = 1;
    const PUSH_FLONUM: u8 = 2;
    const ADD: u8 = 3;
    const SUB: u8 = 4;
    const MUL: u8 = 5;
    const DIV: u8 = 6;
    const EQ: u8 = 7;
    const NE: u8 = 8;
    const GT: u8 = 9;
    const GE: u8 = 10;
    const PUSH_TRUE: u8 = 11;
    const PUSH_FALSE: u8 = 12;
    const PUSH_NIL: u8 = 13;
    const SHR: u8 = 14;
    const SHL: u8 = 15;
    const BIT_OR: u8 = 16;
    const BIT_AND: u8 = 17;
    const BIT_XOR: u8 = 18;
    const JMP: u8 = 19;
    const JMP_IF_FALSE: u8 = 20;
    const SET_LOCAL: u8 = 21;
    const GET_LOCAL: u8 = 22;
    const SEND: u8 = 23;
    const PUSH_SELF: u8 = 24;
    const CREATE_RANGE: u8 = 25;
}

impl VM {
    pub fn new(source_info: SourceInfo, ident_table: IdentifierTable) -> Self {
        let vm = VM {
            iseq: vec![],
            source_info,
            ident_table,
            class_table: GlobalClassTable::new(),
            instance_table: GlobalInstanceTable::new(),
            method_table: HashMap::new(),
            const_table: HashMap::new(),
            // for codegen
            class_stack: vec![],
            scope_stack: vec![LocalScope::new()],
            loop_stack: vec![],
            loc: Loc(0, 0),
        };
        vm
    }

    pub fn init(&mut self, source_info: SourceInfo, ident_table: IdentifierTable) {
        self.source_info = source_info;
        self.ident_table = ident_table;
    }

    /// Get local variable table.
    pub fn lvar_table(&mut self) -> &mut ValueTable {
        &mut self.scope_stack.last_mut().unwrap().lvar_table
    }

    pub fn run(&mut self, node: &Node) -> EvalResult {
        self.iseq.clear();
        //println!("{:?}", node);
        match self.gen(node) {
            Ok(_) => {}
            Err(err) => {
                self.source_info.show_loc(&err.loc);
                println!("{:?}", err);
                return Err(err);
            }
        }
        self.iseq.push(Inst::END);
        let val = match self.vm_run() {
            Ok(v) => v,
            Err(err) => {
                self.source_info.show_loc(&err.loc);
                println!("{:?}", err);
                return Err(err);
            }
        };
        Ok(val)
    }

    pub fn vm_run(&mut self) -> EvalResult {
        let mut stack: Vec<Value> = vec![];
        let mut pc = 0;
        loop {
            match self.iseq[pc] {
                Inst::END => match stack.pop() {
                    Some(v) => return Ok(v),
                    None => return Ok(Value::Nil),
                },
                Inst::PUSH_NIL => {
                    stack.push(Value::Nil);
                    pc += 1;
                    #[cfg(debug_assertions)]
                    println!("PUSH_NIL");
                }
                Inst::PUSH_TRUE => {
                    stack.push(Value::Bool(true));
                    pc += 1;
                    #[cfg(debug_assertions)]
                    println!("PUSH_TRUE");
                }
                Inst::PUSH_FALSE => {
                    stack.push(Value::Bool(false));
                    pc += 1;
                    #[cfg(debug_assertions)]
                    println!("PUSH_FALSE");
                }
                Inst::PUSH_SELF => {
                    stack.push(Value::Nil);
                    pc += 1;
                    #[cfg(debug_assertions)]
                    println!("PUSH_SELF");
                }
                Inst::PUSH_FIXNUM => {
                    let num = read64(&self.iseq, pc + 1);
                    pc += 9;
                    stack.push(Value::FixNum(num as i64));
                    #[cfg(debug_assertions)]
                    println!("PUSH_FIXNUM {}", num as i64);
                }
                Inst::PUSH_FLONUM => {
                    let num = unsafe { std::mem::transmute(read64(&self.iseq, pc + 1)) };
                    pc += 9;
                    stack.push(Value::FloatNum(num));
                    #[cfg(debug_assertions)]
                    println!("PUSH_FLOAT {}", num);
                }

                Inst::ADD => {
                    let val = self.eval_add(stack.pop().unwrap(), stack.pop().unwrap())?;
                    stack.push(val);
                    pc += 1;
                    #[cfg(debug_assertions)]
                    println!("ADD");
                }
                Inst::SUB => {
                    let val = self.eval_sub(stack.pop().unwrap(), stack.pop().unwrap())?;
                    pc += 1;
                    stack.push(val);
                    #[cfg(debug_assertions)]
                    println!("SUB");
                }
                Inst::MUL => {
                    let val = self.eval_mul(stack.pop().unwrap(), stack.pop().unwrap())?;
                    pc += 1;
                    stack.push(val);
                    #[cfg(debug_assertions)]
                    println!("MUL");
                }
                Inst::DIV => {
                    let val = self.eval_div(stack.pop().unwrap(), stack.pop().unwrap())?;
                    pc += 1;
                    stack.push(val);
                    #[cfg(debug_assertions)]
                    println!("DIV");
                }
                Inst::SHR => {
                    let val = self.eval_shr(stack.pop().unwrap(), stack.pop().unwrap())?;
                    pc += 1;
                    stack.push(val);
                    #[cfg(debug_assertions)]
                    println!("SHR");
                }
                Inst::SHL => {
                    let val = self.eval_shl(stack.pop().unwrap(), stack.pop().unwrap())?;
                    pc += 1;
                    stack.push(val);
                    #[cfg(debug_assertions)]
                    println!("SHL");
                }
                Inst::BIT_AND => {
                    let val = self.eval_bitand(stack.pop().unwrap(), stack.pop().unwrap())?;
                    pc += 1;
                    stack.push(val);
                    #[cfg(debug_assertions)]
                    println!("BIT_AND");
                }
                Inst::BIT_OR => {
                    let val = self.eval_bitor(stack.pop().unwrap(), stack.pop().unwrap())?;
                    pc += 1;
                    stack.push(val);
                    #[cfg(debug_assertions)]
                    println!("BIT_OR");
                }
                Inst::BIT_XOR => {
                    let val = self.eval_bitxor(stack.pop().unwrap(), stack.pop().unwrap())?;
                    pc += 1;
                    stack.push(val);
                    #[cfg(debug_assertions)]
                    println!("BIT_XOR");
                }
                Inst::EQ => {
                    let val = self.eval_eq(stack.pop().unwrap(), stack.pop().unwrap())?;
                    pc += 1;
                    stack.push(val);
                    #[cfg(debug_assertions)]
                    println!("EQ");
                }
                Inst::NE => {
                    let val = self.eval_neq(stack.pop().unwrap(), stack.pop().unwrap())?;
                    pc += 1;
                    stack.push(val);
                    #[cfg(debug_assertions)]
                    println!("NE");
                }
                Inst::GT => {
                    let val = self.eval_gt(stack.pop().unwrap(), stack.pop().unwrap())?;
                    pc += 1;
                    stack.push(val);
                    #[cfg(debug_assertions)]
                    println!("GT");
                }
                Inst::GE => {
                    let val = self.eval_ge(stack.pop().unwrap(), stack.pop().unwrap())?;
                    pc += 1;
                    stack.push(val);
                    #[cfg(debug_assertions)]
                    println!("GE");
                }
                Inst::SET_LOCAL => {
                    let id = IdentId::from_usize(read32(&self.iseq, pc + 1) as usize);
                    let val = stack.last().unwrap().clone();
                    self.lvar_table().insert(id, val);
                    pc += 5;
                    #[cfg(debug_assertions)]
                    println!("SET_LOCAL {}", self.ident_table.get_name(id));
                }
                Inst::GET_LOCAL => {
                    let id = IdentId::from_usize(read32(&self.iseq, pc + 1) as usize);
                    let val = match self.lvar_table().get(&id) {
                        Some(val) => val,
                        None => return Err(self.error_nomethod("undefined local variable.")),
                    };
                    stack.push(val.clone());
                    pc += 5;
                    #[cfg(debug_assertions)]
                    println!("GET_LOCAL {}", self.ident_table.get_name(id));
                }
                Inst::CREATE_RANGE => {
                    let start = stack.pop().unwrap();
                    let end = stack.pop().unwrap();
                    let exclude = stack.pop().unwrap();
                    let range =
                        Value::Range(Box::new(start), Box::new(end), self.val_to_bool(&exclude));
                    stack.push(range);
                    pc += 1;
                }
                Inst::JMP => {
                    let disp = read32(&self.iseq, pc + 1) as i32 as i64;
                    pc = ((pc as i64) + 5 + disp) as usize;
                    #[cfg(debug_assertions)]
                    println!("JMP {}", disp);
                }
                Inst::JMP_IF_FALSE => {
                    let val = stack.pop().unwrap();
                    if self.val_to_bool(&val) {
                        pc += 5;
                        #[cfg(debug_assertions)]
                        println!("JMP_IF_FALSE: NO JMP");
                    } else {
                        let disp = read32(&self.iseq, pc + 1) as i32 as i64;
                        pc = ((pc as i64) + 5 + disp) as usize;
                        #[cfg(debug_assertions)]
                        println!("JMP_IF_FALSE: JMP{}", disp);
                    }
                }
                Inst::SEND => {
                    let receiver = stack.pop().unwrap();
                    let method_id = IdentId::from_usize(read32(&self.iseq, pc + 1) as usize);
                    let info = match self.method_table.get(&method_id) {
                        Some(info) => info,
                        None => return Err(self.error_unimplemented("method not defined.")),
                    };
                    let args_num = read32(&self.iseq, pc + 5);
                    let mut args = vec![];
                    for _ in 0..args_num {
                        args.push(stack.pop().unwrap());
                    }
                    match info {
                        MethodInfo::BuiltinFunc { name, func } => {
                            #[cfg(debug_assertions)]
                            println!("SEND BuiltinFunc {} args:{}", name, args_num);
                            let val = func(self, receiver, args)?;
                            stack.push(val);
                        }
                        _ => return Err(self.error_unimplemented("ruby func.")),
                    }
                    pc += 9;
                }

                _ => unimplemented!("Illegal instruction."),
            }
        }

        fn read64(iseq: &ISeq, pc: usize) -> u64 {
            let mut num: u64 = (iseq[pc] as u64) << 56;
            num += (iseq[pc + 1] as u64) << 48;
            num += (iseq[pc + 2] as u64) << 40;
            num += (iseq[pc + 3] as u64) << 32;
            num += (iseq[pc + 4] as u64) << 24;
            num += (iseq[pc + 5] as u64) << 16;
            num += (iseq[pc + 6] as u64) << 8;
            num += iseq[pc + 7] as u64;
            num
        }

        fn read32(iseq: &ISeq, pc: usize) -> u32 {
            let mut num: u32 = (iseq[pc] as u32) << 24;
            num += (iseq[pc + 1] as u32) << 16;
            num += (iseq[pc + 2] as u32) << 8;
            num += iseq[pc + 3] as u32;
            num
        }
    }

    pub fn current(&self) -> ISeqPos {
        ISeqPos(self.iseq.len())
    }

    fn gen_jmp_if_false(&mut self) -> ISeqPos {
        self.iseq.push(Inst::JMP_IF_FALSE);
        self.iseq.push(0);
        self.iseq.push(0);
        self.iseq.push(0);
        self.iseq.push(0);
        ISeqPos(self.iseq.len())
    }

    fn gen_jmp_back(&mut self, pos: ISeqPos) {
        let disp = self.current().disp(pos) - 5;
        self.iseq.push(Inst::JMP);
        self.push32(disp as u32);
    }

    fn gen_jmp(&mut self) -> ISeqPos {
        self.iseq.push(Inst::JMP);
        self.iseq.push(0);
        self.iseq.push(0);
        self.iseq.push(0);
        self.iseq.push(0);
        ISeqPos(self.iseq.len())
    }

    fn gen_set_local(&mut self, id: IdentId) {
        self.iseq.push(Inst::SET_LOCAL);
        self.push32(id.as_usize() as u32);
    }

    fn gen_fixnum(&mut self, num: i64) {
        self.iseq.push(Inst::PUSH_FIXNUM);
        self.push64(num as u64);
    }

    fn gen_get_local(&mut self, id: IdentId) {
        self.iseq.push(Inst::GET_LOCAL);
        self.push32(id.as_usize() as u32);
    }

    fn gen_send(&mut self, method: IdentId, args_num: usize) {
        self.iseq.push(Inst::SEND);
        self.push32(method.as_usize() as u32);
        self.push32(args_num as u32);
    }

    fn write_disp_from_cur(&mut self, src: ISeqPos) {
        let dest = self.current();
        self.write_disp(src, dest);
    }

    fn write_disp(&mut self, src: ISeqPos, dest: ISeqPos) {
        let num = src.disp(dest) as u32;
        self.iseq[src.0 - 4] = (num >> 24) as u8;
        self.iseq[src.0 - 3] = (num >> 16) as u8;
        self.iseq[src.0 - 2] = (num >> 8) as u8;
        self.iseq[src.0 - 1] = num as u8;
    }

    fn push32(&mut self, num: u32) {
        self.iseq.push((num >> 24) as u8);
        self.iseq.push((num >> 16) as u8);
        self.iseq.push((num >> 8) as u8);
        self.iseq.push(num as u8);
    }

    fn push64(&mut self, num: u64) {
        self.iseq.push((num >> 56) as u8);
        self.iseq.push((num >> 48) as u8);
        self.iseq.push((num >> 40) as u8);
        self.iseq.push((num >> 32) as u8);
        self.iseq.push((num >> 24) as u8);
        self.iseq.push((num >> 16) as u8);
        self.iseq.push((num >> 8) as u8);
        self.iseq.push(num as u8);
    }

    /// Generate ISeq.
    pub fn gen(&mut self, node: &Node) -> Result<(), RuntimeError> {
        self.loc = node.loc();
        match &node.kind {
            NodeKind::TopLevel(node, _) => self.gen(node)?,
            NodeKind::Nil => self.iseq.push(Inst::PUSH_NIL),
            NodeKind::Bool(b) => {
                if *b {
                    self.iseq.push(Inst::PUSH_TRUE)
                } else {
                    self.iseq.push(Inst::PUSH_FALSE)
                }
            }
            NodeKind::Number(num) => {
                self.gen_fixnum(*num);
            }
            NodeKind::Float(num) => {
                self.iseq.push(Inst::PUSH_FLONUM);
                unsafe { self.push64(std::mem::transmute(*num)) };
            }
            NodeKind::SelfValue => {
                self.iseq.push(Inst::PUSH_SELF);
            }
            NodeKind::Range(start, end, exclude) => {
                if *exclude {
                    self.iseq.push(Inst::PUSH_TRUE);
                } else {
                    self.iseq.push(Inst::PUSH_FALSE)
                };
                self.gen(end)?;
                self.gen(start)?;
                self.iseq.push(Inst::CREATE_RANGE);
            }
            NodeKind::Ident(id) => {
                self.gen_get_local(*id);
            }
            NodeKind::BinOp(op, lhs, rhs) => match op {
                BinOp::Add => {
                    self.gen(lhs)?;
                    self.gen(rhs)?;
                    self.iseq.push(Inst::ADD);
                }
                BinOp::Sub => {
                    self.gen(lhs)?;
                    self.gen(rhs)?;
                    self.iseq.push(Inst::SUB);
                }
                BinOp::Mul => {
                    self.gen(lhs)?;
                    self.gen(rhs)?;
                    self.iseq.push(Inst::MUL);
                }
                BinOp::Div => {
                    self.gen(lhs)?;
                    self.gen(rhs)?;
                    self.iseq.push(Inst::DIV);
                }
                BinOp::Shr => {
                    self.gen(lhs)?;
                    self.gen(rhs)?;
                    self.iseq.push(Inst::SHR);
                }
                BinOp::Shl => {
                    self.gen(lhs)?;
                    self.gen(rhs)?;
                    self.iseq.push(Inst::SHL);
                }
                BinOp::BitOr => {
                    self.gen(lhs)?;
                    self.gen(rhs)?;
                    self.iseq.push(Inst::BIT_OR);
                }
                BinOp::BitAnd => {
                    self.gen(lhs)?;
                    self.gen(rhs)?;
                    self.iseq.push(Inst::BIT_AND);
                }
                BinOp::BitXor => {
                    self.gen(lhs)?;
                    self.gen(rhs)?;
                    self.iseq.push(Inst::BIT_XOR);
                }
                BinOp::Eq => {
                    self.gen(lhs)?;
                    self.gen(rhs)?;
                    self.iseq.push(Inst::EQ);
                }
                BinOp::Ne => {
                    self.gen(lhs)?;
                    self.gen(rhs)?;
                    self.iseq.push(Inst::NE);
                }
                BinOp::Ge => {
                    self.gen(lhs)?;
                    self.gen(rhs)?;
                    self.iseq.push(Inst::GE);
                }
                BinOp::Gt => {
                    self.gen(lhs)?;
                    self.gen(rhs)?;
                    self.iseq.push(Inst::GT);
                }
                BinOp::Le => {
                    self.gen(rhs)?;
                    self.gen(lhs)?;
                    self.iseq.push(Inst::GE);
                }
                BinOp::Lt => {
                    self.gen(rhs)?;
                    self.gen(lhs)?;
                    self.iseq.push(Inst::GT);
                }
                BinOp::LAnd => {
                    self.gen(lhs)?;
                    let src1 = self.gen_jmp_if_false();
                    self.gen(rhs)?;
                    let src2 = self.gen_jmp();
                    self.write_disp_from_cur(src1);
                    self.iseq.push(Inst::PUSH_FALSE);
                    self.write_disp_from_cur(src2);
                }
                BinOp::LOr => {
                    self.gen(lhs)?;
                    let src1 = self.gen_jmp_if_false();
                    self.iseq.push(Inst::PUSH_TRUE);
                    let src2 = self.gen_jmp();
                    self.write_disp_from_cur(src1);
                    self.gen(rhs)?;
                    self.write_disp_from_cur(src2);
                }
            },
            NodeKind::CompStmt(nodes) => {
                for node in nodes {
                    self.gen(&node)?;
                }
            }
            NodeKind::If(cond_, then_, else_) => {
                self.gen(&cond_)?;
                let src1 = self.gen_jmp_if_false();
                self.gen(&then_)?;
                let src2 = self.gen_jmp();
                self.write_disp_from_cur(src1);
                self.gen(&else_)?;
                self.write_disp_from_cur(src2);
            }
            NodeKind::For(id, iter, body) => {
                let id = match id.kind {
                    NodeKind::Ident(id) => id,
                    _ => return Err(self.error_nomethod("Expected an identifier.")),
                };
                let (start, end, exclude) = match &iter.kind {
                    NodeKind::Range(start, end, exclude) => (start, end, exclude),
                    _ => return Err(self.error_nomethod("Expected Range.")),
                };
                self.loop_stack.push(vec![]);
                self.gen(start)?;
                self.gen_set_local(id);
                let loop_start = self.current();
                self.gen(end)?;
                self.gen_get_local(id);
                self.iseq.push(if *exclude { Inst::GT } else { Inst::GE });
                let src = self.gen_jmp_if_false();
                self.gen(body)?;
                let loop_continue = self.current();
                self.gen_get_local(id);
                self.gen_fixnum(1);
                self.iseq.push(Inst::ADD);
                self.gen_set_local(id);

                self.gen_jmp_back(loop_start);
                self.write_disp_from_cur(src);
                for p in self.loop_stack.pop().unwrap() {
                    match p.1 {
                        EscapeKind::Break => {
                            self.write_disp_from_cur(p.0);
                        }
                        EscapeKind::Next => self.write_disp(p.0, loop_continue),
                    }
                }
            }
            NodeKind::Assign(lhs, rhs) => {
                self.gen(rhs)?;
                match lhs.kind {
                    NodeKind::Ident(id) => {
                        self.gen_set_local(id);
                    }
                    _ => (),
                }
            }
            NodeKind::Send(receiver, method, args) => {
                let id = match method.kind {
                    NodeKind::Ident(id) => id,
                    _ => return Err(self.error_unimplemented(format!("Expected identifier."))),
                };
                for arg in args.iter().rev() {
                    self.gen(arg)?;
                }
                self.gen(receiver)?;
                self.gen_send(id, args.len());
            }
            NodeKind::Break => {
                self.iseq.push(Inst::PUSH_NIL);
                let src = self.gen_jmp();
                match self.loop_stack.last_mut() {
                    Some(x) => {
                        x.push((src, EscapeKind::Break));
                    }
                    None => {
                        return Err(self.error_unimplemented("Can't escape from eval with break."));
                    }
                }
            }
            NodeKind::Next => {
                self.iseq.push(Inst::PUSH_NIL);
                let src = self.gen_jmp();
                match self.loop_stack.last_mut() {
                    Some(x) => {
                        x.push((src, EscapeKind::Next));
                    }
                    None => {
                        return Err(self.error_unimplemented("Can't escape from eval with next."));
                    }
                }
            }
            /*

            NodeKind::String(s) => Ok(Value::String(s.clone())),
            NodeKind::SelfValue => {
                /*
                let classref = self
                    .class_stack
                    .last()
                    .unwrap_or_else(|| panic!("Evaluator#eval_node: class stack is empty"));
                    */
                Ok(self.self_value.clone())
            }

            NodeKind::InstanceVar(id) => match self.self_value {
                Value::Instance(instance) => {
                    let info = self.get_instance_info(instance);
                    match info.instance_var.get(id) {
                        Some(v) => Ok(v.clone()),
                        None => Ok(Value::Nil),
                    }
                }
                Value::Class(class) => {
                    let info = self.get_class_info(class);
                    match info.instance_var.get(id) {
                        Some(v) => Ok(v.clone()),
                        None => Ok(Value::Nil),
                    }
                }
                _ => {
                    return Err(self.error_unimplemented(
                        format!("Instance variable can be referred only in instance method."),
                        node.loc(),
                    ))
                }
            },
            NodeKind::Const(id) => match self.const_table.get(&id) {
                Some(val) => Ok(val.clone()),
                None => {
                    self.source_info.show_loc(&node.loc());
                    println!("{:?}", self.lvar_table());
                    let name = self.ident_table.get_name(*id).clone();
                    Err(self.error_name(format!("Uninitialized constant '{}'.", name), node.loc()))
                }
            },

            NodeKind::MethodDecl(id, params, body) => {
                let info = MethodInfo::RubyFunc {
                    params: params.clone(),
                    body: body.clone(),
                };
                if self.class_stack.len() == 1 {
                    // A method defined in "top level" is registered to the global method table.
                    self.method_table.insert(*id, info);
                } else {
                    // A method defined in a class definition is registered as a instance method of the class.
                    let class = self.class_stack.last().unwrap();
                    let class_info = self.class_table.get_mut(*class);
                    class_info.instance_method.insert(*id, info);
                }
                Ok(Value::Nil)
            }
            NodeKind::ClassMethodDecl(id, params, body) => {
                let info = MethodInfo::RubyFunc {
                    params: params.clone(),
                    body: body.clone(),
                };
                if self.class_stack.len() == 1 {
                    return Err(self.error_unimplemented(
                        "Currently, class method definition in the top level is not allowed.",
                        node.loc(),
                    ));
                } else {
                    // A method defined in a class definition is registered as a class method of the class.
                    let class = self.class_stack.last().unwrap();
                    let class_info = self.class_table.get_mut(*class);
                    class_info.class_method.insert(*id, info);
                }
                Ok(Value::Nil)
            }
            NodeKind::ClassDecl(id, body) => {
                let classref = self.new_class(*id, *body.clone());
                let val = Value::Class(classref);
                self.const_table.insert(*id, val);
                self.scope_stack.push(LocalScope::new());
                self.class_stack.push(classref);
                let self_old = self.self_value.clone();
                self.self_value = Value::Class(classref);
                self.eval_node(body)?;
                self.self_value = self_old;
                self.class_stack.pop();
                self.scope_stack.pop();
                Ok(Value::Nil)
            }
            NodeKind::Send(receiver, method, args) => {
                let id = match method.kind {
                    NodeKind::Ident(id) => id,
                    _ => {
                        return Err(
                            self.error_unimplemented(format!("Expected identifier."), method.loc())
                        )
                    }
                };
                let receiver_val = self.eval_node(receiver)?;
                let rec = if receiver.kind == NodeKind::SelfValue {
                    None
                } else {
                    Some(self.eval_node(receiver)?)
                };
                let mut args_val = vec![];
                for arg in args {
                    args_val.push(self.eval_node(arg)?);
                }
                let info = match rec {
                    None => match self.method_table.get(&id) {
                        Some(info) => info.clone(),
                        None => {
                            return Err(self.error_nomethod("undefined method.", receiver.loc()))
                        }
                    },
                    Some(rec) => match rec {
                        Value::Instance(instance) => self.get_instance_method(instance, method)?,
                        Value::Class(class) => self.get_class_method(class, method)?,
                        Value::FixNum(i) => {
                            let id = match method.kind {
                                NodeKind::Ident(id) => id,
                                _ => {
                                    return Err(self.error_unimplemented(
                                        format!("Expected identifier."),
                                        method.loc(),
                                    ))
                                }
                            };
                            if self.ident_table.get_name(id) == "chr" {
                                return Ok(Value::Char(i as u8));
                            } else {
                                return Err(self.error_unimplemented(
                                    format!("Expected identifier."),
                                    method.loc(),
                                ));
                            }
                        }
                        _ => {
                            return Err(self.error_unimplemented(
                                format!("Receiver must be a class or instance. {:?}", rec),
                                receiver.loc(),
                            ))
                        }
                    },
                };

                match info {
                    MethodInfo::RubyFunc { params, body } => {
                        let args_len = args.len();
                        self.scope_stack.push(LocalScope::new());
                        for (i, param) in params.clone().iter().enumerate() {
                            if let Node {
                                kind: NodeKind::Param(param_id),
                                ..
                            } = param.clone()
                            {
                                let arg = if args_len > i {
                                    args_val[i].clone()
                                } else {
                                    Value::Nil
                                };
                                self.lvar_table().insert(param_id, arg);
                            } else {
                                panic!("Illegal parameter.");
                            }
                        }
                        let self_old = self.self_value.clone();
                        self.self_value = receiver_val;
                        let val = self.eval_node(&body.clone());
                        self.self_value = self_old;
                        self.scope_stack.pop();
                        val
                    }
                    MethodInfo::BuiltinFunc { func, .. } => func(self, receiver_val, args_val),
                }
            }*/
            _ => unimplemented!("{:?}", node.kind),
        };
        Ok(())
    }
}

impl VM {
    pub fn error_nomethod(&self, msg: impl Into<String>) -> RuntimeError {
        RuntimeError::new(RuntimeErrKind::NoMethod(msg.into()), self.loc)
    }
    pub fn error_unimplemented(&self, msg: impl Into<String>) -> RuntimeError {
        RuntimeError::new(RuntimeErrKind::Unimplemented(msg.into()), self.loc)
    }
}

impl VM {
    fn eval_add(&mut self, rhs: Value, lhs: Value) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs + rhs)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum(lhs as f64 + rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(Value::FloatNum(lhs + rhs as f64)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum(lhs + rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '-'")),
        }
    }
    fn eval_sub(&mut self, rhs: Value, lhs: Value) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs - rhs)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum(lhs as f64 - rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(Value::FloatNum(lhs - rhs as f64)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum(lhs - rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '-'")),
        }
    }

    fn eval_mul(&mut self, rhs: Value, lhs: Value) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs * rhs)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum(lhs as f64 * rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(Value::FloatNum(lhs * rhs as f64)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum(lhs * rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '*'")),
        }
    }

    fn eval_div(&mut self, rhs: Value, lhs: Value) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs / rhs)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum((lhs as f64) / rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(Value::FloatNum(lhs / (rhs as f64))),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum(lhs / rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '*'")),
        }
    }

    fn eval_shl(&mut self, rhs: Value, lhs: Value) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs << rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '<<'")),
        }
    }

    fn eval_shr(&mut self, rhs: Value, lhs: Value) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs >> rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>>'")),
        }
    }

    fn eval_bitand(&mut self, rhs: Value, lhs: Value) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs & rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>>'")),
        }
    }

    fn eval_bitor(&mut self, rhs: Value, lhs: Value) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs | rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>>'")),
        }
    }

    fn eval_bitxor(&mut self, rhs: Value, lhs: Value) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs ^ rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>>'")),
        }
    }

    pub fn eval_eq(&mut self, rhs: Value, lhs: Value) -> EvalResult {
        match (&lhs, &rhs) {
            (Value::Nil, Value::Nil) => Ok(Value::Bool(true)),
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs == rhs)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::Bool(lhs == rhs)),
            (Value::Bool(lhs), Value::Bool(rhs)) => Ok(Value::Bool(lhs == rhs)),
            (Value::Class(lhs), Value::Class(rhs)) => Ok(Value::Bool(lhs == rhs)),
            (Value::Instance(lhs), Value::Instance(rhs)) => Ok(Value::Bool(lhs == rhs)),
            _ => Err(self.error_nomethod(format!("NoMethodError: {:?} == {:?}", lhs, rhs))),
        }
    }

    fn eval_neq(&mut self, rhs: Value, lhs: Value) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs != rhs)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::Bool(lhs != rhs)),
            (Value::Bool(lhs), Value::Bool(rhs)) => Ok(Value::Bool(lhs != rhs)),
            (Value::Class(lhs), Value::Class(rhs)) => Ok(Value::Bool(lhs != rhs)),
            (Value::Instance(lhs), Value::Instance(rhs)) => Ok(Value::Bool(lhs != rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '!='")),
        }
    }

    fn eval_ge(&mut self, rhs: Value, lhs: Value) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs >= rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs >= rhs as f64)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(Value::Bool(lhs as f64 >= rhs)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::Bool(lhs >= rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>='")),
        }
    }

    fn eval_gt(&mut self, rhs: Value, lhs: Value) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs > rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs > rhs as f64)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(Value::Bool(lhs as f64 > rhs)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::Bool(lhs > rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>'")),
        }
    }
}

impl VM {
    pub fn val_to_bool(&self, val: &Value) -> bool {
        match val {
            Value::Nil | Value::Bool(false) => false,
            _ => true,
        }
    }

    pub fn val_to_s(&mut self, val: &Value) -> String {
        match val {
            Value::Nil => "".to_string(),
            Value::Bool(b) => match b {
                true => "true".to_string(),
                false => "false".to_string(),
            },
            Value::FixNum(i) => i.to_string(),
            Value::FloatNum(f) => f.to_string(),
            Value::String(s) => format!("{}", s),
            //Value::Class(class) => self.get_class_name(*class),
            //Value::Instance(instance) => self.get_instance_name(*instance),
            Value::Range(start, end, exclude) => {
                let start = self.val_to_s(start);
                let end = self.val_to_s(end);
                let sym = if *exclude { "..." } else { ".." };
                format!("({}{}{})", start, sym, end)
            }
            Value::Char(c) => format!("{:x}", c),
            _ => "".to_string(),
        }
    }
}

impl VM {
    pub fn init_builtin(&mut self) {
        let id = self.ident_table.get_ident_id(&"puts".to_string());
        let info = MethodInfo::BuiltinFunc {
            name: "puts".to_string(),
            func: builtin_puts,
        };
        self.method_table.insert(id, info);

        let id = self.ident_table.get_ident_id(&"print".to_string());
        let info = MethodInfo::BuiltinFunc {
            name: "print".to_string(),
            func: builtin_print,
        };
        self.method_table.insert(id, info);

        let id = self.ident_table.get_ident_id(&"assert".to_string());
        let info = MethodInfo::BuiltinFunc {
            name: "assert".to_string(),
            func: builtin_assert,
        };
        self.method_table.insert(id, info);

        /// Built-in function "puts".
        pub fn builtin_puts(eval: &mut VM, _receiver: Value, args: Vec<Value>) -> EvalResult {
            for arg in args {
                println!("{}", eval.val_to_s(&arg));
            }
            Ok(Value::Nil)
        }

        /// Built-in function "print".
        pub fn builtin_print(eval: &mut VM, _receiver: Value, args: Vec<Value>) -> EvalResult {
            for arg in args {
                if let Value::Char(ch) = arg {
                    let v = [ch];
                    use std::io::{self, Write};
                    io::stdout().write(&v).unwrap();
                } else {
                    print!("{}", eval.val_to_s(&arg));
                }
            }
            Ok(Value::Nil)
        }

        /// Built-in function "assert".
        pub fn builtin_assert(eval: &mut VM, _receiver: Value, args: Vec<Value>) -> EvalResult {
            if args.len() != 2 {
                panic!("Invalid number of arguments.");
            }
            if eval.eval_eq(args[0].clone(), args[1].clone())? != Value::Bool(true) {
                panic!(
                    "Assertion error: Expected: {:?} Actual: {:?}",
                    args[0], args[1]
                );
            } else {
                Ok(Value::Nil)
            }
        }
        /*
        /// Built-in function "new".
        pub fn builtin_new(eval: &mut VM, receiver: Value, _args: Vec<Value>) -> EvalResult {
            match receiver {
                Value::Class(class_ref) => {
                    let instance = eval.new_instance(class_ref);
                    Ok(Value::Instance(instance))
                }
                _ => Err(eval.error_unimplemented(
                    format!("Receiver must be a class! {:?}", receiver),
                    eval.loc,
                )),
            }
        }*/
    }
}
