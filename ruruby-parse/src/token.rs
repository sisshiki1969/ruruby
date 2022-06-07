use super::parser::NReal;
use super::*;
use enum_iterator::Sequence;
use node::BinOp;
use num::BigInt;
use std::fmt::*;

pub(crate) type Token = Annot<TokenKind>;

#[cfg(test)]
impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match &self.kind {
            TokenKind::Eof => write!(f, "Token![{:?}, {}],", self.kind, self.loc().0),
            TokenKind::Punct(punct) => write!(
                f,
                "Token![Punct(Punct::{:?}), {}, {}],",
                punct,
                self.loc().0,
                self.loc().1
            ),
            TokenKind::Reserved(reserved) => write!(
                f,
                "Token![Reserved(Reserved::{:?}), {}, {}],",
                reserved,
                self.loc().0,
                self.loc().1
            ),
            _ => write!(
                f,
                "Token![{:?}, {}, {}],",
                self.kind,
                self.loc().0,
                self.loc().1
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TokenKind {
    Eof,
    Ident(String),
    InstanceVar(String),
    GlobalVar(String),
    SpecialVar(usize),
    ClassVar(String),
    Const(String),
    IntegerLit(i64),
    BignumLit(BigInt),
    FloatLit(f64),
    ImaginaryLit(NReal),
    StringLit(String),
    CommandLit(String),
    Reserved(Reserved),
    Punct(Punct),
    OpenString(String, Option<char>, usize), // (content, delimiter, paren_level)
    OpenRegex(String),
    OpenCommand(String, Option<char>, usize),
    PercentNotation(char, String),
    LineTerm,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Sequence)]
pub enum Reserved {
    BEGIN,
    END,
    Alias,
    And,
    Begin,
    Break,
    Case,
    Class,
    Def,
    Defined,
    Do,
    Else,
    Elsif,
    End,
    Ensure,
    For,
    If,
    In,
    Module,
    Next,
    Or,
    Rescue,
    Return,
    Super,
    Then,
    Until,
    Unless,
    When,
    While,
    Yield,
}

impl Debug for Reserved {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let s = match self {
            Reserved::BEGIN => "BEGIN",
            Reserved::END => "END",
            Reserved::Alias => "alias",
            Reserved::And => "and",
            Reserved::Begin => "begin",
            Reserved::Break => "break",
            Reserved::Case => "case",
            Reserved::Class => "class",
            Reserved::Def => "def",
            Reserved::Defined => "defined?",
            Reserved::Do => "do",
            Reserved::Else => "else",
            Reserved::Elsif => "elsif",
            Reserved::End => "end",
            Reserved::Ensure => "ensure",
            Reserved::For => "for",
            Reserved::If => "if",
            Reserved::In => "in",
            Reserved::Module => "module",
            Reserved::Next => "next",
            Reserved::Or => "or",
            Reserved::Rescue => "rescue",
            Reserved::Return => "return",
            Reserved::Super => "super",
            Reserved::Then => "then",
            Reserved::Until => "until",
            Reserved::Unless => "unless",
            Reserved::When => "when",
            Reserved::While => "while",
            Reserved::Yield => "yield",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Punct {
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Semi,
    Colon,
    Scope,
    Comma,
    Dot,
    Question,
    Range2,
    Range3,

    Plus,
    Minus,
    Mul,
    Div,
    Rem,
    DMul,
    Shr,
    Shl,
    BitOr,
    BitAnd,
    BitXor,
    BitNot,
    Not,
    Assign,
    AssignOp(BinOp),
    Eq,
    TEq,
    Ne,
    Gt,
    Ge,
    Lt,
    Le,
    Cmp,
    LAnd,
    LOr,
    Match,
    Unmatch,
    SafeNav,

    Backslash,
    Arrow,
    FatArrow,
}

#[allow(unused)]
impl Token {
    pub(crate) fn new_ident(ident: impl Into<String>, loc: Loc) -> Self {
        Annot::new(TokenKind::Ident(ident.into()), loc)
    }

    pub(crate) fn new_instance_var(ident: impl Into<String>, loc: Loc) -> Self {
        Annot::new(TokenKind::InstanceVar(ident.into()), loc)
    }

    pub(crate) fn new_const(ident: impl Into<String>, has_suffix: bool, loc: Loc) -> Self {
        Annot::new(TokenKind::Const(ident.into()), loc)
    }

    pub(crate) fn new_global_var(ident: impl Into<String>, loc: Loc) -> Self {
        Annot::new(TokenKind::GlobalVar(ident.into()), loc)
    }

    pub(crate) fn new_reserved(ident: Reserved, loc: Loc) -> Self {
        Annot::new(TokenKind::Reserved(ident), loc)
    }

    pub(crate) fn new_intlit(num: i64, loc: Loc) -> Self {
        Annot::new(TokenKind::IntegerLit(num), loc)
    }

    pub(crate) fn new_bignumlit(num: BigInt, loc: Loc) -> Self {
        Annot::new(TokenKind::BignumLit(num), loc)
    }

    pub(crate) fn new_floatlit(num: f64, loc: Loc) -> Self {
        Annot::new(TokenKind::FloatLit(num), loc)
    }

    pub(crate) fn new_imaginarylit(num: NReal, loc: Loc) -> Self {
        Annot::new(TokenKind::ImaginaryLit(num), loc)
    }

    pub(crate) fn new_stringlit(string: impl Into<String>, loc: Loc) -> Self {
        Annot::new(TokenKind::StringLit(string.into()), loc)
    }

    pub(crate) fn new_open_string(
        s: impl Into<String>,
        delimiter: Option<char>,
        level: usize,
        loc: Loc,
    ) -> Self {
        Annot::new(TokenKind::OpenString(s.into(), delimiter, level), loc)
    }

    pub(crate) fn new_open_command(
        s: impl Into<String>,
        delimiter: Option<char>,
        level: usize,
        loc: Loc,
    ) -> Self {
        Annot::new(TokenKind::OpenCommand(s.into(), delimiter, level), loc)
    }

    pub(crate) fn new_open_reg(s: impl Into<String>, loc: Loc) -> Self {
        Annot::new(TokenKind::OpenRegex(s.into()), loc)
    }

    pub(crate) fn new_percent(kind: char, content: String, loc: Loc) -> Self {
        Annot::new(TokenKind::PercentNotation(kind, content), loc)
    }

    pub(crate) fn new_punct(punct: Punct, loc: Loc) -> Self {
        Annot::new(TokenKind::Punct(punct), loc)
    }

    pub(crate) fn new_line_term(loc: Loc) -> Self {
        Annot::new(TokenKind::LineTerm, loc)
    }

    pub(crate) fn new_eof(pos: usize) -> Self {
        Annot::new(TokenKind::Eof, Loc(pos, pos))
    }
}

impl Token {
    /// Examine the token, and return true if it is a line terminator.
    pub(crate) fn is_line_term(&self) -> bool {
        self.kind == TokenKind::LineTerm || self.kind == TokenKind::Punct(Punct::Semi)
    }

    /// Examine the token, and return true if it is EOF.
    pub(crate) fn is_eof(&self) -> bool {
        self.kind == TokenKind::Eof
    }

    /// Examine the token, and return true if it is a line terminator or ';' or EOF.
    pub(crate) fn is_term(&self) -> bool {
        matches!(
            self.kind,
            TokenKind::LineTerm | TokenKind::Eof | TokenKind::Punct(Punct::Semi)
        )
    }

    pub(crate) fn can_be_symbol(&self) -> Option<IdentId> {
        let id = match &self.kind {
            TokenKind::Ident(ident) => IdentId::get_id(ident),
            TokenKind::Const(ident) => IdentId::get_id(ident),
            TokenKind::InstanceVar(ident) => IdentId::get_id(ident),
            TokenKind::StringLit(ident) => IdentId::get_id(ident),
            TokenKind::Reserved(reserved) => {
                let s = get_string_from_reserved(reserved);
                IdentId::get_id_from_string(s)
            }
            _ => return None,
        };
        Some(id)
    }

    pub(crate) fn check_stmt_end(&self) -> bool {
        match self.kind {
            TokenKind::Eof => true,
            TokenKind::Reserved(reserved) => matches!(
                reserved,
                Reserved::Else
                    | Reserved::Elsif
                    | Reserved::End
                    | Reserved::When
                    | Reserved::Rescue
                    | Reserved::Ensure
            ),
            TokenKind::Punct(punct) => {
                matches!(punct, Punct::RParen | Punct::RBrace | Punct::RBracket)
            }
            _ => false,
        }
    }
}
