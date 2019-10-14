use crate::node::*;
use crate::util::*;
use crate::value::*;
use std::collections::HashMap;

type ValueTable = HashMap<IdentId, Value>;
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
type FuncTable = HashMap<IdentId, FuncInfo>;

#[derive(Debug, Clone, PartialEq)]
pub struct ExecContext {
    lvar_table: ValueTable,
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
    // Global info
    pub source_info: SourceInfo,
    pub ident_table: IdentifierTable,
    pub class_table: GlobalClassTable,
    pub method_table: FuncTable,
    pub const_table: ValueTable,
    // State
    pub class_stack: Vec<ClassRef>,
    pub exec_context: Vec<ExecContext>,
}

impl Evaluator {
    pub fn new(source_info: SourceInfo, ident_table: IdentifierTable) -> Self {
        let mut eval = Evaluator {
            source_info,
            ident_table,
            class_table: GlobalClassTable::new(),
            method_table: HashMap::new(),
            const_table: HashMap::new(),
            class_stack: vec![],
            exec_context: vec![ExecContext::new()],
        };
        let id = eval.ident_table.get_ident_id(&"puts".to_string());
        let info = FuncInfo::BuiltinFunc {
            name: "puts".to_string(),
            func: Evaluator::builtin_puts,
        };
        eval.method_table.insert(id, info);

        let id = eval.ident_table.get_ident_id(&"main".to_string());
        let classref = eval.new_class_info(id, Node::new_comp_stmt(Loc(0, 0)));
        eval.class_stack.push(classref);

        eval
    }

    /// Built-in function "puts".
    pub fn builtin_puts(eval: &mut Evaluator, args: Vec<Value>) -> Value {
        for arg in args {
            println!("{}", eval.val_to_s(&arg));
        }
        Value::Nil
    }

    /// Get local variable table.
    pub fn lvar_table(&mut self) -> &mut ValueTable {
        &mut self.exec_context.last_mut().unwrap().lvar_table
    }

    fn new_class_info(&mut self, id: IdentId, body: Node) -> ClassRef {
        let name = self.ident_table.get_name(id).clone();
        self.class_table.new_class(id, name, body)
    }

    /// Evaluate AST.
    pub fn eval_node(&mut self, node: &Node) -> Value {
        match &node.kind {
            NodeKind::Number(num) => Value::FixNum(*num),
            NodeKind::SelfValue => {
                let classref = self
                    .class_stack
                    .last()
                    .unwrap_or_else(|| panic!("Evaluator#eval_node: class stack is empty"));
                Value::Class(*classref)
            }
            NodeKind::LocalVar(id) => match self.lvar_table().get(&id) {
                Some(val) => val.clone(),
                None => {
                    self.source_info.show_loc(&node.loc());
                    println!("{:?}", self.lvar_table());
                    panic!("NameError: undefined local variable.");
                }
            },
            NodeKind::Const(id) => match self.const_table.get(&id) {
                Some(val) => val.clone(),
                None => {
                    self.source_info.show_loc(&node.loc());
                    println!("{:?}", self.lvar_table());
                    panic!("NameError: uninitialized constant.");
                }
            },
            NodeKind::BinOp(op, lhs, rhs) => {
                match op {
                    BinOp::LAnd => {
                        let lhs_v = self.eval_node(&lhs);
                        if let Value::Bool(b) = lhs_v {
                            if !b {
                                return Value::Bool(false);
                            }
                            let rhs_v = self.eval_node(&rhs);
                            if let Value::Bool(b) = rhs_v {
                                return Value::Bool(b);
                            } else {
                                self.source_info.show_loc(&rhs.loc());
                                panic!("Expected bool.");
                            }
                        } else {
                            self.source_info.show_loc(&lhs.loc());
                            panic!("Expected bool.");
                        }
                    }
                    BinOp::LOr => {
                        let lhs_v = self.eval_node(&lhs);
                        if let Value::Bool(b) = lhs_v {
                            if b {
                                return Value::Bool(true);
                            }
                            let rhs_v = self.eval_node(&rhs);
                            if let Value::Bool(b) = rhs_v {
                                return Value::Bool(b);
                            } else {
                                self.source_info.show_loc(&rhs.loc());
                                panic!("Expected bool.");
                            }
                        } else {
                            self.source_info.show_loc(&lhs.loc());
                            panic!("Expected bool.");
                        }
                    }
                    _ => {}
                }
                let lhs = self.eval_node(&lhs);
                let rhs = self.eval_node(&rhs);
                match op {
                    BinOp::Add => self.eval_add(lhs, rhs),
                    BinOp::Sub => self.eval_sub(lhs, rhs),
                    BinOp::Mul => self.eval_mul(lhs, rhs),
                    BinOp::Eq => self.eval_eq(lhs, rhs),
                    BinOp::Ne => self.eval_neq(lhs, rhs),
                    BinOp::Ge => self.eval_ge(lhs, rhs),
                    BinOp::Gt => self.eval_gt(lhs, rhs),
                    BinOp::Le => self.eval_ge(rhs, lhs),
                    BinOp::Lt => self.eval_gt(rhs, lhs),
                    _ => unimplemented!("{:?}", op),
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
                let cond_val = self.eval_node(&cond_);
                if self.val_to_bool(&cond_val) {
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
            NodeKind::ClassDecl(id, body) => {
                let info = self.new_class_info(*id, *body.clone());
                let val = Value::Class(info);
                self.const_table.insert(*id, val);
                self.class_stack.push(info);
                self.eval_node(body);
                self.class_stack.pop();
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
            _ => unimplemented!("{:?}", node.kind),
        }
    }

    fn eval_add(&mut self, lhs: Value, rhs: Value) -> Value {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Value::FixNum(lhs + rhs),
            (_, _) => unimplemented!("NoMethodError: '+'"),
        }
    }

    fn eval_sub(&mut self, lhs: Value, rhs: Value) -> Value {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Value::FixNum(lhs - rhs),
            (_, _) => unimplemented!("NoMethodError: '-'"),
        }
    }

    fn eval_mul(&mut self, lhs: Value, rhs: Value) -> Value {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Value::FixNum(lhs * rhs),
            (_, _) => unimplemented!("NoMethodError: '*'"),
        }
    }

    fn eval_eq(&mut self, lhs: Value, rhs: Value) -> Value {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Value::Bool(lhs == rhs),
            (Value::Bool(lhs), Value::Bool(rhs)) => Value::Bool(lhs == rhs),
            (_, _) => unimplemented!("NoMethodError: '=='"),
        }
    }

    fn eval_neq(&mut self, lhs: Value, rhs: Value) -> Value {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Value::Bool(lhs != rhs),
            (Value::Bool(lhs), Value::Bool(rhs)) => Value::Bool(lhs != rhs),
            (_, _) => unimplemented!("NoMethodError: '!='"),
        }
    }

    fn eval_ge(&mut self, lhs: Value, rhs: Value) -> Value {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Value::Bool(lhs >= rhs),
            (_, _) => unimplemented!("NoMethodError: '>='"),
        }
    }

    fn eval_gt(&mut self, lhs: Value, rhs: Value) -> Value {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Value::Bool(lhs > rhs),
            (_, _) => unimplemented!("NoMethodError: '>'"),
        }
    }
}

impl Evaluator {
    pub fn val_to_bool(&self, val: &Value) -> bool {
        match val {
            Value::Nil => false,
            Value::Bool(b) => *b,
            Value::FixNum(_) => true,
            Value::String(_) => true,
            _ => unimplemented!(),
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
            Value::String(s) => s.clone(),
            Value::Class(class) => {
                let class_info = self.class_table.get(*class);
                format!("{}", class_info.name)
            }
        }
    }
}
