use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Lexer {
    len: usize,
    line_top_pos: usize,
    token_start_pos: usize,
    pos: usize,
    line: usize,
    reserved: HashMap<String, Reserved>,
    reserved_rev: HashMap<Reserved, String>,
    source_info: SourceInfo,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceInfo {
    code: Vec<char>,
    line_pos: Vec<(usize, usize, usize)>, // (line_no, line_top_pos, line_end_pos)
}

impl SourceInfo {
    fn new(code: Vec<char>) -> Self {
        SourceInfo {
            code,
            line_pos: vec![],
        }
    }

    /// Show the location of the Loc in the source code using '^^^'.
    pub fn show_loc(&self, loc: &Loc) {
        for line in &self.line_pos {
            if line.2 < loc.0 || line.1 > loc.1 {
                continue;
            }
            println!(
                "{}",
                self.code[(line.1)..(line.2)].iter().collect::<String>()
            );
            use std::cmp::*;
            let read = if loc.0 < line.1 { 0 } else { loc.0 - line.1 };
            let length = min(loc.1, line.2) + 1 - max(loc.0, line.1);
            println!("{}{}", " ".repeat(read), "^".repeat(length));
        }
    }
}

#[derive(Debug, Clone)]
pub struct LexerResult {
    pub tokens: Vec<Token>,
    pub source_info: SourceInfo,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Error {
    EOF,
    UnexpectedChar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reserved {
    BEGIN,
    END,
    Alias,
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
    False,
    If,
    Return,
    Then,
    True,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Nop,
    EOF,
    Ident(String),
    NumLit(i64),
    Reserved(Reserved),
    Punct(Punct),
    Space,
    LineTerm,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Punct {
    LParen,
    RParen,
    Semi,

    Plus,
    Minus,
    Mul,
    Assign,
    Equal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Annot<T> {
    pub kind: T,
    loc: Loc,
}

impl<T> Annot<T> {
    fn new(kind: T, loc: Loc) -> Self {
        Annot { kind, loc }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Loc(pub usize, pub usize);

impl Loc {
    pub fn new(start: usize, end: usize) -> Self {
        Loc(start, end)
    }

    pub fn merge(&self, loc: Loc) -> Self {
        use std::cmp::*;
        Loc(min(self.0, loc.0), max(self.1, loc.1))
    }
}

pub type Token = Annot<TokenKind>;

impl Token {
    pub fn loc(&self) -> Loc {
        self.loc
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            TokenKind::EOF => write!(f, "Token![{:?}, {}],", self.kind, self.loc.0),
            TokenKind::Punct(punct) => write!(
                f,
                "Token![Punct(Punct::{:?}), {}, {}],",
                punct, self.loc.0, self.loc.1
            ),
            TokenKind::Reserved(reserved) => write!(
                f,
                "Token![Reserved(Reserved::{:?}), {}, {}],",
                reserved, self.loc.0, self.loc.1
            ),
            _ => write!(
                f,
                "Token![{:?}, {}, {}],",
                self.kind, self.loc.0, self.loc.1
            ),
        }
    }
}

#[allow(unused)]
impl Token {
    fn new_ident(ident: impl Into<String>, loc: Loc) -> Self {
        Annot::new(TokenKind::Ident(ident.into()), loc)
    }

    fn new_reserved(ident: Reserved, loc: Loc) -> Self {
        Annot::new(TokenKind::Reserved(ident), loc)
    }

    fn new_numlit(num: i64, loc: Loc) -> Self {
        Annot::new(TokenKind::NumLit(num), loc)
    }

    fn new_punct(punct: Punct, loc: Loc) -> Self {
        Annot::new(TokenKind::Punct(punct), loc)
    }

    fn new_space(loc: Loc) -> Self {
        Annot::new(TokenKind::Space, loc)
    }

    fn new_line_term(loc: Loc) -> Self {
        Annot::new(TokenKind::LineTerm, loc)
    }
    fn new_eof(pos: usize) -> Self {
        Annot::new(TokenKind::EOF, Loc(pos, pos))
    }
}

impl Token {
    pub fn is_line_term(&self) -> bool {
        self.kind == TokenKind::LineTerm
    }

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
}

impl Lexer {
    pub fn new(code_text: impl Into<String>) -> Self {
        let code = code_text.into().chars().collect::<Vec<char>>();
        let len = code.len();
        let mut reserved = HashMap::new();
        let mut reserved_rev = HashMap::new();
        macro_rules! reg_reserved {
            ( $($id:expr => $variant:path),+ ) => {
                $(
                    reserved.insert($id.to_string(), $variant);
                    reserved_rev.insert($variant, $id.to_string());
                )+
            };
        }
        reg_reserved! {
            "BEGIN" => Reserved::BEGIN,
            "END" => Reserved::END,
            "alias" => Reserved::Alias,
            "begin" => Reserved::Begin,
            "break" => Reserved::Break,
            "case" => Reserved::Case,
            "class" => Reserved::Class,
            "def" => Reserved::Def,
            "defined?" => Reserved::Defined,
            "do" => Reserved::Do,
            "else" => Reserved::Else,
            "elsif" => Reserved::Elsif,
            "end" => Reserved::End,
            "false" => Reserved::False,
            "if" => Reserved::If,
            "return" => Reserved::Return,
            "then" => Reserved::Then,
            "true" => Reserved::True
        };
        Lexer {
            len,
            line_top_pos: 0,
            token_start_pos: 0,
            pos: 0,
            line: 1,
            reserved,
            reserved_rev,
            source_info: SourceInfo::new(code),
        }
    }

    pub fn tokenize(mut self) -> Result<LexerResult, Error> {
        let mut tokens: Vec<Token> = vec![];
        loop {
            if let Some(tok) = self.skip_whitespace() {
                if tok.kind == TokenKind::LineTerm {
                    tokens.push(tok);
                }
            };
            self.token_start_pos = self.pos;
            let ch = match self.get() {
                Ok(ch) => ch,
                Err(_) => break,
            };

            let token = if ch.is_ascii_alphabetic() || ch == '_' {
                // read identifier or reserved keyword
                let mut tok = ch.to_string();
                loop {
                    let ch = match self.peek() {
                        Ok(ch) => ch,
                        Err(_) => {
                            break;
                        }
                    };
                    if ch.is_ascii_alphanumeric() || ch == '_' {
                        tok.push(self.get()?);
                    } else {
                        break;
                    }
                }
                match self.reserved.get(&tok) {
                    Some(reserved) => self.new_reserved(*reserved),
                    None => self.new_ident(tok),
                }
            } else if ch.is_numeric() {
                self.lex_number_literal(ch)?
            } else if ch.is_ascii_punctuation() {
                match ch {
                    '#' => {
                        self.goto_eol();
                        self.new_nop()
                    }
                    ';' => self.new_punct(Punct::Semi),
                    '+' => self.new_punct(Punct::Plus),
                    '-' => self.new_punct(Punct::Minus),
                    '*' => self.new_punct(Punct::Mul),
                    '(' => self.new_punct(Punct::LParen),
                    ')' => self.new_punct(Punct::RParen),
                    '=' => {
                        let ch1 = self.peek()?;
                        if ch1 == '=' {
                            self.get()?;
                            self.new_punct(Punct::Equal)
                        } else {
                            self.new_punct(Punct::Assign)
                        }
                    }
                    _ => unimplemented!("{}", ch),
                }
            } else {
                return Err(Error::UnexpectedChar);
            };
            if token.kind != TokenKind::Nop {
                tokens.push(token);
            }
        }
        tokens.push(self.new_eof(self.source_info.code.len()));
        Ok(LexerResult::new(tokens, self))
    }

    /// Read number literal
    fn lex_number_literal(&mut self, ch: char) -> Result<Token, Error> {
        let mut tok = ch.to_string();
        loop {
            let ch = match self.peek() {
                Ok(ch) => ch,
                Err(_) => {
                    break;
                }
            };
            if ch.is_numeric() {
                tok.push(self.get()?);
            } else if ch == '_' {
                self.get()?;
            } else {
                break;
            }
        }
        let i = tok.parse::<i64>().unwrap();
        Ok(self.new_numlit(i))
    }
    /*
    pub fn show_loc(&self, loc: &Loc) {
        let line = self.line_pos.iter().find(|x| x.2 >= loc.0).unwrap();
        println!(
            "{}",
            self.code[(line.1)..(line.2)].iter().collect::<String>()
        );
        println!(
            "{}{}",
            " ".repeat(loc.0 - line.1),
            "^".repeat(loc.1 - loc.0 + 1)
        );
    }
    */
}

impl Lexer {
    /// Get one char and move to the next.
    /// Returns Ok(char) or an error if the cursor reached EOF.
    fn get(&mut self) -> Result<char, Error> {
        if self.pos >= self.len {
            self.source_info
                .line_pos
                .push((self.line, self.line_top_pos, self.len));
            Err(Error::EOF)
        } else {
            let ch = self.source_info.code[self.pos];
            if ch == '\n' {
                self.source_info
                    .line_pos
                    .push((self.line, self.line_top_pos, self.pos));
                self.line += 1;
                self.line_top_pos = self.pos + 1;
            }
            self.pos += 1;
            Ok(ch)
        }
    }

    /// Peek one char and no move.
    /// Returns Ok(char) or an error if the cursor reached EOF.
    fn peek(&mut self) -> Result<char, Error> {
        if self.pos >= self.len {
            Err(Error::EOF)
        } else {
            Ok(self.source_info.code[self.pos])
        }
    }

    /// Skip whitespace and line terminator.
    /// Returns Some(Space or LineTerm) or None if the cursor reached EOF.
    fn skip_whitespace(&mut self) -> Option<Token> {
        let mut res = None;
        loop {
            match self.peek() {
                Ok('\n') => {
                    self.get().unwrap();
                    self.token_start_pos = self.pos;
                    res = Some(self.new_line_term());
                }
                Ok(ch) if ch.is_ascii_whitespace() => {
                    self.get().unwrap();
                    self.token_start_pos = self.pos;
                    if res.is_none() {
                        res = Some(self.new_space());
                    }
                }
                _ => {
                    return res;
                }
            };
        }
    }

    fn goto_eol(&mut self) {
        loop {
            match self.get() {
                Ok('\n') | Err(_) => return,
                _ => {}
            }
        }
    }

    fn cur_loc(&self) -> Loc {
        Loc(self.token_start_pos, self.pos - 1)
    }
}

impl Lexer {
    fn new_ident(&self, ident: impl Into<String>) -> Token {
        Annot::new(TokenKind::Ident(ident.into()), self.cur_loc())
    }

    fn new_reserved(&self, ident: Reserved) -> Token {
        Annot::new(TokenKind::Reserved(ident), self.cur_loc())
    }

    fn new_numlit(&self, num: i64) -> Token {
        Annot::new(TokenKind::NumLit(num), self.cur_loc())
    }

    fn new_punct(&self, punc: Punct) -> Token {
        Annot::new(TokenKind::Punct(punc), self.cur_loc())
    }

    fn new_space(&self) -> Token {
        Annot::new(TokenKind::Space, self.cur_loc())
    }

    fn new_line_term(&self) -> Token {
        Annot::new(TokenKind::LineTerm, self.cur_loc())
    }

    fn new_nop(&self) -> Token {
        Annot::new(TokenKind::Nop, Loc(0, 0))
    }
    fn new_eof(&self, pos: usize) -> Token {
        Annot::new(TokenKind::EOF, Loc(pos, pos))
    }
}

impl LexerResult {
    fn new(tokens: Vec<Token>, lexer: Lexer) -> Self {
        LexerResult {
            tokens,
            source_info: lexer.source_info,
        }
    }
}

#[cfg(test)]
#[allow(unused_imports, dead_code)]
mod test {
    use crate::lexer::*;
    fn assert_tokens(program: impl Into<String>, ans: Vec<Token>) {
        let lexer = Lexer::new(program.into());
        match lexer.tokenize() {
            Err(err) => panic!("{:?}", err),
            Ok(LexerResult { tokens, .. }) => {
                let len = tokens.len();
                if len != ans.len() {
                    print_tokens(&tokens, &ans);
                }
                for i in 0..len {
                    if tokens[i] != ans[i] {
                        print_tokens(&tokens, &ans);
                    }
                }
            }
        };
    }

    fn print_tokens(tokens: &Vec<Token>, ans: &Vec<Token>) {
        println!("Expected:");
        for t in ans {
            println!("{}", t);
        }
        println!("Got:");
        for t in tokens {
            println!("{}", t);
        }
        panic!();
    }

    macro_rules! Token (
        (Ident($item:expr), $loc_0:expr, $loc_1:expr) => {
            Token::new_ident($item, Loc($loc_0, $loc_1))
        };
        (Space, $loc_0:expr, $loc_1:expr) => {
            Token::new_space(Loc($loc_0, $loc_1))
        };
        (Punct($item:path), $loc_0:expr, $loc_1:expr) => {
            Token::new_punct($item, Loc($loc_0, $loc_1))
        };
        (Reserved($item:path), $loc_0:expr, $loc_1:expr) => {
            Token::new_reserved($item, Loc($loc_0, $loc_1))
        };
        (NumLit($num:expr), $loc_0:expr, $loc_1:expr) => {
            Token::new_numlit($num, Loc($loc_0, $loc_1))
        };
        (LineTerm, $loc_0:expr, $loc_1:expr) => {
            Token::new_line_term(Loc($loc_0, $loc_1))
        };
        (EOF, $pos:expr) => {
            Token::new_eof($pos)
        };
    );

    #[test]
    fn lexer_test1() {
        let program = "a = 1\n if a==5 then 5 else 8";
        let ans = vec![
            Token![Ident("a"), 0, 0],
            Token![Punct(Punct::Assign), 2, 2],
            Token![NumLit(1), 4, 4],
            Token![LineTerm, 6, 5],
            Token![Reserved(Reserved::If), 7, 8],
            Token![Ident("a"), 10, 10],
            Token![Punct(Punct::Equal), 11, 12],
            Token![NumLit(5), 13, 13],
            Token![Reserved(Reserved::Then), 15, 18],
            Token![NumLit(5), 20, 20],
            Token![Reserved(Reserved::Else), 22, 25],
            Token![NumLit(8), 27, 27],
            Token![EOF, 28],
        ];
        assert_tokens(program, ans);
    }

    #[test]
    fn lexer_test2() {
        let program = r"
        a = 0;
        if a == 1_000 then
            5 # this is a comment
        else
            10 # also a comment";
        let ans = vec![
            Token![LineTerm, 1, 0],
            Token![Ident("a"), 9, 9],
            Token![Punct(Punct::Assign), 11, 11],
            Token![NumLit(0), 13, 13],
            Token![Punct(Punct::Semi), 14, 14],
            Token![LineTerm, 16, 15],
            Token![Reserved(Reserved::If), 24, 25],
            Token![Ident("a"), 27, 27],
            Token![Punct(Punct::Equal), 29, 30],
            Token![NumLit(1000), 32, 36],
            Token![Reserved(Reserved::Then), 38, 41],
            Token![LineTerm, 43, 42],
            Token![NumLit(5), 55, 55],
            Token![Reserved(Reserved::Else), 85, 88],
            Token![LineTerm, 90, 89],
            Token![NumLit(10), 102, 103],
            Token![EOF, 121],
        ];
        assert_tokens(program, ans);
    }
}
