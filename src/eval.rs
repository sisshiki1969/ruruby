use crate::lexer::*;
use crate::node::*;
use crate::parser::*;
use crate::value::Value;
use std::collections::HashMap;

type LvarTable = HashMap<usize, Value>;
type BuiltinFunc = fn(eval: &mut Evaluator, args: Vec<Value>) -> Value;

#[derive(Clone)]
pub enum FuncInfo {
    RubyFunc { params: Vec<Node>, body: Box<Node> },
    BuiltinFunc { name: String, func: BuiltinFunc },
}

impl std::fmt::Debug for FuncInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FuncInfo::RubyFunc { params, body } => write!(f, "RubyFunc {:?} {:?}", params, body),
            FuncInfo::BuiltinFunc { name, .. } => write!(f, "BuiltinFunc {:?}", name),
        }
    }
}
type FuncTable = HashMap<usize, FuncInfo>;

#[derive(Debug, Clone, PartialEq)]
pub struct ExecContext {
    lvar_table: LvarTable,
}

impl ExecContext {
    pub fn new() -> Self {
        ExecContext {
            lvar_table: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Evaluator {
    pub source_info: SourceInfo,
    pub ident_table: IdentifierTable,
    pub method_table: FuncTable,
    pub exec_context: Vec<ExecContext>,
}

impl Evaluator {
    pub fn new(source_info: SourceInfo, ident_table: IdentifierTable) -> Self {
        let mut eval = Evaluator {
            source_info,
            ident_table,
            method_table: HashMap::new(),
            exec_context: vec![ExecContext::new()],
        };
        let id = eval.ident_table.get_ident_id(&"puts".to_string());
        let info = FuncInfo::BuiltinFunc {
            name: "puts".to_string(),
            func: Evaluator::builtin_puts,
        };
        eval.method_table.insert(id, info);
        eval
    }

    pub fn builtin_puts(_eval: &mut Evaluator, args: Vec<Value>) -> Value {
        for arg in args {
            println!("{}", arg.to_s());
        }
        Value::Nil
    }

    pub fn lvar_table(&mut self) -> &mut LvarTable {
        &mut self.exec_context.last_mut().unwrap().lvar_table
    }

    /// Evaluate AST.
    pub fn eval_node(&mut self, node: &Node) -> Value {
        match &node.kind {
            NodeKind::Number(num) => Value::FixNum(*num),
            NodeKind::LocalVar(id) => match self.lvar_table().get(&id) {
                Some(val) => val.clone(),
                None => {
                    self.source_info.show_loc(&node.loc());
                    println!("{:?}", self.lvar_table());
                    panic!("undefined local variable.");
                }
            },
            NodeKind::BinOp(op, lhs, rhs) => {
                let lhs = self.eval_node(&lhs);
                let rhs = self.eval_node(&rhs);
                match op {
                    BinOp::Add => self.eval_add(lhs, rhs),
                    BinOp::Sub => self.eval_sub(lhs, rhs),
                    BinOp::Mul => self.eval_mul(lhs, rhs),
                    BinOp::Eq => self.eval_eq(lhs, rhs),
                }
            }
            NodeKind::Assign(lhs, rhs) => match lhs.kind {
                NodeKind::LocalVar(id) => {
                    let rhs = self.eval_node(&rhs);
                    match self.lvar_table().get_mut(&id) {
                        Some(val) => {
                            *val = rhs.clone();
                        }
                        None => {
                            self.lvar_table().insert(id, rhs.clone());
                        }
                    }
                    rhs
                }
                _ => unimplemented!(),
            },
            NodeKind::CompStmt(nodes) => {
                let mut val = Value::Nil;
                for node in nodes {
                    val = self.eval_node(&node);
                }
                val
            }
            NodeKind::If(cond_, then_, else_) => {
                if self.eval_node(&cond_).to_bool() {
                    self.eval_node(&then_)
                } else {
                    self.eval_node(&else_)
                }
            }
            NodeKind::FuncDecl(id, params, body) => {
                self.method_table.insert(
                    *id,
                    FuncInfo::RubyFunc {
                        params: params.clone(),
                        body: body.clone(),
                    },
                );
                Value::Nil
            }
            NodeKind::Send(id, args) => {
                let args_val: Vec<Value> = args.iter().map(|x| self.eval_node(x)).collect();
                let info = match self.method_table.get(id) {
                    Some(info) => info.clone(),
                    None => unimplemented!("undefined function."),
                };
                match info {
                    FuncInfo::RubyFunc { params, body } => {
                        let args_len = args.len();
                        self.exec_context.push(ExecContext::new());
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
                                unimplemented!("Illegal parameter.");
                            }
                        }
                        let val = self.eval_node(&body.clone());
                        self.exec_context.pop();
                        val
                    }
                    FuncInfo::BuiltinFunc { func, .. } => func(self, args_val),
                }
            }
            _ => unimplemented!(),
        }
    }

    fn eval_add(&mut self, lhs: Value, rhs: Value) -> Value {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Value::FixNum(lhs + rhs),
            (_, _) => unimplemented!(),
        }
    }

    fn eval_sub(&mut self, lhs: Value, rhs: Value) -> Value {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Value::FixNum(lhs - rhs),
            (_, _) => unimplemented!(),
        }
    }

    fn eval_mul(&mut self, lhs: Value, rhs: Value) -> Value {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Value::FixNum(lhs * rhs),
            (_, _) => unimplemented!(),
        }
    }

    fn eval_eq(&mut self, lhs: Value, rhs: Value) -> Value {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Value::Bool(lhs == rhs),
            (Value::Bool(lhs), Value::Bool(rhs)) => Value::Bool(lhs == rhs),
            (_, _) => unimplemented!(),
        }
    }
}
