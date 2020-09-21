use super::parser::LvarCollector;
use crate::id_table::IdentId;
use crate::util::{Annot, Loc};
use crate::value::real::Real;

pub type Node = Annot<NodeKind>;

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    SelfValue,
    Nil,
    Integer(i64),
    Float(f64),
    Imaginary(Real),
    Bool(bool),
    String(String),
    InterporatedString(Vec<Node>),
    Symbol(IdentId),
    Range {
        start: Box<Node>,
        end: Box<Node>,
        exclude_end: bool,
    }, // start, end, exclude_end
    Array(Vec<Node>, bool),        // Vec<ELEM>, is_constant_expr
    Hash(Vec<(Node, Node)>, bool), // Vec<KEY, VALUE>, is_constant_expr
    RegExp(Vec<Node>, bool),       // Vec<STRING>, is_constant_expr

    LocalVar(IdentId),
    Ident(IdentId),
    InstanceVar(IdentId),
    GlobalVar(IdentId),
    Const {
        toplevel: bool,
        id: IdentId,
    },
    Scope(Box<Node>, IdentId),

    BinOp(BinOp, Box<Node>, Box<Node>),
    UnOp(UnOp, Box<Node>),
    Index {
        base: Box<Node>,
        index: Vec<Node>,
    },
    Splat(Box<Node>),
    AssignOp(BinOp, Box<Node>, Box<Node>),
    MulAssign(Vec<Node>, Vec<Node>), // mlhs, mrhs

    CompStmt(Vec<Node>),
    If {
        cond: Box<Node>,
        then_: Box<Node>,
        else_: Box<Node>,
    },
    For {
        param: Box<Node>,
        iter: Box<Node>,
        body: Box<Node>,
    },
    While {
        cond: Box<Node>,
        body: Box<Node>,
        cond_op: bool, // true: While, false: Until
    },
    Case {
        cond: Option<Box<Node>>,
        when_: Vec<CaseBranch>,
        else_: Box<Node>,
    },
    Begin {
        body: Box<Node>,
        rescue: Vec<(Node, Node)>, // (ex_class_list, ex_param)
        else_: Box<Node>,
        ensure: Box<Node>,
    },
    Proc {
        params: Vec<Node>,
        body: Box<Node>,
        lvar: LvarCollector,
    },
    Break(Box<Node>),
    Next(Box<Node>),
    Return(Box<Node>),
    Yield(SendArgs),

    Param(IdentId),
    PostParam(IdentId),
    OptionalParam(IdentId, Box<Node>),
    RestParam(IdentId),
    KeywordParam(IdentId, Box<Option<Node>>),
    BlockParam(IdentId),

    MethodDef(IdentId, Vec<Node>, Box<Node>, LvarCollector), // id, params, body
    SingletonMethodDef(Box<Node>, IdentId, Vec<Node>, Box<Node>, LvarCollector), // singleton_class, id, params, body
    ClassDef {
        id: IdentId,
        superclass: Box<Node>,
        body: Box<Node>,
        lvar: LvarCollector,
        is_module: bool,
    },
    SingletonClassDef {
        singleton: Box<Node>,
        body: Box<Node>,
        lvar: LvarCollector,
    },
    Send {
        receiver: Box<Node>,
        method: IdentId,
        send_args: SendArgs,
        completed: bool,
        safe_nav: bool,
    }, //receiver, method_name, args
}

#[derive(Debug, Clone, PartialEq)]
pub struct SendArgs {
    pub args: Vec<Node>,
    pub kw_args: Vec<(IdentId, Node)>,
    pub block: Option<Box<Node>>,
}

impl SendArgs {
    pub fn default() -> Self {
        SendArgs {
            args: vec![],
            kw_args: vec![],
            block: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseBranch {
    pub when: Vec<Node>,
    pub body: Box<Node>,
}

impl CaseBranch {
    pub fn new(when: Vec<Node>, body: Node) -> Self {
        CaseBranch {
            when,
            body: Box::new(body),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Exp,
    Shr,
    Shl,
    BitAnd,
    BitOr,
    BitXor,
    Eq,
    Ne,
    TEq,
    Gt,
    Ge,
    Lt,
    Le,
    Cmp,
    LAnd,
    LOr,
    Match,
}

impl BinOp {
    pub fn is_cmp_op(&self) -> bool {
        match self {
            BinOp::Eq | BinOp::Ne | BinOp::Ge | BinOp::Gt | BinOp::Le | BinOp::Lt => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    BitNot,
    Not,
    Pos,
    Neg,
}

impl Node {
    pub fn is_empty(&self) -> bool {
        match &self.kind {
            NodeKind::CompStmt(nodes) => nodes.len() == 0,
            _ => false,
        }
    }

    pub fn is_const_expr(&self) -> bool {
        match &self.kind {
            NodeKind::Bool(_)
            | NodeKind::Integer(_)
            | NodeKind::Float(_)
            | NodeKind::Nil
            | NodeKind::Symbol(_)
            | NodeKind::String(_)
            | NodeKind::Hash(_, true)
            | NodeKind::RegExp(_, true)
            | NodeKind::Array(_, true) => true,
            _ => false,
        }
    }

    pub fn is_variable(&self) -> bool {
        match &self.kind {
            NodeKind::Ident(_)
            | NodeKind::LocalVar(_)
            | NodeKind::Const { .. }
            | NodeKind::InstanceVar(_)
            | NodeKind::GlobalVar(_) => true,
            _ => false,
        }
    }

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

    pub fn new_imaginary(num: Real, loc: Loc) -> Self {
        Node::new(NodeKind::Imaginary(num), loc)
    }

    pub fn new_string(s: String, loc: Loc) -> Self {
        Node::new(NodeKind::String(s), loc)
    }

    pub fn new_array(nodes: Vec<Node>, loc: Loc) -> Self {
        let loc = match nodes.last() {
            Some(node) => loc.merge(node.loc()),
            None => loc,
        };
        let is_const = nodes.iter().all(|n| n.is_const_expr());
        Node::new(NodeKind::Array(nodes, is_const), loc)
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

    pub fn new_hash(key_value: Vec<(Node, Node)>, loc: Loc) -> Self {
        let is_const = key_value
            .iter()
            .all(|(k, v)| k.is_const_expr() && v.is_const_expr());
        Node::new(NodeKind::Hash(key_value, is_const), loc)
    }

    pub fn new_regexp(regex: Vec<Node>, loc: Loc) -> Self {
        let is_const = regex.iter().all(|n| n.is_const_expr());
        Node::new(NodeKind::RegExp(regex, is_const), loc)
    }

    pub fn new_self(loc: Loc) -> Self {
        Node::new(NodeKind::SelfValue, loc)
    }

    pub fn new_interporated_string(nodes: Vec<Node>, loc: Loc) -> Self {
        Node::new(NodeKind::InterporatedString(nodes), loc)
    }

    pub fn new_comp_stmt(nodes: Vec<Node>, mut loc: Loc) -> Self {
        if let Some(node) = nodes.first() {
            loc = node.loc();
        };
        if let Some(node) = nodes.last() {
            loc = loc.merge(node.loc());
        };
        Node::new(NodeKind::CompStmt(nodes), loc)
    }

    pub fn new_nop(loc: Loc) -> Self {
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

    pub fn new_array_member(array: Node, index: Vec<Node>, loc: Loc) -> Self {
        let kind = NodeKind::Index {
            base: Box::new(array),
            index,
        };
        Node::new(kind, loc)
    }

    pub fn new_splat(array: Node, loc: Loc) -> Self {
        let loc = loc.merge(array.loc());
        Node::new(NodeKind::Splat(Box::new(array)), loc)
    }

    pub fn new_lvar(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::LocalVar(id), loc)
    }

    pub fn new_param(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::Param(id), loc)
    }

    pub fn new_optional_param(id: IdentId, default: Node, loc: Loc) -> Self {
        Node::new(NodeKind::OptionalParam(id, Box::new(default)), loc)
    }

    pub fn new_splat_param(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::RestParam(id), loc)
    }

    pub fn new_post_param(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::PostParam(id), loc)
    }

    pub fn new_keyword_param(id: IdentId, default: Option<Node>, loc: Loc) -> Self {
        Node::new(NodeKind::KeywordParam(id, Box::new(default)), loc)
    }

    pub fn new_block_param(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::BlockParam(id), loc)
    }

    pub fn new_identifier(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::Ident(id), loc)
    }

    pub fn new_symbol(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::Symbol(id), loc)
    }

    pub fn new_instance_var(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::InstanceVar(id), loc)
    }

    pub fn new_global_var(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::GlobalVar(id), loc)
    }

    pub fn new_const(id: IdentId, toplevel: bool, loc: Loc) -> Self {
        Node::new(NodeKind::Const { toplevel, id }, loc)
    }

    pub fn new_scope(parent: Node, id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::Scope(Box::new(parent), id), loc)
    }

    pub fn new_mul_assign(mlhs: Vec<Node>, mrhs: Vec<Node>) -> Self {
        let splat_flag = mrhs.iter().find(|n| n.is_splat()).is_some();
        let mrhs = if splat_flag || mlhs.len() == 1 && mrhs.len() != 1 {
            let loc = mrhs[0].loc();
            vec![Node::new_array(mrhs, loc)]
        } else {
            mrhs
        };
        let loc = mlhs[0].loc().merge(mrhs.last().unwrap().loc());
        Node::new(NodeKind::MulAssign(mlhs, mrhs), loc)
    }

    pub fn new_single_assign(lhs: Node, rhs: Node) -> Self {
        let loc = lhs.loc().merge(rhs.loc());
        Node::new(NodeKind::MulAssign(vec![lhs], vec![rhs]), loc)
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

    pub fn new_singleton_method_decl(
        singleton: Node,
        id: IdentId,
        params: Vec<Node>,
        body: Node,
        lvar: LvarCollector,
    ) -> Self {
        let loc = body.loc();
        Node::new(
            NodeKind::SingletonMethodDef(Box::new(singleton), id, params, Box::new(body), lvar),
            loc,
        )
    }

    pub fn new_class_decl(
        id: IdentId,
        superclass: Node,
        body: Node,
        lvar: LvarCollector,
        is_module: bool,
        loc: Loc,
    ) -> Self {
        Node::new(
            NodeKind::ClassDef {
                id,
                superclass: Box::new(superclass),
                body: Box::new(body),
                is_module,
                lvar,
            },
            loc,
        )
    }

    pub fn new_singleton_class_decl(
        singleton: Node,
        body: Node,
        lvar: LvarCollector,
        loc: Loc,
    ) -> Self {
        Node::new(
            NodeKind::SingletonClassDef {
                singleton: Box::new(singleton),
                body: Box::new(body),
                lvar,
            },
            loc,
        )
    }

    pub fn new_send(
        receiver: Node,
        method: IdentId,
        send_args: SendArgs,
        completed: bool,
        safe_nav: bool,
        loc: Loc,
    ) -> Self {
        Node::new(
            NodeKind::Send {
                receiver: Box::new(receiver),
                method,
                send_args,
                completed,
                safe_nav,
            },
            loc,
        )
    }

    pub fn new_send_noarg(
        receiver: Node,
        method: IdentId,
        completed: bool,
        safe_nav: bool,
        loc: Loc,
    ) -> Self {
        let send_args = SendArgs {
            args: vec![],
            kw_args: vec![],
            block: None,
        };
        Node::new(
            NodeKind::Send {
                receiver: Box::new(receiver),
                method,
                send_args,
                completed,
                safe_nav,
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

    pub fn new_while(cond: Node, body: Node, cond_op: bool, loc: Loc) -> Self {
        let loc = loc.merge(body.loc());
        Node::new(
            NodeKind::While {
                cond: Box::new(cond),
                body: Box::new(body),
                cond_op,
            },
            loc,
        )
    }

    pub fn new_case(cond: Option<Node>, when_: Vec<CaseBranch>, else_: Node, loc: Loc) -> Self {
        let loc = loc.merge(else_.loc());
        Node::new(
            NodeKind::Case {
                cond: match cond {
                    Some(cond) => Some(Box::new(cond)),
                    None => None,
                },
                when_,
                else_: Box::new(else_),
            },
            loc,
        )
    }

    pub fn new_begin(
        body: Node,
        rescue: Vec<(Node, Node)>,
        else_: Node,
        ensure: Node,
        loc: Loc,
    ) -> Self {
        Node::new(
            NodeKind::Begin {
                body: Box::new(body),
                rescue,
                else_: Box::new(else_),
                ensure: Box::new(ensure),
            },
            loc,
        )
    }

    pub fn new_break(val: Node, loc: Loc) -> Self {
        Node::new(NodeKind::Break(Box::new(val)), loc)
    }

    pub fn new_next(val: Node, loc: Loc) -> Self {
        Node::new(NodeKind::Next(Box::new(val)), loc)
    }

    pub fn new_return(val: Node, loc: Loc) -> Self {
        Node::new(NodeKind::Return(Box::new(val)), loc)
    }

    pub fn new_yield(args: SendArgs, loc: Loc) -> Self {
        Node::new(NodeKind::Yield(args), loc)
    }

    pub fn new_proc(params: Vec<Node>, body: Node, lvar: LvarCollector, loc: Loc) -> Self {
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
            NodeKind::Ident(_) => true,
            _ => false,
        }
    }

    pub fn is_lvar(&self) -> bool {
        match self.kind {
            NodeKind::Ident(_) | NodeKind::LocalVar(_) => true,
            _ => false,
        }
    }

    pub fn is_splat(&self) -> bool {
        match self.kind {
            NodeKind::Splat(_) => true,
            _ => false,
        }
    }

    pub fn is_imm_u32(&self) -> Option<u32> {
        if let NodeKind::Integer(i) = self.kind {
            if 0 <= i && i <= u32::max_value() as i64 {
                Some(i as u32)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn is_imm_i32(&self) -> Option<i32> {
        if let NodeKind::Integer(i) = self.kind {
            if i32::min_value() as i64 <= i && i <= i32::max_value() as i64 {
                Some(i as i32)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn as_method_name(&self) -> Option<IdentId> {
        match self.kind {
            NodeKind::Const { id, .. } | NodeKind::Ident(id) | NodeKind::LocalVar(id) => Some(id),
            _ => None,
        }
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            NodeKind::BinOp(op, lhs, rhs) => write!(f, "({:?}: {}, {})", op, lhs, rhs),
            NodeKind::Ident(id) => write!(f, "(Ident {:?})", id),
            NodeKind::LocalVar(id) => write!(f, "(LocalVar {:?})", id),
            NodeKind::Send {
                receiver,
                method,
                send_args,
                ..
            } => {
                write!(f, "[ Send [{}]: [{:?}]", receiver, method)?;
                for node in &send_args.args {
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
