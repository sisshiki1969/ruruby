mod class;
mod instance;
pub mod value;

use crate::error::*;
use crate::node::*;
use crate::util::{Annot, IdentId, IdentifierTable, Loc, SourceInfo};
use class::*;
use instance::*;
use std::collections::HashMap;
use value::*;

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

pub type EvalResult = Result<Value, EvalError>;

pub type EvalError = Annot<EvalErrKind>;

#[derive(Debug, Clone, PartialEq)]
pub enum EvalErrKind {
    RuntimeError(RuntimeErrKind),
    Break,
}

impl EvalError {
    pub fn nomethod(msg: impl Into<String>, loc: Loc) -> Self {
        Annot::new(
            EvalErrKind::RuntimeError(RuntimeErrKind::NoMethod(msg.into())),
            loc,
        )
    }

    pub fn unimplemented(msg: impl Into<String>, loc: Loc) -> Self {
        Annot::new(
            EvalErrKind::RuntimeError(RuntimeErrKind::Unimplemented(msg.into())),
            loc,
        )
    }

    pub fn name(msg: impl Into<String>, loc: Loc) -> Self {
        Annot::new(
            EvalErrKind::RuntimeError(RuntimeErrKind::Name(msg.into())),
            loc,
        )
    }

    pub fn raise_break(loc: Loc) -> Self {
        Annot::new(EvalErrKind::Break, loc)
    }
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
    pub self_value: Value,
    pub loc: Loc,
}

impl Evaluator {
    pub fn new(source_info: SourceInfo, ident_table: IdentifierTable) -> Self {
        let mut eval = Evaluator {
            source_info,
            ident_table,
            class_table: GlobalClassTable::new(),
            instance_table: GlobalInstanceTable::new(),
            method_table: HashMap::new(),
            const_table: HashMap::new(),
            class_stack: vec![],
            scope_stack: vec![LocalScope::new()],
            self_value: Value::Nil,
            loc: Loc(0, 0),
        };
        eval.init_builtin();
        eval.init_context();
        eval
    }
    pub fn init(&mut self, source_info: SourceInfo, ident_table: IdentifierTable) {
        self.source_info = source_info;
        self.ident_table = ident_table;
        self.init_builtin();
    }

    pub fn init_builtin(&mut self) {
        let id = self.ident_table.get_ident_id(&"puts".to_string());
        let info = MethodInfo::BuiltinFunc {
            name: "puts".to_string(),
            func: Evaluator::builtin_puts,
        };
        self.method_table.insert(id, info);

        let id = self.ident_table.get_ident_id(&"print".to_string());
        let info = MethodInfo::BuiltinFunc {
            name: "print".to_string(),
            func: Evaluator::builtin_print,
        };
        self.method_table.insert(id, info);

        let id = self.ident_table.get_ident_id(&"assert".to_string());
        let info = MethodInfo::BuiltinFunc {
            name: "assert".to_string(),
            func: Evaluator::builtin_assert,
        };
        self.method_table.insert(id, info);
    }

    pub fn init_context(&mut self) {
        let id = self.ident_table.get_ident_id(&"main".to_string());
        let classref = self.new_class(id, Node::new_comp_stmt(Loc(0, 0)));
        self.class_stack.push(classref);
        self.self_value = Value::Class(classref);
    }

    pub fn error_unimplemented(&self, msg: impl Into<String>, loc: Loc) -> EvalError {
        EvalError::unimplemented(msg.into(), loc)
    }

    pub fn error_name(&self, msg: impl Into<String>, loc: Loc) -> EvalError {
        EvalError::name(msg.into(), loc)
    }

    pub fn error_nomethod(&self, msg: impl Into<String>, loc: Loc) -> EvalError {
        EvalError::nomethod(msg.into(), loc)
    }

    pub fn error_break(&self, loc: Loc) -> EvalError {
        EvalError::raise_break(loc)
    }

    /// Built-in function "puts".
    pub fn builtin_puts(eval: &mut Evaluator, _receiver: Value, args: Vec<Value>) -> EvalResult {
        for arg in args {
            println!("{}", eval.val_to_s(&arg));
        }
        Ok(Value::Nil)
    }

    /// Built-in function "print".
    pub fn builtin_print(eval: &mut Evaluator, _receiver: Value, args: Vec<Value>) -> EvalResult {
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

    pub fn get_instance_info(&self, instance: InstanceRef) -> &InstanceInfo {
        self.instance_table.get(instance)
    }

    pub fn get_class_info(&self, class: ClassRef) -> &ClassInfo {
        self.class_table.get(class)
    }

    pub fn get_instance_method(
        &mut self,
        instance: InstanceRef,
        method: &Node,
    ) -> Result<MethodInfo, EvalError> {
        let id = match method.kind {
            NodeKind::Ident(id) => id,
            _ => {
                return Err(self.error_unimplemented(format!("Expected identifier."), method.loc()))
            }
        };
        let info = self.get_instance_info(instance);
        let class_info = self.get_class_info(info.class_id);
        match class_info.get_instance_method(id) {
            Some(info) => Ok(info.clone()),
            None => {
                return Err(self.error_nomethod(format!("No instance method found."), method.loc()))
            }
        }
    }

    pub fn get_class_method(
        &mut self,
        class: ClassRef,
        method: &Node,
    ) -> Result<MethodInfo, EvalError> {
        let id = match method.kind {
            NodeKind::Ident(id) => id,
            _ => {
                return Err(self.error_unimplemented(format!("Expected identifier."), method.loc()))
            }
        };
        match self.class_table.get(class).get_class_method(id) {
            Some(info) => Ok(info.clone()),
            None => {
                return Err(self.error_nomethod(format!("No class method found."), method.loc()));
            }
        }
    }

    pub fn eval(&mut self, node: &Node) -> EvalResult {
        match self.eval_node(node) {
            Ok(res) => Ok(res),
            Err(err) => {
                self.source_info.show_loc(&err.loc());
                match &err.kind {
                    EvalErrKind::RuntimeError(kind) => match kind {
                        RuntimeErrKind::Name(s) => println!("NameError ({})", s),
                        RuntimeErrKind::NoMethod(s) => println!("NoMethodError ({})", s),
                        RuntimeErrKind::Unimplemented(s) => println!("Unimplemented ({})", s),
                        RuntimeErrKind::Unreachable(s) => println!("Unreachable ({})", s),
                    },
                    EvalErrKind::Break => println!("Break"),
                }
                Err(err)
            }
        }
    }

    /// Evaluate AST.
    pub fn eval_node(&mut self, node: &Node) -> EvalResult {
        let loc = node.loc();
        match &node.kind {
            NodeKind::Nil => Ok(Value::Nil),
            NodeKind::Number(num) => Ok(Value::FixNum(*num)),
            NodeKind::Bool(b) => Ok(Value::Bool(*b)),
            NodeKind::Float(num) => Ok(Value::FloatNum(*num)),
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
            NodeKind::Ident(id) => match self.lvar_table().get(&id) {
                Some(val) => Ok(val.clone()),
                None => {
                    let name = self.ident_table.get_name(*id).clone();
                    Err(self
                        .error_name(format!("undefined local variable `{}'.", name), node.loc()))
                }
            },
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
                                return Err(
                                    self.error_unimplemented(format!("Expected bool."), rhs.loc())
                                );
                            }
                        } else {
                            return Err(
                                self.error_unimplemented(format!("Expected bool."), lhs.loc())
                            );
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
                                return Err(
                                    self.error_unimplemented(format!("Expected bool."), rhs.loc())
                                );
                            }
                        } else {
                            return Err(
                                self.error_unimplemented(format!("Expected bool."), lhs.loc())
                            );
                        }
                    }
                    _ => {}
                }
                let lhs = self.eval_node(&lhs)?;
                let rhs = self.eval_node(&rhs)?;
                match op {
                    BinOp::Add => self.eval_add(lhs, rhs, loc),
                    BinOp::Sub => self.eval_sub(lhs, rhs, loc),
                    BinOp::Mul => self.eval_mul(lhs, rhs, loc),
                    BinOp::Div => self.eval_div(lhs, rhs, loc),

                    BinOp::Shl => self.eval_shl(lhs, rhs, loc),
                    BinOp::Shr => self.eval_shr(lhs, rhs, loc),
                    BinOp::BitAnd => self.eval_bitand(lhs, rhs, loc),
                    BinOp::BitOr => self.eval_bitor(lhs, rhs, loc),
                    BinOp::BitXor => self.eval_bitxor(lhs, rhs, loc),

                    BinOp::Eq => self.eval_eq(lhs, rhs, loc),
                    BinOp::Ne => self.eval_neq(lhs, rhs, loc),
                    BinOp::Ge => self.eval_ge(lhs, rhs, loc),
                    BinOp::Gt => self.eval_gt(lhs, rhs, loc),
                    BinOp::Le => self.eval_ge(rhs, lhs, loc),
                    BinOp::Lt => self.eval_gt(rhs, lhs, loc),
                    _ => {
                        return Err(self.error_unimplemented(
                            format!("Unimplemented operator {:?}.", op),
                            node.loc(),
                        ))
                    }
                }
            }
            NodeKind::Assign(lhs, rhs) => {
                let rhs = self.eval_node(rhs)?;
                self.eval_assign(lhs, &rhs)
            }
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
            NodeKind::For(id, iter, body) => {
                let (start, end, exclude) = match &iter.kind {
                    NodeKind::Range(start, end, exclude) => (start, end, exclude),
                    _ => {
                        return Err(self.error_unimplemented(
                            "Currently, loop iterator must be Range.",
                            iter.loc(),
                        ))
                    }
                };
                let start_v = self.eval_node(start)?;
                self.eval_assign(id, &start_v)?;
                loop {
                    let var_v = self.eval_node(id)?;
                    let end_v = self.eval_node(&*end)?;
                    let cond = if *exclude {
                        self.eval_ge(var_v, end_v, end.loc())?
                    } else {
                        self.eval_gt(var_v, end_v, end.loc())?
                    };
                    if self.val_to_bool(&cond) {
                        break;
                    };
                    match self.eval_node(body) {
                        Ok(_) => {}
                        Err(err) => {
                            if let EvalErrKind::Break = err.kind {
                                break;
                            } else {
                                return Err(err);
                            }
                        }
                    };
                    let var_v = self.eval_node(id)?;
                    let new_v = self.eval_add(var_v, Value::FixNum(1), id.loc())?;
                    self.eval_assign(id, &new_v)?;
                }
                let (start, end) = (self.eval_node(&*start)?, self.eval_node(&*end)?);
                Ok(Value::Range(Box::new(start), Box::new(end), *exclude))
            }
            NodeKind::Break => {
                return Err(EvalError::raise_break(loc));
            }
            NodeKind::MethodDecl(id, params, body, _) => {
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
            NodeKind::ClassMethodDecl(id, params, body, _) => {
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
            NodeKind::ClassDecl(id, body, _) => {
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
            }
            _ => unimplemented!("{:?}", node.kind),
        }
    }

    fn eval_add(&mut self, lhs: Value, rhs: Value, loc: Loc) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs + rhs)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum(lhs as f64 + rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(Value::FloatNum(lhs + rhs as f64)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum(lhs + rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '+'", loc)),
        }
    }

    fn eval_sub(&mut self, lhs: Value, rhs: Value, loc: Loc) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs - rhs)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum(lhs as f64 - rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(Value::FloatNum(lhs - rhs as f64)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum(lhs - rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '-'", loc)),
        }
    }

    fn eval_mul(&mut self, lhs: Value, rhs: Value, loc: Loc) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs * rhs)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum(lhs as f64 * rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(Value::FloatNum(lhs * rhs as f64)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum(lhs * rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '*'", loc)),
        }
    }

    fn eval_div(&mut self, lhs: Value, rhs: Value, loc: Loc) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs / rhs)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum((lhs as f64) / rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(Value::FloatNum(lhs / (rhs as f64))),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::FloatNum(lhs / rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '*'", loc)),
        }
    }

    fn eval_shl(&mut self, lhs: Value, rhs: Value, loc: Loc) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs << rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '<<'", loc)),
        }
    }

    fn eval_shr(&mut self, lhs: Value, rhs: Value, loc: Loc) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs >> rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>>'", loc)),
        }
    }

    fn eval_bitand(&mut self, lhs: Value, rhs: Value, loc: Loc) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs & rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>>'", loc)),
        }
    }

    fn eval_bitor(&mut self, lhs: Value, rhs: Value, loc: Loc) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs | rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>>'", loc)),
        }
    }

    fn eval_bitxor(&mut self, lhs: Value, rhs: Value, loc: Loc) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::FixNum(lhs ^ rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>>'", loc)),
        }
    }

    pub fn eval_eq(&mut self, lhs: Value, rhs: Value, loc: Loc) -> EvalResult {
        match (&lhs, &rhs) {
            (Value::Nil, Value::Nil) => Ok(Value::Bool(true)),
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs == rhs)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::Bool(lhs == rhs)),
            (Value::Bool(lhs), Value::Bool(rhs)) => Ok(Value::Bool(lhs == rhs)),
            (Value::Class(lhs), Value::Class(rhs)) => Ok(Value::Bool(lhs == rhs)),
            (Value::Instance(lhs), Value::Instance(rhs)) => Ok(Value::Bool(lhs == rhs)),
            _ => Err(self.error_nomethod(format!("NoMethodError: {:?} == {:?}", lhs, rhs), loc)),
        }
    }

    fn eval_neq(&mut self, lhs: Value, rhs: Value, loc: Loc) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs != rhs)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::Bool(lhs != rhs)),
            (Value::Bool(lhs), Value::Bool(rhs)) => Ok(Value::Bool(lhs != rhs)),
            (Value::Class(lhs), Value::Class(rhs)) => Ok(Value::Bool(lhs != rhs)),
            (Value::Instance(lhs), Value::Instance(rhs)) => Ok(Value::Bool(lhs != rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '!='", loc)),
        }
    }

    fn eval_ge(&mut self, lhs: Value, rhs: Value, loc: Loc) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs >= rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs >= rhs as f64)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(Value::Bool(lhs as f64 >= rhs)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::Bool(lhs >= rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>='", loc)),
        }
    }

    fn eval_gt(&mut self, lhs: Value, rhs: Value, loc: Loc) -> EvalResult {
        match (lhs, rhs) {
            (Value::FixNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs > rhs)),
            (Value::FloatNum(lhs), Value::FixNum(rhs)) => Ok(Value::Bool(lhs > rhs as f64)),
            (Value::FixNum(lhs), Value::FloatNum(rhs)) => Ok(Value::Bool(lhs as f64 > rhs)),
            (Value::FloatNum(lhs), Value::FloatNum(rhs)) => Ok(Value::Bool(lhs > rhs)),
            (_, _) => Err(self.error_nomethod("NoMethodError: '>'", loc)),
        }
    }

    fn eval_assign(&mut self, lhs: &Node, rhs: &Value) -> EvalResult {
        match lhs.kind {
            NodeKind::Ident(id) => {
                self.lvar_table().insert(id, rhs.clone());
                Ok(rhs.clone())
            }
            NodeKind::Const(id) => {
                self.const_table.insert(id, rhs.clone());
                Ok(rhs.clone())
            }
            NodeKind::InstanceVar(id) => {
                match self.self_value {
                    Value::Instance(instance) => {
                        let info = self.instance_table.get_mut(instance);
                        info.instance_var.insert(id, rhs.clone());
                    }
                    Value::Class(class) => {
                        let info = self.class_table.get_mut(class);
                        info.instance_var.insert(id, rhs.clone());
                    }
                    _ => unreachable!("eval: Illegal self value. {:?}", self.self_value),
                };
                Ok(rhs.clone())
            }
            _ => unimplemented!(),
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
            Value::Class(class) => self.get_class_name(*class),
            Value::Instance(instance) => self.get_instance_name(*instance),
            Value::Range(start, end, exclude) => {
                let start = self.val_to_s(start);
                let end = self.val_to_s(end);
                let sym = if *exclude { "..." } else { ".." };
                format!("({}{}{})", start, sym, end)
            }
            Value::Char(c) => format!("{:x}", c),
        }
    }
}
