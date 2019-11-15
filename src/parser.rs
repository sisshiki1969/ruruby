use crate::error::{ParseErrKind, RubyError};
use crate::lexer::Lexer;
use crate::node::*;
use crate::token::*;
use crate::util::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Parser {
    pub lexer: Lexer,
    tokens: Vec<Token>,
    cursor: usize,
    prev_cursor: usize,
    context_stack: Vec<Context>,
    pub ident_table: IdentifierTable,
    lvar_collector: Vec<LvarCollector>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseResult {
    pub node: Node,
    pub ident_table: IdentifierTable,
    pub lvar_collector: LvarCollector,
    pub source_info: SourceInfo,
}

impl ParseResult {
    pub fn default(node: Node, lvar_collector: LvarCollector) -> Self {
        ParseResult {
            node,
            ident_table: IdentifierTable::new(),
            lvar_collector,
            source_info: SourceInfo::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LvarId(usize);

impl std::ops::Deref for LvarId {
    type Target = usize;
    fn deref(&self) -> &usize {
        &self.0
    }
}

impl std::hash::Hash for LvarId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl LvarId {
    pub fn as_usize(&self) -> usize {
        self.0
    }

    pub fn from_usize(id: usize) -> Self {
        LvarId(id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LvarCollector {
    id: usize,
    pub table: HashMap<IdentId, LvarId>,
}

impl LvarCollector {
    pub fn new() -> Self {
        LvarCollector {
            id: 0,
            table: HashMap::new(),
        }
    }

    fn insert(&mut self, val: IdentId) -> LvarId {
        match self.table.get(&val) {
            Some(id) => *id,
            None => {
                let id = self.id;
                self.table.insert(val, LvarId(id));
                self.id += 1;
                LvarId(id)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Context {
    Class,
    Method,
}

impl Parser {
    pub fn new() -> Self {
        let lexer = Lexer::new();
        Parser {
            lexer,
            tokens: vec![],
            cursor: 0,
            prev_cursor: 0,
            context_stack: vec![Context::Class],
            ident_table: IdentifierTable::new(),
            lvar_collector: vec![],
        }
    }

    pub fn get_context_depth(&self) -> usize {
        self.context_stack.len()
    }

    fn add_local_var(&mut self, id: IdentId) {
        self.lvar_collector.last_mut().unwrap().insert(id);
    }

    fn get_ident_id(&mut self, method: &String) -> IdentId {
        self.ident_table.get_ident_id(&method)
    }

    pub fn show_tokens(&self) {
        for tok in &self.tokens {
            println!("{:?}", tok);
        }
    }

    /// Peek next token (skipping line terminators).
    fn peek(&self) -> &Token {
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
    fn peek_no_skip_line_term(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    /// Examine the next token, and return true if it is a line terminator.
    fn is_line_term(&self) -> bool {
        self.peek_no_skip_line_term().is_line_term()
    }

    fn loc(&self) -> Loc {
        self.tokens[self.cursor].loc()
    }

    fn prev_loc(&self) -> Loc {
        self.tokens[self.prev_cursor].loc()
    }

    /// Get next token (skipping line terminators).
    /// Return RubyError if it was EOF.
    fn get(&mut self) -> Result<&Token, RubyError> {
        loop {
            let token = &self.tokens[self.cursor];
            if token.is_eof() {
                return Err(self.error_eof(token.loc()));
            }
            self.prev_cursor = self.cursor;
            self.cursor += 1;
            if !token.is_line_term() {
                return Ok(token);
            }
        }
    }

    /// Get next token (no skipping line terminators).
    fn get_no_skip_line_term(&mut self) -> Token {
        let token = self.tokens[self.cursor].clone();
        if !token.is_eof() {
            self.prev_cursor = self.cursor;
            self.cursor += 1;
        }
        token
    }

    /// If next token is an expected kind of Punctuator, get it and return true.
    /// Otherwise, return false.
    fn consume_punct(&mut self, expect: Punct) -> bool {
        match &self.peek().kind {
            TokenKind::Punct(punct) if *punct == expect => {
                let _ = self.get();
                true
            }
            _ => false,
        }
    }

    /// If next token is an expected kind of Reserved keyeord, get it and return true.
    /// Otherwise, return false.
    fn consume_reserved(&mut self, expect: Reserved) -> bool {
        match &self.peek().kind {
            TokenKind::Reserved(reserved) if *reserved == expect => {
                let _ = self.get();
                true
            }
            _ => false,
        }
    }

    /// Get the next token if it is a line terminator or ';' or EOF, and return true,
    /// Otherwise, return false.
    fn consume_term(&mut self) -> bool {
        if self.peek_no_skip_line_term().is_term() {
            self.get_no_skip_line_term();
            true
        } else {
            false
        }
    }

    /// Get the next token and examine whether it is an expected Reserved.
    /// If not, return RubyError.
    fn expect_reserved(&mut self, expect: Reserved) -> Result<(), RubyError> {
        match &self.get()?.kind {
            TokenKind::Reserved(reserved) if *reserved == expect => Ok(()),
            _ => Err(self.error_unexpected(self.prev_loc(), format!("Expect {:?}", expect))),
        }
    }

    /// Get the next token and examine whether it is an expected Punct.
    /// If not, return RubyError.
    fn expect_punct(&mut self, expect: Punct) -> Result<(), RubyError> {
        match &self.get()?.kind {
            TokenKind::Punct(punct) if *punct == expect => Ok(()),
            _ => Err(self.error_unexpected(self.prev_loc(), format!("Expect '{:?}'", expect))),
        }
    }

    /// Get the next token and examine whether it is Ident.
    /// Return IdentId of the Ident.
    /// If not, return RubyError.
    fn expect_ident(&mut self) -> Result<IdentId, RubyError> {
        let name = match &self.get()?.kind {
            TokenKind::Ident(s) => s.clone(),
            _ => {
                return Err(self.error_unexpected(self.prev_loc(), "Expect identifier."));
            }
        };
        Ok(self.get_ident_id(&name))
    }

    fn error_unexpected(&self, loc: Loc, msg: impl Into<String>) -> RubyError {
        RubyError::new_parse_err(ParseErrKind::SyntaxError(msg.into()), loc)
    }

    fn error_eof(&self, loc: Loc) -> RubyError {
        RubyError::new_parse_err(ParseErrKind::UnexpectedEOF, loc)
    }

    pub fn show_loc(&self, loc: &Loc) {
        self.lexer.source_info.show_loc(&loc)
    }
}

impl Parser {
    pub fn parse_program(
        &mut self,
        program: String,
        lvar_collector: Option<LvarCollector>,
    ) -> Result<ParseResult, RubyError> {
        //println!("{:?}", program);
        self.tokens = self.lexer.tokenize(program.clone())?.tokens;
        self.cursor = 0;
        self.prev_cursor = 0;
        self.lvar_collector
            .push(lvar_collector.unwrap_or(LvarCollector::new()));
        let node = self.parse_comp_stmt()?;
        let lvar = self.lvar_collector.pop().unwrap();
        #[cfg(feature = "trace_iseq")]
        {
            let list = lvar
                .table
                .clone()
                .into_iter()
                .collect::<Vec<(IdentId, LvarId)>>();
            for l in list {
                let name = self.get_ident_name(l.0);
                eprintln!("({}, {:?})", name, l.1);
            }
        }
        let tok = self.peek();
        if tok.kind == TokenKind::EOF {
            let mut result = ParseResult::default(node, lvar);
            std::mem::swap(&mut result.ident_table, &mut self.ident_table);
            std::mem::swap(&mut result.source_info, &mut self.lexer.source_info);
            Ok(result)
        } else {
            Err(self.error_unexpected(tok.loc(), "Expected end-of-input."))
        }
    }

    fn parse_comp_stmt(&mut self) -> Result<Node, RubyError> {
        // STMT (TERM EXPR)* [TERM]

        fn return_comp_stmt(nodes: Vec<Node>, mut loc: Loc) -> Result<Node, RubyError> {
            if let Some(node) = nodes.last() {
                loc = loc.merge(node.loc());
            };
            Ok(Node::new(NodeKind::CompStmt(nodes), loc))
        }

        let loc = self.loc();
        let mut nodes = vec![];

        loop {
            match self.peek().kind {
                TokenKind::EOF
                | TokenKind::IntermediateDoubleQuote(_)
                | TokenKind::CloseDoubleQuote(_) => return return_comp_stmt(nodes, loc),
                TokenKind::Reserved(reserved) => match reserved {
                    Reserved::Else | Reserved::Elsif | Reserved::End => {
                        return return_comp_stmt(nodes, loc);
                    }
                    _ => {}
                },
                _ => {}
            };
            let node = self.parse_expr()?;
            nodes.push(node);
            if !self.consume_term() {
                break;
            }
        }

        return_comp_stmt(nodes, loc)
    }
    /*
        fn parse_stmt(&mut self) -> Result<Node, RubyError> {
            self.parse_expr()
        }
    */
    fn parse_expr(&mut self) -> Result<Node, RubyError> {
        self.parse_arg()
    }

    fn parse_arg(&mut self) -> Result<Node, RubyError> {
        self.parse_arg_assign()
    }

    fn parse_arg_assign(&mut self) -> Result<Node, RubyError> {
        let lhs = self.parse_arg_ternary()?;
        if self.is_line_term() {
            return Ok(lhs);
        }
        if self.consume_punct(Punct::Assign) {
            let rhs = self.parse_arg()?;
            match lhs.kind {
                NodeKind::Ident(id) => {
                    self.add_local_var(id);
                }
                _ => {}
            };
            Ok(Node::new_assign(lhs, rhs))
        } else {
            Ok(lhs)
        }
    }

    fn parse_arg_ternary(&mut self) -> Result<Node, RubyError> {
        let loc = self.prev_loc();
        let cond = self.parse_arg_range()?;
        if self.consume_punct(Punct::Question) {
            let then_ = self.parse_arg_ternary()?;
            self.expect_punct(Punct::Colon)?;
            let else_ = self.parse_arg_ternary()?;
            let loc = loc.merge(else_.loc());
            let node = Node::new(
                NodeKind::If(Box::new(cond), Box::new(then_), Box::new(else_)),
                loc,
            );
            Ok(node)
        } else {
            Ok(cond)
        }
    }

    fn parse_arg_range(&mut self) -> Result<Node, RubyError> {
        let lhs = self.parse_arg_logical_or()?;
        if self.is_line_term() {
            return Ok(lhs);
        }
        if self.consume_punct(Punct::Range2) {
            let rhs = self.parse_arg_logical_or()?;
            let loc = lhs.loc().merge(rhs.loc());
            Ok(Node::new_range(lhs, rhs, false, loc))
        } else if self.consume_punct(Punct::Range3) {
            let rhs = self.parse_arg_logical_or()?;
            let loc = lhs.loc().merge(rhs.loc());
            Ok(Node::new_range(lhs, rhs, true, loc))
        } else {
            Ok(lhs)
        }
    }

    fn parse_arg_logical_or(&mut self) -> Result<Node, RubyError> {
        let lhs = self.parse_arg_logical_and()?;
        if self.is_line_term() {
            return Ok(lhs);
        }
        if self.consume_punct(Punct::LOr) {
            let rhs = self.parse_arg_logical_or()?;
            Ok(Node::new_binop(BinOp::LOr, lhs, rhs))
        } else {
            Ok(lhs)
        }
    }

    fn parse_arg_logical_and(&mut self) -> Result<Node, RubyError> {
        let lhs = self.parse_arg_eq()?;
        if self.is_line_term() {
            return Ok(lhs);
        }
        if self.consume_punct(Punct::LAnd) {
            let rhs = self.parse_arg_logical_and()?;
            Ok(Node::new_binop(BinOp::LAnd, lhs, rhs))
        } else {
            Ok(lhs)
        }
    }

    // 4==4==4 => SyntaxError
    fn parse_arg_eq(&mut self) -> Result<Node, RubyError> {
        let lhs = self.parse_arg_comp()?;
        if self.is_line_term() {
            return Ok(lhs);
        }
        if self.consume_punct(Punct::Eq) {
            let rhs = self.parse_arg_eq()?;
            Ok(Node::new_binop(BinOp::Eq, lhs, rhs))
        } else if self.consume_punct(Punct::Ne) {
            let rhs = self.parse_arg_eq()?;
            Ok(Node::new_binop(BinOp::Ne, lhs, rhs))
        } else {
            Ok(lhs)
        }
    }

    fn parse_arg_comp(&mut self) -> Result<Node, RubyError> {
        let mut lhs = self.parse_arg_bitor()?;
        if self.is_line_term() {
            return Ok(lhs);
        }
        loop {
            if self.consume_punct(Punct::Ge) {
                let rhs = self.parse_arg_bitor()?;
                lhs = Node::new_binop(BinOp::Ge, lhs, rhs);
            } else if self.consume_punct(Punct::Gt) {
                let rhs = self.parse_arg_bitor()?;
                lhs = Node::new_binop(BinOp::Gt, lhs, rhs);
            } else if self.consume_punct(Punct::Le) {
                let rhs = self.parse_arg_bitor()?;
                lhs = Node::new_binop(BinOp::Le, lhs, rhs);
            } else if self.consume_punct(Punct::Lt) {
                let rhs = self.parse_arg_bitor()?;
                lhs = Node::new_binop(BinOp::Lt, lhs, rhs);
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_arg_bitor(&mut self) -> Result<Node, RubyError> {
        let mut lhs = self.parse_arg_bitand()?;
        if self.is_line_term() {
            return Ok(lhs);
        }
        loop {
            if self.consume_punct(Punct::BitOr) {
                lhs = Node::new_binop(BinOp::BitOr, lhs, self.parse_arg_bitand()?);
            } else if self.consume_punct(Punct::BitXor) {
                lhs = Node::new_binop(BinOp::BitXor, lhs, self.parse_arg_bitand()?);
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_arg_bitand(&mut self) -> Result<Node, RubyError> {
        let mut lhs = self.parse_arg_shift()?;
        if self.is_line_term() {
            return Ok(lhs);
        }
        loop {
            if self.consume_punct(Punct::BitAnd) {
                lhs = Node::new_binop(BinOp::BitAnd, lhs, self.parse_arg_shift()?);
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_arg_shift(&mut self) -> Result<Node, RubyError> {
        let mut lhs = self.parse_arg_add()?;
        if self.is_line_term() {
            return Ok(lhs);
        }
        loop {
            if self.consume_punct(Punct::Shl) {
                lhs = Node::new_binop(BinOp::Shl, lhs, self.parse_arg_add()?);
            } else if self.consume_punct(Punct::Shr) {
                lhs = Node::new_binop(BinOp::Shr, lhs, self.parse_arg_add()?);
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_arg_add(&mut self) -> Result<Node, RubyError> {
        let mut lhs = self.parse_arg_mul()?;
        if self.is_line_term() {
            return Ok(lhs);
        }
        loop {
            if self.consume_punct(Punct::Plus) {
                let rhs = self.parse_arg_mul()?;
                lhs = Node::new_binop(BinOp::Add, lhs, rhs);
            } else if self.consume_punct(Punct::Minus) {
                let rhs = self.parse_arg_mul()?;
                lhs = Node::new_binop(BinOp::Sub, lhs, rhs);
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_arg_mul(&mut self) -> Result<Node, RubyError> {
        let mut lhs = self.parse_unary_minus()?;
        if self.is_line_term() {
            return Ok(lhs);
        }
        loop {
            if self.consume_punct(Punct::Mul) {
                let rhs = self.parse_unary_minus()?;
                lhs = Node::new_binop(BinOp::Mul, lhs, rhs);
            } else if self.consume_punct(Punct::Div) {
                let rhs = self.parse_unary_minus()?;
                lhs = Node::new_binop(BinOp::Div, lhs, rhs);
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_unary_minus(&mut self) -> Result<Node, RubyError> {
        let loc = self.loc();
        if self.consume_punct(Punct::Minus) {
            let lhs = self.parse_unary_minus()?;
            let loc = loc.merge(lhs.loc());
            let lhs = Node::new_binop(BinOp::Mul, lhs, Node::new_number(-1, loc));
            Ok(lhs)
        } else {
            let lhs = self.parse_unary_bitnot()?;
            Ok(lhs)
        }
    }

    fn parse_unary_bitnot(&mut self) -> Result<Node, RubyError> {
        let loc = self.loc();
        if self.consume_punct(Punct::BitNot) {
            let lhs = self.parse_unary_bitnot()?;
            let lhs = Node::new_unop(UnOp::BitNot, lhs, loc);
            Ok(lhs)
        } else {
            let lhs = self.parse_primary_ext()?;
            Ok(lhs)
        }
    }

    fn parse_primary_ext(&mut self) -> Result<Node, RubyError> {
        // FUNCTION : OPERATION [`(' [CALL_ARGS] `)']
        //        | PRIMARY `.' FNAME `(' [CALL_ARGS] `)'
        //        | PRIMARY `::' FNAME `(' [CALL_ARGS] `)'
        //        | PRIMARY `.' FNAME
        //        | PRIMARY `::' FNAME
        //        | super [`(' [CALL_ARGS] `)']
        let loc = self.loc();
        let mut node = self.parse_primary()?;
        if self.peek_no_skip_line_term().kind == TokenKind::Punct(Punct::LParen) {
            // OPERATION `(' [CALL_ARGS] `)'
            self.get()?;
            let args = self.parse_parenthesize_args()?;
            let end_loc = self.loc();

            return Ok(Node::new_send(
                Node::new(NodeKind::SelfValue, loc),
                node,
                args,
                loc.merge(end_loc),
            ));
        };
        loop {
            let tok = self.peek_no_skip_line_term();
            node = match tok.kind {
                TokenKind::Punct(Punct::Dot) => {
                    // PRIMARY `.' FNAME `(' [CALL_ARGS] `)'
                    // PRIMARY `.' FNAME
                    self.get()?;
                    let tok = self.get()?.clone();
                    let method = match &tok.kind {
                        TokenKind::Ident(s) => s,
                        _ => {
                            return Err(self
                                .error_unexpected(tok.loc(), "method name must be an identifier."))
                        }
                    };
                    let id = self.get_ident_id(&method);
                    let mut args = vec![];
                    if self.peek_no_skip_line_term().kind == TokenKind::Punct(Punct::LParen) {
                        self.get()?;
                        args = self.parse_parenthesize_args()?;
                    }
                    Node::new_send(
                        node,
                        Node::new_identifier(id, tok.loc()),
                        args,
                        loc.merge(self.loc()),
                    )
                }
                _ => return Ok(node),
            }
        }
    }

    fn parse_parenthesize_args(&mut self) -> Result<Vec<Node>, RubyError> {
        let mut args = vec![];
        if self.consume_punct(Punct::RParen) {
            return Ok(args);
        }
        loop {
            args.push(self.parse_arg()?);
            if !self.consume_punct(Punct::Comma) {
                break;
            }
        }
        self.expect_punct(Punct::RParen)?;
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Node, RubyError> {
        let tok = self.get()?.clone();
        let loc = tok.loc();
        match &tok.kind {
            TokenKind::Ident(name) => {
                let id = self.get_ident_id(name);
                if name == "self" {
                    return Ok(Node::new(NodeKind::SelfValue, loc));
                };
                return Ok(Node::new_identifier(id, loc));
            }
            TokenKind::InstanceVar(name) => {
                let id = self.get_ident_id(name);
                return Ok(Node::new_instance_var(id, loc));
            }
            TokenKind::Const(name) => {
                let id = self.get_ident_id(name);
                Ok(Node::new_const(id, loc))
            }
            TokenKind::NumLit(num) => Ok(Node::new_number(*num, loc)),
            TokenKind::FloatLit(num) => Ok(Node::new_float(*num, loc)),
            TokenKind::StringLit(s) => Ok(self.parse_string_literal(s)?),
            TokenKind::OpenDoubleQuote(s) => Ok(self.parse_interporated_string_literal(s)?),
            TokenKind::Punct(punct) if *punct == Punct::LParen => {
                let node = self.parse_comp_stmt()?;
                self.expect_punct(Punct::RParen)?;
                Ok(node)
            }
            TokenKind::Reserved(Reserved::If) => {
                let node = self.parse_if_then()?;
                self.expect_reserved(Reserved::End)?;
                Ok(node)
            }
            TokenKind::Reserved(Reserved::For) => {
                let loc = self.prev_loc();
                let var = self.expect_ident()?;
                let var = Node::new_identifier(var, self.prev_loc());
                if let NodeKind::Ident(id) = var.kind {
                    self.add_local_var(id);
                }
                self.expect_reserved(Reserved::In)?;
                let iter = self.parse_expr()?;
                self.parse_do()?;
                let body = self.parse_comp_stmt()?;
                self.expect_reserved(Reserved::End)?;
                let node = Node::new(
                    NodeKind::For(Box::new(var), Box::new(iter), Box::new(body)),
                    loc.merge(self.prev_loc()),
                );
                Ok(node)
            }
            TokenKind::Reserved(Reserved::Def) => {
                self.context_stack.push(Context::Method);
                let node = self.parse_def()?;
                self.context_stack.pop();
                Ok(node)
            }
            TokenKind::Reserved(Reserved::Class) => {
                if *self.context_stack.last().unwrap_or_else(|| panic!()) == Context::Method {
                    return Err(
                        self.error_unexpected(loc, "SyntaxError: class definition in method body.")
                    );
                }
                self.context_stack.push(Context::Class);
                let node = self.parse_class()?;
                self.context_stack.pop();
                Ok(node)
            }
            TokenKind::Reserved(Reserved::Break) => Ok(Node::new_break(loc)),
            TokenKind::Reserved(Reserved::Next) => Ok(Node::new_next(loc)),
            TokenKind::Reserved(Reserved::True) => Ok(Node::new_bool(true, loc)),
            TokenKind::Reserved(Reserved::False) => Ok(Node::new_bool(false, loc)),
            TokenKind::Reserved(Reserved::Nil) => Ok(Node::new_nil(loc)),
            TokenKind::EOF => {
                return Err(self.error_eof(loc));
            }
            _ => {
                return Err(self.error_unexpected(loc, format!("Unexpected token: {:?}", tok.kind)))
            }
        }
    }

    fn parse_string_literal(&mut self, s: &String) -> Result<Node, RubyError> {
        let loc = self.prev_loc();
        let mut s = s.clone();
        while let TokenKind::StringLit(next_s) = &self.peek_no_skip_line_term().clone().kind {
            self.get()?;
            s = format!("{}{}", s, next_s);
        }
        Ok(Node::new_string(s, loc))
    }

    fn parse_interporated_string_literal(&mut self, s: &String) -> Result<Node, RubyError> {
        let start_loc = self.prev_loc();
        let mut nodes = vec![Node::new_string(s.clone(), start_loc)];
        loop {
            match &self.peek().kind {
                TokenKind::CloseDoubleQuote(s) => {
                    let end_loc = self.loc();
                    nodes.push(Node::new_string(s.clone(), end_loc));
                    self.get()?;
                    return Ok(Node::new_interporated_string(
                        nodes,
                        start_loc.merge(end_loc),
                    ));
                }
                TokenKind::IntermediateDoubleQuote(s) => {
                    nodes.push(Node::new_string(s.clone(), self.loc()));
                    self.get()?;
                }
                TokenKind::OpenDoubleQuote(s) => {
                    let s = s.clone();
                    self.get()?;
                    self.parse_interporated_string_literal(&s)?;
                }
                TokenKind::EOF => {
                    return Err(self.error_unexpected(self.loc(), "Unexpectd EOF."));
                }
                _ => {
                    nodes.push(self.parse_comp_stmt()?);
                }
            }
        }
    }

    fn parse_if_then(&mut self) -> Result<Node, RubyError> {
        //  if EXPR THEN
        //      COMPSTMT
        //      (elsif EXPR THEN COMPSTMT)*
        //      [else COMPSTMT]
        //  end
        let mut loc = self.prev_loc();
        let cond = self.parse_expr()?;
        self.parse_then()?;
        let then_ = self.parse_comp_stmt()?;
        let mut else_ = Node::new_comp_stmt(self.loc());
        if self.consume_reserved(Reserved::Elsif) {
            else_ = self.parse_if_then()?;
        } else if self.consume_reserved(Reserved::Else) {
            else_ = self.parse_comp_stmt()?;
        }
        loc = loc.merge(else_.loc());
        Ok(Node::new(
            NodeKind::If(Box::new(cond), Box::new(then_), Box::new(else_)),
            loc,
        ))
    }

    fn parse_then(&mut self) -> Result<(), RubyError> {
        if self.consume_term() {
            self.consume_reserved(Reserved::Then);
            return Ok(());
        }
        self.expect_reserved(Reserved::Then)?;
        Ok(())
    }

    fn parse_do(&mut self) -> Result<(), RubyError> {
        if self.consume_term() {
            self.consume_reserved(Reserved::Do);
            return Ok(());
        }
        self.expect_reserved(Reserved::Do)?;
        Ok(())
    }

    fn parse_def(&mut self) -> Result<Node, RubyError> {
        //  def FNAME ARGDECL
        //      COMPSTMT
        //      [rescue [ARGS] [`=>' LHS] THEN COMPSTMT]+
        //      [else COMPSTMT]
        //      [ensure COMPSTMT]
        //  end
        let mut is_class_method = false;
        let self_id = self.get_ident_id(&"self".to_string());
        let mut id = match self.peek().kind {
            TokenKind::Ident(_) => self.expect_ident()?,
            TokenKind::Punct(Punct::Plus) => {
                self.get()?;
                self.get_ident_id(&"@add".to_string())
            }
            TokenKind::Punct(Punct::Minus) => {
                self.get()?;
                self.get_ident_id(&"@sub".to_string())
            }
            TokenKind::Punct(Punct::Mul) => {
                self.get()?;
                self.get_ident_id(&"@mul".to_string())
            }
            _ => return Err(self.error_unexpected(self.loc(), "Expected identifier or operator.")),
        };
        if id == self_id {
            is_class_method = true;
            self.expect_punct(Punct::Dot)?;
            id = self.expect_ident()?;
        };
        self.lvar_collector.push(LvarCollector::new());
        let args = self.parse_params()?;
        let body = self.parse_comp_stmt()?;
        self.expect_reserved(Reserved::End)?;
        let lvar = self.lvar_collector.pop().unwrap();
        if is_class_method {
            Ok(Node::new_class_method_decl(id, args, body, lvar))
        } else {
            Ok(Node::new_method_decl(id, args, body, lvar))
        }
    }

    fn parse_params(&mut self) -> Result<Vec<Node>, RubyError> {
        if self.consume_term() {
            return Ok(vec![]);
        };
        self.expect_punct(Punct::LParen)?;
        let mut args = vec![];
        if self.consume_punct(Punct::RParen) {
            if !self.consume_term() {
                return Err(self.error_unexpected(self.loc(), "Expect terminator"));
            }
            return Ok(args);
        }
        loop {
            let id = self.expect_ident()?;
            args.push(Node::new(NodeKind::Param(id), self.prev_loc()));
            self.add_local_var(id);
            if !self.consume_punct(Punct::Comma) {
                break;
            }
        }
        self.expect_punct(Punct::RParen)?;
        if !self.consume_term() {
            return Err(self.error_unexpected(self.loc(), "Expect terminator."));
        }
        Ok(args)
    }

    fn parse_class(&mut self) -> Result<Node, RubyError> {
        //  class identifier [`<' identifier]
        //      COMPSTMT
        //  end
        let loc = self.loc();
        let name = match &self.get_no_skip_line_term().kind {
            TokenKind::Const(s) => s.clone(),
            _ => return Err(self.error_unexpected(loc.dec(), "Expect class name.")),
        };
        let id = self.get_ident_id(&name);
        self.lvar_collector.push(LvarCollector::new());
        let body = self.parse_comp_stmt()?;
        self.expect_reserved(Reserved::End)?;
        let lvar = self.lvar_collector.pop().unwrap();
        Ok(Node::new_class_decl(id, body, lvar))
    }
}

#[cfg(test)]
#[allow(unused_imports, dead_code)]
mod eval_tests {
    extern crate test;

    use crate::eval::value::Value;
    use crate::eval::Evaluator;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use test::Bencher;

    fn eval_script(script: impl Into<String>, expected: Value) {
        let mut parser = Parser::new();
        let result = match parser.parse_program(script.into(), None) {
            Ok(result) => result,
            Err(err) => {
                parser.lexer.source_info.show_loc(&err.loc());
                panic!("Got parse error: {:?}", err);
            }
        };
        let mut eval = Evaluator::new(result.source_info, result.ident_table);
        match eval.eval_node(&result.node) {
            Ok(res) => {
                if res != expected {
                    panic!("Expected:{:?} Got:{:?}", expected, res);
                }
            }
            Err(err) => {
                eval.source_info.show_loc(&err.loc());
                panic!("Got runtime error: {:?}", err);
            }
        }
    }

    #[test]
    fn bool_lit1() {
        let program = "(3==3)==true";
        let expected = Value::Bool(true);
        eval_script(program, expected);
    }

    #[test]
    fn bool_lit2() {
        let program = "(3==9)==false";
        let expected = Value::Bool(true);
        eval_script(program, expected);
    }

    #[test]
    fn nil_lit1() {
        let program = "nil";
        let expected = Value::Nil;
        eval_script(program, expected);
    }

    #[test]
    fn expr1() {
        let program = "4*(4+7*3)-95";
        let expected = Value::FixNum(5);
        eval_script(program, expected);
    }

    #[test]
    fn expr2() {
        let program = "2.0 + 4.0";
        let expected = Value::FloatNum(6.0);
        eval_script(program, expected);
    }

    #[test]
    fn expr3() {
        let program = "5.0 / 2";
        let expected = Value::FloatNum(2.5);
        eval_script(program, expected);
    }

    #[test]
    fn expr4() {
        let program = "15<<30";
        let expected = Value::FixNum(16106127360);
        eval_script(program, expected);
    }

    #[test]
    fn expr5() {
        let program = "23456>>3";
        let expected = Value::FixNum(2932);
        eval_script(program, expected);
    }

    #[test]
    fn expr6() {
        let program = "24+17 >> 3 == 5";
        let expected = Value::Bool(true);
        eval_script(program, expected);
    }
    #[test]
    fn expr7() {
        let program = "864 == 3+24<<5";
        let expected = Value::Bool(true);
        eval_script(program, expected);
    }

    #[test]
    fn expr8() {
        let program = "
        assert(320, 12745&854)
        assert(100799, 2486|98331)
        assert(1033, 8227^9258)
        assert(201, -275&475)
        assert(-1301, 487555|-25879)
        ";
        let expected = Value::Nil;
        eval_script(program, expected);
    }

    #[test]
    fn expr9() {
        let program = "
        a=19
        a==17?23*45:14+7
        ";
        let expected = Value::FixNum(21);
        eval_script(program, expected);
    }

    #[test]
    fn op1() {
        let program = "4==5";
        let expected = Value::Bool(false);
        eval_script(program, expected);
    }

    #[test]
    fn op2() {
        let program = "4!=5";
        let expected = Value::Bool(true);
        eval_script(program, expected);
    }

    #[test]
    fn op9() {
        let program = "4!=4 || 1==1 && 2==3";
        let expected = Value::Bool(false);
        eval_script(program, expected);
    }

    #[test]
    fn op10() {
        let program = "4==4 && 4!=5 && 3<4 && 5>4 && 4<=4 && 4>=4";
        let expected = Value::Bool(true);
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
    fn for1() {
        let program = "
            y = 0
            for x in 0..9
            y=y+x
            end
            y";
        let expected = Value::FixNum(45);
        eval_script(program, expected);
    }

    #[test]
    fn for2() {
        let program = "
            y = 0
            for x in 0..9
            if x == 5 then break end
            y=y+x
            end
            y";
        let expected = Value::FixNum(10);
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

    #[bench]
    fn func3(b: &mut Bencher) {
        let program = "
            def fibo(x)
                if x <= 2
                    1
                else
                    fibo(x-1) + fibo(x-2)
                end
            end

            fibo(20)";
        let expected = Value::FixNum(6765);
        b.iter(|| eval_script(program, expected.clone()));
    }

    #[test]
    fn local_scope() {
        let program = "
        a = 1
        class Foo
            a = 2
            def bar
                a = 3
                a
            end
            def boo(x)
                x * 2
            end
            assert(2,a)
        end
        assert(1,a)
        assert(3,Foo.new.bar)
        assert(10,Foo.new.boo(5))
        assert(1,a)";
        let expected = Value::Nil;
        eval_script(program, expected);
    }

    #[test]
    fn class1() {
        let program = "
        class Vec
            assert(Vec, self)
            def len(x,y)
                def sq(x)
                    x*x
                end
                sq(x)+sq(y)
            end
        end

        Vec.new.len(3,4)";
        let expected = Value::FixNum(25);
        eval_script(program, expected);
    }

    #[test]
    fn eval_class2() {
        let program = "
        class Vec
            @xxx=100
            def set_xxx(x); @xxx = x; end
            def len(x,y)
                def sq(x); x*x; end
                sq(x)+sq(y)
            end
            def get_xxx; @xxx; end
            def self.get_xxx; @xxx = @xxx + 1; @xxx; end
        end

        foo1 = Vec.new
        assert(25, foo1.len(3,4))
        foo1.set_xxx(777)
        assert(777, foo1.get_xxx)
        foo2 = Vec.new
        foo2.set_xxx(999)
        assert(777, foo1.get_xxx)
        assert(999, foo2.get_xxx)
        assert(nil, Vec.new.get_xxx)
        assert(101, Vec.get_xxx)
        assert(102, Vec.get_xxx)
        assert(103, Vec.get_xxx)";
        let expected = Value::Nil;
        eval_script(program, expected);
    }
}
