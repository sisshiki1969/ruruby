use super::*;
use crate::error::{ParseErrKind, RubyError};
use crate::util::*;
use crate::value::real::Real;
use fxhash::FxHashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct Lexer {
    code: String,
    len: usize,
    token_start_pos: usize,
    pos: usize,
    buf: Option<Token>,
    buf_skip_lt: Option<Token>,
    reserved: FxHashMap<String, Reserved>,
    reserved_rev: FxHashMap<Reserved, String>,
    pub source_info: SourceInfoRef,
    state_save: Vec<(usize, usize)>, // (token_start_pos, pos)
}

#[derive(Debug, Clone)]
pub struct LexerResult {
    pub tokens: Vec<Token>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum VarKind {
    Identifier,
    InstanceVar,
    ClassVar,
    GlobalVar,
}
#[derive(Debug, Clone, PartialEq)]
enum InterpolateState {
    Finished(String),
    NewInterpolation(String, usize), // (string, paren_level)
}

impl Lexer {
    pub fn new(path: impl Into<PathBuf>, code: impl Into<String>) -> Self {
        let mut reserved = FxHashMap::default();
        let mut reserved_rev = FxHashMap::default();
        macro_rules! reg_reserved {
            ( $($id:expr => $variant:path),+ ) => {
                $(
                    reserved.insert($id.to_string(), $variant);
                    reserved_rev.insert($variant, $id.to_string());
                )+
            };
        }
        reg_reserved! {
            "and" => Reserved::And,
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
            "ensure"=> Reserved::Ensure,
            "elsif" => Reserved::Elsif,
            "end" => Reserved::End,
            "for" => Reserved::For,
            "false" => Reserved::False,
            "if" => Reserved::If,
            "in" => Reserved::In,
            "module" => Reserved::Module,
            "next" => Reserved::Next,
            "nil" => Reserved::Nil,
            "or" => Reserved::Or,
            "return" => Reserved::Return,
            "rescue" => Reserved::Rescue,
            "self" => Reserved::Self_,
            "then" => Reserved::Then,
            "true" => Reserved::True,
            "until" => Reserved::Until,
            "unless" => Reserved::Unless,
            "when" => Reserved::When,
            "while" => Reserved::While,
            "yield" => Reserved::Yield
        };
        let code = code.into();
        let source_info = SourceInfoRef::new(SourceInfo::new(path, &code));
        Lexer {
            code,
            len: source_info.code.len(),
            token_start_pos: 0,
            pos: 0,
            buf: None,
            buf_skip_lt: None,
            reserved,
            reserved_rev,
            source_info,
            state_save: vec![],
        }
    }

    pub fn get_string_from_reserved(&self, reserved: Reserved) -> &str {
        self.reserved_rev.get(&reserved).unwrap()
    }

    fn error_unexpected(&self, pos: usize) -> RubyError {
        let loc = Loc(pos, pos);
        RubyError::new_parse_err(
            ParseErrKind::SyntaxError(format!(
                "Unexpected char. '{}'",
                self.code.get(pos..).unwrap().chars().next().unwrap()
            )),
            self.source_info,
            0,
            loc,
        )
    }

    fn error_eof(&self, pos: usize) -> RubyError {
        let loc = Loc(pos, pos);
        RubyError::new_parse_err(ParseErrKind::UnexpectedEOF, self.source_info, 0, loc)
    }

    fn error_parse(&self, msg: &str, pos: usize) -> RubyError {
        let loc = Loc(pos, pos);
        RubyError::new_parse_err(
            ParseErrKind::SyntaxError(format!("Parse error. '{}'", msg)),
            self.source_info,
            0,
            loc,
        )
    }

    #[cfg(test)]
    pub fn tokenize(&mut self) -> Result<LexerResult, RubyError> {
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

    pub fn append(&mut self, code_text: &str) {
        self.pos = self.code.len();
        self.code.push_str(code_text);
        self.len = self.code.len();
        self.source_info.code += &code_text;
    }

    pub fn get_token(&mut self) -> Result<Token, RubyError> {
        self.buf = None;
        self.buf_skip_lt = None;
        let tok = self.read_token()?;
        Ok(tok)
    }

    pub fn peek_token(&mut self) -> Result<Token, RubyError> {
        if let Some(tok) = &self.buf {
            return Ok(tok.clone());
        };
        self.save_state();
        let tok = self.read_token()?;
        self.restore_state();
        self.buf = Some(tok.clone());
        Ok(tok)
    }

    pub fn peek_token_skip_lt(&mut self) -> Result<Token, RubyError> {
        if let Some(tok) = &self.buf_skip_lt {
            return Ok(tok.clone());
        };
        self.save_state();
        let mut tok;
        loop {
            tok = self.read_token()?;
            if tok.is_eof() || !tok.is_line_term() {
                break;
            }
        }
        self.restore_state();
        self.buf_skip_lt = Some(tok.clone());
        Ok(tok)
    }

    /// Examine if the next char is a whitespace or not.
    pub fn trailing_space(&self) -> bool {
        match self.peek() {
            Some(ch) => ch.is_ascii_whitespace(),
            _ => false,
        }
    }

    /// Examine if the next char is '('.
    pub fn trailing_lparen(&self) -> bool {
        match self.peek() {
            Some(ch) => ch == '(',
            _ => false,
        }
    }

    /// Examine if the next char of the token is space.
    pub fn has_trailing_space(&self, tok: &Token) -> bool {
        match self
            .code
            .get(tok.loc.1 + 1..)
            .map(|s| s.chars().next())
            .flatten()
        {
            Some(ch) => ch.is_ascii_whitespace(),
            _ => false,
        }
    }

    /// Get token as a regular expression.
    pub fn get_regexp(&mut self) -> Result<Token, RubyError> {
        match self.read_regexp_sub()? {
            InterpolateState::Finished(s) => Ok(self.new_stringlit(s)),
            InterpolateState::NewInterpolation(s, _) => Ok(self.new_open_reg(s)),
        }
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
}

impl Lexer {
    /// Read token.
    fn read_token(&mut self) -> Result<Token, RubyError> {
        loop {
            self.token_start_pos = self.pos;
            if let Some(tok) = self.skip_whitespace() {
                return Ok(tok);
            };

            let pos = self.pos;
            let ch = match self.get() {
                Ok(ch) => ch,
                Err(_) => return Ok(self.new_eof()),
            };

            if ch.is_ascii_alphabetic() || ch == '_' {
                return self.read_identifier(ch, VarKind::Identifier);
            } else if ch.is_numeric() {
                return self.read_number_literal(ch);
            } else if ch.is_ascii_punctuation() {
                match ch {
                    '#' => self.goto_eol(),
                    '"' => return self.read_string_literal_double(None, '\"', 0),
                    '`' => return self.read_command_literal(None, '`', 0),
                    '\'' => {
                        let s = self.read_string_literal_single(None, '\'', false)?;
                        return Ok(self.new_stringlit(s));
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
                    '}' => return Ok(self.new_punct(Punct::RBrace)),
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
                            if self.consume('>') {
                                return Ok(self.new_punct(Punct::Cmp));
                            } else {
                                return Ok(self.new_punct(Punct::Le));
                            }
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
                        } else if self.consume('~') {
                            return Ok(self.new_punct(Punct::Unmatch));
                        } else {
                            return Ok(self.new_punct(Punct::Not));
                        }
                    }
                    '&' => {
                        if self.consume('&') {
                            if self.consume('=') {
                                return Ok(self.new_punct(Punct::AssignOp(BinOp::LAnd)));
                            } else {
                                return Ok(self.new_punct(Punct::LAnd));
                            }
                        } else if self.consume('=') {
                            return Ok(self.new_punct(Punct::AssignOp(BinOp::BitAnd)));
                        } else if self.consume('.') {
                            return Ok(self.new_punct(Punct::SafeNav));
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
                        if self.consume('@') {
                            return self.read_identifier(None, VarKind::ClassVar);
                        } else {
                            return self.read_identifier(None, VarKind::InstanceVar);
                        }
                    }
                    '$' => return self.read_global_var(),
                    _ => return Err(self.error_unexpected(pos)),
                }
            } else {
                return self.read_identifier(ch, VarKind::Identifier);
            };
        }
    }

    fn read_global_var(&mut self) -> Result<Token, RubyError> {
        let tok = match self.peek() {
            Some(ch) if ch.is_ascii_punctuation() => {
                let ch = self.get()?;
                self.new_global_var(format!("${}", ch))
            }
            _ => self.read_identifier(None, VarKind::GlobalVar)?,
        };
        Ok(tok)
    }

    /// Read identifier. ('@@xx', '$x', '@x')
    fn read_identifier(
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
                let pos = self.pos;
                match self.get() {
                    Ok(ch) => {
                        if ch.is_alphanumeric() || ch == '_' || ch == '&' || ch == '\'' {
                            tok.push(ch);
                        } else {
                            return Err(self.error_unexpected(pos));
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
                Some(ch) => ch,
                _ => {
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
            VarKind::InstanceVar => return Ok(self.new_instance_var(tok)),
            VarKind::ClassVar => return Ok(self.new_class_var(tok)),
            VarKind::GlobalVar => return Ok(self.new_global_var(tok)),
            _ => {}
        }

        match self.peek() {
            Some(ch) if (ch == '!' && self.peek2() != Some('=')) || ch == '?' => {
                tok.push(self.get()?);
            }
            _ => {}
        };

        match self.reserved.get(&tok) {
            Some(reserved) => return Ok(self.new_reserved(*reserved)),
            None => {}
        };

        if is_const {
            Ok(self.new_const(tok))
        } else {
            Ok(self.new_ident(tok))
        }
    }

    /// Read number literal.
    fn read_number_literal(&mut self, ch: char) -> Result<Token, RubyError> {
        if ch == '0' {
            if self.consume('x') {
                return self.read_hex_number();
            } else if self.consume('b') {
                return self.read_bin_number();
            }
        };
        let mut s = ch.to_string();
        let mut float_flag = false;
        loop {
            if let Some(ch) = self.consume_numeric() {
                s.push(ch);
            } else if self.consume('_') {
            } else if !float_flag && self.peek() == Some('.') {
                match self.peek2() {
                    Some(ch) if ch.is_ascii() && ch.is_numeric() => {
                        self.get()?;
                        self.get()?;
                        float_flag = true;
                        s.push('.');
                        s.push(ch);
                    }
                    _ => break,
                }
            } else {
                break;
            }
        }
        if self.consume('e') || self.consume('E') {
            s.push('e');
            self.consume('+');
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
            float_flag = true;
        }
        let number = if float_flag {
            match s.parse::<f64>() {
                Ok(f) => Real::Float(f),
                Err(err) => return Err(self.error_parse(&format!("{:?}", err), self.pos)),
            }
        } else {
            match s.parse::<i64>() {
                Ok(i) => Real::Integer(i),
                Err(_err) => {
                    // TODO: parse BigNum
                    Real::Integer(0)
                }
            }
        };
        if self.consume('i') {
            Ok(self.new_imaginarylit(number))
        } else {
            match number {
                Real::Integer(i) => Ok(self.new_numlit(i)),
                Real::Float(f) => Ok(self.new_floatlit(f)),
            }
        }
    }

    /// Read hexadecimal number.
    fn read_hex_number(&mut self) -> Result<Token, RubyError> {
        let mut val = match self.peek() {
            Some(ch @ '0'..='9') => (ch as u64 - '0' as u64),
            Some(ch @ 'a'..='f') => (ch as u64 - 'a' as u64 + 10),
            Some(ch @ 'A'..='F') => (ch as u64 - 'A' as u64 + 10),
            _ => return Err(self.error_unexpected(self.pos)),
        };
        self.get()?;
        loop {
            match self.peek() {
                Some(ch @ '0'..='9') => val = val * 16 + (ch as u64 - '0' as u64),
                Some(ch @ 'a'..='f') => val = val * 16 + (ch as u64 - 'a' as u64 + 10),
                Some(ch @ 'A'..='F') => val = val * 16 + (ch as u64 - 'A' as u64 + 10),
                Some('_') => {}
                _ => break,
            }
            self.get()?;
        }
        Ok(self.new_numlit(val as i64))
    }

    /// Read binary number.
    fn read_bin_number(&mut self) -> Result<Token, RubyError> {
        let mut val = match self.peek() {
            Some(ch @ '0'..='1') => (ch as u64 - '0' as u64),
            Some(_) => {
                return Err(self.error_unexpected(self.pos));
            }
            None => return Err(self.error_eof(self.pos)),
        };
        self.get()?;
        loop {
            match self.peek() {
                Some(ch @ '0'..='1') => val = val * 2 + (ch as u64 - '0' as u64),
                Some('_') => {}
                _ => break,
            }
            self.get()?;
        }
        Ok(self.new_numlit(val as i64))
    }

    /// Read string literal ("..", %Q{..}, %{..})
    pub fn read_string_literal_double(
        &mut self,
        open: Option<char>,
        term: char,
        level: usize,
    ) -> Result<Token, RubyError> {
        match self.read_interpolate(open, term, level)? {
            InterpolateState::Finished(s) => Ok(self.new_stringlit(s)),
            InterpolateState::NewInterpolation(s, level) => {
                Ok(self.new_open_string(s, term, level))
            }
        }
    }

    /// Read command literal (`..`)
    pub fn read_command_literal(
        &mut self,
        open: Option<char>,
        term: char,
        level: usize,
    ) -> Result<Token, RubyError> {
        match self.read_interpolate(open, term, level)? {
            InterpolateState::Finished(s) => Ok(self.new_commandlit(s)),
            InterpolateState::NewInterpolation(s, level) => {
                Ok(self.new_open_command(s, term, level))
            }
        }
    }

    /// Read interpolation string with `open` as opening char and `term` as a terminator.
    fn read_interpolate(
        &mut self,
        open: Option<char>,
        term: char,
        mut level: usize,
    ) -> Result<InterpolateState, RubyError> {
        let mut s = "".to_string();
        loop {
            match self.get()? {
                c if open == Some(c) => {
                    s.push(c);
                    level += 1;
                }
                c if c == term => {
                    if level == 0 {
                        return Ok(InterpolateState::Finished(s));
                    } else {
                        s.push(c);
                        level -= 1;
                    }
                }
                '\\' => s.push(self.read_escaped_char()?),

                '#' => match self.peek() {
                    Some(ch) if ch == '{' || ch == '$' || ch == '@' => {
                        return Ok(InterpolateState::NewInterpolation(s, level))
                    }
                    _ => s.push('#'),
                },
                c => s.push(c),
            }
        }
    }

    /// Read string literal '..' or %q{..}
    fn read_string_literal_single(
        &mut self,
        open: Option<char>,
        term: char,
        escape_backslash: bool,
    ) -> Result<String, RubyError> {
        let mut s = "".to_string();
        let mut level = 0;
        loop {
            match self.get()? {
                c if open == Some(c) => {
                    s.push(c);
                    level += 1;
                }
                c if c == term => {
                    if level == 0 {
                        return Ok(s);
                    } else {
                        s.push(c);
                        level -= 1;
                    }
                }
                '\\' => {
                    let c = self.get()?;
                    if c == '\'' {
                        s.push('\'');
                    } else if c == '\\' {
                        s.push('\\');
                        if escape_backslash {
                            s.push('\\');
                        }
                    } else {
                        s.push('\\');
                        s.push(c);
                    }
                }
                c => s.push(c),
            }
        }
    }

    /// Read char literal.
    pub fn read_char_literal(&mut self) -> Result<Token, RubyError> {
        let c = self.get()?;
        self.buf = None;
        if c == '\\' {
            let ch = self.read_escaped_char()?;
            Ok(self.new_stringlit(ch))
        } else {
            Ok(self.new_stringlit(c))
        }
    }

    /// Convert postfix of regular expression.
    fn check_postfix(&mut self, s: &mut String) {
        if self.consume('i') {
            s.push('i');
        } else if self.consume('m') {
            s.push('m');
        } else if self.consume('x') {
            s.push('x');
        } else if self.consume('o') {
            s.push('o');
        } else {
            s.push('-');
        };
    }

    /// Scan as regular expression.
    fn read_regexp_sub(&mut self) -> Result<InterpolateState, RubyError> {
        let mut s = "".to_string();
        loop {
            match self.get()? {
                '/' => {
                    self.check_postfix(&mut s);
                    return Ok(InterpolateState::Finished(s));
                }
                '\\' => {
                    s.push('\\');
                    // TODO: It is necessary to count capture groups
                    // to determine whether backref or octal digit.
                    // Current impl. may cause problems.
                    let ch = self.get()?;
                    if '1' >= ch && ch <= '9' && !self.peek_digit() {
                        s.push(ch);
                    } else if '0' <= ch && ch <= '7' {
                        let hex = format!("x{:02x}", self.consume_tri_octal(ch).unwrap());
                        hex.chars().for_each(|c| s.push(c));
                    } else {
                        s.push(ch);
                    }
                }
                '#' => match self.peek() {
                    Some(ch) if ch == '{' || ch == '$' || ch == '@' => {
                        return Ok(InterpolateState::NewInterpolation(s, 0))
                    }
                    _ => s.push('#'),
                },
                c => s.push(c),
            }
        }
    }

    pub fn get_percent_notation(&mut self) -> Result<Token, RubyError> {
        let pos = self.pos;
        let c = self.get()?;
        let (kind, delimiter) = match c {
            'q' | 'Q' | 'x' | 'r' | 'w' | 'W' | 's' | 'i' | 'I' => {
                let pos = self.pos;
                let delimiter = self.get()?;
                if delimiter.is_ascii_alphanumeric() {
                    return Err(self.error_unexpected(pos));
                }
                (Some(c), delimiter)
            }
            delimiter if !c.is_ascii_alphanumeric() => (None, delimiter),
            _ => return Err(self.error_unexpected(pos)),
        };
        let (open, term) = match delimiter {
            '(' => (Some('('), ')'),
            '{' => (Some('{'), '}'),
            '[' => (Some('['), ']'),
            '<' => (Some('<'), '>'),
            ' ' | '\n' => match kind {
                Some('i') | Some('I') | Some('w') | Some('W') => {
                    return Err(self.error_unexpected(self.pos - 1))
                }
                _ => (None, delimiter),
            },
            ch => (None, ch),
        };

        match kind {
            Some('q') => {
                let s = self.read_string_literal_single(open, term, false)?;
                Ok(self.new_stringlit(s))
            }
            Some('Q') | None => Ok(self.read_string_literal_double(open, term, 0)?),
            Some('r') => {
                let s = self.read_string_literal_single(open, term, true)?;
                Ok(self.new_percent('r', s))
            }
            Some(kind) => {
                let s = self.read_string_literal_single(open, term, false)?;
                Ok(self.new_percent(kind, s))
            }
        }
    }

    fn char_to_hex(&self, c: char) -> Result<u32, RubyError> {
        match c {
            ch @ '0'..='9' => Ok(ch as u32 - '0' as u32),
            ch @ 'a'..='f' => Ok(ch as u32 - 'a' as u32 + 10),
            ch @ 'A'..='F' => Ok(ch as u32 - 'A' as u32 + 10),
            _ => Err(self.error_unexpected(self.pos - c.len_utf8())),
        }
    }

    fn read_escaped_char(&mut self) -> Result<char, RubyError> {
        let ch = match self.get()? {
            c @ '\'' | c @ '"' | c @ '?' | c @ '\\' => c,
            'a' => '\x07',
            'b' => '\x08',
            'e' => '\x1b',
            'f' => '\x0c',
            'n' => '\x0a',
            'r' => '\x0d',
            's' => '\x20',
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
            'u' => {
                let mut code = 0;
                for _ in 0..4 {
                    let c = self.get()?;
                    code = code * 16 + self.char_to_hex(c)?;
                }
                match std::char::from_u32(code) {
                    Some(ch) => ch,
                    None => return Err(self.error_parse("Invalid UTF-8 character.", self.pos)),
                }
            }
            c if '0' <= c && c <= '7' => {
                if let Some(num) = self.consume_tri_octal(c) {
                    num as char
                } else {
                    c
                }
            }
            c => c,
        };
        Ok(ch)
    }

    pub fn read_heredocument(&mut self) -> Result<String, RubyError> {
        #[derive(Clone, PartialEq)]
        enum Mode {
            Normal,
            AllowIndent,
        }
        let mut mode = Mode::Normal;
        let mut delimiter = vec![];
        match self.peek() {
            Some(ch) if ch == '-' => {
                mode = Mode::AllowIndent;
                self.get()?;
            }
            _ => {}
        };
        loop {
            match self.peek() {
                Some(ch) if ch.is_ascii_alphanumeric() || ch == '_' => {
                    self.get()?;
                    delimiter.push(ch);
                }
                _ => break,
            };
        }
        if delimiter.len() == 0 {
            return Err(self.error_unexpected(self.pos));
        }
        let delimiter: String = delimiter.iter().collect();
        //eprintln!("delimiter:{}", delimiter);
        //self.save_state();
        self.goto_eol();
        self.get()?;
        let mut res = String::new();
        loop {
            let start = self.pos;
            self.goto_eol();
            let end = self.pos;
            let line: String = self.code[start..end].to_string();
            //eprintln!("line:[{}]", line);
            if mode == Mode::AllowIndent {
                if line.trim_start() == delimiter {
                    break;
                }
            } else {
                if line == delimiter {
                    break;
                }
            }
            if self.get().is_err() {
                return Err(self.error_parse(
                    &format!(
                        r#"Can not find string "{}" anywhere before EOF."#,
                        delimiter
                    ),
                    self.pos,
                ));
            };
            res = format!("{}{}\n", res, line);
        }
        //self.restore_state();
        Ok(res)
    }
}

// Low level API
impl Lexer {
    /// Peek the next char.
    /// Returns Some(char) or None if the cursor reached EOF.
    fn peek(&self) -> Option<char> {
        self.code.get(self.pos..)?.chars().next()
    }

    /// Peek the next next char.
    /// Returns Some(char) or None if the cursor reached EOF.
    fn peek2(&self) -> Option<char> {
        let mut iter = self.code.get(self.pos..)?.chars();
        iter.next()?;
        iter.next()
    }

    /// Get one char and move to the next.
    /// Returns Ok(char) or RubyError if the cursor reached EOF.
    fn get(&mut self) -> Result<char, RubyError> {
        match self.peek() {
            Some(ch) => {
                self.pos += ch.len_utf8();
                Ok(ch)
            }
            _ => Err(self.error_eof(self.pos)),
        }
    }

    /// Consume the next char, if the char is equal to the given one.
    /// Return true if the char was consumed.
    fn consume(&mut self, expect: char) -> bool {
        match self.peek() {
            Some(ch) if ch == expect => {
                self.pos += ch.len_utf8();
                true
            }
            _ => false,
        }
    }

    /// Consume continue line. ("\\n")
    /// Return true if consumed.
    fn consume_cont_line(&mut self) -> bool {
        if self.peek2() == Some('\n') && self.peek() == Some('\\') {
            self.pos += 2;
            true
        } else {
            false
        }
    }

    /// Consume the next char, if the char is numeric char.
    /// Return Some(ch) if the token (ch) was consumed.
    fn consume_numeric(&mut self) -> Option<char> {
        match self.peek() {
            Some(ch) if ch.is_ascii() && ch.is_numeric() => {
                self.pos += ch.len_utf8();
                Some(ch)
            }
            _ => None,
        }
    }

    /// Consume the next char, if the char is '0' - '7'.
    /// Return Some(ch) if the token (ch) was consumed.
    fn consume_octal(&mut self) -> Option<u8> {
        match self.peek() {
            Some(ch) if '0' <= ch && ch <= '7' => {
                self.pos += ch.len_utf8();
                Some(ch as u8 - '0' as u8)
            }
            _ => None,
        }
    }

    /// Consume the next char, if the char is ascii-whitespace char.
    /// Return Some(ch) if the token (ch) was consumed.
    fn consume_whitespace(&mut self) -> bool {
        match self.peek() {
            Some(ch) if ch.is_ascii_whitespace() => {
                self.pos += ch.len_utf8();
                true
            }
            _ => false,
        }
    }

    fn consume_tri_octal(&mut self, first_ch: char) -> Option<u8> {
        let mut o = first_ch as u8 - '0' as u8;
        for _ in 0..2 {
            match self.consume_octal() {
                Some(n) => o = o.wrapping_mul(8) + n,
                None => break,
            };
        }
        Some(o)
    }

    /// Peek the next char.
    /// Returns Some(char) or None if the cursor reached EOF.
    fn peek_digit(&self) -> bool {
        match self.peek() {
            Some(ch) => ch.is_ascii_digit(),
            None => false,
        }
    }

    /// Skip whitespace and line terminator.
    ///
    /// Returns Some(Space or LineTerm) or None if the cursor reached EOF.
    fn skip_whitespace(&mut self) -> Option<Token> {
        let mut res = None;
        loop {
            if self.consume('\n') {
                res = Some(self.new_line_term());
            } else if !self.consume_cont_line() && !self.consume_whitespace() {
                self.token_start_pos = self.pos;
                return res;
            }
        }
    }

    fn goto_eol(&mut self) {
        loop {
            match self.peek() {
                Some('\n') | None => return,
                Some(ch) => self.pos += ch.len_utf8(),
            };
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

    fn new_class_var(&self, ident: impl Into<String>) -> Token {
        Annot::new(TokenKind::ClassVar(ident.into()), self.cur_loc())
    }

    fn new_global_var(&self, ident: impl Into<String>) -> Token {
        Annot::new(TokenKind::GlobalVar(ident.into()), self.cur_loc())
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

    fn new_imaginarylit(&self, num: Real) -> Token {
        Token::new_imaginarylit(num, self.cur_loc())
    }

    fn new_stringlit(&self, string: impl Into<String>) -> Token {
        Annot::new(TokenKind::StringLit(string.into()), self.cur_loc())
    }

    fn new_commandlit(&self, string: impl Into<String>) -> Token {
        Annot::new(TokenKind::CommandLit(string.into()), self.cur_loc())
    }

    fn new_punct(&self, punc: Punct) -> Token {
        Annot::new(TokenKind::Punct(punc), self.cur_loc())
    }

    fn new_open_string(&self, s: String, delimiter: char, level: usize) -> Token {
        Token::new_open_string(s, delimiter, level, self.cur_loc())
    }

    fn new_open_reg(&self, s: String) -> Token {
        Token::new_open_reg(s, self.cur_loc())
    }

    fn new_open_command(&self, s: String, delimiter: char, level: usize) -> Token {
        Token::new_open_command(s, delimiter, level, self.cur_loc())
    }

    fn new_percent(&self, kind: char, content: String) -> Token {
        Token::new_percent(kind, content, self.cur_loc())
    }

    fn new_line_term(&self) -> Token {
        Annot::new(TokenKind::LineTerm, self.cur_loc())
    }

    fn new_eof(&self) -> Token {
        Annot::new(TokenKind::EOF, Loc(self.pos, self.pos))
    }
}

#[cfg(test)]
impl LexerResult {
    fn new(tokens: Vec<Token>) -> Self {
        LexerResult { tokens }
    }
}

#[cfg(test)]
#[allow(unused_imports, dead_code)]
mod test {
    use crate::parse::lexer::*;
    fn assert_tokens(program: impl Into<String>, ans: Vec<Token>) {
        let mut lexer = Lexer::new("test", program);
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

    macro_rules! Token {
        (Ident($item:expr), $loc_0:expr, $loc_1:expr) => {
            Token::new_ident($item, Loc($loc_0, $loc_1))
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
        (OpenString($item:expr, $delimiter:expr), $loc_0:expr, $loc_1:expr) => {
            Token::new_open_dq($item, $delimiter, Loc($loc_0, $loc_1))
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
    }

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
    fn identifier1() {
        let program = "amber";
        let ans = vec![Token![Ident("amber"), 0, 4], Token![EOF, 5]];
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

    #[test]
    fn cont_line() {
        let program = r###"a \
\
\
  =\
77"###;
        let ans = vec![
            Token![Ident("a"), 0, 0],
            Token![Punct(Punct::Assign), 10, 10],
            Token![NumLit(77), 13, 14],
            Token![EOF, 15],
        ];
        assert_tokens(program, ans);
    }

    #[test]
    fn octal() {
        let program = "/173";
        let ans = vec![
            Token![Punct(Punct::Div), 0, 0],
            Token![NumLit(173), 1, 3],
            Token![EOF, 4],
        ];
        assert_tokens(program, ans);
    }
}
