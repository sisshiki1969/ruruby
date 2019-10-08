use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Lexer {
    code: Vec<char>,
    len: usize,
    pos: usize,
    line: usize,
    reserved: HashMap<String, Reserved>,
    reserved_rev: HashMap<Reserved, String>,
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
    Ident(String),
    NumLit(i64),
    Reserved(Reserved),
    Punct(char),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Annot<T> {
    value: T,
    loc: Loc,
}

impl<T> Annot<T> {
    fn new(value: T, loc: Loc) -> Self {
        Annot { value, loc }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Loc(usize, usize);

pub type Token = Annot<TokenKind>;

impl Token {
    fn new_ident(ident: String, loc: Loc) -> Self {
        Annot::new(TokenKind::Ident(ident), loc)
    }

    fn new_reserved(ident: Reserved, loc: Loc) -> Self {
        Annot::new(TokenKind::Reserved(ident), loc)
    }

    fn new_numlit(num: i64, loc: Loc) -> Self {
        Annot::new(TokenKind::NumLit(num), loc)
    }

    fn new_punct(ch: char, loc: Loc) -> Self {
        Annot::new(TokenKind::Punct(ch), loc)
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
            code,
            len,
            pos: 0,
            line: 0,
            reserved,
            reserved_rev,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, Error> {
        let mut tokens: Vec<Token> = vec![];
        loop {
            self.skip_whitespace();
            let start_pos = self.pos;
            let ch = match self.get() {
                Ok(ch) => ch,
                Err(_) => break,
            };
            macro_rules! cur_loc {
                () => {
                    Loc(start_pos, self.pos - 1)
                };
            }
            let token = if ch.is_ascii_alphabetic() || ch == '_' {
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
                    Some(reserved) => Token::new_reserved(*reserved, cur_loc!()),
                    None => Token::new_ident(tok, cur_loc!()),
                }
            } else if ch.is_numeric() {
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
                Token::new_numlit(i, cur_loc!())
            } else if ch.is_ascii_punctuation() {
                Token::new_punct(ch, cur_loc!())
            } else {
                return Err(Error::UnexpectedChar);
            };
            tokens.push(token);
        }
        Ok(tokens)
    }
}

impl Lexer {
    fn get(&mut self) -> Result<char, Error> {
        if self.pos >= self.len {
            Err(Error::EOF)
        } else {
            let ch = self.code[self.pos];
            if ch == '\n' {
                self.line += 1;
            }
            self.pos += 1;
            Ok(ch)
        }
    }

    fn peek(&mut self) -> Result<char, Error> {
        if self.pos >= self.len {
            Err(Error::EOF)
        } else {
            Ok(self.code[self.pos])
        }
    }

    fn skip_whitespace(&mut self) {
        for p in self.pos..self.len {
            let ch = self.code[p];
            if ch == '\n' {
                self.line += 1;
            } else if !ch.is_ascii_whitespace() {
                self.pos = p;
                return;
            };
        }
        self.pos = self.len;
    }
}

#[allow(unused_imports)]
#[allow(dead_code)]
mod test {
    use crate::lexer::*;
    fn assert_tokens(program: impl Into<String>, ans: Vec<Token>) {
        let mut lexer = Lexer::new(program.into());
        match lexer.tokenize() {
            Err(err) => panic!("{:?}", err),
            Ok(tokens) => {
                let len = tokens.len();
                for i in 0..len {
                    if tokens[i] != ans[i] {
                        panic!("Expected:{:?} Got:{:?}", tokens[i], ans[i]);
                    }
                }
                if len != ans.len() {
                    panic!("Expected:{:?} Got:{:?}", tokens, ans);
                }
            }
        };
    }

    #[test]
    fn lexer_test() {
        let program = "a = 0;\n if a == 1_000 then 5 else 10";
        let ans = vec![
            Annot {
                value: TokenKind::Ident("a".to_string()),
                loc: Loc(0, 0),
            },
            Annot {
                value: TokenKind::Punct('='),
                loc: Loc(2, 2),
            },
            Annot {
                value: TokenKind::NumLit(0),
                loc: Loc(4, 4),
            },
            Annot {
                value: TokenKind::Punct(';'),
                loc: Loc(5, 5),
            },
            Annot {
                value: TokenKind::Reserved(Reserved::If),
                loc: Loc(8, 9),
            },
            Annot {
                value: TokenKind::Ident("a".to_string()),
                loc: Loc(11, 11),
            },
            Annot {
                value: TokenKind::Punct('='),
                loc: Loc(13, 13),
            },
            Annot {
                value: TokenKind::Punct('='),
                loc: Loc(14, 14),
            },
            Annot {
                value: TokenKind::NumLit(1000),
                loc: Loc(16, 20),
            },
            Annot {
                value: TokenKind::Reserved(Reserved::Then),
                loc: Loc(22, 25),
            },
            Annot {
                value: TokenKind::NumLit(5),
                loc: Loc(27, 27),
            },
            Annot {
                value: TokenKind::Reserved(Reserved::Else),
                loc: Loc(29, 32),
            },
            Annot {
                value: TokenKind::NumLit(10),
                loc: Loc(34, 35),
            },
        ];
        assert_tokens(program, ans);
    }
}
