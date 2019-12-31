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
    state_save: Vec<(usize, usize)>,
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

    pub fn as_u32(&self) -> u32 {
        self.0 as u32
    }

    pub fn from_usize(id: usize) -> Self {
        LvarId(id)
    }

    pub fn from_u32(id: u32) -> Self {
        LvarId(id as usize)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LvarCollector {
    id: usize,
    table: HashMap<IdentId, LvarId>,
    block: Option<LvarId>,
}

impl LvarCollector {
    pub fn new() -> Self {
        LvarCollector {
            id: 0,
            table: HashMap::new(),
            block: None,
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

    fn insert_block_param(&mut self, val: IdentId) -> LvarId {
        let lvar = self.insert(val);
        self.block = Some(lvar);
        lvar
    }

    pub fn get(&self, val: &IdentId) -> Option<&LvarId> {
        self.table.get(val)
    }

    pub fn block_param(&self) -> Option<LvarId> {
        self.block
    }

    pub fn len(&self) -> usize {
        self.table.len()
    }

    pub fn clone_table(&self) -> HashMap<IdentId, LvarId> {
        self.table.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Context {
    lvar: LvarCollector,
    kind: ContextKind,
}

impl Context {
    fn new_method() -> Self {
        Context {
            lvar: LvarCollector::new(),
            kind: ContextKind::Method,
        }
    }
    fn new_class(lvar_collector: Option<LvarCollector>) -> Self {
        Context {
            lvar: lvar_collector.unwrap_or(LvarCollector::new()),
            kind: ContextKind::Class,
        }
    }
    fn new_block() -> Self {
        Context {
            lvar: LvarCollector::new(),
            kind: ContextKind::Block,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ContextKind {
    Class,
    Method,
    Block,
}

impl Parser {
    pub fn new() -> Self {
        let lexer = Lexer::new();
        Parser {
            lexer,
            tokens: vec![],
            cursor: 0,
            prev_cursor: 0,
            context_stack: vec![],
            ident_table: IdentifierTable::new(),
            state_save: vec![],
        }
    }

    fn save_state(&mut self) {
        self.state_save.push((self.cursor, self.prev_cursor));
    }

    fn restore_state(&mut self) {
        let state = self.state_save.pop().unwrap();
        self.cursor = state.0;
        self.prev_cursor = state.1;
    }

    fn discard_state(&mut self) {
        self.state_save.pop().unwrap();
    }

    pub fn get_context_depth(&self) -> usize {
        self.context_stack.len()
    }

    // If the identifier(IdentId) does not exist in the current scope,
    // add the identifier as a local variable in the current context.
    fn add_local_var_if_new(&mut self, id: IdentId) {
        if !self.is_local_var(id) {
            self.context_stack.last_mut().unwrap().lvar.insert(id);
        }
    }

    // Add the identifier(IdentId) as a local variable in the current context.
    fn add_local_var(&mut self, id: IdentId) {
        self.context_stack.last_mut().unwrap().lvar.insert(id);
    }

    // Add the identifier(IdentId) as a block parameter in the current context.
    fn add_block_param(&mut self, id: IdentId) {
        self.context_stack
            .last_mut()
            .unwrap()
            .lvar
            .insert_block_param(id);
    }

    // Examine whether the identifier(IdentId) exists in the current scope.
    // If exiets, return true.
    fn is_local_var(&mut self, id: IdentId) -> bool {
        let len = self.context_stack.len();
        for i in 0..len {
            let context = &self.context_stack[len - 1 - i];
            if context.lvar.table.contains_key(&id) {
                return true;
            }
            if context.kind != ContextKind::Block {
                return false;
            }
        }
        return false;
    }

    fn get_ident_id(&mut self, method: &String) -> IdentId {
        self.ident_table.get_ident_id(method)
    }

    pub fn show_tokens(&self) {
        for tok in &self.tokens {
            eprintln!("{:?}", tok);
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

    fn consume_punct_no_skip_line_term(&mut self, expect: Punct) -> bool {
        if TokenKind::Punct(expect) == self.peek_no_skip_line_term().kind {
            let _ = self.get();
            true
        } else {
            false
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

    fn consume_reserved_no_skip_line_term(&mut self, expect: Reserved) -> bool {
        if TokenKind::Reserved(expect) == self.peek_no_skip_line_term().kind {
            let _ = self.get();
            true
        } else {
            false
        }
    }

    /// Get the next token if it is a line terminator or ';' or EOF, and return true,
    /// Otherwise, return false.
    fn consume_term(&mut self) -> bool {
        if !self.peek_no_skip_line_term().is_term() {
            return false;
        };
        while self.peek_no_skip_line_term().is_term() {
            if self.get_no_skip_line_term().is_eof() {
                return true;
            }
        }
        return true;
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
            TokenKind::Ident(s, _) => s.clone(),
            _ => {
                return Err(self.error_unexpected(self.prev_loc(), "Expect identifier."));
            }
        };
        Ok(self.get_ident_id(&name))
    }

    /// Get the next token and examine whether it is Const.
    /// Return IdentId of the Const.
    /// If not, return RubyError.
    fn expect_const(&mut self) -> Result<IdentId, RubyError> {
        let name = match &self.get()?.kind {
            TokenKind::Const(s) => s.clone(),
            _ => {
                return Err(self.error_unexpected(self.prev_loc(), "Expect constant."));
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
        self.context_stack.push(Context::new_class(lvar_collector));
        let node = self.parse_comp_stmt()?;
        let lvar = self.context_stack.pop().unwrap().lvar;

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
        // COMP_STMT : STMT [TERM+ STMT]* [TERM+]?

        fn return_comp_stmt(nodes: Vec<Node>, mut loc: Loc) -> Result<Node, RubyError> {
            if let Some(node) = nodes.last() {
                loc = loc.merge(node.loc());
            };
            Ok(Node::new_comp_stmt(nodes, loc))
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
            let node = self.parse_stmt()?;
            //println!("node {:?}", node);
            nodes.push(node);
            if !self.consume_term() {
                break;
            }
        }

        return_comp_stmt(nodes, loc)
    }

    fn parse_stmt(&mut self) -> Result<Node, RubyError> {
        let node = self.parse_expr()?;
        if self.consume_reserved_no_skip_line_term(Reserved::If) {
            // STMT : STMT if EXPR
            let loc = self.prev_loc();
            let cond = self.parse_expr()?;
            Ok(Node::new_if(
                cond,
                node,
                Node::new_comp_stmt(vec![], loc),
                loc,
            ))
        } else {
            // STMT : EXPR
            Ok(node)
        }
    }

    fn parse_expr(&mut self) -> Result<Node, RubyError> {
        // EXPR : NOT
        // | KEYWORD-AND
        // | KEYWORD-OR
        // NOT : ARG
        // | UNPARENTHESIZED-METHOD
        // | ! UNPARENTHESIZED-METHOD
        // | KEYWORD-NOT
        // UNPARENTHESIZED-METHOD :
        // | FNAME ARGS
        // | PRIMARY . FNAME ARGS
        // | PRIMARY :: FNAME ARGS
        // | COMMAND-WITH-DO-BLOCK [CHAIN-METHOD]*
        // | COMMAND-WITH-DO-BLOCK [CHAIN-METHOD]* . FNAME ARGS
        // | COMMAND-WITH-DO-BLOCK [CHAIN-METHOD]* :: FNAME ARGS
        // CHAIN-METOD : . FNAME
        // | :: FNAME
        // | . FNAME( ARGS )
        // | :: FNAME( ARGS )
        // COMMAND-WITH-DO-BLOCK : FNAME ARGS DO-BLOCK
        // | PRIMARY . FNAME ARGS DO-BLOCK [CHAIN-METHOD]* [ . FNAME ARGS]
        let node = self.parse_arg()?;
        if self.consume_punct_no_skip_line_term(Punct::Comma)
        /*&& node.is_lvar()*/
        {
            // EXPR : MLHS `=' MRHS
            return Ok(self.parse_mul_assign(node)?);
        }
        if node.is_operation() && self.is_command() {
            // FNAME ARGS
            // FNAME ARGS DO-BLOCK
            Ok(self.parse_command(node.as_method_name().unwrap(), node.loc())?)
        } else if let Node {
            // PRIMARY . FNAME ARGS
            // PRIMARY . FNAME ARGS DO_BLOCK [CHAIN-METHOD]* [ . FNAME ARGS]
            kind:
                NodeKind::Send {
                    method,
                    receiver,
                    mut args,
                    completed: false,
                    ..
                },
            mut loc,
        } = node.clone()
        {
            if self.is_command() {
                args = self.parse_arglist()?;
                loc = loc.merge(args[0].loc());
            }
            let block = self.parse_block()?;
            let node = Node::new_send(*receiver, method, args, block, true, loc);
            Ok(node)
        } else {
            // EXPR : ARG
            Ok(node)
        }
    }

    fn parse_mul_assign(&mut self, node: Node) -> Result<Node, RubyError> {
        // EXPR : MLHS `=' MRHS
        let mut new_lvar = vec![];
        if let NodeKind::Ident(id, has_suffix) = node.kind {
            if has_suffix {
                return Err(self.error_unexpected(node.loc(), "Illegal identifier for left hand."));
            };
            new_lvar.push(id);
        };
        let mut mlhs = vec![node];
        loop {
            let node = self.parse_function()?;
            if let NodeKind::Ident(id, has_suffix) = node.kind {
                if has_suffix {
                    return Err(
                        self.error_unexpected(node.loc(), "Illegal identifier for left hand.")
                    );
                };
                new_lvar.push(id);
            };
            mlhs.push(node);
            if !self.consume_punct_no_skip_line_term(Punct::Comma) {
                break;
            }
        }

        if !self.consume_punct_no_skip_line_term(Punct::Assign) {
            return Err(self.error_unexpected(self.loc(), "Expected '='."));
        }

        let mut mrhs = vec![];
        loop {
            mrhs.push(self.parse_arg()?);
            if !self.consume_punct_no_skip_line_term(Punct::Comma) {
                break;
            }
        }
        for lvar in new_lvar {
            self.add_local_var_if_new(lvar);
        }
        return Ok(Node::new_mul_assign(mlhs, mrhs));
    }

    fn parse_command(&mut self, operation: IdentId, loc: Loc) -> Result<Node, RubyError> {
        // FNAME ARGS
        // FNAME ARGS DO-BLOCK
        let args = self.parse_arglist()?;
        let block = self.parse_block()?;
        Ok(Node::new_send(
            Node::new_self(loc),
            operation,
            args,
            block,
            true,
            loc,
        ))
    }

    fn parse_arglist(&mut self) -> Result<NodeVec, RubyError> {
        let first_arg = self.parse_arg()?;

        if first_arg.is_operation() && self.is_command() {
            return Ok(vec![self.parse_command(
                first_arg.as_method_name().unwrap(),
                first_arg.loc(),
            )?]);
        }

        let mut args = vec![first_arg];
        if self.consume_punct(Punct::Comma) {
            loop {
                args.push(self.parse_arg()?);
                if !self.consume_punct(Punct::Comma) {
                    break;
                }
            }
        }
        Ok(args)
    }

    fn is_block(&mut self) -> bool {
        match self.peek().kind {
            TokenKind::Reserved(Reserved::Do) | TokenKind::Punct(Punct::LBrace) => true,
            _ => false,
        }
    }

    fn is_command(&mut self) -> bool {
        let tok = self.peek_no_skip_line_term();
        match tok.kind {
            TokenKind::Ident(_, _)
            | TokenKind::InstanceVar(_)
            | TokenKind::Const(_)
            | TokenKind::NumLit(_)
            | TokenKind::FloatLit(_)
            | TokenKind::StringLit(_)
            | TokenKind::OpenDoubleQuote(_) => true,
            TokenKind::Punct(p) => match p {
                Punct::LParen
                | Punct::LBracket
                | Punct::LBrace
                | Punct::Colon
                | Punct::Scope
                | Punct::Plus
                | Punct::Minus
                | Punct::Arrow => true,
                _ => false,
            },
            TokenKind::Reserved(r) => match r {
                Reserved::False | Reserved::Nil | Reserved::True => true,
                _ => false,
            },
            _ => false,
        }
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

            if self.consume_punct_no_skip_line_term(Punct::Comma) {
                let mut mrhs = vec![rhs];
                loop {
                    mrhs.push(self.parse_arg()?);
                    if !self.consume_punct_no_skip_line_term(Punct::Comma) {
                        break;
                    }
                }
                self.check_lhs(&lhs)?;
                Ok(Node::new_mul_assign(vec![lhs], mrhs))
            } else {
                self.check_lhs(&lhs)?;
                Ok(Node::new_assign(lhs, rhs))
            }
        } else if let TokenKind::Punct(Punct::AssignOp(op)) = self.peek_no_skip_line_term().kind {
            //let loc = self.loc();
            self.get()?;
            let rhs = self.parse_arg()?;
            self.check_lhs(&lhs)?;
            Ok(Node::new_assign(lhs.clone(), Node::new_binop(op, lhs, rhs)))
        } else {
            Ok(lhs)
        }
    }

    fn check_lhs(&mut self, lhs: &Node) -> Result<(), RubyError> {
        if let NodeKind::Ident(id, has_suffix) = lhs.kind {
            if has_suffix {
                return Err(self.error_unexpected(lhs.loc(), "Illegal identifier for left hand."));
            };
            self.add_local_var_if_new(id);
        };
        Ok(())
    }

    fn parse_arg_ternary(&mut self) -> Result<Node, RubyError> {
        let loc = self.loc();
        let cond = self.parse_arg_range()?;
        if self.consume_punct(Punct::Question) {
            let then_ = self.parse_arg_ternary()?;
            self.expect_punct(Punct::Colon)?;
            let else_ = self.parse_arg_ternary()?;
            let node = Node::new_if(cond, then_, else_, loc);
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
        self.save_state();
        if self.consume_punct(Punct::Minus) {
            match self.peek().kind {
                TokenKind::NumLit(_) | TokenKind::FloatLit(_) => {
                    self.restore_state();
                    let lhs = self.parse_unary_bitnot()?;
                    return Ok(lhs);
                }
                _ => self.discard_state(),
            };
            let lhs = self.parse_unary_minus()?;
            let loc = loc.merge(lhs.loc());
            let lhs = Node::new_binop(BinOp::Mul, lhs, Node::new_integer(-1, loc));
            Ok(lhs)
        } else {
            self.discard_state();
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
            let lhs = self.parse_function()?;
            Ok(lhs)
        }
    }

    fn parse_function(&mut self) -> Result<Node, RubyError> {
        let loc = self.loc();
        let mut node = self.parse_primary()?;
        if node.is_operation() {
            if self.consume_punct_no_skip_line_term(Punct::LParen) {
                // PRIMARY-METHOD : FNAME ( ARGS ) BLOCK?
                let args = if self.is_block() {
                    vec![]
                } else {
                    self.parse_args(Punct::RParen)?
                };
                let block = self.parse_block()?;

                node = Node::new_send(
                    Node::new_self(loc),
                    node.as_method_name().unwrap(),
                    args,
                    block,
                    true,
                    loc,
                );
            } else if let Some(block) = self.parse_block()? {
                // PRIMARY-METHOD : FNAME BLOCK
                node = Node::new_send(
                    Node::new_self(loc),
                    node.as_method_name().unwrap(),
                    vec![],
                    Some(block),
                    true,
                    loc,
                );
            }
        }
        loop {
            let tok = self.peek();
            node = match tok.kind {
                TokenKind::Punct(Punct::Dot) => {
                    // PRIMARY-METHOD :
                    // | PRIMARY . FNAME ( ARGS )? BLOCK? => completed: true
                    // | PRIMARY . FNAME => completed: false
                    self.get()?;
                    let tok = self.get()?.clone();
                    let method = match &tok.kind {
                        TokenKind::Ident(s, _) => s,
                        TokenKind::Reserved(r) => {
                            let string = self.lexer.get_string_from_reserved(*r);
                            string
                        }
                        _ => {
                            return Err(self
                                .error_unexpected(tok.loc(), "method name must be an identifier."))
                        }
                    }
                    .clone();
                    let id = self.get_ident_id(&method);
                    let mut args = vec![];
                    let mut completed = false;
                    if self.consume_punct_no_skip_line_term(Punct::LParen) {
                        args = self.parse_args(Punct::RParen)?;
                        completed = true;
                    }
                    let block = self.parse_block()?;
                    if block.is_some() {
                        completed = true;
                    };
                    let node = match node.kind {
                        NodeKind::Ident(id, _) => {
                            Node::new_send(Node::new_self(loc), id, vec![], None, true, loc)
                        }
                        _ => node,
                    };
                    Node::new_send(node, id, args, block, completed, loc.merge(self.loc()))
                }
                TokenKind::Punct(Punct::LBracket) => {
                    if node.is_operation() {
                        return Ok(node);
                    };
                    let loc = self.loc();
                    self.get()?;
                    let args = self.parse_args(Punct::RBracket)?;
                    let len = args.len();
                    if len < 1 || len > 2 {
                        return Err(self.error_unexpected(
                            loc.merge(self.prev_loc()),
                            "Wrong number of arguments (expected 1 or 2)",
                        ));
                    }
                    Node::new_array_member(node, args)
                }
                TokenKind::Punct(Punct::Scope) => {
                    self.get()?;
                    let loc = self.loc();
                    let id = self.expect_const()?;
                    Node::new_scope(node, id, loc)
                }
                _ => return Ok(node),
            }
        }
    }

    fn parse_args(&mut self, punct: Punct) -> Result<Vec<Node>, RubyError> {
        let mut args = vec![];
        if self.consume_punct(punct) {
            return Ok(args);
        }
        loop {
            args.push(self.parse_arg()?);
            if !self.consume_punct(Punct::Comma) {
                break;
            }
        }
        self.expect_punct(punct)?;
        Ok(args)
    }

    fn parse_block(&mut self) -> Result<Option<Box<Node>>, RubyError> {
        let do_flag = if self.consume_reserved_no_skip_line_term(Reserved::Do) {
            true
        } else {
            if self.consume_punct_no_skip_line_term(Punct::LBrace) {
                false
            } else {
                return Ok(None);
            }
        };
        // BLOCK: do [`|' [BLOCK_VAR] `|'] COMPSTMT end
        let loc = self.prev_loc();
        self.context_stack.push(Context::new_block());
        let mut params = vec![];
        if self.consume_punct(Punct::BitOr) {
            if !self.consume_punct(Punct::BitOr) {
                loop {
                    let id = self.expect_ident()?;
                    params.push(Node::new_param(id, self.prev_loc()));
                    self.add_local_var(id);
                    if !self.consume_punct(Punct::Comma) {
                        break;
                    }
                }
                self.expect_punct(Punct::BitOr)?;
            }
        } else {
            self.consume_punct(Punct::LOr);
        }
        let body = self.parse_comp_stmt()?;
        if do_flag {
            self.expect_reserved(Reserved::End)?;
        } else {
            self.expect_punct(Punct::RBrace)?;
        };
        let lvar = self.context_stack.pop().unwrap().lvar;
        let loc = loc.merge(self.prev_loc());
        let node = Node::new_proc(params, body, lvar, loc);
        Ok(Some(Box::new(node)))
    }

    fn parse_primary(&mut self) -> Result<Node, RubyError> {
        let tok = self.get()?.clone();
        let loc = tok.loc();
        match &tok.kind {
            TokenKind::Ident(name, has_suffix) => {
                let id = self.get_ident_id(name);
                if name == "self" {
                    return Ok(Node::new_self(loc));
                } else if *has_suffix {
                    match self.get()?.kind {
                        TokenKind::Punct(Punct::Question) => {
                            let id = self.get_ident_id(&(name.clone() + "?"));
                            Ok(Node::new_identifier(id, true, loc))
                        }
                        _ => panic!("Illegal method name."),
                    }
                } else if self.is_local_var(id) {
                    Ok(Node::new_lvar(id, loc))
                } else {
                    // FUNCTION or COMMAND or LHS for assignment
                    Ok(Node::new_identifier(id, false, loc))
                }
            }
            TokenKind::InstanceVar(name) => {
                let id = self.get_ident_id(name);
                return Ok(Node::new_instance_var(id, loc));
            }
            TokenKind::Const(name) => {
                let id = self.get_ident_id(name);
                Ok(Node::new_const(id, false, loc))
            }
            TokenKind::NumLit(num) => Ok(Node::new_integer(*num, loc)),
            TokenKind::FloatLit(num) => Ok(Node::new_float(*num, loc)),
            TokenKind::StringLit(s) => Ok(self.parse_string_literal(s)?),
            TokenKind::OpenDoubleQuote(s) => Ok(self.parse_interporated_string_literal(s)?),
            TokenKind::Punct(punct) => match punct {
                Punct::Minus => match self.get()?.kind {
                    TokenKind::NumLit(num) => Ok(Node::new_integer(-num, loc)),
                    TokenKind::FloatLit(num) => Ok(Node::new_float(-num, loc)),
                    _ => unreachable!(),
                },
                Punct::LParen => {
                    let node = self.parse_comp_stmt()?;
                    self.expect_punct(Punct::RParen)?;
                    Ok(node)
                }
                Punct::LBracket => {
                    let nodes = self.parse_args(Punct::RBracket)?;
                    Ok(Node::new(
                        NodeKind::Array(nodes),
                        loc.merge(self.prev_loc()),
                    ))
                }
                Punct::Colon => {
                    let ident = self.expect_ident()?;
                    Ok(Node::new_symbol(ident, loc.merge(self.prev_loc())))
                }
                Punct::Arrow => {
                    let mut params = vec![];
                    self.context_stack.push(Context::new_block());
                    if self.consume_punct(Punct::LParen) {
                        if !self.consume_punct(Punct::RParen) {
                            loop {
                                let id = self.expect_ident()?;
                                params.push(Node::new_param(id, self.prev_loc()));
                                self.add_local_var(id);
                                if !self.consume_punct(Punct::Comma) {
                                    break;
                                }
                            }
                            self.expect_punct(Punct::RParen)?;
                        }
                    } else if let TokenKind::Ident(_, _) = self.peek().kind {
                        let id = self.expect_ident()?;
                        self.add_local_var(id);
                        params.push(Node::new_param(id, self.prev_loc()));
                    };
                    self.expect_punct(Punct::LBrace)?;
                    let body = self.parse_comp_stmt()?;
                    self.expect_punct(Punct::RBrace)?;
                    let lvar = self.context_stack.pop().unwrap().lvar;
                    Ok(Node::new_proc(params, body, lvar, loc))
                }
                Punct::Scope => {
                    let id = self.expect_const()?;
                    Ok(Node::new_const(id, true, loc))
                }
                _ => {
                    return Err(
                        self.error_unexpected(loc, format!("Unexpected token: {:?}", tok.kind))
                    )
                }
            },
            TokenKind::Reserved(Reserved::If) => {
                let node = self.parse_if_then()?;
                self.expect_reserved(Reserved::End)?;
                Ok(node)
            }
            TokenKind::Reserved(Reserved::For) => {
                let loc = self.prev_loc();
                let var_id = self.expect_ident()?;
                let var = Node::new_lvar(var_id, self.prev_loc());
                self.add_local_var_if_new(var_id);
                self.expect_reserved(Reserved::In)?;
                let iter = self.parse_expr()?;
                self.parse_do()?;
                let body = self.parse_comp_stmt()?;
                self.expect_reserved(Reserved::End)?;
                let node = Node::new(
                    NodeKind::For {
                        param: Box::new(var),
                        iter: Box::new(iter),
                        body: Box::new(body),
                    },
                    loc.merge(self.prev_loc()),
                );
                Ok(node)
            }
            TokenKind::Reserved(Reserved::Def) => {
                let node = self.parse_def()?;
                Ok(node)
            }
            TokenKind::Reserved(Reserved::Class) => {
                if self.context_stack.last().unwrap().kind == ContextKind::Method {
                    return Err(
                        self.error_unexpected(loc, "SyntaxError: class definition in method body.")
                    );
                }
                let node = self.parse_class(false)?;
                Ok(node)
            }
            TokenKind::Reserved(Reserved::Module) => {
                if self.context_stack.last().unwrap().kind == ContextKind::Method {
                    return Err(
                        self.error_unexpected(loc, "SyntaxError: class definition in method body.")
                    );
                }
                let node = self.parse_class(true)?;
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
        let loc = self.prev_loc();
        let cond = self.parse_expr()?;
        self.parse_then()?;
        let then_ = self.parse_comp_stmt()?;
        let else_ = if self.consume_reserved(Reserved::Elsif) {
            self.parse_if_then()?
        } else if self.consume_reserved(Reserved::Else) {
            self.parse_comp_stmt()?
        } else {
            Node::new_comp_stmt(vec![], self.loc())
        };
        Ok(Node::new_if(cond, then_, else_, loc))
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
        let mut id = match self.get()?.kind.clone() {
            TokenKind::Ident(name, has_suffix) => {
                if has_suffix {
                    match self.get()?.kind {
                        TokenKind::Punct(Punct::Question) => self.get_ident_id(&(name + "?")),
                        _ => panic!("Illegal method name."),
                    }
                } else {
                    self.get_ident_id(&name)
                }
            }
            TokenKind::Punct(Punct::Plus) => self.get_ident_id(&"@add".to_string()),
            TokenKind::Punct(Punct::Minus) => self.get_ident_id(&"@sub".to_string()),
            TokenKind::Punct(Punct::Mul) => self.get_ident_id(&"@mul".to_string()),
            _ => return Err(self.error_unexpected(self.loc(), "Expected identifier or operator.")),
        };
        if id == self_id {
            is_class_method = true;
            self.expect_punct(Punct::Dot)?;
            id = self.expect_ident()?;
        };
        self.context_stack.push(Context::new_method());
        let args = self.parse_params()?;
        let body = self.parse_comp_stmt()?;
        self.expect_reserved(Reserved::End)?;
        let lvar = self.context_stack.pop().unwrap().lvar;
        if is_class_method {
            Ok(Node::new_class_method_decl(id, args, body, lvar))
        } else {
            Ok(Node::new_method_decl(id, args, body, lvar))
        }
    }

    // ( )
    // ( ident [, ident]* )
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
            let mut loc = self.loc();
            let is_block = self.consume_punct(Punct::BitAnd);
            let id = self.expect_ident()?;
            loc = loc.merge(self.prev_loc());
            if is_block {
                args.push(Node::new_block_param(id, loc));
                self.add_block_param(id);
            } else {
                args.push(Node::new_param(id, loc));
                self.add_local_var(id);
            };
            self.context_stack.last_mut().unwrap().lvar.insert(id);
            if is_block || !self.consume_punct(Punct::Comma) {
                break;
            }
        }
        self.expect_punct(Punct::RParen)?;
        if !self.consume_term() {
            return Err(self.error_unexpected(self.loc(), "Expect terminator."));
        }
        Ok(args)
    }

    fn parse_class(&mut self, is_module: bool) -> Result<Node, RubyError> {
        //  class identifier [`<' EXPR]
        //      COMPSTMT
        //  end
        let loc = self.loc();
        let name = match &self.get()?.kind {
            TokenKind::Const(s) => s.clone(),
            _ => return Err(self.error_unexpected(loc, "Class/Module name must be CONSTANT.")),
        };
        let superclass = if self.consume_punct_no_skip_line_term(Punct::Lt) {
            if is_module {
                return Err(self.error_unexpected(self.prev_loc(), "Unexpected '<'."));
            };
            self.parse_expr()?
        } else {
            Node::new_const(IdentId::OBJECT, true, loc)
        };
        let id = self.get_ident_id(&name);
        self.context_stack.push(Context::new_class(None));
        let body = self.parse_comp_stmt()?;
        self.expect_reserved(Reserved::End)?;
        let lvar = self.context_stack.pop().unwrap().lvar;
        Ok(Node::new_class_decl(
            id, superclass, body, lvar, is_module, loc,
        ))
    }
}
