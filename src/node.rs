use crate::parser::LvarCollector;
use crate::util::{Annot, IdentId, Loc};

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    SelfValue,
    Nil,
    Number(i64),
    Float(f64),
    Bool(bool),
    String(String),
    InterporatedString(Vec<Node>),
    Range(Box<Node>, Box<Node>, bool), // start, end, exclude_end
    Array(NodeVec),
    BinOp(BinOp, Box<Node>, Box<Node>),
    UnOp(UnOp, Box<Node>),
    ArrayMember(Box<Node>, Vec<Node>),
    Assign(Box<Node>, Box<Node>),
    MulAssign(Vec<Node>, Vec<Node>),
    CompStmt(NodeVec),
    If(Box<Node>, Box<Node>, Box<Node>),
    For(Box<Node>, Box<Node>, Box<Node>), // params, iter, body
    Break,
    Next,
    Ident(IdentId),
    InstanceVar(IdentId),
    Const(IdentId),
    Symbol(IdentId),
    Param(IdentId),
    MethodDef(IdentId, NodeVec, Box<Node>, LvarCollector), // id, params, body
    ClassMethodDef(IdentId, NodeVec, Box<Node>, LvarCollector), // id, params, body
    ClassDef(IdentId, Box<Node>, LvarCollector),
    Send(Box<Node>, Box<Node>, NodeVec), //receiver, method_name, args
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Shr,
    Shl,
    BitAnd,
    BitOr,
    BitXor,
    Eq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    LAnd,
    LOr,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnOp {
    BitNot,
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

    pub fn new_interporated_string(nodes: Vec<Node>, loc: Loc) -> Self {
        Node::new(NodeKind::InterporatedString(nodes), loc)
    }

    pub fn new_comp_stmt(loc: Loc) -> Self {
        Node::new(NodeKind::CompStmt(vec![]), loc)
    }

    pub fn new_binop(op: BinOp, lhs: Node, rhs: Node) -> Self {
        let loc = (lhs.loc()).merge(rhs.loc());
        let kind = NodeKind::BinOp(op, Box::new(lhs), Box::new(rhs));
        Node::new(kind, loc)
    }

    pub fn new_unop(op: UnOp, lhs: Node, loc: Loc) -> Self {
        let loc = loc.merge(lhs.loc());
        let kind = NodeKind::UnOp(op, Box::new(lhs));
        Node::new(kind, loc)
    }

    pub fn new_array_member(array: Node, index: Vec<Node>) -> Self {
        // index must be 1 or 2
        let start_loc = index[0].loc();
        let end_loc = index[index.len() - 1].loc();
        let loc = array.loc().merge(start_loc).merge(end_loc);
        let kind = NodeKind::ArrayMember(Box::new(array), index);
        Node::new(kind, loc)
    }

    pub fn new_identifier(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::Ident(id), loc)
    }

    pub fn new_symbol(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::Symbol(id), loc)
    }

    pub fn new_range(start: Node, end: Node, exclude_end: bool, loc: Loc) -> Self {
        Node::new(
            NodeKind::Range(Box::new(start), Box::new(end), exclude_end),
            loc,
        )
    }

    pub fn new_instance_var(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::InstanceVar(id), loc)
    }

    pub fn new_const(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::Const(id), loc)
    }

    pub fn new_assign(lhs: Node, rhs: Node) -> Self {
        let loc = lhs.loc().merge(rhs.loc());
        Node::new(NodeKind::Assign(Box::new(lhs), Box::new(rhs)), loc)
    }

    pub fn new_mul_assign(lhs: Vec<Node>, rhs: Vec<Node>) -> Self {
        let loc = lhs[0].loc().merge(rhs[rhs.len() - 1].loc());
        Node::new(NodeKind::MulAssign(lhs, rhs), loc)
    }

    pub fn new_method_decl(
        id: IdentId,
        params: Vec<Node>,
        body: Node,
        lvar: LvarCollector,
    ) -> Self {
        let loc = Loc::new(body.loc());
        Node::new(NodeKind::MethodDef(id, params, Box::new(body), lvar), loc)
    }

    pub fn new_class_method_decl(
        id: IdentId,
        params: Vec<Node>,
        body: Node,
        lvar: LvarCollector,
    ) -> Self {
        let loc = Loc::new(body.loc());
        Node::new(
            NodeKind::ClassMethodDef(id, params, Box::new(body), lvar),
            loc,
        )
    }

    pub fn new_class_decl(id: IdentId, body: Node, lvar: LvarCollector) -> Self {
        let loc = Loc::new(body.loc());
        Node::new(NodeKind::ClassDef(id, Box::new(body), lvar), loc)
    }

    pub fn new_send(receiver: Node, method_name: Node, args: Vec<Node>, loc: Loc) -> Self {
        Node::new(
            NodeKind::Send(Box::new(receiver), Box::new(method_name), args),
            loc,
        )
    }

    pub fn new_break(loc: Loc) -> Self {
        Node::new(NodeKind::Break, loc)
    }

    pub fn new_next(loc: Loc) -> Self {
        Node::new(NodeKind::Next, loc)
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
            NodeKind::MethodDef(id, args, body, _) => {
                write!(f, "[ MethodDef {:?}: PARAM(", id)?;
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
