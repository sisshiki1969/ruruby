use crate::token::*;
use crate::util::*;
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
                let is_const = ch.is_ascii_uppercase();
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
                    None => {
                        if is_const {
                            self.new_const(tok)
                        } else {
                            self.new_ident(tok)
                        }
                    }
                }
            } else if ch.is_numeric() {
                self.lex_number_literal(ch)?
            } else if ch.is_ascii_punctuation() {
                match ch {
                    '#' => {
                        self.goto_eol();
                        self.new_nop()
                    }
                    '"' => self.lex_string_literal_double()?,
                    ';' => self.new_punct(Punct::Semi),
                    ':' => self.new_punct(Punct::Colon),
                    ',' => self.new_punct(Punct::Comma),
                    '+' => self.new_punct(Punct::Plus),
                    '-' => self.new_punct(Punct::Minus),
                    '*' => self.new_punct(Punct::Mul),
                    '(' => self.new_punct(Punct::LParen),
                    ')' => self.new_punct(Punct::RParen),
                    '.' => self.new_punct(Punct::Dot),
                    '=' => {
                        let ch1 = self.peek()?;
                        if ch1 == '=' {
                            self.get()?;
                            self.new_punct(Punct::Eq)
                        } else {
                            self.new_punct(Punct::Assign)
                        }
                    }
                    '>' => {
                        let ch1 = self.peek()?;
                        if ch1 == '=' {
                            self.get()?;
                            self.new_punct(Punct::Ge)
                        } else {
                            self.new_punct(Punct::Gt)
                        }
                    }
                    '<' => {
                        let ch1 = self.peek()?;
                        if ch1 == '=' {
                            self.get()?;
                            self.new_punct(Punct::Le)
                        } else {
                            self.new_punct(Punct::Lt)
                        }
                    }
                    '!' => {
                        let ch1 = self.peek()?;
                        if ch1 == '=' {
                            self.get()?;
                            self.new_punct(Punct::Ne)
                        } else {
                            unimplemented!("{}", ch)
                        }
                    }
                    '&' => {
                        let ch1 = self.peek()?;
                        if ch1 == '&' {
                            self.get()?;
                            self.new_punct(Punct::LAnd)
                        } else {
                            self.new_punct(Punct::And)
                        }
                    }
                    '|' => {
                        let ch1 = self.peek()?;
                        if ch1 == '|' {
                            self.get()?;
                            self.new_punct(Punct::LOr)
                        } else {
                            self.new_punct(Punct::Or)
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

    /// Read string literal
    fn lex_string_literal_double(&mut self) -> Result<Token, Error> {
        let mut s = "".to_string();
        loop {
            match self.get()? {
                '"' => break,
                '\\' => s.push(self.read_escaped_char()?),
                c => s.push(c),
            }
        }
        Ok(self.new_stringlit(s))
    }

    fn read_escaped_char(&mut self) -> Result<char, Error> {
        let c = self.get()?;
        let ch = match c {
            '\'' | '"' | '?' | '\\' => c,
            'a' => '\x07',
            'b' => '\x08',
            'f' => '\x0c',
            'n' => '\x0a',
            'r' => '\x0d',
            't' => '\x09',
            'v' => '\x0b',
            _ => c,
        };
        Ok(ch)
    }
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
        let end = std::cmp::max(self.token_start_pos, self.pos - 1);
        Loc(self.token_start_pos, end)
    }
}

impl Lexer {
    fn new_ident(&self, ident: impl Into<String>) -> Token {
        Annot::new(TokenKind::Ident(ident.into()), self.cur_loc())
    }

    fn new_const(&self, ident: impl Into<String>) -> Token {
        Annot::new(TokenKind::Const(ident.into()), self.cur_loc())
    }

    fn new_reserved(&self, ident: Reserved) -> Token {
        Annot::new(TokenKind::Reserved(ident), self.cur_loc())
    }

    fn new_numlit(&self, num: i64) -> Token {
        Annot::new(TokenKind::NumLit(num), self.cur_loc())
    }

    fn new_stringlit(&self, string: String) -> Token {
        Annot::new(TokenKind::StringLit(string), self.cur_loc())
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
    fn cmp1() {
        let program = "5 > 0";
        let ans = vec![
            Token![NumLit(5), 0, 0],
            Token![Punct(Punct::Gt), 2, 2],
            Token![NumLit(0), 4, 4],
            Token![EOF, 5],
        ];
        assert_tokens(program, ans);
    }

    #[test]
    fn cmp2() {
        let program = "5 >= 0";
        let ans = vec![
            Token![NumLit(5), 0, 0],
            Token![Punct(Punct::Ge), 2, 3],
            Token![NumLit(0), 5, 5],
            Token![EOF, 6],
        ];
        assert_tokens(program, ans);
    }

    #[test]
    fn cmp3() {
        let program = "5 == 0";
        let ans = vec![
            Token![NumLit(5), 0, 0],
            Token![Punct(Punct::Eq), 2, 3],
            Token![NumLit(0), 5, 5],
            Token![EOF, 6],
        ];
        assert_tokens(program, ans);
    }

    #[test]
    fn cmp4() {
        let program = "5 != 0";
        let ans = vec![
            Token![NumLit(5), 0, 0],
            Token![Punct(Punct::Ne), 2, 3],
            Token![NumLit(0), 5, 5],
            Token![EOF, 6],
        ];
        assert_tokens(program, ans);
    }

    #[test]
    fn cmp5() {
        let program = "5 < 0";
        let ans = vec![
            Token![NumLit(5), 0, 0],
            Token![Punct(Punct::Lt), 2, 2],
            Token![NumLit(0), 4, 4],
            Token![EOF, 5],
        ];
        assert_tokens(program, ans);
    }

    #[test]
    fn cmp6() {
        let program = "5 <= 0";
        let ans = vec![
            Token![NumLit(5), 0, 0],
            Token![Punct(Punct::Le), 2, 3],
            Token![NumLit(0), 5, 5],
            Token![EOF, 6],
        ];
        assert_tokens(program, ans);
    }

    #[test]
    fn lexer_test1() {
        let program = "a = 1\n if a==5 then 5 else 8";
        let ans = vec![
            Token![Ident("a"), 0, 0],
            Token![Punct(Punct::Assign), 2, 2],
            Token![NumLit(1), 4, 4],
            Token![LineTerm, 6, 6],
            Token![Reserved(Reserved::If), 7, 8],
            Token![Ident("a"), 10, 10],
            Token![Punct(Punct::Eq), 11, 12],
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
            Token![LineTerm, 1, 1],
            Token![Ident("a"), 9, 9],
            Token![Punct(Punct::Assign), 11, 11],
            Token![NumLit(0), 13, 13],
            Token![Punct(Punct::Semi), 14, 14],
            Token![LineTerm, 16, 16],
            Token![Reserved(Reserved::If), 24, 25],
            Token![Ident("a"), 27, 27],
            Token![Punct(Punct::Eq), 29, 30],
            Token![NumLit(1000), 32, 36],
            Token![Reserved(Reserved::Then), 38, 41],
            Token![LineTerm, 43, 43],
            Token![NumLit(5), 55, 55],
            Token![Reserved(Reserved::Else), 85, 88],
            Token![LineTerm, 90, 90],
            Token![NumLit(10), 102, 103],
            Token![EOF, 121],
        ];
        assert_tokens(program, ans);
    }
}
