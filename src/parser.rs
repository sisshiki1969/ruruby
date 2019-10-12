use crate::lexer::*;
use crate::value::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    kind: NodeKind,
    loc: Loc,
}

impl Node {
    fn new_number(num: i64, loc: Loc) -> Self {
        Node {
            kind: NodeKind::Number(num),
            loc,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Number(i64),
    Add(Box<Node>, Box<Node>),
    Sub(Box<Node>, Box<Node>),
    Mul(Box<Node>, Box<Node>),
    Assign(Box<Node>, Box<Node>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnexpectedToken,
    EOF,
}

impl Parser {
    pub fn new(result: LexerResult) -> Self {
        Parser {
            tokens: result.tokens,
            cursor: 0,
        }
    }

    /// Get next token (skipping line terminators).
    fn get(&mut self) -> &Token {
        loop {
            let token = &self.tokens[self.cursor];
            if token.is_eof() {
                return token;
            }
            self.cursor += 1;
            if !token.is_line_term() {
                return token;
            }
        }
    }

    fn unget(&mut self) {
        self.cursor -= 1;
    }

    /// Get next token (no skipping line terminators).
    fn get_no_skip_line_term(&mut self) -> &Token {
        let token = &self.tokens[self.cursor];
        if !token.is_eof() {
            self.cursor += 1;
        }
        token
    }

    fn expect_term(&mut self) -> Result<&Token, ParseError> {
        let tok = self.get_no_skip_line_term();
        if tok.is_term() {
            Ok(tok)
        } else {
            Err(ParseError::UnexpectedToken)
        }
    }

    fn peek(&mut self) -> &Token {
        &self.tokens[self.cursor]
    }

    pub fn parse_comp_stmt(&mut self) -> Result<(), ParseError> {
        loop {
            if self.peek().is_eof() {
                break;
            }
            let node = self.parse_expr()?;
            println!("{:?}", node);
            println!("ans {:?}", Parser::eval_node(&node));
            if self.expect_term()?.is_eof() {
                break;
            }
        }
        Ok(())
    }

    pub fn parse_expr(&mut self) -> Result<Node, ParseError> {
        self.parse_arg_add()
    }

    fn parse_arg_add(&mut self) -> Result<Node, ParseError> {
        let lhs = self.parse_arg_mul()?;
        let tok = self.peek().clone();
        match &tok.value {
            TokenKind::Punct(ref punct) => match punct {
                Punct::Plus => {
                    self.get();
                    let rhs = self.parse_arg_add()?;
                    let loc = lhs.loc.merge(rhs.loc);
                    return Ok(Node {
                        kind: NodeKind::Add(Box::new(lhs), Box::new(rhs)),
                        loc,
                    });
                }
                Punct::Minus => {
                    self.get();
                    let rhs = self.parse_arg_add()?;
                    let loc = lhs.loc.merge(rhs.loc);
                    return Ok(Node {
                        kind: NodeKind::Sub(Box::new(lhs), Box::new(rhs)),
                        loc,
                    });
                }
                _ => return Ok(lhs),
            },
            _ => {
                return Ok(lhs);
            }
        }
    }

    pub fn parse_arg_mul(&mut self) -> Result<Node, ParseError> {
        let lhs = self.parse_primary()?;
        let tok = self.peek().clone();
        match &tok.value {
            TokenKind::Punct(ref punct) => match punct {
                Punct::Mul => {
                    self.get();
                    let rhs = self.parse_arg_mul()?;
                    let loc = lhs.loc.merge(rhs.loc);
                    return Ok(Node {
                        kind: NodeKind::Mul(Box::new(lhs), Box::new(rhs)),
                        loc,
                    });
                }
                _ => return Ok(lhs),
            },
            _ => {
                return Ok(lhs);
            }
        }
    }

    fn parse_primary(&mut self) -> Result<Node, ParseError> {
        let tok = self.get();
        match &tok.value {
            TokenKind::NumLit(num) => Ok(Node::new_number(*num, tok.loc())),
            TokenKind::Punct(punct) if *punct == Punct::LParen => Ok({
                let node = self.parse_expr()?;
                let tok = self.get();
                if tok.value == TokenKind::Punct(Punct::RParen) {
                    node
                } else {
                    return Err(ParseError::UnexpectedToken);
                }
            }),
            TokenKind::EOF => {
                return Err(ParseError::EOF);
            }
            _ => unimplemented!(),
        }
    }

    pub fn eval_node(node: &Node) -> Value {
        match &node.kind {
            NodeKind::Number(num) => Value::FixNum(*num),
            NodeKind::Add(lhs, rhs) => {
                let lhs = Parser::eval_node(lhs);
                let rhs = Parser::eval_node(rhs);
                match (lhs, rhs) {
                    (Value::FixNum(lhs), Value::FixNum(rhs)) => Value::FixNum(lhs + rhs),
                    (_, _) => unimplemented!(),
                }
            }
            NodeKind::Sub(lhs, rhs) => {
                let lhs = Parser::eval_node(lhs);
                let rhs = Parser::eval_node(rhs);
                match (lhs, rhs) {
                    (Value::FixNum(lhs), Value::FixNum(rhs)) => Value::FixNum(lhs - rhs),
                    (_, _) => unimplemented!(),
                }
            }
            NodeKind::Mul(lhs, rhs) => {
                let lhs = Parser::eval_node(lhs);
                let rhs = Parser::eval_node(rhs);
                match (lhs, rhs) {
                    (Value::FixNum(lhs), Value::FixNum(rhs)) => Value::FixNum(lhs * rhs),
                    (_, _) => unimplemented!(),
                }
            }
            _ => unimplemented!(),
        }
    }
}
