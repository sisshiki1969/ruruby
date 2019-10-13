use crate::lexer::{Annot, Loc};

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Number(i64),
    BinOp(BinOp, Box<Node>, Box<Node>),
    Assign(Box<Node>, Box<Node>),
    CompStmt(NodeVec),
    If(Box<Node>, Box<Node>, Box<Node>),
    LocalVar(usize),
    Param(usize),
    FuncDecl(usize, NodeVec, Box<Node>),
    Send(usize, NodeVec),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Eq,
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
    pub fn new_number(num: i64, loc: Loc) -> Self {
        Node::new(NodeKind::Number(num), loc)
    }

    pub fn new_comp_stmt(loc: Loc) -> Self {
        Node::new(NodeKind::CompStmt(vec![]), loc)
    }

    pub fn new_binop(op: BinOp, lhs: Node, rhs: Node) -> Self {
        let loc = (lhs.loc()).merge(rhs.loc());
        let kind = NodeKind::BinOp(op, Box::new(lhs), Box::new(rhs));
        Node::new(kind, loc)
    }

    pub fn new_local_var(id: usize, loc: Loc) -> Self {
        Node::new(NodeKind::LocalVar(id), loc)
    }

    pub fn new_assign(lhs: Node, rhs: Node) -> Self {
        let loc = Loc::new(lhs.loc()).merge(rhs.loc());
        Node::new(NodeKind::Assign(Box::new(lhs), Box::new(rhs)), loc)
    }

    pub fn new_method_decl(id: usize, params: Vec<Node>, body: Node) -> Self {
        let loc = Loc::new(body.loc());
        Node::new(NodeKind::FuncDecl(id, params, Box::new(body)), loc)
    }

    pub fn new_send(id: usize, args: Vec<Node>, loc: Loc) -> Self {
        Node::new(NodeKind::Send(id, args), loc)
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            NodeKind::BinOp(op, lhs, rhs) => write!(f, "({:?}: {}, {})", op, lhs, rhs),
            NodeKind::LocalVar(id) => write!(f, "(LocalVar {})", id),
            NodeKind::Send(id, nodes) => {
                write!(f, "[ Send {}: ", id)?;
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
            NodeKind::FuncDecl(id, args, body) => {
                write!(f, "[ FuncDecl {}: PARAM(", id)?;
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
