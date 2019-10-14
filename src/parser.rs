use crate::lexer::*;
use crate::node::*;
use crate::token::*;
use crate::util::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
    pub source_info: SourceInfo,
    pub ident_table: IdentifierTable,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseErrorKind {
    UnexpectedToken,
    EOF,
}

pub type ParseError = Annot<ParseErrorKind>;

impl Parser {
    pub fn new(result: LexerResult) -> Self {
        Parser {
            tokens: result.tokens,
            cursor: 0,
            source_info: result.source_info,
            ident_table: IdentifierTable::new(),
        }
    }

    /// Peek next token (skipping line terminators).
    fn peek(&self) -> (&Token, Loc) {
        let mut c = self.cursor;
        loop {
            let tok = &self.tokens[c];
            if tok.is_eof() || !tok.is_line_term() {
                return (tok, tok.loc);
            } else {
                c += 1;
            }
        }
    }

    /// Peek next token (no skipping line terminators).
    fn peek_no_skip_line_term(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn is_line_term(&self) -> bool {
        self.peek_no_skip_line_term().is_line_term()
    }

    fn loc(&self) -> Loc {
        self.tokens[self.cursor].loc()
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

    /// If next token is an expected kind of Punctuator, get it and return true.
    /// Otherwise, return false.
    fn get_if_punct(&mut self, expect: Punct) -> bool {
        match &self.peek().0.kind {
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

    /// If next token is an expected kind of Reserved keyeord, get it and return true.
    /// Otherwise, return false.
    fn get_if_reserved(&mut self, expect: Reserved) -> bool {
        match &self.peek().0.kind {
            TokenKind::Reserved(reserved) => {
                if *reserved == expect {
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

    fn expect_reserved(&mut self, expect: Reserved) -> Result<(), ParseError> {
        let tok = self.get().clone();
        let loc = self.loc();
        match &tok.kind {
            TokenKind::Reserved(reserved) => {
                if *reserved == expect {
                    Ok(())
                } else {
                    Err(self.error_unexpected(loc))
                }
            }
            _ => Err(self.error_unexpected(loc)),
        }
    }

    fn error_unexpected(&self, loc: Loc) -> ParseError {
        self.source_info.show_loc(&loc);
        ParseError::new(ParseErrorKind::UnexpectedToken, loc)
    }

    fn error_eof(&self, loc: Loc) -> ParseError {
        self.source_info.show_loc(&loc);
        ParseError::new(ParseErrorKind::EOF, loc)
    }
}

impl Parser {
    pub fn parse_program(&mut self) -> Result<Node, ParseError> {
        self.parse_comp_stmt()
    }

    fn parse_comp_stmt(&mut self) -> Result<Node, ParseError> {
        let mut nodes = vec![];
        let mut loc = self.loc();
        loop {
            let (tok, _) = self.peek();
            match tok.kind {
                TokenKind::EOF => break,
                TokenKind::Reserved(reserved) => match reserved {
                    Reserved::Else | Reserved::Elsif | Reserved::End => break,
                    _ => {}
                },
                _ => {}
            };
            let node = self.parse_expr()?;
            nodes.push(node);
            if !self.get_if_term() {
                break;
            }
            //println!("{:?}", node);
        }
        match nodes.last() {
            Some(node) => loc = loc.merge(node.loc()),
            None => {}
        }
        Ok(Node::new(NodeKind::CompStmt(nodes), loc))
    }

    fn parse_expr(&mut self) -> Result<Node, ParseError> {
        self.parse_arg()
    }

    fn parse_arg(&mut self) -> Result<Node, ParseError> {
        self.parse_arg_assign()
    }

    fn parse_arg_assign(&mut self) -> Result<Node, ParseError> {
        let lhs = self.parse_arg_comp()?;
        if self.is_line_term() {
            return Ok(lhs);
        }
        if self.get_if_punct(Punct::Assign) {
            let rhs = self.parse_arg()?;
            Ok(Node::new_assign(lhs, rhs))
        } else {
            Ok(lhs)
        }
    }

    fn parse_arg_comp(&mut self) -> Result<Node, ParseError> {
        let lhs = self.parse_arg_add()?;
        if self.is_line_term() {
            return Ok(lhs);
        }
        if self.get_if_punct(Punct::Equal) {
            let rhs = self.parse_arg_comp()?;
            Ok(Node::new_binop(BinOp::Eq, lhs, rhs))
        } else {
            Ok(lhs)
        }
    }

    fn parse_arg_add(&mut self) -> Result<Node, ParseError> {
        let lhs = self.parse_arg_mul()?;
        if self.is_line_term() {
            return Ok(lhs);
        }
        if self.get_if_punct(Punct::Plus) {
            let rhs = self.parse_arg_add()?;
            Ok(Node::new_binop(BinOp::Add, lhs, rhs))
        } else if self.get_if_punct(Punct::Minus) {
            let rhs = self.parse_arg_add()?;
            Ok(Node::new_binop(BinOp::Sub, lhs, rhs))
        } else {
            Ok(lhs)
        }
    }

    fn parse_arg_mul(&mut self) -> Result<Node, ParseError> {
        let lhs = self.parse_unary_minus()?;
        if self.is_line_term() {
            return Ok(lhs);
        }
        if self.get_if_punct(Punct::Mul) {
            let rhs = self.parse_arg_mul()?;
            Ok(Node::new_binop(BinOp::Mul, lhs, rhs))
        } else {
            Ok(lhs)
        }
    }

    fn parse_unary_minus(&mut self) -> Result<Node, ParseError> {
        let loc = self.loc();
        if self.get_if_punct(Punct::Minus) {
            let lhs = self.parse_primary()?;
            let loc = loc.merge(lhs.loc());
            let lhs = Node::new_binop(BinOp::Mul, lhs, Node::new_number(-1, loc));
            Ok(lhs)
        } else {
            let lhs = self.parse_primary()?;
            Ok(lhs)
        }
    }

    fn parse_primary(&mut self) -> Result<Node, ParseError> {
        let tok = self.get().clone();
        let loc = tok.loc();
        match &tok.kind {
            TokenKind::Ident(name) => {
                let id = self.ident_table.get_ident_id(name);
                if !self.get_if_punct(Punct::LParen) {
                    if name == "self" {
                        return Ok(Node::new(NodeKind::SelfValue, loc));
                    };
                    return Ok(Node::new_local_var(id, loc));
                }
                let mut args = vec![];
                if self.get_if_punct(Punct::RParen) {
                    return Ok(Node::new_send(id, args, loc));
                }
                loop {
                    args.push(self.parse_arg()?);
                    if !self.get_if_punct(Punct::Comma) {
                        break;
                    }
                }
                let end_loc = self.loc();
                if self.get_if_punct(Punct::RParen) {
                    Ok(Node::new_send(id, args, loc.merge(end_loc)))
                } else {
                    Err(self.error_unexpected(self.loc()))
                }
            }
            TokenKind::Const(name) => {
                let id = self.ident_table.get_ident_id(name);
                Ok(Node::new_const(id, loc))
            }
            TokenKind::NumLit(num) => Ok(Node::new_number(*num, loc)),
            TokenKind::Punct(punct) if *punct == Punct::LParen => {
                let node = self.parse_comp_stmt()?;
                if self.get_if_punct(Punct::RParen) {
                    Ok(node)
                } else {
                    Err(self.error_unexpected(self.loc()))
                }
            }
            TokenKind::Reserved(Reserved::If) => {
                let node = self.parse_if_then()?;
                self.expect_reserved(Reserved::End)?;
                Ok(node)
            }
            TokenKind::Reserved(Reserved::Def) => {
                let node = self.parse_def()?;
                Ok(node)
            }
            TokenKind::Reserved(Reserved::Class) => {
                let node = self.parse_class()?;
                Ok(node)
            }
            TokenKind::EOF => {
                return Err(self.error_eof(loc));
            }
            _ => unimplemented!("{:?}", tok.kind),
        }
    }

    fn parse_if_then(&mut self) -> Result<Node, ParseError> {
        //  if EXPR THEN
        //      COMPSTMT
        //      (elsif EXPR THEN COMPSTMT)*
        //      [else COMPSTMT]
        //  end
        let cond = self.parse_expr()?;
        self.parse_then()?;
        let then_ = self.parse_comp_stmt()?;
        let mut else_ = Node::new_comp_stmt(self.loc());
        if self.get_if_reserved(Reserved::Elsif) {
            else_ = self.parse_if_then()?;
        } else if self.get_if_reserved(Reserved::Else) {
            else_ = self.parse_comp_stmt()?;
        }
        let loc = cond.loc().merge(else_.loc());
        Ok(Node::new(
            NodeKind::If(Box::new(cond), Box::new(then_), Box::new(else_)),
            loc,
        ))
    }

    fn parse_then(&mut self) -> Result<(), ParseError> {
        if self.get_if_term() {
            return Ok(());
        }
        self.expect_reserved(Reserved::Then)?;
        Ok(())
    }

    fn parse_def(&mut self) -> Result<Node, ParseError> {
        //  def FNAME ARGDECL
        //      COMPSTMT
        //      [rescue [ARGS] [`=>' LHS] THEN COMPSTMT]+
        //      [else COMPSTMT]
        //      [ensure COMPSTMT]
        //  end
        let loc = self.loc();
        let name = match &self.get().kind {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(self.error_unexpected(loc)),
        };
        let id = self.ident_table.get_ident_id(&name);
        let args = self.parse_params()?;
        let body = self.parse_comp_stmt()?;
        self.expect_reserved(Reserved::End)?;
        Ok(Node::new_method_decl(id, args, body))
    }

    fn parse_params(&mut self) -> Result<Vec<Node>, ParseError> {
        if !self.get_if_punct(Punct::LParen) {
            return Ok(vec![]);
        }
        let mut args = vec![];
        if self.get_if_punct(Punct::RParen) {
            return Ok(args);
        }
        loop {
            let (arg, loc) = match self.get().clone() {
                Token {
                    kind: TokenKind::Ident(s),
                    loc,
                } => (s.clone(), loc),
                Token { loc, .. } => return Err(self.error_unexpected(loc)),
            };
            let id = self.ident_table.get_ident_id(&arg);
            args.push(Node::new(NodeKind::Param(id), loc));
            if !self.get_if_punct(Punct::Comma) {
                break;
            }
        }
        if self.get_if_punct(Punct::RParen) {
            Ok(args)
        } else {
            Err(self.error_unexpected(self.peek().1))
        }
    }

    fn parse_class(&mut self) -> Result<Node, ParseError> {
        //  class identifier [`<' identifier]
        //      COMPSTMT
        //  end
        let loc = self.loc();
        let name = match &self.get().kind {
            TokenKind::Const(s) => s.clone(),
            _ => return Err(self.error_unexpected(loc)),
        };
        let id = self.ident_table.get_ident_id(&name);

        let body = self.parse_comp_stmt()?;
        self.expect_reserved(Reserved::End)?;

        Ok(Node::new_class_decl(id, body))
    }
}

#[cfg(test)]
#[allow(unused_imports, dead_code)]
mod test {
    use crate::eval::Evaluator;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::value::Value;

    fn eval_script(script: impl Into<String>, expected: Value) {
        let lexer = Lexer::new(script);
        let result = lexer.tokenize().unwrap();
        let mut parser = Parser::new(result);
        let node = parser.parse_comp_stmt().unwrap();
        let mut eval = Evaluator::new(parser.source_info, parser.ident_table);
        let res = eval.eval_node(&node);
        if res != expected {
            panic!("Expected:{:?} Got:{:?}", expected, res);
        }
    }

    #[test]
    fn expr1() {
        let program = "4*(4+7*3)-95";
        let expected = Value::FixNum(5);
        eval_script(program, expected);
    }

    #[test]
    fn if1() {
        let program = "if 5*4==16 +4 then 4;2*3+1 end";
        let expected = Value::FixNum(7);
        eval_script(program, expected);
    }

    #[test]
    fn if2() {
        let program = "if 
        5*4 ==16 +
        4
        3*3
        -2 end";
        let expected = Value::FixNum(-2);
        eval_script(program, expected);
    }

    #[test]
    fn if3() {
        let program = "if 5*9==16 +4
        7 elsif 4==4+9 then 8 elsif 3==1+2 then 10
        else 12 end";
        let expected = Value::FixNum(10);
        eval_script(program, expected);
    }

    #[test]
    fn if4() {
        let program = "if
            1+
            2==
            3
            4
            5
            end";
        let expected = Value::FixNum(5);
        eval_script(program, expected);
    }

    #[test]
    fn local_var1() {
        let program = "
            ruby = 7
            mruby = (ruby - 4) * 5
            mruby - ruby";
        let expected = Value::FixNum(8);
        eval_script(program, expected);
    }

    #[test]
    fn func1() {
        let program = "
            def func(a,b,c)
                a+b+c
            end
    
            func(1,2,3)";
        let expected = Value::FixNum(6);
        eval_script(program, expected);
    }

    #[test]
    fn func2() {
        let program = "
            def fact(a)
                puts(a)
                if a == 1
                    1
                else
                    a * fact(a-1)
                end
            end
    
            fact(5)";
        let expected = Value::FixNum(120);
        eval_script(program, expected);
    }

    #[test]
    fn func3() {
        let program = "
            def fibo(x)
                if x == 1
                    1
                elsif x == 2
                    1
                else
                    fibo(x-1) + fibo(x-2)
                end
            end

            fibo(20)";
        let expected = Value::FixNum(6765);
        eval_script(program, expected);
    }
}
