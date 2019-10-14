use crate::util::*;

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    SelfValue,
    Number(i64),
    BinOp(BinOp, Box<Node>, Box<Node>),
    Assign(Box<Node>, Box<Node>),
    CompStmt(NodeVec),
    If(Box<Node>, Box<Node>, Box<Node>),
    LocalVar(IdentId),
    Const(IdentId),
    Param(IdentId),
    FuncDecl(IdentId, NodeVec, Box<Node>),
    ClassDecl(IdentId, Box<Node>),
    Send(IdentId, NodeVec),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
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

    pub fn new_local_var(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::LocalVar(id), loc)
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
        Node::new(NodeKind::FuncDecl(id, params, Box::new(body)), loc)
    }

    pub fn new_class_decl(id: IdentId, body: Node) -> Self {
        let loc = Loc::new(body.loc());
        Node::new(NodeKind::ClassDecl(id, Box::new(body)), loc)
    }

    pub fn new_send(id: IdentId, args: Vec<Node>, loc: Loc) -> Self {
        Node::new(NodeKind::Send(id, args), loc)
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            NodeKind::BinOp(op, lhs, rhs) => write!(f, "({:?}: {}, {})", op, lhs, rhs),
            NodeKind::LocalVar(id) => write!(f, "(LocalVar {:?})", id),
            NodeKind::Send(id, nodes) => {
                write!(f, "[ Send {:?}: ", id)?;
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
                write!(f, "[ FuncDecl {:?}: PARAM(", id)?;
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
