use crate::lexer::*;
use crate::node::*;
use crate::parser::*;
use crate::value::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Evaluator {
    pub source_info: SourceInfo,
    pub ident_table: IdentifierTable,
    lvar_table: HashMap<usize, Value>,
}

impl Evaluator {
    pub fn new(source_info: SourceInfo, ident_table: IdentifierTable) -> Self {
        Evaluator {
            source_info,
            ident_table,
            lvar_table: HashMap::new(),
        }
    }

    /// Evaluate AST.
    pub fn eval_node(&mut self, node: &Node) -> Value {
        match &node.kind {
            NodeKind::Number(num) => Value::FixNum(*num),
            NodeKind::LocalVar(id) => match self.lvar_table.get(&id) {
                Some(val) => val.clone(),
                None => {
                    self.source_info.show_loc(&node.loc());
                    panic!("undefined local variable.");
                }
            },
            NodeKind::BinOp(op, lhs, rhs) => {
                let lhs = self.eval_node(lhs);
                let rhs = self.eval_node(rhs);
                match op {
                    BinOp::Add => self.eval_add(lhs, rhs),
                    BinOp::Sub => self.eval_sub(lhs, rhs),
                    BinOp::Mul => self.eval_mul(lhs, rhs),
                    BinOp::Eq => self.eval_eq(lhs, rhs),
                }
            }
            NodeKind::Assign(lhs, rhs) => match lhs.kind {
                NodeKind::LocalVar(id) => {
                    let rhs = self.eval_node(rhs);
                    match self.lvar_table.get_mut(&id) {
                        Some(val) => {
                            *val = rhs.clone();
                        }
                        None => {
                            self.lvar_table.insert(id, rhs.clone());
                        }
                    }
                    rhs
                }
                _ => unimplemented!(),
            },
            NodeKind::CompStmt(nodes) => {
                let mut val = Value::Bool(false);
                for node in nodes {
                    val = self.eval_node(node);
                    println!("stmt: {:?}", val);
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
