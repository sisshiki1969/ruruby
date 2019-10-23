use crate::util::*;

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    SelfValue,
    Nil,
    Number(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Range(i64, i64),
    BinOp(BinOp, Box<Node>, Box<Node>),
    Assign(Box<Node>, Box<Node>),
    CompStmt(NodeVec),
    If(Box<Node>, Box<Node>, Box<Node>),
    For(Vec<Node>, Box<Node>, Box<Node>), // params, iter, body
    Ident(IdentId),
    InstanceVar(IdentId),
    Const(IdentId),
    Param(IdentId),
    MethodDecl(IdentId, NodeVec, Box<Node>), // id, params, body
    ClassMethodDecl(IdentId, NodeVec, Box<Node>), // id, params, body
    ClassDecl(IdentId, Box<Node>),
    Send(Box<Node>, Box<Node>, NodeVec), //receiver, method_name, args
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    LAnd,
    LOr,
}

/*
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub kind: NodeKind,
    pub loc: Loc,
}
*/
pub type Node = Annot<NodeKind>;
pub type NodeVec = Vec<Node>;

impl Node {
    pub fn new_nil(loc: Loc) -> Self {
        Node::new(NodeKind::Nil, loc)
    }

    pub fn new_number(num: i64, loc: Loc) -> Self {
        Node::new(NodeKind::Number(num), loc)
    }

    pub fn new_bool(b: bool, loc: Loc) -> Self {
        Node::new(NodeKind::Bool(b), loc)
    }

    pub fn new_float(num: f64, loc: Loc) -> Self {
        Node::new(NodeKind::Float(num), loc)
    }

    pub fn new_string(s: String, loc: Loc) -> Self {
        Node::new(NodeKind::String(s), loc)
    }

    pub fn new_comp_stmt(loc: Loc) -> Self {
        Node::new(NodeKind::CompStmt(vec![]), loc)
    }

    pub fn new_binop(op: BinOp, lhs: Node, rhs: Node) -> Self {
        let loc = (lhs.loc()).merge(rhs.loc());
        let kind = NodeKind::BinOp(op, Box::new(lhs), Box::new(rhs));
        Node::new(kind, loc)
    }

    pub fn new_identifier(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::Ident(id), loc)
    }

    pub fn new_instance_var(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::InstanceVar(id), loc)
    }

    pub fn new_const(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::Const(id), loc)
    }

    pub fn new_assign(lhs: Node, rhs: Node) -> Self {
        let loc = Loc::new(lhs.loc()).merge(rhs.loc());
        Node::new(NodeKind::Assign(Box::new(lhs), Box::new(rhs)), loc)
    }

    pub fn new_method_decl(id: IdentId, params: Vec<Node>, body: Node) -> Self {
        let loc = Loc::new(body.loc());
        Node::new(NodeKind::MethodDecl(id, params, Box::new(body)), loc)
    }

    pub fn new_class_method_decl(id: IdentId, params: Vec<Node>, body: Node) -> Self {
        let loc = Loc::new(body.loc());
        Node::new(NodeKind::ClassMethodDecl(id, params, Box::new(body)), loc)
    }

    pub fn new_class_decl(id: IdentId, body: Node) -> Self {
        let loc = Loc::new(body.loc());
        Node::new(NodeKind::ClassDecl(id, Box::new(body)), loc)
    }

    pub fn new_send(receiver: Node, method_name: Node, args: Vec<Node>, loc: Loc) -> Self {
        Node::new(
            NodeKind::Send(Box::new(receiver), Box::new(method_name), args),
            loc,
        )
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            NodeKind::BinOp(op, lhs, rhs) => write!(f, "({:?}: {}, {})", op, lhs, rhs),
            NodeKind::Ident(id) => write!(f, "(Ident {:?})", id),
            NodeKind::Send(receiver, method_name, nodes) => {
                write!(f, "[ Send [{}]: [{}]", receiver, method_name)?;
                for node in nodes {
                    write!(f, "({}) ", node)?;
                }
                write!(f, "]")?;
                Ok(())
            }
            NodeKind::CompStmt(nodes) => {
                write!(f, "[ CompStmt ")?;
                for node in nodes {
                    write!(f, "({}) ", node)?;
                }
                write!(f, "]")?;
                Ok(())
            }
            NodeKind::MethodDecl(id, args, body) => {
                write!(f, "[ MethodDecl {:?}: PARAM(", id)?;
                for arg in args {
                    write!(f, "({}) ", arg)?;
                }
                write!(f, ") BODY({})]", body)?;
                Ok(())
            }
            NodeKind::If(cond_, then_, else_) => {
                write!(f, "[ If COND({}) THEN({}) ELSE({}) ]", cond_, then_, else_)
            }
            _ => write!(f, "[{:?}]", self.kind),
        }
    }
}
