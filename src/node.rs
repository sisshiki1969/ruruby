use crate::parser::LvarCollector;
use crate::util::{Annot, IdentId, Loc};

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    SelfValue,
    Nil,
    Integer(i64),
    Float(f64),
    Bool(bool),
    String(String),
    InterporatedString(Vec<Node>),
    Range {
        start: Box<Node>,
        end: Box<Node>,
        exclude_end: bool,
    }, // start, end, exclude_end
    Array(NodeVec),
    BinOp(BinOp, Box<Node>, Box<Node>),
    UnOp(UnOp, Box<Node>),
    ArrayMember {
        array: Box<Node>,
        index: Vec<Node>,
    },
    Assign(Box<Node>, Box<Node>),
    AssignOp(BinOp, Box<Node>, Box<Node>),
    MulAssign(Vec<Node>, Vec<Node>),
    CompStmt(NodeVec),
    If {
        cond: Box<Node>,
        then_: Box<Node>,
        else_: Box<Node>,
    },
    For {
        param: Box<Node>,
        iter: Box<Node>,
        body: Box<Node>,
    }, // param, iter, body
    Proc {
        params: NodeVec,
        body: Box<Node>,
        lvar: LvarCollector,
    },
    Break,
    Next,
    LocalVar(IdentId),
    Ident(IdentId, bool),
    InstanceVar(IdentId),
    Const {
        toplevel: bool,
        id: IdentId,
    },
    Scope(Box<Node>, IdentId),
    Symbol(IdentId),
    Param(IdentId),
    BlockParam(IdentId),
    MethodDef(IdentId, NodeVec, Box<Node>, LvarCollector), // id, params, body
    ClassMethodDef(IdentId, NodeVec, Box<Node>, LvarCollector), // id, params, body
    ClassDef {
        id: IdentId,
        superclass: Box<Node>,
        body: Box<Node>,
        lvar: LvarCollector,
    },
    Send {
        receiver: Box<Node>,
        method: IdentId,
        args: NodeVec,
        block: Option<Box<Node>>,
        completed: bool,
    }, //receiver, method_name, args
}

#[derive(Debug, Clone, Copy, PartialEq)]
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

pub type Node = Annot<NodeKind>;
pub type NodeVec = Vec<Node>;

impl Node {
    pub fn new_nil(loc: Loc) -> Self {
        Node::new(NodeKind::Nil, loc)
    }

    pub fn new_integer(num: i64, loc: Loc) -> Self {
        Node::new(NodeKind::Integer(num), loc)
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

    pub fn new_self(loc: Loc) -> Self {
        Node::new(NodeKind::SelfValue, loc)
    }

    pub fn new_interporated_string(nodes: Vec<Node>, loc: Loc) -> Self {
        Node::new(NodeKind::InterporatedString(nodes), loc)
    }

    pub fn new_comp_stmt(nodes: Vec<Node>, loc: Loc) -> Self {
        Node::new(NodeKind::CompStmt(nodes), loc)
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
        let kind = NodeKind::ArrayMember {
            array: Box::new(array),
            index,
        };
        Node::new(kind, loc)
    }

    pub fn new_lvar(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::LocalVar(id), loc)
    }

    pub fn new_param(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::Param(id), loc)
    }

    pub fn new_block_param(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::BlockParam(id), loc)
    }

    pub fn new_identifier(id: IdentId, has_suffix: bool, loc: Loc) -> Self {
        Node::new(NodeKind::Ident(id, has_suffix), loc)
    }

    pub fn new_symbol(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::Symbol(id), loc)
    }

    pub fn new_range(start: Node, end: Node, exclude_end: bool, loc: Loc) -> Self {
        Node::new(
            NodeKind::Range {
                start: Box::new(start),
                end: Box::new(end),
                exclude_end,
            },
            loc,
        )
    }

    pub fn new_instance_var(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::InstanceVar(id), loc)
    }

    pub fn new_const(id: IdentId, toplevel: bool, loc: Loc) -> Self {
        Node::new(NodeKind::Const { toplevel, id }, loc)
    }

    pub fn new_scope(parent: Node, id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::Scope(Box::new(parent), id), loc)
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
        let loc = body.loc();
        Node::new(NodeKind::MethodDef(id, params, Box::new(body), lvar), loc)
    }

    pub fn new_class_method_decl(
        id: IdentId,
        params: Vec<Node>,
        body: Node,
        lvar: LvarCollector,
    ) -> Self {
        let loc = body.loc();
        Node::new(
            NodeKind::ClassMethodDef(id, params, Box::new(body), lvar),
            loc,
        )
    }

    pub fn new_class_decl(
        id: IdentId,
        superclass: Node,
        body: Node,
        lvar: LvarCollector,
        loc: Loc,
    ) -> Self {
        Node::new(
            NodeKind::ClassDef {
                id,
                superclass: Box::new(superclass),
                body: Box::new(body),
                lvar,
            },
            loc,
        )
    }

    pub fn new_send(
        receiver: Node,
        method: IdentId,
        args: Vec<Node>,
        block: Option<Box<Node>>,
        completed: bool,
        loc: Loc,
    ) -> Self {
        let loc = match (args.last(), &block) {
            (Some(arg), _) => loc.merge(arg.loc),
            _ => loc,
        };
        Node::new(
            NodeKind::Send {
                receiver: Box::new(receiver),
                method,
                args,
                block,
                completed,
            },
            loc,
        )
    }

    pub fn new_if(cond: Node, then_: Node, else_: Node, loc: Loc) -> Self {
        let loc = loc.merge(then_.loc()).merge(else_.loc());
        Node::new(
            NodeKind::If {
                cond: Box::new(cond),
                then_: Box::new(then_),
                else_: Box::new(else_),
            },
            loc,
        )
    }

    pub fn new_break(loc: Loc) -> Self {
        Node::new(NodeKind::Break, loc)
    }

    pub fn new_next(loc: Loc) -> Self {
        Node::new(NodeKind::Next, loc)
    }

    pub fn new_proc(params: NodeVec, body: Node, lvar: LvarCollector, loc: Loc) -> Self {
        let loc = loc.merge(body.loc());
        Node::new(
            NodeKind::Proc {
                params,
                body: Box::new(body),
                lvar,
            },
            loc,
        )
    }

    pub fn is_operation(&self) -> bool {
        match self.kind {
            NodeKind::Ident(_, _) => true,
            _ => false,
        }
    }

    pub fn is_lvar(&self) -> bool {
        match self.kind {
            NodeKind::Ident(_, false) | NodeKind::LocalVar(_) => true,
            _ => false,
        }
    }

    pub fn as_method_name(&self) -> Option<IdentId> {
        match self.kind {
            NodeKind::Const { id, .. } | NodeKind::Ident(id, _) | NodeKind::LocalVar(id) => {
                Some(id)
            }
            _ => None,
        }
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            NodeKind::BinOp(op, lhs, rhs) => write!(f, "({:?}: {}, {})", op, lhs, rhs),
            NodeKind::Ident(id, _) => write!(f, "(Ident {:?})", id),
            NodeKind::LocalVar(id) => write!(f, "(LocalVar {:?})", id),
            NodeKind::Send {
                receiver,
                method,
                args,
                ..
            } => {
                write!(f, "[ Send [{}]: [{:?}]", receiver, method)?;
                for node in args {
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
            NodeKind::If { cond, then_, else_ } => {
                write!(f, "[ If COND({}) THEN({}) ELSE({}) ]", cond, then_, else_)
            }
            _ => write!(f, "[{:?}]", self.kind),
        }
    }
}
