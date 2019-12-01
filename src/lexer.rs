use crate::error::{ParseErrKind, RubyError};
use crate::token::*;
use crate::util::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Lexer {
    len: usize,
    line_top_pos: usize,
    token_start_pos: usize,
    pos: usize,
    line: usize,
    reserved: HashMap<String, Reserved>,
    reserved_rev: HashMap<Reserved, String>,
    quote_state: Vec<QuoteState>,
    pub source_info: SourceInfo,
}

#[derive(Debug, Clone, PartialEq)]
enum QuoteState {
    DoubleQuote,
    Expr,
}

#[derive(Debug, Clone)]
pub struct LexerResult {
    pub tokens: Vec<Token>,
}

impl Lexer {
    pub fn new() -> Self {
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
            "for" => Reserved::For,
            "false" => Reserved::False,
            "if" => Reserved::If,
            "in" => Reserved::In,
            "next" => Reserved::Next,
            "nil" => Reserved::Nil,
            "return" => Reserved::Return,
            "then" => Reserved::Then,
            "true" => Reserved::True
        };
        Lexer {
            len: 0,
            line_top_pos: 0,
            token_start_pos: 0,
            pos: 0,
            line: 1,
            reserved,
            reserved_rev,
            quote_state: vec![],
            source_info: SourceInfo::new(),
        }
    }

    pub fn get_string_from_reserved(&self, reserved: Reserved) -> &String {
        self.reserved_rev.get(&reserved).unwrap()
    }

    #[allow(dead_code)]
    fn error_unexpected(&self, pos: usize) -> RubyError {
        let loc = Loc(pos, pos);
        RubyError::new_parse_err(
            ParseErrKind::SyntaxError(format!("Unexpected char. '{}'", self.source_info.code[pos])),
            loc,
        )
    }

    fn error_eof(&self, pos: usize) -> RubyError {
        let loc = Loc(pos, pos);
        RubyError::new_parse_err(ParseErrKind::UnexpectedEOF, loc)
    }

    pub fn tokenize(&mut self, code_text: impl Into<String>) -> Result<LexerResult, RubyError> {
        match self.tokenize_main(code_text) {
            Ok(res) => Ok(res),
            Err(err) => {
                self.goto_eol();
                Err(err)
            }
        }
    }

    pub fn tokenize_main(
        &mut self,
        code_text: impl Into<String>,
    ) -> Result<LexerResult, RubyError> {
        let mut code = code_text.into().chars().collect::<Vec<char>>();
        let pop_flag = match self.source_info.line_pos.last() {
            None => false,
            Some(info) => {
                let next_pos = self.source_info.code.len();
                self.line_top_pos = next_pos;
                self.token_start_pos = next_pos;
                self.pos = next_pos;
                self.line = info.0;
                true
            }
        };
        if pop_flag {
            self.source_info.line_pos.pop();
        }

        self.source_info.code.append(&mut code);
        self.len = self.source_info.code.len();
        //println!("{:?}", self);
        let mut tokens: Vec<Token> = vec![];
        loop {
            self.token_start_pos = self.pos;
            if let Some(tok) = self.skip_whitespace() {
                if tok.kind == TokenKind::LineTerm {
                    tokens.push(tok);
                }
            };

            let ch = match self.get() {
                Ok(ch) => ch,
                Err(_) => break,
            };

            let token = if ch.is_ascii_alphabetic() || ch == '_' {
                self.lex_identifier(ch, false)?
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
                    '-' => {
                        let ch1 = self.peek()?;
                        if ch1 == '>' {
                            self.get()?;
                            self.new_punct(Punct::Arrow)
                        } else {
                            self.new_punct(Punct::Minus)
                        }
                    }
                    '*' => self.new_punct(Punct::Mul),
                    '/' => self.new_punct(Punct::Div),
                    '(' => self.new_punct(Punct::LParen),
                    ')' => self.new_punct(Punct::RParen),
                    '^' => self.new_punct(Punct::BitXor),
                    '~' => self.new_punct(Punct::BitNot),
                    '[' => self.new_punct(Punct::LBracket),
                    ']' => self.new_punct(Punct::RBracket),
                    '{' => self.new_punct(Punct::LBrace),
                    '}' => match self.quote_state.last() {
                        Some(QuoteState::Expr) => {
                            self.quote_state.pop().unwrap();
                            self.lex_string_literal_double()?
                        }
                        _ => self.new_punct(Punct::RBrace),
                    },
                    '.' => {
                        let ch1 = self.peek()?;
                        if ch1 == '.' {
                            self.get()?;
                            let ch2 = self.peek()?;
                            if ch2 == '.' {
                                self.get()?;
                                self.new_punct(Punct::Range3)
                            } else {
                                self.new_punct(Punct::Range2)
                            }
                        } else {
                            self.new_punct(Punct::Dot)
                        }
                    }
                    '?' => self.new_punct(Punct::Question),
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
                        } else if ch1 == '>' {
                            self.get()?;
                            self.new_punct(Punct::Shr)
                        } else {
                            self.new_punct(Punct::Gt)
                        }
                    }
                    '<' => {
                        let ch1 = self.peek()?;
                        if ch1 == '=' {
                            self.get()?;
                            self.new_punct(Punct::Le)
                        } else if ch1 == '<' {
                            self.get()?;
                            self.new_punct(Punct::Shl)
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
                            self.new_punct(Punct::BitAnd)
                        }
                    }
                    '|' => {
                        let ch1 = self.peek()?;
                        if ch1 == '|' {
                            self.get()?;
                            self.new_punct(Punct::LOr)
                        } else {
                            self.new_punct(Punct::BitOr)
                        }
                    }
                    '@' => {
                        let ch = self.get()?;
                        self.lex_identifier(ch, true)?
                    }
                    _ => unimplemented!("{}", ch),
                }
            } else {
                self.lex_identifier(ch, false)?
            };
            if token.kind != TokenKind::Nop {
                tokens.push(token);
            }
        }
        tokens.push(self.new_eof(self.source_info.code.len()));
        Ok(LexerResult::new(tokens))
    }

    fn lex_identifier(&mut self, ch: char, is_instance_var: bool) -> Result<Token, RubyError> {
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
            Some(reserved) => Ok(self.new_reserved(*reserved)),
            None => {
                if is_const {
                    Ok(self.new_const(tok))
                } else if is_instance_var {
                    Ok(self.new_instance_var(tok))
                } else {
                    Ok(self.new_ident(tok))
                }
            }
        }
    }

    /// Read number literal
    fn lex_number_literal(&mut self, ch: char) -> Result<Token, RubyError> {
        let mut int = ch.to_string();
        let mut decimal_flag = false;
        loop {
            let ch = match self.peek() {
                Ok(ch) => ch,
                Err(_) => {
                    break;
                }
            };
            if ch.is_numeric() {
                int.push(self.get()?);
            } else if ch == '_' {
                self.get()?;
            } else if ch == '.' {
                if decimal_flag {
                    break;
                }
                let ch2 = match self.peek2() {
                    Ok(ch2) => ch2,
                    Err(_) => {
                        break;
                    }
                };
                if !ch2.is_numeric() {
                    break;
                }

                decimal_flag = true;
                int.push(self.get()?);
            } else {
                break;
            }
        }
        if decimal_flag {
            let f = int.parse::<f64>().unwrap();
            Ok(self.new_floatlit(f))
        } else {
            let i = int.parse::<i64>().unwrap();
            Ok(self.new_numlit(i))
        }
    }

    /// Read string literal
    fn lex_string_literal_double(&mut self) -> Result<Token, RubyError> {
        let mut s = "".to_string();
        loop {
            match self.get()? {
                '"' => break,
                '\\' => s.push(self.read_escaped_char()?),
                '#' => {
                    if self.peek()? == '{' {
                        self.get()?;
                        match self.quote_state.last() {
                            None => {
                                self.quote_state.push(QuoteState::DoubleQuote);
                                self.quote_state.push(QuoteState::Expr);
                                return Ok(self.new_open_dq(s));
                            }
                            Some(QuoteState::DoubleQuote) => {
                                self.quote_state.push(QuoteState::Expr);
                                return Ok(self.new_inter_dq(s));
                            }
                            _ => return Err(self.error_unexpected(self.pos - 1)),
                        }
                    } else {
                        s.push('#');
                    }
                }
                c => s.push(c),
            }
        }

        match self.quote_state.last() {
            None => Ok(self.new_stringlit(s)),
            Some(QuoteState::DoubleQuote) => {
                self.quote_state.pop().unwrap();
                Ok(self.new_close_dq(s))
            }
            _ => Err(self.error_unexpected(self.pos - 1)),
        }
    }

    fn read_escaped_char(&mut self) -> Result<char, RubyError> {
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
    /// Returns Ok(char) or RubyError if the cursor reached EOF.
    fn get(&mut self) -> Result<char, RubyError> {
        if self.pos >= self.len {
            self.source_info
                .line_pos
                .push((self.line, self.line_top_pos, self.len));
            Err(self.error_eof(self.len))
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

    /// Peek the next char and no move.
    /// Returns Some(char) or None if the cursor reached EOF.
    fn peek(&mut self) -> Result<char, RubyError> {
        if self.pos >= self.len {
            Err(self.error_eof(self.pos))
        } else {
            Ok(self.source_info.code[self.pos])
        }
    }

    /// Peek the char after the next and no move.
    /// Returns Some(char) or None if the cursor reached EOF.
    fn peek2(&mut self) -> Result<char, RubyError> {
        if self.pos + 1 >= self.len {
            Err(self.error_eof(self.pos))
        } else {
            Ok(self.source_info.code[self.pos + 1])
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
                    res = Some(self.new_line_term());
                    self.token_start_pos = self.pos;
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
            match self.peek() {
                Ok('\n') | Err(_) => return,
                _ => {
                    let _ = self.get();
                }
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
        Token::new_ident(ident.into(), self.cur_loc())
    }

    fn new_instance_var(&self, ident: impl Into<String>) -> Token {
        Annot::new(TokenKind::InstanceVar(ident.into()), self.cur_loc())
    }

    fn new_const(&self, ident: impl Into<String>) -> Token {
        Annot::new(TokenKind::Const(ident.into()), self.cur_loc())
    }

    fn new_reserved(&self, ident: Reserved) -> Token {
        Annot::new(TokenKind::Reserved(ident), self.cur_loc())
    }

    fn new_numlit(&self, num: i64) -> Token {
        Token::new_numlit(num, self.cur_loc())
    }

    fn new_floatlit(&self, num: f64) -> Token {
        Token::new_floatlit(num, self.cur_loc())
    }

    fn new_stringlit(&self, string: impl Into<String>) -> Token {
        Annot::new(TokenKind::StringLit(string.into()), self.cur_loc())
    }

    fn new_punct(&self, punc: Punct) -> Token {
        Annot::new(TokenKind::Punct(punc), self.cur_loc())
    }

    fn new_open_dq(&self, s: String) -> Token {
        Token::new_open_dq(s, self.cur_loc())
    }

    fn new_inter_dq(&self, s: String) -> Token {
        Token::new_inter_dq(s, self.cur_loc())
    }

    fn new_close_dq(&self, s: String) -> Token {
        Token::new_close_dq(s, self.cur_loc())
    }

    fn new_space(&self) -> Token {
        Annot::new(TokenKind::Space, self.cur_loc())
    }

    fn new_line_term(&self) -> Token {
        Annot::new(TokenKind::LineTerm, self.cur_loc())
    }

    fn new_eof(&self, pos: usize) -> Token {
        Annot::new(TokenKind::EOF, Loc(pos, pos))
    }

    fn new_nop(&self) -> Token {
        Token::new_nop()
    }
}

impl LexerResult {
    fn new(tokens: Vec<Token>) -> Self {
        LexerResult { tokens }
    }
}

#[cfg(test)]
#[allow(unused_imports, dead_code)]
mod test {
    use crate::lexer::*;
    fn assert_tokens(program: impl Into<String>, ans: Vec<Token>) {
        let mut lexer = Lexer::new();
        match lexer.tokenize(program.into()) {
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
        (InstanceVar($item:expr), $loc_0:expr, $loc_1:expr) => {
            Token::new_instance_var($item, Loc($loc_0, $loc_1))
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
        (StringLit($item:expr), $loc_0:expr, $loc_1:expr) => {
            Token::new_stringlit($item, Loc($loc_0, $loc_1))
        };
        (OpenDoubleQuote($item:expr), $loc_0:expr, $loc_1:expr) => {
            Token::new_open_dq($item, Loc($loc_0, $loc_1))
        };
        (IntermediateDoubleQuote($item:expr), $loc_0:expr, $loc_1:expr) => {
            Token::new_inter_dq($item, Loc($loc_0, $loc_1))
        };
        (CloseDoubleQuote($item:expr), $loc_0:expr, $loc_1:expr) => {
            Token::new_close_dq($item, Loc($loc_0, $loc_1))
        };
        (LineTerm, $loc_0:expr, $loc_1:expr) => {
            Token::new_line_term(Loc($loc_0, $loc_1))
        };
        (EOF, $pos:expr) => {
            Token::new_eof($pos)
        };
    );

    #[test]
    fn string_literal1() {
        let program = r#""""#;
        let ans = vec![Token![StringLit(""), 0, 1], Token![EOF, 2]];
        assert_tokens(program, ans);
    }

    #[test]
    fn string_literal2() {
        let program = r#""flower""#;
        let ans = vec![Token![StringLit("flower"), 0, 7], Token![EOF, 8]];
        assert_tokens(program, ans);
    }

    #[test]
    fn string_literal3() {
        let program = r#""this is #{item1} and #{item2}. ""#;
        let ans = vec![
            Token![OpenDoubleQuote("this is "), 0, 10],
            Token![Ident("item1"), 11, 15],
            Token![IntermediateDoubleQuote(" and "), 16, 23],
            Token![Ident("item2"), 24, 28],
            Token![CloseDoubleQuote(". "), 29, 32],
            Token![EOF, 33],
        ];
        assert_tokens(program, ans);
    }

    #[test]
    fn identifier1() {
        let program = "amber";
        let ans = vec![Token![Ident("amber"), 0, 4], Token![EOF, 5]];
        assert_tokens(program, ans);
    }

    #[test]
    fn identifier2() {
        let program = "@amber";
        let ans = vec![Token![InstanceVar("amber"), 0, 5], Token![EOF, 6]];
        assert_tokens(program, ans);
    }

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
            Token![LineTerm, 5, 5],
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
            Token![LineTerm, 0, 0],
            Token![Ident("a"), 9, 9],
            Token![Punct(Punct::Assign), 11, 11],
            Token![NumLit(0), 13, 13],
            Token![Punct(Punct::Semi), 14, 14],
            Token![LineTerm, 15, 15],
            Token![Reserved(Reserved::If), 24, 25],
            Token![Ident("a"), 27, 27],
            Token![Punct(Punct::Eq), 29, 30],
            Token![NumLit(1000), 32, 36],
            Token![Reserved(Reserved::Then), 38, 41],
            Token![LineTerm, 42, 42],
            Token![NumLit(5), 55, 55],
            Token![LineTerm, 76, 76],
            Token![Reserved(Reserved::Else), 85, 88],
            Token![LineTerm, 89, 89],
            Token![NumLit(10), 102, 103],
            Token![EOF, 121],
        ];
        assert_tokens(program, ans);
    }
}
