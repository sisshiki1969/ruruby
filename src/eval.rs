use crate::class::*;
use crate::instance::*;
use crate::node::*;
use crate::util::*;
use crate::value::*;
use std::collections::HashMap;

pub type ValueTable = HashMap<IdentId, Value>;
pub type BuiltinFunc = fn(eval: &mut Evaluator, receiver: Value, args: Vec<Value>) -> EvalResult;

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

pub type EvalResult = Result<Value, RuntimeError>;

pub type RuntimeError = Annot<RuntimeErrKind>;

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeErrKind {
    Unimplemented(String),
    Unreachable(String),
    Name(String),
    NoMethod(String),
}

#[derive(Debug, Clone)]
pub struct Evaluator {
    // Global info
    pub source_info: SourceInfo,
    pub ident_table: IdentifierTable,
    pub class_table: GlobalClassTable,
    pub instance_table: GlobalInstanceTable,
    pub method_table: MethodTable,
    pub const_table: ValueTable,
    // State
    pub class_stack: Vec<ClassRef>,
    pub scope_stack: Vec<LocalScope>,
    pub loc: Loc,
}

impl Evaluator {
    pub fn new() -> Self {
        let eval = Evaluator {
            source_info: SourceInfo::new(),
            ident_table: IdentifierTable::new(),
            class_table: GlobalClassTable::new(),
            instance_table: GlobalInstanceTable::new(),
            method_table: HashMap::new(),
            const_table: HashMap::new(),
            class_stack: vec![],
            scope_stack: vec![LocalScope::new()],
            loc: Loc(0, 0),
        };
        eval
    }
    pub fn init(&mut self, source_info: SourceInfo, ident_table: IdentifierTable) {
        self.repl_init(source_info, ident_table);
        self.repl_set_main();
    }

    pub fn repl_init(&mut self, source_info: SourceInfo, ident_table: IdentifierTable) {
        self.source_info = source_info;
        self.ident_table = ident_table;

        let id = self.ident_table.get_ident_id(&"puts".to_string());
        let info = MethodInfo::BuiltinFunc {
            name: "puts".to_string(),
            func: Evaluator::builtin_puts,
        };
        self.method_table.insert(id, info);

        let id = self.ident_table.get_ident_id(&"assert".to_string());
        let info = MethodInfo::BuiltinFunc {
            name: "assert".to_string(),
            func: Evaluator::builtin_assert,
        };
        self.method_table.insert(id, info);
    }

    pub fn repl_set_main(&mut self) {
        let id = self.ident_table.get_ident_id(&"main".to_string());
        let classref = self.new_class(id, Node::new_comp_stmt(Loc(0, 0)));
        self.class_stack.push(classref);
    }

    pub fn error_unimplemented(&self, msg: impl Into<String>, loc: Loc) -> RuntimeError {
        Annot::new(RuntimeErrKind::Unimplemented(msg.into()), loc)
    }

    pub fn error_name(&self, msg: impl Into<String>, loc: Loc) -> RuntimeError {
        Annot::new(RuntimeErrKind::Name(msg.into()), loc)
    }

    pub fn error_nomethod(&self, msg: impl Into<String>, loc: Loc) -> RuntimeError {
        Annot::new(RuntimeErrKind::NoMethod(msg.into()), loc)
    }

    /// Built-in function "puts".
    pub fn builtin_puts(eval: &mut Evaluator, _receiver: Value, args: Vec<Value>) -> EvalResult {
        for arg in args {
            println!("{}", eval.val_to_s(&arg));
        }
        Ok(Value::Nil)
    }

    /// Built-in function "new".
    pub fn builtin_new(eval: &mut Evaluator, receiver: Value, _args: Vec<Value>) -> EvalResult {
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
    }

    /// Built-in function "assert".
    pub fn builtin_assert(eval: &mut Evaluator, _receiver: Value, args: Vec<Value>) -> EvalResult {
        if args.len() != 2 {
            panic!("Invalid number of arguments.");
        }
        if eval.eval_eq(args[0].clone(), args[1].clone(), eval.loc)? != Value::Bool(true) {
            panic!(
                "Assertion error: Expected: {:?} Actual: {:?}",
                args[0], args[1]
            );
        } else {
            Ok(Value::Nil)
        }
    }

    /// Get local variable table.
    pub fn lvar_table(&mut self) -> &mut ValueTable {
        &mut self.scope_stack.last_mut().unwrap().lvar_table
    }

    pub fn eval(&mut self, node: &Node) -> EvalResult {
        match self.eval_node(node) {
            Ok(res) => Ok(res),
            Err(err) => {
                self.source_info.show_loc(&err.loc);
                match &err.kind {
                    RuntimeErrKind::Name(s) => println!("NameError ({})", s),
                    RuntimeErrKind::NoMethod(s) => println!("NoMethodError ({})", s),
                    RuntimeErrKind::Unimplemented(s) => println!("Unimplemented ({})", s),
                    RuntimeErrKind::Unreachable(s) => println!("Unreachable ({})", s),
                }
                Err(err)
            }
        }
    }

    /// Evaluate AST.
    pub fn eval_node(&mut self, node: &Node) -> EvalResult {
        let loc = node.loc();
        match &node.kind {
            NodeKind::Number(num) => Ok(Value::FixNum(*num)),
            NodeKind::String(s) => Ok(Value::String(s.clone())),
            NodeKind::SelfValue => {
                let classref = self
                    .class_stack
                    .last()
                    .unwrap_or_else(|| panic!("Evaluator#eval_node: class stack is empty"));
                Ok(Value::Class(*classref))
            }
            NodeKind::Ident(id) => match self.lvar_table().get(&id) {
                Some(val) => Ok(val.clone()),
                None => {
                    let name = self.ident_table.get_name(*id).clone();
                    Err(self.error_name(
                        format!("NameError: undefined local variable `{}'.", name),
                        node.loc(),
                    ))
                }
            },
            NodeKind::Const(id) => match self.const_table.get(&id) {
                Some(val) => Ok(val.clone()),
                None => {
                    self.source_info.show_loc(&node.loc());
                    println!("{:?}", self.lvar_table());
                    panic!("NameError: uninitialized constant.");
                }
            },
            NodeKind::BinOp(op, lhs, rhs) => {
                match op {
                    BinOp::LAnd => {
                        let lhs_v = self.eval_node(&lhs)?;
                        if let Value::Bool(b) = lhs_v {
                            if !b {
                                return Ok(Value::Bool(false));
                            }
                            let rhs_v = self.eval_node(&rhs)?;
                            if let Value::Bool(b) = rhs_v {
                                return Ok(Value::Bool(b));
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
                        let lhs_v = self.eval_node(&lhs)?;
                        if let Value::Bool(b) = lhs_v {
                            if b {
                                return Ok(Value::Bool(true));
                            }
                            let rhs_v = self.eval_node(&rhs)?;
                            if let Value::Bool(b) = rhs_v {
                                return Ok(Value::Bool(b));
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
                let lhs = self.eval_node(&lhs)?;
                let rhs = self.eval_node(&rhs)?;
                match op {
                    BinOp::Add => self.eval_add(lhs, rhs),
                    BinOp::Sub => self.eval_sub(lhs, rhs),
                    BinOp::Mul => self.eval_mul(lhs, rhs),
                    BinOp::Eq => self.eval_eq(lhs, rhs, loc),
                    BinOp::Ne => self.eval_neq(lhs, rhs),
                    BinOp::Ge => self.eval_ge(lhs, rhs),
                    BinOp::Gt => self.eval_gt(lhs, rhs),
                    BinOp::Le => self.eval_ge(rhs, lhs),
                    BinOp::Lt => self.eval_gt(rhs, lhs),
                    _ => unimplemented!("{:?}", op),
                }
            }
            NodeKind::Assign(lhs, rhs) => match lhs.kind {
                NodeKind::Ident(id) => {
                    let rhs = self.eval_node(&rhs)?;
                    match self.lvar_table().get_mut(&id) {
                        Some(val) => {
                            *val = rhs.clone();
                        }
                        None => {
                            self.lvar_table().insert(id, rhs.clone());
                        }
                    }
                    Ok(rhs)
                }
                _ => unimplemented!(),
            },
            NodeKind::CompStmt(nodes) => {
                let mut val = Value::Nil;
                for node in nodes {
                    val = self.eval_node(&node)?;
                }
                Ok(val)
            }
            NodeKind::If(cond_, then_, else_) => {
                let cond_val = self.eval_node(&cond_)?;
                if self.val_to_bool(&cond_val) {
                    self.eval_node(&then_)
                } else {
                    self.eval_node(&else_)
                }
            }
            NodeKind::FuncDecl(id, params, body) => {
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
                    class_info.instance_method_table.insert(*id, info);
                }
                Ok(Value::Nil)
            }
            NodeKind::ClassDecl(id, body) => {
                let info = self.new_class(*id, *body.clone());
                let val = Value::Class(info);
                self.const_table.insert(*id, val);
                self.scope_stack.push(LocalScope::new());
                self.class_stack.push(info);
                self.eval_node(body)?;
                self.class_stack.pop();
                self.scope_stack.pop();
                Ok(Value::Nil)
            }
            NodeKind::Send(receiver, method, args) => {
                let id = match method.kind {
                    NodeKind::Ident(id) => id,
                    _ => {
                        unimplemented!("method must be identifier.");
                    }
                };
                let receiver_val = self.eval_node(receiver)?;
                let rec = if receiver.kind == NodeKind::SelfValue {
                    None
                } else {
                    Some(self.eval_node(receiver)?)
                };
                let args_val: Vec<Value> =
                    args.iter().map(|x| self.eval_node(x).unwrap()).collect();
                let info = match rec {
                    None => match self.method_table.get(&id) {
                        Some(info) => info.clone(),
                        None => {
                            return Err(self.error_nomethod("undefined method.", receiver.loc()))
                        }
                    },
                    Some(rec) => match rec {
                        Value::Instance(instance) => {
                            let info = self.instance_table.get(instance);
                            let class_info = self.class_table.get(info.class_id);
                            class_info.get_instance_method(id).clone()
                        }
                        Value::Class(class) => {
                            self.class_table.get(class).get_class_method(id).clone()
                        }
                        _ => unimplemented!("Receiver must be a class or instance. {:?}", rec),
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
                                unimplemented!("Illegal parameter.");
                            }
                        }
                        let val = self.eval_node(&body.clone());
                        self.scope_stack.pop();
                        val
                    }
                    MethodInfo::BuiltinFunc { func, .. } => func(self, receiver_val, args_val),
                }
            }
            _ => unimplemented!("{:?}", node.kind),
        }
    }

    fn eval_add(&mut self, lhs: Value, rhs: Value) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs + rhs)),
            (_, _) => unimplemented!("NoMethodError: '+'"),
        }
    }

    fn eval_sub(&mut self, lhs: Value, rhs: Value) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs - rhs)),
            (_, _) => unimplemented!("NoMethodError: '-'"),
        }
    }

    fn eval_mul(&mut self, lhs: Value, rhs: Value) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs * rhs)),
            (_, _) => unimplemented!("NoMethodError: '*'"),
        }
    }

    pub fn eval_eq(&mut self, lhs: Value, rhs: Value, loc: Loc) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs == rhs)),
            (Value::Bool(lhs), Value::Bool(rhs)) => Ok(Value::Bool(lhs == rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '=='", loc)),
        }
    }

    fn eval_neq(&mut self, lhs: Value, rhs: Value) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs != rhs)),
            (Value::Bool(lhs), Value::Bool(rhs)) => Ok(Value::Bool(lhs != rhs)),
            (_, _) => unimplemented!("NoMethodError: '!='"),
        }
    }

    fn eval_ge(&mut self, lhs: Value, rhs: Value) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs >= rhs)),
            (_, _) => unimplemented!("NoMethodError: '>='"),
        }
    }

    fn eval_gt(&mut self, lhs: Value, rhs: Value) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs > rhs)),
            (_, _) => unimplemented!("NoMethodError: '>'"),
        }
    }
}

impl Evaluator {
    pub fn new_class(&mut self, id: IdentId, body: Node) -> ClassRef {
        let name = self.ident_table.get_name(id).clone();
        let class_ref = self.class_table.new_class(id, name, body);
        let id = self.ident_table.get_ident_id(&"new".to_string());
        let info = MethodInfo::BuiltinFunc {
            name: "new".to_string(),
            func: Evaluator::builtin_new,
        };

        self.class_table
            .get_mut(class_ref)
            .add_class_method(id, info);
        class_ref
    }

    pub fn new_instance(&mut self, class_id: ClassRef) -> InstanceRef {
        let class_info = self.class_table.get(class_id);
        let class_name = class_info.name.clone();
        self.instance_table.new_instance(class_id, class_name)
    }

    pub fn get_class_name(&self, class_id: ClassRef) -> String {
        let class_info = self.class_table.get(class_id);
        class_info.name.clone()
    }

    pub fn get_instance_name(&self, instance: InstanceRef) -> String {
        let info = self.instance_table.get(instance);
        format!("#<{}:{:?}>", info.class_name, instance)
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
            Value::String(s) => format!("{}", s),
            Value::Class(class) => self.get_class_name(*class),
            Value::Instance(instance) => self.get_instance_name(*instance),
        }
    }
}
