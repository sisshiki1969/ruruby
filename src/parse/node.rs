use super::parser::{LvarCollector, RescueEntry};
use crate::id_table::IdentId;
use crate::util::{Annot, Loc};
use crate::value::real::Real;
use num::BigInt;

pub type Node = Annot<NodeKind>;

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    SelfValue,
    Nil,
    Integer(i64),
    Bignum(BigInt),
    Float(f64),
    Imaginary(Real),
    Bool(bool),
    String(String),
    InterporatedString(Vec<Node>),
    Command(Box<Node>),
    Symbol(IdentId),
    Range {
        start: Box<Node>,
        end: Box<Node>,
        exclude_end: bool,
        is_const: bool,
    }, // start, end, exclude_end
    Array(Vec<Node>, bool),        // Vec<ELEM>, is_constant_expr
    Hash(Vec<(Node, Node)>, bool), // Vec<KEY, VALUE>, is_constant_expr
    RegExp(Vec<Node>, bool),       // Vec<STRING>, is_constant_expr

    LocalVar(IdentId),
    Ident(IdentId),
    InstanceVar(IdentId),
    GlobalVar(IdentId),
    SpecialVar(usize),
    ClassVar(IdentId),
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
        param: Vec<IdentId>,
        iter: Box<Node>,
        body: Block,
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
        rescue: Vec<RescueEntry>, // (ex_class_list, ex_param)
        else_: Option<Box<Node>>,
        ensure: Option<Box<Node>>,
    },
    Lambda(Block),
    Break(Box<Node>),
    Next(Box<Node>),
    Return(Box<Node>),
    Yield(ArgList),
    MethodDef(IdentId, Vec<FormalParam>, Box<Node>, LvarCollector), // id, params, body
    SingletonMethodDef(
        Box<Node>,
        IdentId,
        Vec<FormalParam>,
        Box<Node>,
        LvarCollector,
    ), // singleton_class, id, params, body
    ClassDef {
        base: Box<Node>,
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
        arglist: ArgList,
        safe_nav: bool,
    },

    Defined(Box<Node>),
    Super(Option<ArgList>),
    AliasMethod(Box<Node>, Box<Node>), // (new_method, old_method)
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub params: Vec<FormalParam>,
    pub body: Box<Node>,
    pub lvar: LvarCollector,
}

impl Block {
    pub(crate) fn new(params: Vec<FormalParam>, body: Node, lvar: LvarCollector) -> Self {
        Block {
            params,
            body: Box::new(body),
            lvar,
        }
    }
}

pub type FormalParam = Annot<ParamKind>;

#[derive(Debug, Clone, PartialEq)]
pub enum ParamKind {
    Param(IdentId),
    Post(IdentId),
    Optional(IdentId, Box<Node>), // name, default expr
    Rest(IdentId),
    RestDiscard,
    Keyword(IdentId, Option<Box<Node>>), // name, default expr
    KWRest(IdentId),
    Block(IdentId),
    Delegate,
}

impl FormalParam {
    pub(crate) fn req_param(id: IdentId, loc: Loc) -> Self {
        FormalParam::new(ParamKind::Param(id), loc)
    }

    pub(crate) fn optional(id: IdentId, default: Node, loc: Loc) -> Self {
        FormalParam::new(ParamKind::Optional(id, Box::new(default)), loc)
    }

    pub(crate) fn rest(id: IdentId, loc: Loc) -> Self {
        FormalParam::new(ParamKind::Rest(id), loc)
    }

    pub(crate) fn rest_discard(loc: Loc) -> Self {
        FormalParam::new(ParamKind::RestDiscard, loc)
    }

    pub(crate) fn post(id: IdentId, loc: Loc) -> Self {
        FormalParam::new(ParamKind::Post(id), loc)
    }

    pub(crate) fn keyword(id: IdentId, default: Option<Node>, loc: Loc) -> Self {
        FormalParam::new(
            ParamKind::Keyword(id, default.map_or(None, |x| Some(Box::new(x)))),
            loc,
        )
    }

    pub(crate) fn kwrest(id: IdentId, loc: Loc) -> Self {
        FormalParam::new(ParamKind::KWRest(id), loc)
    }

    pub(crate) fn block(id: IdentId, loc: Loc) -> Self {
        FormalParam::new(ParamKind::Block(id), loc)
    }

    pub(crate) fn delegeate(loc: Loc) -> Self {
        FormalParam::new(ParamKind::Delegate, loc)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArgList {
    /// positional args
    pub args: Vec<Node>,
    /// keyword args
    pub kw_args: Vec<(IdentId, Node)>,
    /// double splat args (**{})
    pub hash_splat: Vec<Node>,
    /// block
    pub block: Option<Box<Node>>,
    /// args delegate
    pub delegate: bool,
}

impl ArgList {
    pub(crate) fn default() -> Self {
        ArgList {
            args: vec![],
            kw_args: vec![],
            hash_splat: vec![],
            block: None,
            delegate: false,
        }
    }

    /*pub(crate) fn with_args(args: Vec<Node>) -> Self {
        ArgList {
            args: args,
            kw_args: vec![],
            hash_splat: vec![],
            block: None,
            delegate: false,
        }
    }*/

    pub(crate) fn with_block(block: Box<Node>) -> Self {
        ArgList {
            args: vec![],
            kw_args: vec![],
            hash_splat: vec![],
            block: Some(block),
            delegate: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseBranch {
    pub when: Vec<Node>,
    pub body: Box<Node>,
}

impl CaseBranch {
    pub(crate) fn new(when: Vec<Node>, body: Node) -> Self {
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
    pub(crate) fn is_cmp_op(&self) -> bool {
        match self {
            BinOp::Eq | BinOp::Ne | BinOp::Ge | BinOp::Gt | BinOp::Le | BinOp::Lt => true,
            _ => false,
        }
    }

    pub(crate) fn to_method(&self) -> IdentId {
        let s = match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
            Self::Rem => "%",
            Self::Exp => "**",
            Self::Shr => ">>",
            Self::Shl => "<<",
            Self::BitAnd => "&",
            Self::BitOr => "|",
            Self::BitXor => "^",
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::TEq => "===",
            Self::Gt => ">",
            Self::Ge => ">=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Cmp => "<=>",
            Self::LAnd => "&&",
            Self::LOr => "||",
            Self::Match => "=~",
        };
        IdentId::get_id(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    BitNot,
    Not,
    Pos,
    Neg,
}

impl UnOp {
    pub(crate) fn to_method(&self) -> IdentId {
        let s = match self {
            Self::BitNot => "~",
            Self::Not => "!",
            Self::Pos => "+@",
            Self::Neg => "-@",
        };
        IdentId::get_id(s)
    }
}

impl Node {
    pub(crate) fn is_empty(&self) -> bool {
        match &self.kind {
            NodeKind::CompStmt(nodes) => nodes.len() == 0,
            _ => false,
        }
    }

    pub(crate) fn is_integer(&self) -> bool {
        match &self.kind {
            NodeKind::Integer(_) => true,
            _ => false,
        }
    }

    pub(crate) fn is_const_expr(&self) -> bool {
        match &self.kind {
            NodeKind::Bool(_)
            | NodeKind::Integer(_)
            | NodeKind::Float(_)
            | NodeKind::Nil
            | NodeKind::Symbol(_)
            | NodeKind::String(_) => true,
            _ => false,
        }
    }

    pub(crate) fn new_nil(loc: Loc) -> Self {
        Node::new(NodeKind::Nil, loc)
    }

    pub(crate) fn new_integer(num: i64, loc: Loc) -> Self {
        Node::new(NodeKind::Integer(num), loc)
    }

    pub(crate) fn new_bignum(num: BigInt, loc: Loc) -> Self {
        Node::new(NodeKind::Bignum(num), loc)
    }

    pub(crate) fn new_bool(b: bool, loc: Loc) -> Self {
        Node::new(NodeKind::Bool(b), loc)
    }

    pub(crate) fn new_float(num: f64, loc: Loc) -> Self {
        Node::new(NodeKind::Float(num), loc)
    }

    pub(crate) fn new_imaginary(num: Real, loc: Loc) -> Self {
        Node::new(NodeKind::Imaginary(num), loc)
    }

    pub(crate) fn new_string(s: String, loc: Loc) -> Self {
        Node::new(NodeKind::String(s), loc)
    }

    pub(crate) fn new_array(nodes: Vec<Node>, loc: Loc) -> Self {
        let loc = match nodes.last() {
            Some(node) => loc.merge(node.loc()),
            None => loc,
        };
        let is_const = nodes.iter().all(|n| n.is_const_expr());
        Node::new(NodeKind::Array(nodes, is_const), loc)
    }

    pub(crate) fn new_range(start: Node, end: Node, exclude_end: bool, loc: Loc) -> Self {
        let is_const = start.is_integer() && end.is_integer();
        Node::new(
            NodeKind::Range {
                start: Box::new(start),
                end: Box::new(end),
                exclude_end,
                is_const,
            },
            loc,
        )
    }

    pub(crate) fn new_hash(key_value: Vec<(Node, Node)>, loc: Loc) -> Self {
        let is_const = key_value
            .iter()
            .all(|(k, v)| k.is_const_expr() && v.is_const_expr());
        Node::new(NodeKind::Hash(key_value, is_const), loc)
    }

    pub(crate) fn new_regexp(regex: Vec<Node>, loc: Loc) -> Self {
        let is_const = regex.iter().all(|n| n.is_const_expr());
        Node::new(NodeKind::RegExp(regex, is_const), loc)
    }

    pub(crate) fn new_self(loc: Loc) -> Self {
        Node::new(NodeKind::SelfValue, loc)
    }

    pub(crate) fn new_interporated_string(nodes: Vec<Node>, loc: Loc) -> Self {
        Node::new(NodeKind::InterporatedString(nodes), loc)
    }

    pub(crate) fn new_command(node: Node) -> Self {
        let loc = node.loc;
        Node::new(NodeKind::Command(Box::new(node)), loc)
    }

    pub(crate) fn new_defined(node: Node) -> Self {
        let loc = node.loc;
        Node::new(NodeKind::Defined(Box::new(node)), loc)
    }

    pub(crate) fn new_alias(new: Node, old: Node, loc: Loc) -> Self {
        Node::new(NodeKind::AliasMethod(Box::new(new), Box::new(old)), loc)
    }

    pub(crate) fn new_comp_stmt(nodes: Vec<Node>, mut loc: Loc) -> Self {
        if let Some(node) = nodes.first() {
            loc = node.loc();
        };
        if let Some(node) = nodes.last() {
            loc = loc.merge(node.loc());
        };
        Node::new(NodeKind::CompStmt(nodes), loc)
    }

    /*pub(crate) fn new_nop(loc: Loc) -> Self {
        Node::new(NodeKind::CompStmt(vec![]), loc)
    }*/

    pub(crate) fn new_binop(op: BinOp, lhs: Node, rhs: Node) -> Self {
        let loc = (lhs.loc()).merge(rhs.loc());
        let kind = NodeKind::BinOp(op, Box::new(lhs), Box::new(rhs));
        Node::new(kind, loc)
    }

    pub(crate) fn new_unop(op: UnOp, lhs: Node, loc: Loc) -> Self {
        let loc = loc.merge(lhs.loc());
        let kind = NodeKind::UnOp(op, Box::new(lhs));
        Node::new(kind, loc)
    }

    pub(crate) fn new_array_member(array: Node, index: Vec<Node>, loc: Loc) -> Self {
        let kind = NodeKind::Index {
            base: Box::new(array),
            index,
        };
        Node::new(kind, loc)
    }

    pub(crate) fn new_splat(array: Node, loc: Loc) -> Self {
        let loc = loc.merge(array.loc());
        Node::new(NodeKind::Splat(Box::new(array)), loc)
    }

    pub(crate) fn new_lvar(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::LocalVar(id), loc)
    }

    pub(crate) fn new_identifier(name: &str, loc: Loc) -> Self {
        let id = IdentId::get_id(name);
        Node::new(NodeKind::Ident(id), loc)
    }

    pub(crate) fn new_symbol(id: IdentId, loc: Loc) -> Self {
        Node::new(NodeKind::Symbol(id), loc)
    }

    pub(crate) fn new_instance_var(name: &str, loc: Loc) -> Self {
        let id = IdentId::get_id(name);
        Node::new(NodeKind::InstanceVar(id), loc)
    }

    pub(crate) fn new_class_var(name: &str, loc: Loc) -> Self {
        let id = IdentId::get_id(name);
        Node::new(NodeKind::ClassVar(id), loc)
    }

    pub(crate) fn new_global_var(name: &str, loc: Loc) -> Self {
        let id = IdentId::get_id(name);
        Node::new(NodeKind::GlobalVar(id), loc)
    }

    pub(crate) fn new_special_var(id: usize, loc: Loc) -> Self {
        Node::new(NodeKind::SpecialVar(id), loc)
    }

    pub(crate) fn new_const(name: &str, toplevel: bool, loc: Loc) -> Self {
        let id = IdentId::get_id(name);
        Node::new(NodeKind::Const { toplevel, id }, loc)
    }

    pub(crate) fn new_scope(parent: Node, name: &str, loc: Loc) -> Self {
        let id = IdentId::get_id(name);
        Node::new(NodeKind::Scope(Box::new(parent), id), loc)
    }

    pub(crate) fn new_mul_assign(mlhs: Vec<Node>, mrhs: Vec<Node>) -> Self {
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

    /*pub(crate) fn new_single_assign(lhs: Node, rhs: Node) -> Self {
        let loc = lhs.loc().merge(rhs.loc());
        Node::new(NodeKind::MulAssign(vec![lhs], vec![rhs]), loc)
    }*/

    pub(crate) fn new_assign_op(op: BinOp, lhs: Node, rhs: Node) -> Self {
        let loc = lhs.loc().merge(rhs.loc());
        Node::new(NodeKind::AssignOp(op, Box::new(lhs), Box::new(rhs)), loc)
    }

    pub(crate) fn new_method_decl(
        id: IdentId,
        params: Vec<FormalParam>,
        body: Node,
        lvar: LvarCollector,
        loc: Loc,
    ) -> Self {
        let loc = body.loc().merge(loc);
        Node::new(NodeKind::MethodDef(id, params, Box::new(body), lvar), loc)
    }

    pub(crate) fn new_singleton_method_decl(
        singleton: Node,
        id: IdentId,
        params: Vec<FormalParam>,
        body: Node,
        lvar: LvarCollector,
        loc: Loc,
    ) -> Self {
        let loc = body.loc().merge(loc);
        Node::new(
            NodeKind::SingletonMethodDef(Box::new(singleton), id, params, Box::new(body), lvar),
            loc,
        )
    }

    pub(crate) fn new_class_decl(
        base: Node,
        id: IdentId,
        superclass: Node,
        body: Node,
        lvar: LvarCollector,
        is_module: bool,
        loc: Loc,
    ) -> Self {
        Node::new(
            NodeKind::ClassDef {
                base: Box::new(base),
                id,
                superclass: Box::new(superclass),
                body: Box::new(body),
                is_module,
                lvar,
            },
            loc,
        )
    }

    pub(crate) fn new_singleton_class_decl(
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

    pub(crate) fn new_send(
        receiver: Node,
        method: IdentId,
        arglist: ArgList,
        safe_nav: bool,
        loc: Loc,
    ) -> Self {
        Node::new(
            NodeKind::Send {
                receiver: Box::new(receiver),
                method,
                arglist,
                safe_nav,
            },
            loc,
        )
    }

    pub(crate) fn new_send_noarg(
        receiver: Node,
        method: IdentId,
        safe_nav: bool,
        loc: Loc,
    ) -> Self {
        let arglist = ArgList::default();
        Node::new(
            NodeKind::Send {
                receiver: Box::new(receiver),
                method,
                arglist,
                safe_nav,
            },
            loc,
        )
    }

    pub(crate) fn new_if(cond: Node, then_: Node, else_: Node, loc: Loc) -> Self {
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

    pub(crate) fn new_while(cond: Node, body: Node, cond_op: bool, loc: Loc) -> Self {
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

    pub(crate) fn new_case(
        cond: Option<Node>,
        when_: Vec<CaseBranch>,
        else_: Node,
        loc: Loc,
    ) -> Self {
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

    pub(crate) fn new_begin(
        body: Node,
        rescue: Vec<RescueEntry>, //Vec<(Vec<Node>, Box<Node>)>,
        else_: Option<Node>,
        ensure: Option<Node>,
    ) -> Self {
        let mut loc = body.loc();
        Node::new(
            NodeKind::Begin {
                body: Box::new(body),
                rescue,
                else_: match else_ {
                    Some(else_) => {
                        loc = loc.merge(else_.loc);
                        Some(Box::new(else_))
                    }
                    None => None,
                },
                ensure: match ensure {
                    Some(ensure) => {
                        loc = loc.merge(ensure.loc());
                        Some(Box::new(ensure))
                    }
                    None => None,
                },
            },
            loc,
        )
    }

    pub(crate) fn new_break(val: Node, loc: Loc) -> Self {
        Node::new(NodeKind::Break(Box::new(val)), loc)
    }

    pub(crate) fn new_next(val: Node, loc: Loc) -> Self {
        Node::new(NodeKind::Next(Box::new(val)), loc)
    }

    pub(crate) fn new_return(val: Node, loc: Loc) -> Self {
        Node::new(NodeKind::Return(Box::new(val)), loc)
    }

    pub(crate) fn new_yield(args: ArgList, loc: Loc) -> Self {
        Node::new(NodeKind::Yield(args), loc)
    }

    pub(crate) fn new_super(args: impl Into<Option<ArgList>>, loc: Loc) -> Self {
        Node::new(NodeKind::Super(args.into()), loc)
    }

    pub(crate) fn new_lambda(
        params: Vec<FormalParam>,
        body: Node,
        lvar: LvarCollector,
        loc: Loc,
    ) -> Self {
        let loc = loc.merge(body.loc());
        Node::new(NodeKind::Lambda(Block::new(params, body, lvar)), loc)
    }

    pub(crate) fn is_splat(&self) -> bool {
        match self.kind {
            NodeKind::Splat(_) => true,
            _ => false,
        }
    }

    pub(crate) fn is_imm_u32(&self) -> Option<u32> {
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

    pub(crate) fn is_imm_i32(&self) -> Option<i32> {
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

    pub(crate) fn as_method_name(&self) -> Option<IdentId> {
        match self.kind {
            NodeKind::Const { id, .. } | NodeKind::Ident(id) | NodeKind::LocalVar(id) => Some(id),
            _ => None,
        }
    }
}
/*
impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            NodeKind::BinOp(op, lhs, rhs) => write!(f, "({:?}: {}, {})", op, lhs, rhs),
            NodeKind::Ident(id) => write!(f, "(Ident {:?})", id),
            NodeKind::LocalVar(id) => write!(f, "(LocalVar {:?})", id),
            NodeKind::Send {
                receiver,
                method,
                arglist,
                ..
            } => {
                write!(f, "[ Send [{}]: [{:?}]", receiver, method)?;
                for param in &arglist.args {
                    write!(f, "({:?}) ", param)?;
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
                for param in args {
                    write!(f, "({:?}) ", param)?;
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
*/
