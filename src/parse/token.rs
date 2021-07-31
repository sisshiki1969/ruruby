use crate::parse::node::BinOp;
use crate::value::real::Real;
use crate::*;

pub type Token = Annot<TokenKind>;

#[cfg(test)]
impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            TokenKind::EOF => write!(f, "Token![{:?}, {}],", self.kind, self.loc().0),
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
pub enum TokenKind {
    EOF,
    Ident(String),
    InstanceVar(String),
    GlobalVar(String),
    ClassVar(String),
    Const(String),
    IntegerLit(i64),
    FloatLit(f64),
    ImaginaryLit(Real),
    StringLit(String),
    CommandLit(String),
    Reserved(Reserved),
    Punct(Punct),
    OpenString(String, char, usize), // (content, delimiter, paren_level)
    OpenRegex(String),
    OpenCommand(String, char, usize),
    PercentNotation(char, String),
    LineTerm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    False,
    For,
    If,
    In,
    Module,
    Next,
    Nil,
    Or,
    Rescue,
    Return,
    Self_,
    Super,
    Then,
    True,
    Until,
    Unless,
    When,
    While,
    Yield,
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
    pub fn new_ident(ident: impl Into<String>, loc: Loc) -> Self {
        Annot::new(TokenKind::Ident(ident.into()), loc)
    }

    pub fn new_instance_var(ident: impl Into<String>, loc: Loc) -> Self {
        Annot::new(TokenKind::InstanceVar(ident.into()), loc)
    }

    pub fn new_const(ident: impl Into<String>, has_suffix: bool, loc: Loc) -> Self {
        Annot::new(TokenKind::Const(ident.into()), loc)
    }

    pub fn new_global_var(ident: impl Into<String>, loc: Loc) -> Self {
        Annot::new(TokenKind::GlobalVar(ident.into()), loc)
    }

    pub fn new_reserved(ident: Reserved, loc: Loc) -> Self {
        Annot::new(TokenKind::Reserved(ident), loc)
    }

    pub fn new_numlit(num: i64, loc: Loc) -> Self {
        Annot::new(TokenKind::IntegerLit(num), loc)
    }

    pub fn new_floatlit(num: f64, loc: Loc) -> Self {
        Annot::new(TokenKind::FloatLit(num), loc)
    }

    pub fn new_imaginarylit(num: Real, loc: Loc) -> Self {
        Annot::new(TokenKind::ImaginaryLit(num), loc)
    }

    pub fn new_stringlit(string: impl Into<String>, loc: Loc) -> Self {
        Annot::new(TokenKind::StringLit(string.into()), loc)
    }

    pub fn new_open_string(s: impl Into<String>, delimiter: char, level: usize, loc: Loc) -> Self {
        Annot::new(TokenKind::OpenString(s.into(), delimiter, level), loc)
    }

    pub fn new_open_command(s: impl Into<String>, delimiter: char, level: usize, loc: Loc) -> Self {
        Annot::new(TokenKind::OpenCommand(s.into(), delimiter, level), loc)
    }

    pub fn new_open_reg(s: impl Into<String>, loc: Loc) -> Self {
        Annot::new(TokenKind::OpenRegex(s.into()), loc)
    }

    pub fn new_percent(kind: char, content: String, loc: Loc) -> Self {
        Annot::new(TokenKind::PercentNotation(kind, content), loc)
    }

    pub fn new_punct(punct: Punct, loc: Loc) -> Self {
        Annot::new(TokenKind::Punct(punct), loc)
    }

    pub fn new_line_term(loc: Loc) -> Self {
        Annot::new(TokenKind::LineTerm, loc)
    }

    pub fn new_eof(pos: usize) -> Self {
        Annot::new(TokenKind::EOF, Loc(pos, pos))
    }
}

impl Token {
    /// Examine the token, and return true if it is a line terminator.
    pub fn is_line_term(&self) -> bool {
        self.kind == TokenKind::LineTerm || self.kind == TokenKind::Punct(Punct::Semi)
    }

    /// Examine the token, and return true if it is EOF.
    pub fn is_eof(&self) -> bool {
        self.kind == TokenKind::EOF
    }

    /// Examine the token, and return true if it is a line terminator or ';' or EOF.
    pub fn is_term(&self) -> bool {
        match self.kind {
            TokenKind::LineTerm | TokenKind::EOF | TokenKind::Punct(Punct::Semi) => true,
            _ => false,
        }
    }

    pub fn can_be_symbol(&self) -> Option<IdentId> {
        let id = match &self.kind {
            TokenKind::Ident(ident) => IdentId::get_id(ident),
            TokenKind::Const(ident) => IdentId::get_id(ident),
            TokenKind::InstanceVar(ident) => IdentId::get_id(ident),
            TokenKind::StringLit(ident) => IdentId::get_id(ident),
            TokenKind::Reserved(reserved) => {
                let s = crate::parse::Lexer::get_string_from_reserved(&reserved);
                IdentId::get_id(s)
            }
            _ => return None,
        };
        Some(id)
    }

    pub fn check_stmt_end(&self) -> bool {
        match self.kind {
            TokenKind::EOF => true,
            TokenKind::Reserved(reserved) => match reserved {
                Reserved::Else
                | Reserved::Elsif
                | Reserved::End
                | Reserved::When
                | Reserved::Rescue
                | Reserved::Ensure => true,
                _ => false,
            },
            TokenKind::Punct(punct) => match punct {
                Punct::RParen | Punct::RBrace | Punct::RBracket => true,
                _ => false,
            },
            _ => false,
        }
    }
}
