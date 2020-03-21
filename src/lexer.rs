use crate::error::{ParseErrKind, RubyError};
use crate::node::BinOp;
use crate::token::*;
use crate::util::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Lexer {
    len: usize,
    token_start_pos: u32,
    pos: u32,
    buf: Option<Token>,
    buf_skip_lt: Option<Token>,
    reserved: HashMap<String, Reserved>,
    reserved_rev: HashMap<Reserved, String>,
    quote_state: Vec<QuoteState>,
    pub source_info: SourceInfoRef,
    state_save: Vec<(u32, u32)>, // (token_start_pos, pos)
}

#[derive(Debug, Clone, PartialEq)]
enum QuoteState {
    DoubleQuote,
    RegEx,
    Brace,
    //Expr,
}

#[derive(Debug, Clone)]
pub struct LexerResult {
    pub tokens: Vec<Token>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum VarKind {
    Identifier,
    InstanceVar,
    ClassVar,
    GlobalVar,
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
            "module" => Reserved::Module,
            "next" => Reserved::Next,
            "nil" => Reserved::Nil,
            "return" => Reserved::Return,
            "rescue" => Reserved::Rescue,
            "self" => Reserved::Self_,
            "then" => Reserved::Then,
            "true" => Reserved::True,
            "until" => Reserved::Until,
            "unless" => Reserved::Unless,
            "when" => Reserved::When,
            "while" => Reserved::While
        };
        Lexer {
            len: 0,
            token_start_pos: 0,
            pos: 0,
            buf: None,
            buf_skip_lt: None,
            reserved,
            reserved_rev,
            quote_state: vec![],
            source_info: SourceInfoRef::new(SourceInfo::new(std::path::PathBuf::default())),
            state_save: vec![],
        }
    }

    pub fn get_string_from_reserved(&self, reserved: Reserved) -> &str {
        self.reserved_rev.get(&reserved).unwrap()
    }

    fn error_unexpected(&self, pos: u32) -> RubyError {
        let loc = Loc(pos, pos);
        RubyError::new_parse_err(
            ParseErrKind::SyntaxError(format!(
                "Unexpected char. '{}'",
                self.source_info.code[pos as usize]
            )),
            self.source_info,
            0,
            loc,
        )
    }

    fn error_eof(&self, pos: u32) -> RubyError {
        let loc = Loc(pos, pos);
        RubyError::new_parse_err(ParseErrKind::UnexpectedEOF, self.source_info, 0, loc)
    }

    fn error_parse(&self, msg: &str, pos: u32) -> RubyError {
        let loc = Loc(pos, pos);
        RubyError::new_parse_err(
            ParseErrKind::SyntaxError(format!("Parse error. '{}'", msg)),
            self.source_info,
            0,
            loc,
        )
    }

    pub fn tokenize(&mut self, code_text: impl Into<String>) -> Result<LexerResult, RubyError> {
        self.init(std::path::PathBuf::new(), code_text);
        let mut tokens = vec![];
        loop {
            match self.get_token() {
                Ok(res) => {
                    if res.is_eof() {
                        tokens.push(res);
                        break;
                    } else {
                        tokens.push(res);
                    }
                }
                Err(err) => return Err(err),
            }
        }
        return Ok(LexerResult::new(tokens));
    }

    pub fn init(&mut self, path: std::path::PathBuf, code_text: impl Into<String>) {
        let mut code = code_text.into().chars().collect::<Vec<char>>();
        self.pos = self.source_info.code.len() as u32;
        self.source_info.code.append(&mut code);
        self.len = self.source_info.code.len();
        self.source_info.path = path;
    }

    pub fn get_token(&mut self) -> Result<Token, RubyError> {
        self.buf = None;
        self.buf_skip_lt = None;
        let tok = self.fetch_token()?;
        match tok.kind {
            TokenKind::Punct(Punct::LBrace) => {
                self.quote_state.push(QuoteState::Brace);
            }
            TokenKind::Punct(Punct::RBrace) => {
                self.quote_state.pop().unwrap();
            }
            TokenKind::OpenString(_) => {
                self.quote_state.push(QuoteState::DoubleQuote);
                //self.quote_state.push(QuoteState::Expr);
            }
            TokenKind::CloseString(_) => {
                //assert_eq!(self.quote_state.pop().unwrap(), QuoteState::Expr);
                self.quote_state.pop().unwrap();
            }
            _ => {}
        };
        Ok(tok)
    }

    pub fn peek_token(&mut self) -> Result<Token, RubyError> {
        if let Some(tok) = &self.buf {
            return Ok(tok.clone());
        };
        self.save_state();
        let tok = self.fetch_token()?;
        self.restore_state();
        self.buf = Some(tok.clone());
        Ok(tok)
    }

    pub fn peek_token_skip_lt(&mut self) -> Result<Token, RubyError> {
        if let Some(tok) = &self.buf_skip_lt {
            return Ok(tok.clone());
        };
        let state_save = (self.pos, self.token_start_pos);
        let mut tok;
        loop {
            tok = self.fetch_token()?;
            if tok.is_eof() || !tok.is_line_term() {
                break;
            }
        }
        self.pos = state_save.0;
        self.token_start_pos = state_save.1;
        self.buf_skip_lt = Some(tok.clone());
        Ok(tok)
    }

    pub fn save_state(&mut self) {
        self.state_save.push((self.token_start_pos, self.pos));
    }

    pub fn restore_state(&mut self) {
        let state = self.state_save.pop().unwrap();
        self.token_start_pos = state.0;
        self.pos = state.1;
    }

    pub fn discard_state(&mut self) {
        self.state_save.pop().unwrap();
    }

    fn fetch_token(&mut self) -> Result<Token, RubyError> {
        loop {
            self.token_start_pos = self.pos;
            if let Some(tok) = self.skip_whitespace() {
                if tok.kind == TokenKind::LineTerm {
                    return Ok(tok);
                }
            };

            let pos = self.pos;
            let ch = match self.get() {
                Ok(ch) => ch,
                Err(_) => return Ok(self.new_eof(self.pos)),
            };

            if ch.is_ascii_alphabetic() || ch == '_' {
                return self.lex_identifier(ch, VarKind::Identifier);
            } else if ch.is_numeric() {
                return self.lex_number_literal(ch);
            } else if ch.is_ascii_punctuation() {
                match ch {
                    '#' => {
                        self.goto_eol();
                    }
                    '"' => {
                        return self.lex_string_literal_double();
                    }
                    ';' => return Ok(self.new_punct(Punct::Semi)),
                    ':' => {
                        if self.consume(':') {
                            return Ok(self.new_punct(Punct::Scope));
                        } else {
                            return Ok(self.new_punct(Punct::Colon));
                        }
                    }
                    ',' => return Ok(self.new_punct(Punct::Comma)),
                    '+' => {
                        if self.consume('=') {
                            return Ok(self.new_punct(Punct::AssignOp(BinOp::Add)));
                        } else {
                            return Ok(self.new_punct(Punct::Plus));
                        }
                    }
                    '-' => {
                        if self.consume('>') {
                            return Ok(self.new_punct(Punct::Arrow));
                        } else if self.consume('=') {
                            return Ok(self.new_punct(Punct::AssignOp(BinOp::Sub)));
                        } else {
                            return Ok(self.new_punct(Punct::Minus));
                        }
                    }
                    '*' => {
                        if self.consume('=') {
                            return Ok(self.new_punct(Punct::AssignOp(BinOp::Mul)));
                        } else if self.consume('*') {
                            return Ok(self.new_punct(Punct::DMul));
                        } else {
                            return Ok(self.new_punct(Punct::Mul));
                        }
                    }
                    '%' => {
                        if self.consume('=') {
                            return Ok(self.new_punct(Punct::AssignOp(BinOp::Rem)));
                        } else {
                            return Ok(self.new_punct(Punct::Rem));
                        }
                    }
                    '/' => {
                        if self.consume('=') {
                            return Ok(self.new_punct(Punct::AssignOp(BinOp::Div)));
                        } else {
                            return Ok(self.new_punct(Punct::Div));
                        }
                    }
                    '(' => return Ok(self.new_punct(Punct::LParen)),
                    ')' => return Ok(self.new_punct(Punct::RParen)),
                    '^' => {
                        if self.consume('=') {
                            return Ok(self.new_punct(Punct::AssignOp(BinOp::BitXor)));
                        } else {
                            return Ok(self.new_punct(Punct::BitXor));
                        }
                    }
                    '~' => return Ok(self.new_punct(Punct::BitNot)),
                    '[' => return Ok(self.new_punct(Punct::LBracket)),
                    ']' => return Ok(self.new_punct(Punct::RBracket)),
                    '{' => return Ok(self.new_punct(Punct::LBrace)),
                    '}' => match self.quote_state.last() {
                        Some(QuoteState::DoubleQuote) => return self.lex_interpolate_string(),
                        Some(QuoteState::RegEx) => return self.lex_interpolate_regexp(),
                        Some(QuoteState::Brace) => return Ok(self.new_punct(Punct::RBrace)),
                        _ => return Err(self.error_unexpected(pos)),
                    },
                    '.' => {
                        if self.consume('.') {
                            if self.consume('.') {
                                return Ok(self.new_punct(Punct::Range3));
                            } else {
                                return Ok(self.new_punct(Punct::Range2));
                            }
                        } else {
                            return Ok(self.new_punct(Punct::Dot));
                        }
                    }
                    '?' => return Ok(self.new_punct(Punct::Question)),
                    '\\' => return Ok(self.new_punct(Punct::Backslash)),
                    '=' => {
                        if self.consume('=') {
                            if self.consume('=') {
                                return Ok(self.new_punct(Punct::TEq));
                            } else {
                                return Ok(self.new_punct(Punct::Eq));
                            }
                        } else if self.consume('>') {
                            return Ok(self.new_punct(Punct::FatArrow));
                        } else if self.consume('~') {
                            return Ok(self.new_punct(Punct::Match));
                        } else {
                            return Ok(self.new_punct(Punct::Assign));
                        }
                    }
                    '>' => {
                        if self.consume('=') {
                            return Ok(self.new_punct(Punct::Ge));
                        } else if self.consume('>') {
                            if self.consume('=') {
                                return Ok(self.new_punct(Punct::AssignOp(BinOp::Shr)));
                            } else {
                                return Ok(self.new_punct(Punct::Shr));
                            }
                        } else {
                            return Ok(self.new_punct(Punct::Gt));
                        }
                    }
                    '<' => {
                        if self.consume('=') {
                            return Ok(self.new_punct(Punct::Le));
                        } else if self.consume('<') {
                            if self.consume('=') {
                                return Ok(self.new_punct(Punct::AssignOp(BinOp::Shl)));
                            } else {
                                return Ok(self.new_punct(Punct::Shl));
                            }
                        } else {
                            return Ok(self.new_punct(Punct::Lt));
                        }
                    }
                    '!' => {
                        if self.consume('=') {
                            return Ok(self.new_punct(Punct::Ne));
                        } else {
                            return Ok(self.new_punct(Punct::Not));
                        }
                    }
                    '&' => {
                        if self.consume('&') {
                            return Ok(self.new_punct(Punct::LAnd));
                        } else if self.consume('=') {
                            return Ok(self.new_punct(Punct::AssignOp(BinOp::BitAnd)));
                        } else {
                            return Ok(self.new_punct(Punct::BitAnd));
                        }
                    }
                    '|' => {
                        if self.consume('|') {
                            if self.consume('=') {
                                return Ok(self.new_punct(Punct::AssignOp(BinOp::LOr)));
                            } else {
                                return Ok(self.new_punct(Punct::LOr));
                            }
                        } else if self.consume('=') {
                            return Ok(self.new_punct(Punct::AssignOp(BinOp::BitOr)));
                        } else {
                            return Ok(self.new_punct(Punct::BitOr));
                        }
                    }
                    '@' => {
                        return self.lex_identifier(None, VarKind::InstanceVar);
                    }
                    '$' => {
                        return self.lex_identifier(None, VarKind::GlobalVar);
                    }
                    _ => return Err(self.error_unexpected(pos)),
                }
            } else {
                return self.lex_identifier(ch, VarKind::Identifier);
            };
        }
    }

    fn lex_identifier(
        &mut self,
        ch: impl Into<Option<char>>,
        var_kind: VarKind,
    ) -> Result<Token, RubyError> {
        // read identifier or reserved keyword
        let mut tok = match var_kind {
            VarKind::ClassVar => String::from("@@"),
            VarKind::GlobalVar => String::from("$"),
            VarKind::InstanceVar => String::from("@"),
            VarKind::Identifier => String::new(),
        };
        let is_const = match ch.into() {
            Some(ch) => {
                tok.push(ch);
                ch.is_ascii_uppercase()
            }
            None => {
                match self.get() {
                    Ok(ch) => {
                        if ch.is_alphanumeric() || ch == '_' || ch == '&' || ch == '\'' {
                            tok.push(ch);
                        } else {
                            return Err(self.error_unexpected(self.pos));
                        }
                    }
                    Err(_) => {
                        return Err(self.error_eof(self.pos));
                    }
                };
                false
            }
        };
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
        match var_kind {
            VarKind::InstanceVar => {
                return Ok(self.new_instance_var(tok));
            }
            VarKind::GlobalVar => {
                return Ok(self.new_global_var(tok));
            }
            _ => {}
        }

        match self.reserved.get(&tok) {
            Some(reserved) => Ok(self.new_reserved(*reserved)),
            None => {
                if is_const {
                    let (has_suffix, trailing_space) = match self.peek() {
                        Ok(ch) if ch == ':' || ch == '=' || ch == '(' => (true, false),
                        Ok(ch) if ch.is_ascii_whitespace() => (false, true),
                        _ => (false, false),
                    };
                    Ok(self.new_const(tok, has_suffix, trailing_space))
                } else {
                    match self.peek() {
                        Ok(ch) if ch == '!' || ch == '?' => {
                            tok.push(self.get()?);
                        }
                        _ => {}
                    };
                    let (has_suffix, trailing_space) = match self.peek() {
                        Ok(ch) if ch == ':' || ch == '=' || ch == '(' => (true, false),
                        Ok(ch) if ch.is_ascii_whitespace() => (false, true),
                        _ => (false, false),
                    };
                    Ok(self.new_ident(tok, has_suffix, trailing_space))
                }
            }
        }
    }

    /// Read number literal
    fn lex_number_literal(&mut self, ch: char) -> Result<Token, RubyError> {
        if ch == '0' {
            if self.consume('x') {
                return self.lex_hex_number();
            } else if self.consume('b') {
                return self.lex_bin_number();
            }
        };
        let mut s = ch.to_string();
        let mut decimal_flag = false;
        loop {
            if let Some(ch) = self.consume_numeric() {
                s.push(ch);
            } else if self.consume('_') {
            } else if !decimal_flag && self.consume('.') {
                if let Some(ch) = self.consume_numeric() {
                    decimal_flag = true;
                    s.push('.');
                    s.push(ch);
                } else {
                    self.push_back();
                    break;
                }
            } else {
                break;
            }
        }
        if self.consume('e') || self.consume('E') {
            s.push('e');
            if self.consume('-') {
                s.push('-');
            }
            if let Some(ch) = self.consume_numeric() {
                s.push(ch);
            } else {
                return Err(self.error_unexpected(self.pos));
            }
            loop {
                if let Some(ch) = self.consume_numeric() {
                    s.push(ch);
                } else if self.consume('_') {
                } else {
                    break;
                }
            }
            decimal_flag = true;
        }
        if decimal_flag {
            match s.parse::<f64>() {
                Ok(f) => Ok(self.new_floatlit(f)),
                Err(err) => Err(self.error_parse(&format!("{:?}", err), self.pos)),
            }
        } else {
            match s.parse::<i64>() {
                Ok(i) => Ok(self.new_numlit(i)),
                Err(err) => Err(self.error_parse(&format!("{:?}", err), self.pos)),
            }
        }
    }

    fn lex_hex_number(&mut self) -> Result<Token, RubyError> {
        let mut val = match self.get() {
            Ok(ch @ '0'..='9') => (ch as u64 - '0' as u64),
            Ok(ch @ 'a'..='f') => (ch as u64 - 'a' as u64 + 10),
            Ok(ch @ 'A'..='F') => (ch as u64 - 'A' as u64 + 10),
            Ok(_) => {
                self.push_back();
                return Err(self.error_unexpected(self.pos));
            }
            Err(_) => return Err(self.error_unexpected(self.pos)),
        };
        loop {
            match self.get() {
                Ok(ch @ '0'..='9') => val = val * 16 + (ch as u64 - '0' as u64),
                Ok(ch @ 'a'..='f') => val = val * 16 + (ch as u64 - 'a' as u64 + 10),
                Ok(ch @ 'A'..='F') => val = val * 16 + (ch as u64 - 'A' as u64 + 10),
                Ok('_') => {}
                Ok(_) => {
                    self.push_back();
                    break;
                }
                Err(_) => break,
            }
        }
        Ok(self.new_numlit(val as i64))
    }

    fn lex_bin_number(&mut self) -> Result<Token, RubyError> {
        let mut val = match self.get() {
            Ok(ch @ '0'..='1') => (ch as u64 - '0' as u64),
            Ok(_) => {
                self.push_back();
                return Err(self.error_unexpected(self.pos));
            }
            Err(_) => return Err(self.error_unexpected(self.pos)),
        };
        loop {
            match self.get() {
                Ok(ch @ '0'..='1') => val = val * 2 + (ch as u64 - '0' as u64),
                Ok('_') => {}
                Ok(_) => {
                    self.push_back();
                    break;
                }
                Err(_) => break,
            }
        }
        Ok(self.new_numlit(val as i64))
    }

    /// Read string literal
    fn lex_string_literal_double(&mut self) -> Result<Token, RubyError> {
        let mut s = "".to_string();
        loop {
            match self.get()? {
                '"' => return Ok(self.new_stringlit(s)),
                '\\' => s.push(self.read_escaped_char()?),
                '#' => {
                    if self.consume('{') {
                        return Ok(self.new_open_dq(s));
                    } else {
                        s.push('#');
                    }
                }
                c => s.push(c),
            }
        }
    }

    fn lex_interpolate_string(&mut self) -> Result<Token, RubyError> {
        let mut s = "".to_string();
        loop {
            match self.get()? {
                '"' => return Ok(self.new_close_dq(s)),
                '\\' => s.push(self.read_escaped_char()?),
                '#' => {
                    if self.consume('{') {
                        return Ok(self.new_inter_dq(s));
                    } else {
                        s.push('#');
                    }
                }
                c => s.push(c),
            }
        }
    }

    pub fn lex_regexp(&mut self) -> Result<Token, RubyError> {
        let mut s = "".to_string();
        loop {
            match self.get()? {
                '/' => {
                    if self.consume('i') {
                        s.push('i');
                    } else if self.consume('m') {
                        s.push('m');
                    } else if self.consume('x') {
                        s.push('x');
                    } else if self.consume('o') {
                        s.push('o');
                    } else {
                        s.push(' ');
                    };
                    return Ok(self.new_stringlit(s));
                }
                '\\' => {
                    s.push('\\');
                    s.push(self.get()?);
                }
                '#' => {
                    if self.consume('{') {
                        self.quote_state.push(QuoteState::RegEx);
                        //self.quote_state.push(QuoteState::Expr);
                        return Ok(self.new_open_reg(s));
                    } else {
                        s.push('#');
                    }
                }
                c => {
                    s.push(c);
                }
            }
        }
    }

    fn lex_interpolate_regexp(&mut self) -> Result<Token, RubyError> {
        let mut s = "".to_string();
        loop {
            match self.get()? {
                '/' => return Ok(self.new_close_dq(s)),
                '\\' => {
                    s.push('\\');
                    s.push(self.get()?);
                }
                '#' => {
                    if self.consume('{') {
                        return Ok(self.new_inter_dq(s));
                    } else {
                        s.push('#');
                    }
                }
                c => s.push(c),
            }
        }
    }

    pub fn lex_percent_notation(&mut self) -> Result<Token, RubyError> {
        if self.consume('w') {
            let mut s = "".to_string();
            if !self.consume('(') {
                return Err(self.error_unexpected(self.pos));
            }
            loop {
                match self.get()? {
                    ')' => return Ok(self.new_percent('w', s)),
                    ch => s.push(ch),
                }
            }
        } else {
            return Err(self.error_unexpected(self.pos));
        }
    }

    fn char_to_hex(&self, c: char) -> Result<u32, RubyError> {
        match c {
            ch @ '0'..='9' => Ok(ch as u32 - '0' as u32),
            ch @ 'a'..='f' => Ok(ch as u32 - 'a' as u32 + 10),
            ch @ 'A'..='F' => Ok(ch as u32 - 'A' as u32 + 10),
            _ => Err(self.error_unexpected(self.pos - 1)),
        }
    }

    fn read_escaped_char(&mut self) -> Result<char, RubyError> {
        let ch = match self.get()? {
            c @ '\'' | c @ '"' | c @ '?' | c @ '\\' => c,
            'a' => '\x07',
            'b' => '\x08',
            'f' => '\x0c',
            'n' => '\x0a',
            'r' => '\x0d',
            't' => '\x09',
            'v' => '\x0b',
            'x' => {
                let c1 = self.get()?;
                let c1 = self.char_to_hex(c1)?;
                let c2 = self.get()?;
                let c2 = self.char_to_hex(c2)?;
                match std::char::from_u32(c1 * 16 + c2) {
                    Some(c) => c,
                    None => return Err(self.error_unexpected(self.pos)),
                }
            }
            c => c,
        };
        Ok(ch)
    }
}

impl Lexer {
    /// Get one char and move to the next.
    /// Returns Ok(char) or RubyError if the cursor reached EOF.
    fn get(&mut self) -> Result<char, RubyError> {
        if self.pos as usize >= self.len {
            Err(self.error_eof(self.pos))
        } else {
            let ch = self.source_info.code[self.pos as usize];
            self.pos += 1;
            Ok(ch)
        }
    }

    /// Push back the last token.
    fn push_back(&mut self) {
        self.pos -= 1;
    }

    /// Consume the next char, if the char is equal to the given one.
    /// Return true if the token was consumed.
    fn consume(&mut self, ch: char) -> bool {
        if self.pos as usize >= self.len {
            false
        } else if ch == self.source_info.code[self.pos as usize] {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Consume the next char, if the char is numeric char.
    /// Return Some(ch) if the token (ch) was consumed.
    fn consume_numeric(&mut self) -> Option<char> {
        if self.pos as usize >= self.len {
            return None;
        };
        let ch = self.source_info.code[self.pos as usize];
        if ch.is_ascii() && ch.is_numeric() {
            self.pos += 1;
            Some(ch)
        } else {
            None
        }
    }

    /// Consume the next char, if the char is ascii-whitespace char.
    /// Return Some(ch) if the token (ch) was consumed.
    fn consume_whitespace(&mut self) -> bool {
        if self.pos as usize >= self.len {
            return false;
        };
        if self.source_info.code[self.pos as usize].is_ascii_whitespace() {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Peek the next char.
    /// Returns Some(char) or None if the cursor reached EOF.
    fn peek(&mut self) -> Result<char, RubyError> {
        if self.pos as usize >= self.len {
            Err(self.error_eof(self.pos))
        } else {
            Ok(self.source_info.code[self.pos as usize])
        }
    }

    /// Skip whitespace and line terminator.
    /// Returns Some(Space or LineTerm) or None if the cursor reached EOF.
    fn skip_whitespace(&mut self) -> Option<Token> {
        let mut res = None;
        loop {
            if self.consume('\n') {
                res = Some(self.new_line_term());
                self.token_start_pos = self.pos;
            } else if self.consume_whitespace() {
                self.token_start_pos = self.pos;
                if res.is_none() {
                    res = Some(self.new_space());
                }
            } else {
                return res;
            }
        }
    }

    fn goto_eol(&mut self) {
        loop {
            match self.peek() {
                Ok('\n') | Err(_) => return,
                _ => self.get().unwrap(),
            };
        }
    }

    fn cur_loc(&self) -> Loc {
        let end = std::cmp::max(self.token_start_pos, self.pos - 1);
        Loc(self.token_start_pos, end)
    }
}

impl Lexer {
    fn new_ident(&self, ident: impl Into<String>, has_suffix: bool, trailing_space: bool) -> Token {
        Token::new_ident(ident.into(), has_suffix, trailing_space, self.cur_loc())
    }

    fn new_instance_var(&self, ident: impl Into<String>) -> Token {
        Annot::new(TokenKind::InstanceVar(ident.into()), self.cur_loc())
    }

    fn new_global_var(&self, ident: impl Into<String>) -> Token {
        Annot::new(TokenKind::GlobalVar(ident.into()), self.cur_loc())
    }

    fn new_const(&self, ident: impl Into<String>, has_suffix: bool, trailing_space: bool) -> Token {
        Annot::new(
            TokenKind::Const(ident.into(), has_suffix, trailing_space),
            self.cur_loc(),
        )
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

    fn new_open_reg(&self, s: String) -> Token {
        Token::new_open_reg(s, self.cur_loc())
    }

    fn new_percent(&self, kind: char, content: String) -> Token {
        Token::new_percent(kind, content, self.cur_loc())
    }

    fn new_space(&self) -> Token {
        Annot::new(TokenKind::Space, self.cur_loc())
    }

    fn new_line_term(&self) -> Token {
        Annot::new(TokenKind::LineTerm, self.cur_loc())
    }

    fn new_eof(&self, pos: u32) -> Token {
        Annot::new(TokenKind::EOF, Loc(pos, pos))
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

    fn print_tokens(tokens: &[Token], ans: &[Token]) {
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
        (Ident($item:expr, $flag:expr, $space:expr), $loc_0:expr, $loc_1:expr) => {
            Token::new_ident($item, $flag, $space, Loc($loc_0, $loc_1))
        };
        (InstanceVar($item:expr), $loc_0:expr, $loc_1:expr) => {
            Token::new_instance_var($item, Loc($loc_0, $loc_1))
        };
        (GlobalVar($item:expr), $loc_0:expr, $loc_1:expr) => {
            Token::new_global_var($item, Loc($loc_0, $loc_1))
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
        (OpenString($item:expr), $loc_0:expr, $loc_1:expr) => {
            Token::new_open_dq($item, Loc($loc_0, $loc_1))
        };
        (InterString($item:expr), $loc_0:expr, $loc_1:expr) => {
            Token::new_inter_dq($item, Loc($loc_0, $loc_1))
        };
        (CloseString($item:expr), $loc_0:expr, $loc_1:expr) => {
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
            Token![OpenString("this is "), 0, 10],
            Token![Ident("item1", false, false), 11, 15],
            Token![InterString(" and "), 16, 23],
            Token![Ident("item2", false, false), 24, 28],
            Token![CloseString(". "), 29, 32],
            Token![EOF, 33],
        ];
        assert_tokens(program, ans);
    }

    #[test]
    fn identifier1() {
        let program = "amber";
        let ans = vec![Token![Ident("amber", false, false), 0, 4], Token![EOF, 5]];
        assert_tokens(program, ans);
    }

    #[test]
    fn instance_var() {
        let program = "@amber";
        let ans = vec![Token![InstanceVar("@amber"), 0, 5], Token![EOF, 6]];
        assert_tokens(program, ans);
    }

    #[test]
    fn global_var() {
        let program = "$amber";
        let ans = vec![Token![GlobalVar("$amber"), 0, 5], Token![EOF, 6]];
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
            Token![Ident("a", false, true), 0, 0],
            Token![Punct(Punct::Assign), 2, 2],
            Token![NumLit(1), 4, 4],
            Token![LineTerm, 5, 5],
            Token![Reserved(Reserved::If), 7, 8],
            Token![Ident("a", true, false), 10, 10],
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
            Token![Ident("a", false, true), 9, 9],
            Token![Punct(Punct::Assign), 11, 11],
            Token![NumLit(0), 13, 13],
            Token![Punct(Punct::Semi), 14, 14],
            Token![LineTerm, 15, 15],
            Token![Reserved(Reserved::If), 24, 25],
            Token![Ident("a", false, true), 27, 27],
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
