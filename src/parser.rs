use crate::lexer::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub kind: NodeKind,
    pub loc: Loc,
}

impl Node {
    fn new_number(num: i64, loc: Loc) -> Self {
        Node {
            kind: NodeKind::Number(num),
            loc,
        }
    }
}

impl std::fmt::Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            NodeKind::BinOp(op, lhs, rhs) => write!(f, "[{:?} ( {}, {} )]", op, lhs, rhs),
            _ => write!(f, "[{:?}]", self.kind),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    Number(i64),
    BinOp(BinOp, Box<Node>, Box<Node>),
    Assign(Box<Node>, Box<Node>),
    CompStmt(Vec<Node>),
    If(Box<Node>, Box<Node>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Eq,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    pub source_info: SourceInfo,
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
            source_info: result.source_info,
        }
    }

    /// Peek next token (skipping line terminators).
    fn peek(&mut self) -> &Token {
        let mut c = self.cursor;
        loop {
            let tok = &self.tokens[c];
            if tok.is_eof() || !tok.is_line_term() {
                return tok;
            } else {
                c += 1;
            }
        }
    }

    /// Peek next token (no skipping line terminators).
    fn peek_no_skip_line_term(&mut self) -> &Token {
        &self.tokens[self.cursor]
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

    /// Get next token (no skipping line terminators).
    fn get_no_skip_line_term(&mut self) -> Token {
        let token = self.tokens[self.cursor].clone();
        if !token.is_eof() {
            self.cursor += 1;
        }
        token
    }

    /// If next token is a expected kind of Punct, get it and return true.
    /// Otherwise, return false.
    fn get_if_punct(&mut self, expect: Punct) -> bool {
        match &self.peek().kind {
            TokenKind::Punct(punct) => {
                if *punct == expect {
                    self.get();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Get the next token if it is a line terminator, and return true,
    /// Otherwise, return false.
    fn get_if_term(&mut self) -> bool {
        if self.peek_no_skip_line_term().is_term() {
            self.get_no_skip_line_term();
            true
        } else {
            false
        }
    }

    fn expect_term(&mut self) -> Result<(), ParseError> {
        let tok = self.get_no_skip_line_term();
        if tok.is_term() {
            Ok(())
        } else {
            Err(self.error_unexpected(&tok))
        }
    }

    fn expect_reserved(&mut self, expect: Reserved) -> Result<(), ParseError> {
        let tok = self.get().clone();
        match tok.kind {
            TokenKind::Reserved(reserved) => {
                if reserved == expect {
                    Ok(())
                } else {
                    Err(self.error_unexpected(&tok))
                }
            }
            _ => Err(self.error_unexpected(&tok)),
        }
    }

    fn error_unexpected(&self, tok: &Token) -> ParseError {
        self.source_info.show_loc(&tok.loc());
        ParseError::UnexpectedToken
    }

    pub fn parse_comp_stmt(&mut self) -> Result<Node, ParseError> {
        let mut nodes = vec![];
        loop {
            let tok = self.peek();
            match tok.kind {
                TokenKind::EOF => break,
                TokenKind::Reserved(reserved) => match reserved {
                    Reserved::Else | Reserved::Elsif | Reserved::End => break,
                    _ => {}
                },
                _ => {}
            };
            nodes.push(self.parse_expr()?);
            if !self.get_if_term() {
                break;
            }
            //println!("{:?}", node);
        }
        let mut loc;
        if nodes.len() == 0 {
            loc = Loc::new(0, 0);
        } else {
            loc = nodes[0].loc;
            for node in &nodes {
                loc = loc.merge(node.loc);
            }
        }
        Ok(Node {
            kind: NodeKind::CompStmt(nodes),
            loc,
        })
    }

    pub fn parse_expr(&mut self) -> Result<Node, ParseError> {
        self.parse_arg_comp()
    }

    fn parse_arg_comp(&mut self) -> Result<Node, ParseError> {
        let lhs = self.parse_arg_add()?;
        let tok = self.peek().clone();
        match &tok.kind {
            TokenKind::Punct(ref punct) => match punct {
                Punct::Equal => {
                    self.get();
                    let rhs = self.parse_arg_comp()?;
                    let loc = lhs.loc.merge(rhs.loc);
                    return Ok(Node {
                        kind: NodeKind::BinOp(BinOp::Eq, Box::new(lhs), Box::new(rhs)),
                        loc,
                    });
                }
                _ => return Ok(lhs),
            },
            _ => return Ok(lhs),
        }
    }

    fn parse_arg_add(&mut self) -> Result<Node, ParseError> {
        let lhs = self.parse_arg_mul()?;
        if self.get_if_punct(Punct::Plus) {
            let rhs = self.parse_arg_add()?;
            let loc = lhs.loc.merge(rhs.loc);
            let kind = NodeKind::BinOp(BinOp::Add, Box::new(lhs), Box::new(rhs));
            Ok(Node { kind, loc })
        } else if self.get_if_punct(Punct::Minus) {
            let rhs = self.parse_arg_add()?;
            let loc = lhs.loc.merge(rhs.loc);
            let kind = NodeKind::BinOp(BinOp::Sub, Box::new(lhs), Box::new(rhs));
            Ok(Node { kind, loc })
        } else {
            Ok(lhs)
        }
    }

    pub fn parse_arg_mul(&mut self) -> Result<Node, ParseError> {
        let lhs = self.parse_primary()?;
        if self.get_if_punct(Punct::Mul) {
            let rhs = self.parse_arg_mul()?;
            let loc = lhs.loc.merge(rhs.loc);
            let kind = NodeKind::BinOp(BinOp::Mul, Box::new(lhs), Box::new(rhs));
            Ok(Node { kind, loc })
        } else {
            Ok(lhs)
        }
    }

    fn parse_primary(&mut self) -> Result<Node, ParseError> {
        let tok = self.get().clone();
        match &tok.kind {
            TokenKind::NumLit(num) => Ok(Node::new_number(*num, tok.loc())),
            TokenKind::Punct(punct) if *punct == Punct::LParen => {
                let node = self.parse_expr()?;
                let tok = self.get();
                if tok.kind == TokenKind::Punct(Punct::RParen) {
                    Ok(node)
                } else {
                    Err(ParseError::UnexpectedToken)
                }
            }
            TokenKind::Reserved(Reserved::If) => {
                let cond = self.parse_expr()?;
                println!("if cond {}", cond);
                self.parse_then()?;
                let then = self.parse_comp_stmt()?;
                println!("if then {}", then);
                self.expect_reserved(Reserved::End)?;
                let loc = tok.loc().merge(then.loc);
                Ok(Node {
                    kind: NodeKind::If(Box::new(cond), Box::new(then)),
                    loc,
                })
            }
            TokenKind::EOF => {
                return Err(ParseError::EOF);
            }
            _ => unimplemented!("{:?}", tok.kind),
        }
    }

    fn parse_then(&mut self) -> Result<(), ParseError> {
        if self.get_if_term() {
            return Ok(());
        }
        self.expect_reserved(Reserved::Then)?;
        Ok(())
    }
}

#[cfg(test)]
#[allow(unused_imports, dead_code)]
mod test {
    use crate::eval::eval_node;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::value::Value;

    fn eval_script(script: impl Into<String>, expected: Value) {
        let lexer = Lexer::new(script);
        let result = lexer.tokenize().unwrap();
        let mut parser = Parser::new(result);
        let node = parser.parse_comp_stmt().unwrap();
        let res = eval_node(&node);
        if res != expected {
            panic!("Expected:{:?} Got:{:?}", expected, res);
        }
    }

    #[test]
    fn if1() {
        let program = "if 5*4==16 +4 then 7; end";
        let expected = Value::FixNum(7);
        eval_script(program, expected);
    }

    #[test]
    fn if2() {
        let program = "if 
        5*4==16 +4
        7 end";
        let expected = Value::FixNum(7);
        eval_script(program, expected);
    }
}
