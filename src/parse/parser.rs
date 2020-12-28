use super::*;
use crate::error::{ParseErrKind, RubyError};
use crate::id_table::IdentId;
use crate::util::*;
use crate::vm::context::{ContextRef, ISeqKind};
use fxhash::FxHashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct Parser {
    pub lexer: Lexer,
    prev_loc: Loc,
    context_stack: Vec<ParseContext>,
    extern_context: Option<ContextRef>,
    /// this flag suppress accesory assignment. e.g. x=3
    supress_acc_assign: bool,
    /// this flag suppress accesory multiple assignment. e.g. x = 2,3
    supress_mul_assign: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseResult {
    pub node: Node,
    pub lvar_collector: LvarCollector,
    pub source_info: SourceInfoRef,
}

impl ParseResult {
    pub fn default(node: Node, lvar_collector: LvarCollector, source_info: SourceInfoRef) -> Self {
        ParseResult {
            node,
            lvar_collector,
            source_info,
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
}

impl From<usize> for LvarId {
    fn from(id: usize) -> Self {
        LvarId(id)
    }
}

impl From<u32> for LvarId {
    fn from(id: u32) -> Self {
        LvarId(id as usize)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LvarCollector {
    id: usize,
    table: FxHashMap<IdentId, LvarId>,
    kwrest: Option<LvarId>,
    block: Option<LvarId>,
}

impl LvarCollector {
    /// Create new `LvarCollector`.
    pub fn new() -> Self {
        LvarCollector {
            id: 0,
            table: FxHashMap::default(),
            kwrest: None,
            block: None,
        }
    }

    /// Check whether `val` exists in `LvarCollector` or not, and return `LvarId` if exists.
    /// If not, add new variable `val` to the `LvarCollector`.
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

    /// Add a new variable `val` to the `LvarCollector`.
    /// Return None if `val` already exists.
    fn insert_new(&mut self, val: IdentId) -> Option<LvarId> {
        let id = self.id;
        if self.table.insert(val, LvarId(id)).is_some() {
            return None;
        };
        self.id += 1;
        Some(LvarId(id))
    }

    fn insert_block_param(&mut self, val: IdentId) -> Option<LvarId> {
        let lvar = match self.insert_new(val) {
            Some(lvar) => lvar,
            None => return None,
        };
        self.block = Some(lvar);
        Some(lvar)
    }

    fn insert_kwrest_param(&mut self, val: IdentId) -> Option<LvarId> {
        let lvar = match self.insert_new(val) {
            Some(lvar) => lvar,
            None => return None,
        };
        self.kwrest = Some(lvar);
        Some(lvar)
    }

    pub fn get(&self, val: &IdentId) -> Option<&LvarId> {
        self.table.get(val)
    }

    pub fn get_name(&self, id: LvarId) -> Option<IdentId> {
        for (k, v) in self.table.iter() {
            if *v == id {
                return Some(*k);
            }
        }
        None
    }

    pub fn kwrest_param(&self) -> Option<LvarId> {
        self.kwrest
    }

    pub fn block_param(&self) -> Option<LvarId> {
        self.block
    }

    pub fn len(&self) -> usize {
        self.table.len()
    }

    pub fn table(&self) -> &FxHashMap<IdentId, LvarId> {
        &self.table
    }

    pub fn block(&self) -> &Option<LvarId> {
        &self.block
    }

    pub fn clone_table(&self) -> FxHashMap<IdentId, LvarId> {
        self.table.clone()
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ParseContext {
    lvar: LvarCollector,
    kind: ContextKind,
}

impl ParseContext {
    fn new_method() -> Self {
        ParseContext {
            lvar: LvarCollector::new(),
            kind: ContextKind::Method,
        }
    }
    fn new_class(lvar_collector: Option<LvarCollector>) -> Self {
        ParseContext {
            lvar: lvar_collector.unwrap_or(LvarCollector::new()),
            kind: ContextKind::Class,
        }
    }
    fn new_block() -> Self {
        ParseContext {
            lvar: LvarCollector::new(),
            kind: ContextKind::Block,
        }
    }
    fn new_for() -> Self {
        ParseContext {
            lvar: LvarCollector::new(),
            kind: ContextKind::For,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RescueEntry {
    /// The exception classes for this rescue clause.
    pub exception_list: Vec<Node>,
    /// Assignment destination for error value in rescue clause.
    pub assign: Option<Box<Node>>,
    /// The body of this rescue clause.
    pub body: Box<Node>,
}

impl RescueEntry {
    pub fn new(exception_list: Vec<Node>, assign: Option<Node>, body: Node) -> Self {
        Self {
            exception_list,
            assign: match assign {
                Some(assign) => Some(Box::new(assign)),
                None => None,
            },
            body: Box::new(body),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ContextKind {
    Class,
    Method,
    Block,
    For,
}

impl Parser {
    pub fn new() -> Self {
        let lexer = Lexer::new();
        Parser {
            lexer,
            prev_loc: Loc(0, 0),
            context_stack: vec![],
            extern_context: None,
            supress_acc_assign: false,
            supress_mul_assign: false,
        }
    }

    fn save_state(&mut self) {
        self.lexer.save_state();
    }

    fn restore_state(&mut self) {
        self.lexer.restore_state();
    }

    fn discard_state(&mut self) {
        self.lexer.discard_state();
    }

    pub fn get_context_depth(&self) -> usize {
        self.context_stack.len()
    }

    fn context_mut(&mut self) -> &mut ParseContext {
        self.context_stack.last_mut().unwrap()
    }

    fn is_method_context(&self) -> bool {
        self.context_stack.last().unwrap().kind == ContextKind::Method
    }

    /// If the `id` does not exist in the scope chain,
    /// add `id` as a local variable in the current context.
    fn add_local_var_if_new(&mut self, id: IdentId) {
        if !self.is_local_var(id) {
            for c in self.context_stack.iter_mut().rev() {
                match c.kind {
                    ContextKind::For => {}
                    _ => {
                        c.lvar.insert(id);
                    }
                };
            }
        }
    }

    /// Add the `id` as a new parameter in the current context.
    /// If a parameter with the same name already exists, return error.
    fn new_param(&mut self, id: IdentId, loc: Loc) -> Result<(), RubyError> {
        if self.context_mut().lvar.insert_new(id).is_none() {
            return Err(self.error_unexpected(loc, "Duplicated argument name."));
        }
        Ok(())
    }

    /// Add the `id` as a new parameter in the current context.
    /// If a parameter with the same name already exists, return error.
    fn new_kwrest_param(&mut self, id: IdentId, loc: Loc) -> Result<(), RubyError> {
        if self.context_mut().lvar.insert_kwrest_param(id).is_none() {
            return Err(self.error_unexpected(loc, "Duplicated argument name."));
        }
        Ok(())
    }

    /// Add the `id` as a new block parameter in the current context.
    /// If a parameter with the same name already exists, return error.
    fn new_block_param(&mut self, id: IdentId, loc: Loc) -> Result<(), RubyError> {
        if self.context_mut().lvar.insert_block_param(id).is_none() {
            return Err(self.error_unexpected(loc, "Duplicated argument name."));
        }
        Ok(())
    }

    /// Examine whether `id` exists in the scope chain.
    /// If exiets, return true.
    fn is_local_var(&mut self, id: IdentId) -> bool {
        for c in self.context_stack.iter().rev() {
            if c.lvar.table.contains_key(&id) {
                return true;
            }
            match c.kind {
                ContextKind::Block | ContextKind::For => {}
                _ => return false,
            }
        }
        let mut ctx = match self.extern_context {
            None => return false,
            Some(ctx) => ctx,
        };
        loop {
            if ctx.iseq_ref.unwrap().lvar.table.contains_key(&id) {
                return true;
            };
            if let ISeqKind::Method(_) = ctx.kind {
                return false;
            }
            match ctx.outer {
                Some(outer) => ctx = outer,
                None => return false,
            }
        }
    }

    fn get_ident_id(&self, method: &str) -> IdentId {
        IdentId::get_id(method)
    }

    /// Peek next token (skipping line terminators).
    fn peek(&mut self) -> Result<Token, RubyError> {
        self.lexer.peek_token_skip_lt()
    }

    /// Peek next token (no skipping line terminators).
    fn peek_no_term(&mut self) -> Result<Token, RubyError> {
        self.lexer.peek_token()
    }

    /// Peek next token (no skipping line terminators), and check whether the token is `punct` or not.
    fn peek_punct_no_term(&mut self, punct: Punct) -> bool {
        match self.lexer.peek_token() {
            Ok(tok) => tok.kind == TokenKind::Punct(punct),
            Err(_) => false,
        }
    }

    /// Examine the next token, and return true if it is a line terminator.
    fn is_line_term(&mut self) -> Result<bool, RubyError> {
        Ok(self.peek_no_term()?.is_line_term())
    }

    fn loc(&mut self) -> Loc {
        self.peek_no_term().unwrap().loc()
    }

    fn prev_loc(&self) -> Loc {
        self.prev_loc
    }

    /// Get next token (skipping line terminators).
    /// Return RubyError if it was EOF.
    fn get(&mut self) -> Result<Token, RubyError> {
        loop {
            let tok = self.lexer.get_token()?;
            if tok.is_eof() {
                return Err(self.error_eof(tok.loc()));
            }
            if !tok.is_line_term() {
                self.prev_loc = tok.loc;
                return Ok(tok);
            }
        }
    }

    /// Get next token (no skipping line terminators).
    fn get_no_skip_line_term(&mut self) -> Result<Token, RubyError> {
        let tok = self.lexer.get_token()?;
        self.prev_loc = tok.loc;
        Ok(tok)
    }

    /// If the next token is Ident, consume and return Some(it).
    /// If not, return None.
    fn consume_ident(&mut self) -> Result<Option<IdentId>, RubyError> {
        match self.peek()?.kind {
            TokenKind::Ident(s) => {
                self.get()?;
                Ok(Some(self.get_ident_id(&s)))
            }
            _ => Ok(None),
        }
    }

    /// If the next token is an expected kind of Punctuator, get it and return true.
    /// Otherwise, return false.
    fn consume_punct(&mut self, expect: Punct) -> Result<bool, RubyError> {
        match self.peek()?.kind {
            TokenKind::Punct(punct) if punct == expect => {
                self.get()?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn consume_punct_no_term(&mut self, expect: Punct) -> Result<bool, RubyError> {
        if TokenKind::Punct(expect) == self.peek_no_term()?.kind {
            self.get()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn consume_assign_op_no_term(&mut self) -> Result<Option<BinOp>, RubyError> {
        if let TokenKind::Punct(Punct::AssignOp(op)) = self.peek_no_term()?.kind {
            Ok(Some(op))
        } else {
            Ok(None)
        }
    }

    /// If next token is an expected kind of Reserved keyeord, get it and return true.
    /// Otherwise, return false.
    fn consume_reserved(&mut self, expect: Reserved) -> Result<bool, RubyError> {
        match self.peek()?.kind {
            TokenKind::Reserved(reserved) if reserved == expect => {
                self.get()?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn consume_reserved_no_skip_line_term(&mut self, expect: Reserved) -> Result<bool, RubyError> {
        if TokenKind::Reserved(expect) == self.peek_no_term()?.kind {
            self.get()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get the next token if it is a line terminator or ';' or EOF, and return true,
    /// Otherwise, return false.
    fn consume_term(&mut self) -> Result<bool, RubyError> {
        if !self.peek_no_term()?.is_term() {
            return Ok(false);
        };
        while self.peek_no_term()?.is_term() {
            if self.get_no_skip_line_term()?.is_eof() {
                return Ok(true);
            }
        }
        return Ok(true);
    }

    /// Get the next token and examine whether it is an expected Reserved.
    /// If not, return RubyError.
    fn expect_reserved(&mut self, expect: Reserved) -> Result<(), RubyError> {
        match &self.get()?.kind {
            TokenKind::Reserved(reserved) if *reserved == expect => Ok(()),
            t => {
                Err(self
                    .error_unexpected(self.prev_loc(), format!("Expect {:?} Got {:?}", expect, t)))
            }
        }
    }

    /// Get the next token and examine whether it is an expected Punct.
    /// If not, return RubyError.
    fn expect_punct(&mut self, expect: Punct) -> Result<(), RubyError> {
        match &self.get()?.kind {
            TokenKind::Punct(punct) if *punct == expect => Ok(()),
            t => {
                Err(self
                    .error_unexpected(self.prev_loc(), format!("Expect {:?} Got {:?}", expect, t)))
            }
        }
    }

    /// Get the next token and examine whether it is Ident.
    /// Return IdentId of the Ident.
    /// If not, return RubyError.
    fn expect_ident(&mut self) -> Result<IdentId, RubyError> {
        match &self.get()?.kind {
            TokenKind::Ident(s) => Ok(self.get_ident_id(s)),
            _ => Err(self.error_unexpected(self.prev_loc(), "Expect identifier.")),
        }
    }

    /// Get the next token and examine whether it is Const.
    /// Return IdentId of the Const.
    /// If not, return RubyError.
    fn expect_const(&mut self) -> Result<String, RubyError> {
        match self.get()?.kind {
            TokenKind::Const(s) => Ok(s),
            _ => Err(self.error_unexpected(self.prev_loc(), "Expect constant.")),
        }
    }

    fn token_as_symbol(&self, token: &Token) -> String {
        match token.kind.clone() {
            TokenKind::Ident(ident) => ident,
            TokenKind::Const(ident) => ident,
            TokenKind::InstanceVar(ident) => ident,
            TokenKind::StringLit(ident) => ident,
            TokenKind::Reserved(reserved) => {
                self.lexer.get_string_from_reserved(reserved).to_string()
            }
            _ => unreachable!(),
        }
    }

    fn error_unexpected(&self, loc: Loc, msg: impl Into<String>) -> RubyError {
        RubyError::new_parse_err(
            ParseErrKind::SyntaxError(msg.into()),
            self.lexer.source_info,
            0,
            loc,
        )
    }

    fn error_eof(&self, loc: Loc) -> RubyError {
        RubyError::new_parse_err(ParseErrKind::UnexpectedEOF, self.lexer.source_info, 0, loc)
    }
}

impl Parser {
    pub fn parse_program(mut self, path: PathBuf, program: &str) -> Result<ParseResult, RubyError> {
        let (node, lvar) = self.parse_program_core(path, program, None)?;

        let tok = self.peek()?;
        if tok.is_eof() {
            let result = ParseResult::default(node, lvar, self.lexer.source_info);
            Ok(result)
        } else {
            Err(self.error_unexpected(tok.loc(), "Expected end-of-input."))
        }
    }

    pub fn parse_program_repl(
        mut self,
        path: PathBuf,
        program: &str,
        extern_context: Option<ContextRef>,
    ) -> Result<ParseResult, RubyError> {
        let (node, lvar) = match self.parse_program_core(path, program, extern_context) {
            Ok((node, lvar)) => (node, lvar),
            Err(mut err) => {
                err.set_level(self.context_stack.len() - 1);
                return Err(err);
            }
        };

        let tok = self.peek()?;
        if tok.is_eof() {
            let result = ParseResult::default(node, lvar, self.lexer.source_info);
            Ok(result)
        } else {
            let mut err = self.error_unexpected(tok.loc(), "Expected end-of-input.");
            err.set_level(0);
            Err(err)
        }
    }

    pub fn parse_program_core(
        &mut self,
        path: PathBuf,
        program: &str,
        extern_context: Option<ContextRef>,
    ) -> Result<(Node, LvarCollector), RubyError> {
        self.lexer.init(path, program);
        self.extern_context = extern_context;
        self.context_stack
            .push(ParseContext::new_class(match extern_context {
                Some(ctx) => Some(ctx.iseq_ref.unwrap().lvar.clone()),
                None => None,
            }));
        let node = self.parse_comp_stmt()?;
        let lvar = self.context_stack.pop().unwrap().lvar;
        Ok((node, lvar))
    }

    pub fn parse_program_eval(
        mut self,
        path: PathBuf,
        program: &str,
        extern_context: Option<ContextRef>,
    ) -> Result<ParseResult, RubyError> {
        self.lexer.init(path, program);
        self.extern_context = extern_context;
        self.context_stack.push(ParseContext::new_block());
        let node = self.parse_comp_stmt()?;
        let lvar = self.context_stack.pop().unwrap().lvar;
        let tok = self.peek()?;
        if tok.is_eof() {
            let result = ParseResult::default(node, lvar, self.lexer.source_info);
            Ok(result)
        } else {
            Err(self.error_unexpected(tok.loc(), "Expected end-of-input."))
        }
    }

    fn parse_comp_stmt(&mut self) -> Result<Node, RubyError> {
        // COMP_STMT : (STMT (TERM STMT)*)? (TERM+)?
        self.peek()?;
        let loc = self.loc();
        let mut nodes = vec![];

        loop {
            if self.peek()?.check_stmt_end() {
                let node = Node::new_comp_stmt(nodes, loc);
                //println!("comp_node_escape {:?}", node);
                return Ok(node);
            }

            let node = self.parse_stmt()?;
            //println!("node {:?}", node);
            nodes.push(node);
            //println!("next {:?}", self.peek_no_term()?);
            if !self.consume_term()? {
                break;
            }
        }
        let node = Node::new_comp_stmt(nodes, loc);
        //println!("comp_node {:?}", node);
        Ok(node)
    }

    fn parse_stmt(&mut self) -> Result<Node, RubyError> {
        // STMT : EXPR
        // | ALIAS-STMT
        // | UNDEF-STMT
        // | STMT [no-term] if EXPR
        // | STMT [no-term] unless EXPR
        // | STMT [no-term] while EXPR
        // | STMT [no-term] until EXPR
        // | STMT [no-term] rescie EXPR
        // | STMT - NORET-STMT [no-term] rescie EXPR
        // | VAR [no term] = UNPARENTHESIZED-METHOD-CALL
        // | PRIMARY :: CONST [no term] = UNPARENTHESIZED-METHOD-CALL
        // | :: CONST [no term] = UNPARENTHESIZED-METHOD-CALL
        // | PRIMARY [no term] (.|::) LOCAL-VAR [no term] = UNPARENTHESIZED-METHOD-CALL
        // | PRIMARY [no term] . CONST [no term] = UNPARENTHESIZED-METHOD-CALL
        // | VAR [no term] <assign-op> UNPARENTHESIZED-METHOD-CALL
        // | PRIMARY [no term] [INDEX-LIST] [no term] <assign-op> UNPARENTHESIZED-METHOD-CALL
        // | LHS [no term] = MRHS
        // | * LHS [no term] = (UNPARENTHESIZED-METHOD-CALL | ARG)
        // | MLHS [no term] = MRHS
        let mut node = self.parse_expr()?;
        loop {
            if self.consume_reserved_no_skip_line_term(Reserved::If)? {
                // STMT : STMT if EXPR
                let loc = self.prev_loc();
                let cond = self.parse_expr()?;
                node = Node::new_if(cond, node, Node::new_comp_stmt(vec![], loc), loc);
            } else if self.consume_reserved_no_skip_line_term(Reserved::Unless)? {
                // STMT : STMT unless EXPR
                let loc = self.prev_loc();
                let cond = self.parse_expr()?;
                node = Node::new_if(cond, Node::new_comp_stmt(vec![], loc), node, loc);
            } else if self.consume_reserved_no_skip_line_term(Reserved::While)? {
                // STMT : STMT while EXPR
                let loc = self.prev_loc();
                let cond = self.parse_expr()?;
                let loc = loc.merge(self.prev_loc());
                node = Node::new_while(cond, node, true, loc);
            } else if self.consume_reserved_no_skip_line_term(Reserved::Until)? {
                // STMT : STMT until EXPR
                let loc = self.prev_loc();
                let cond = self.parse_expr()?;
                let loc = loc.merge(self.prev_loc());
                node = Node::new_while(cond, node, false, loc);
            } else {
                break;
            }
        }
        // STMT : EXPR
        Ok(node)
    }

    fn parse_expr(&mut self) -> Result<Node, RubyError> {
        // EXPR : NOT
        // | EXPR [no term] and NOT
        // | EXPR [no term] or NOT
        let mut node = self.parse_not()?;
        loop {
            if self.consume_reserved_no_skip_line_term(Reserved::And)? {
                let rhs = self.parse_not()?;
                node = Node::new_binop(BinOp::LAnd, node, rhs);
            } else if self.consume_reserved_no_skip_line_term(Reserved::Or)? {
                let rhs = self.parse_not()?;
                node = Node::new_binop(BinOp::LOr, node, rhs);
            } else {
                return Ok(node);
            }
        }
    }

    fn parse_not(&mut self) -> Result<Node, RubyError> {
        // NOT : ARG
        // | UNPARENTHESIZED-METHOD
        // | ! UNPARENTHESIZED-METHOD
        // | not NOT
        let node = self.parse_arg()?;
        if self.consume_punct_no_term(Punct::Comma)? {
            // EXPR : MLHS `=' MRHS
            return Ok(self.parse_mul_assign(node)?);
        }
        Ok(node)
    }

    fn parse_mul_assign(&mut self, node: Node) -> Result<Node, RubyError> {
        // EXPR : MLHS `=' MRHS
        let mut mlhs = vec![node];
        let old = self.supress_acc_assign;
        self.supress_acc_assign = true;
        loop {
            if self.peek_punct_no_term(Punct::Assign) {
                break;
            }
            let node = self.parse_method_call()?;
            mlhs.push(node);
            if !self.consume_punct_no_term(Punct::Comma)? {
                break;
            }
        }
        self.supress_acc_assign = old;
        if !self.consume_punct_no_term(Punct::Assign)? {
            let loc = self.loc();
            return Err(self.error_unexpected(loc, "Expected '='."));
        }

        let mrhs = self.parse_mul_assign_rhs()?;
        for lhs in &mlhs {
            self.check_lhs(lhs)?;
        }

        return Ok(Node::new_mul_assign(mlhs, mrhs));
    }

    /// Parse rhs of multiple assignment.
    /// If Parser.mul_assign_rhs is true, only a single assignment is allowed.
    fn parse_mul_assign_rhs(&mut self) -> Result<Vec<Node>, RubyError> {
        if self.supress_mul_assign {
            let node = vec![self.parse_arg()?];
            Ok(node)
        } else {
            let mrhs = self.parse_arg_list(None)?;
            Ok(mrhs)
        }
    }

    fn parse_arg_list(&mut self, term: impl Into<Option<Punct>>) -> Result<Vec<Node>, RubyError> {
        let term = term.into();
        let old = self.supress_mul_assign;
        // multiple assignment must be suppressed in parsing arg list.
        self.supress_mul_assign = true;

        let mut args = vec![];
        loop {
            if let Some(term) = term {
                if self.consume_punct(term)? {
                    self.supress_mul_assign = old;
                    return Ok(args);
                }
            };
            if self.consume_punct(Punct::Mul)? {
                // splat argument
                let loc = self.prev_loc();
                let array = self.parse_arg()?;
                args.push(Node::new_splat(array, loc));
            } else {
                let node = self.parse_arg()?;
                args.push(node);
            }
            if !self.consume_punct(Punct::Comma)? {
                break;
            }
        }
        self.supress_mul_assign = old;
        match term {
            Some(term) => self.expect_punct(term)?,
            None => {}
        };
        Ok(args)
    }

    fn parse_command(&mut self, operation: IdentId, loc: Loc) -> Result<Node, RubyError> {
        // FNAME ARGS
        // FNAME ARGS DO-BLOCK
        let send_args = self.parse_arglist_block()?;
        Ok(Node::new_send(
            Node::new_self(loc),
            operation,
            send_args,
            false,
            loc,
        ))
    }

    fn parse_arglist_block(&mut self) -> Result<ArgList, RubyError> {
        let mut arglist = self.parse_argument_list(None)?;
        match self.parse_block()? {
            Some(actual_block) => {
                if arglist.block.is_some() {
                    return Err(self.error_unexpected(
                        actual_block.loc(),
                        "Both block arg and actual block given.",
                    ));
                }
                arglist.block = Some(actual_block);
            }
            None => {}
        };
        Ok(arglist)
    }

    fn parse_arg(&mut self) -> Result<Node, RubyError> {
        if self.peek()?.kind == TokenKind::Reserved(Reserved::Defined) {
            self.save_state();
            self.consume_reserved(Reserved::Defined).unwrap();
            if self.peek_punct_no_term(Punct::LParen) {
                self.restore_state();
            } else {
                self.discard_state();
                let node = self.parse_arg()?;
                return Ok(Node::new_defined(node));
            }
        }
        self.parse_arg_assign()
    }

    fn parse_arg_assign(&mut self) -> Result<Node, RubyError> {
        let lhs = self.parse_arg_ternary()?;
        if self.is_line_term()? {
            return Ok(lhs);
        }
        if self.consume_punct_no_term(Punct::Assign)? {
            let mrhs = self.parse_arg_list(None)?;
            self.check_lhs(&lhs)?;
            Ok(Node::new_mul_assign(vec![lhs], mrhs))
        } else if let Some(op) = self.consume_assign_op_no_term()? {
            // <lhs> <assign_op> <arg>
            self.parse_assign_op(lhs, op)
        } else {
            Ok(lhs)
        }
    }

    /// Parse assign-op.
    /// <lhs> <assign_op> <arg>
    fn parse_assign_op(&mut self, mut lhs: Node, op: BinOp) -> Result<Node, RubyError> {
        match op {
            BinOp::LOr | BinOp::LAnd => {
                self.get()?;
                let rhs = self.parse_arg()?;
                self.check_lhs(&lhs)?;
                if let NodeKind::Ident(id) = lhs.kind {
                    lhs = Node::new_lvar(id, lhs.loc());
                };
                let node =
                    Node::new_binop(op, lhs.clone(), Node::new_mul_assign(vec![lhs], vec![rhs]));
                Ok(node)
            }
            _ => {
                self.get()?;
                let rhs = self.parse_arg()?;
                self.check_lhs(&lhs)?;
                Ok(Node::new_mul_assign(
                    vec![lhs.clone()],
                    vec![Node::new_binop(op, lhs, rhs)],
                ))
            }
        }
    }

    /// Check whether `lhs` is a local variable or not.
    fn check_lhs(&mut self, lhs: &Node) -> Result<(), RubyError> {
        if let NodeKind::Ident(id) = lhs.kind {
            self.add_local_var_if_new(id);
        } else if let NodeKind::Const { .. } = lhs.kind {
            for c in self.context_stack.iter().rev() {
                match c.kind {
                    ContextKind::Class => return Ok(()),
                    ContextKind::Method => {
                        return Err(self.error_unexpected(lhs.loc(), "Dynamic constant assignment."))
                    }
                    _ => {}
                }
            }
        };
        Ok(())
    }

    fn parse_arg_ternary(&mut self) -> Result<Node, RubyError> {
        let cond = self.parse_arg_range()?;
        let loc = cond.loc();
        if self.consume_punct_no_term(Punct::Question)? {
            let then_ = self.parse_arg()?;
            if !self.consume_punct_no_term(Punct::Colon)? {
                let loc = self.loc();
                return Err(self.error_unexpected(loc, "Expect ':'."));
            };
            let else_ = self.parse_arg()?;
            let node = Node::new_if(cond, then_, else_, loc);
            Ok(node)
        } else {
            Ok(cond)
        }
    }

    fn parse_arg_range(&mut self) -> Result<Node, RubyError> {
        let lhs = self.parse_arg_logical_or()?;
        if self.is_line_term()? {
            return Ok(lhs);
        }
        if self.consume_punct(Punct::Range2)? {
            let rhs = self.parse_arg_logical_or()?;
            let loc = lhs.loc().merge(rhs.loc());
            Ok(Node::new_range(lhs, rhs, false, loc))
        } else if self.consume_punct(Punct::Range3)? {
            let rhs = self.parse_arg_logical_or()?;
            let loc = lhs.loc().merge(rhs.loc());
            Ok(Node::new_range(lhs, rhs, true, loc))
        } else {
            Ok(lhs)
        }
    }

    fn parse_arg_logical_or(&mut self) -> Result<Node, RubyError> {
        let mut lhs = self.parse_arg_logical_and()?;
        while self.consume_punct_no_term(Punct::LOr)? {
            let rhs = self.parse_arg_logical_and()?;
            lhs = Node::new_binop(BinOp::LOr, lhs, rhs);
        }
        Ok(lhs)
    }

    fn parse_arg_logical_and(&mut self) -> Result<Node, RubyError> {
        let mut lhs = self.parse_arg_eq()?;
        while self.consume_punct_no_term(Punct::LAnd)? {
            let rhs = self.parse_arg_eq()?;
            lhs = Node::new_binop(BinOp::LAnd, lhs, rhs);
        }
        Ok(lhs)
    }

    // 4==4==4 => SyntaxError
    fn parse_arg_eq(&mut self) -> Result<Node, RubyError> {
        let lhs = self.parse_arg_comp()?;
        if self.consume_punct_no_term(Punct::Eq)? {
            let rhs = self.parse_arg_comp()?;
            Ok(Node::new_binop(BinOp::Eq, lhs, rhs))
        } else if self.consume_punct_no_term(Punct::Ne)? {
            let rhs = self.parse_arg_comp()?;
            Ok(Node::new_binop(BinOp::Ne, lhs, rhs))
        } else if self.consume_punct_no_term(Punct::TEq)? {
            let rhs = self.parse_arg_comp()?;
            Ok(Node::new_binop(BinOp::TEq, lhs, rhs))
        } else if self.consume_punct_no_term(Punct::Match)? {
            let rhs = self.parse_arg_comp()?;
            Ok(Node::new_binop(BinOp::Match, lhs, rhs))
        } else if self.consume_punct_no_term(Punct::Unmatch)? {
            let rhs = self.parse_arg_comp()?;
            let loc = lhs.loc().merge(rhs.loc());
            let node = Node::new_binop(BinOp::Match, lhs, rhs);
            Ok(Node::new_unop(UnOp::Not, node, loc))
        } else {
            Ok(lhs)
        }
    }

    fn parse_arg_comp(&mut self) -> Result<Node, RubyError> {
        let mut lhs = self.parse_arg_bitor()?;
        if self.is_line_term()? {
            return Ok(lhs);
        }
        loop {
            if self.consume_punct_no_term(Punct::Ge)? {
                let rhs = self.parse_arg_bitor()?;
                lhs = Node::new_binop(BinOp::Ge, lhs, rhs);
            } else if self.consume_punct_no_term(Punct::Gt)? {
                let rhs = self.parse_arg_bitor()?;
                lhs = Node::new_binop(BinOp::Gt, lhs, rhs);
            } else if self.consume_punct_no_term(Punct::Le)? {
                let rhs = self.parse_arg_bitor()?;
                lhs = Node::new_binop(BinOp::Le, lhs, rhs);
            } else if self.consume_punct_no_term(Punct::Lt)? {
                let rhs = self.parse_arg_bitor()?;
                lhs = Node::new_binop(BinOp::Lt, lhs, rhs);
            } else if self.consume_punct_no_term(Punct::Cmp)? {
                let rhs = self.parse_arg_bitor()?;
                lhs = Node::new_binop(BinOp::Cmp, lhs, rhs);
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_arg_bitor(&mut self) -> Result<Node, RubyError> {
        let mut lhs = self.parse_arg_bitand()?;
        loop {
            if self.consume_punct_no_term(Punct::BitOr)? {
                lhs = Node::new_binop(BinOp::BitOr, lhs, self.parse_arg_bitand()?);
            } else if self.consume_punct_no_term(Punct::BitXor)? {
                lhs = Node::new_binop(BinOp::BitXor, lhs, self.parse_arg_bitand()?);
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_arg_bitand(&mut self) -> Result<Node, RubyError> {
        let mut lhs = self.parse_arg_shift()?;
        loop {
            if self.consume_punct_no_term(Punct::BitAnd)? {
                lhs = Node::new_binop(BinOp::BitAnd, lhs, self.parse_arg_shift()?);
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_arg_shift(&mut self) -> Result<Node, RubyError> {
        let mut lhs = self.parse_arg_add()?;
        loop {
            if self.consume_punct_no_term(Punct::Shl)? {
                lhs = Node::new_binop(BinOp::Shl, lhs, self.parse_arg_add()?);
            } else if self.consume_punct_no_term(Punct::Shr)? {
                lhs = Node::new_binop(BinOp::Shr, lhs, self.parse_arg_add()?);
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_arg_add(&mut self) -> Result<Node, RubyError> {
        let mut lhs = self.parse_arg_mul()?;
        loop {
            if self.consume_punct_no_term(Punct::Plus)? {
                let rhs = self.parse_arg_mul()?;
                lhs = Node::new_binop(BinOp::Add, lhs, rhs);
            } else if self.consume_punct_no_term(Punct::Minus)? {
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
        if self.is_line_term()? {
            return Ok(lhs);
        }
        loop {
            if self.consume_punct_no_term(Punct::Mul)? {
                let rhs = self.parse_unary_minus()?;
                lhs = Node::new_binop(BinOp::Mul, lhs, rhs);
            } else if self.consume_punct_no_term(Punct::Div)? {
                let rhs = self.parse_unary_minus()?;
                lhs = Node::new_binop(BinOp::Div, lhs, rhs);
            } else if self.consume_punct_no_term(Punct::Rem)? {
                let rhs = self.parse_unary_minus()?;
                lhs = Node::new_binop(BinOp::Rem, lhs, rhs);
            } else {
                break;
            }
        }
        Ok(lhs)
    }

    fn parse_unary_minus(&mut self) -> Result<Node, RubyError> {
        self.save_state();
        let lhs = if self.consume_punct(Punct::Minus)? {
            let loc = self.prev_loc();
            match self.peek()?.kind {
                TokenKind::IntegerLit(_) | TokenKind::FloatLit(_) => {
                    self.restore_state();
                    let lhs = self.parse_exponent()?;
                    return Ok(lhs);
                }
                _ => self.discard_state(),
            };
            let lhs = self.parse_unary_minus()?;
            let loc = loc.merge(lhs.loc());
            Node::new_unop(UnOp::Neg, lhs, loc)
        } else {
            self.discard_state();
            self.parse_exponent()?
        };
        match self.parse_accesory_assign(&lhs)? {
            Some(node) => Ok(node),
            None => Ok(lhs),
        }
    }

    fn parse_exponent(&mut self) -> Result<Node, RubyError> {
        let lhs = self.parse_unary()?;
        if self.consume_punct_no_term(Punct::DMul)? {
            let rhs = self.parse_exponent()?;
            Ok(Node::new_binop(BinOp::Exp, lhs, rhs))
        } else {
            Ok(lhs)
        }
    }

    fn parse_unary(&mut self) -> Result<Node, RubyError> {
        if self.consume_punct(Punct::BitNot)? {
            let loc = self.prev_loc();
            let lhs = self.parse_unary()?;
            let lhs = Node::new_unop(UnOp::BitNot, lhs, loc);
            Ok(lhs)
        } else if self.consume_punct(Punct::Not)? {
            let loc = self.prev_loc();
            let lhs = self.parse_unary()?;
            let lhs = Node::new_unop(UnOp::Not, lhs, loc);
            Ok(lhs)
        } else if self.consume_punct(Punct::Plus)? {
            let loc = self.prev_loc();
            let lhs = self.parse_unary()?;
            let lhs = Node::new_unop(UnOp::Pos, lhs, loc);
            Ok(lhs)
        } else {
            let lhs = self.parse_method_call()?;
            Ok(lhs)
        }
    }

    fn parse_method_call(&mut self) -> Result<Node, RubyError> {
        if self.consume_reserved(Reserved::Yield)? {
            return self.parse_yield();
        }
        // 一次式メソッド呼び出し
        // スコープ付き定数参照 :: 一次式 [行終端子禁止][空白類禁止] "::" 定数識別子
        //      ｜"::" 定数識別子
        let mut node = self.parse_primary(false)?;
        loop {
            node = if self.consume_punct(Punct::Dot)? {
                self.parse_primary_method(node, false)?
            } else if self.consume_punct_no_term(Punct::SafeNav)? {
                self.parse_primary_method(node, true)?
            } else if self.consume_punct_no_term(Punct::Scope)? {
                let loc = self.prev_loc();
                if let TokenKind::Const(_) = self.peek()?.kind {
                    let name = self.expect_const()?;
                    Node::new_scope(node, &name, loc)
                } else {
                    self.parse_primary_method(node, false)?
                }
            } else if self.consume_punct_no_term(Punct::LBracket)? {
                let member_loc = self.prev_loc();
                let args = self.parse_arg_list(Punct::RBracket)?;
                let member_loc = member_loc.merge(self.prev_loc());
                Node::new_array_member(node, args, member_loc)
            } else {
                return Ok(node);
            };
        }
    }

    fn parse_function_args(&mut self, node: Node) -> Result<Node, RubyError> {
        let loc = node.loc();
        if self.consume_punct_no_term(Punct::LParen)? {
            // PRIMARY-METHOD : FNAME ( ARGS ) BLOCK?
            let mut send_args = self.parse_argument_list(Punct::RParen)?;
            match self.parse_block()? {
                Some(actual_block) => {
                    if send_args.block.is_some() {
                        return Err(self.error_unexpected(
                            actual_block.loc(),
                            "Both block arg and actual block given.",
                        ));
                    }
                    send_args.block = Some(actual_block);
                }
                None => {}
            };

            Ok(Node::new_send(
                Node::new_self(loc),
                node.as_method_name().unwrap(),
                send_args,
                false,
                loc,
            ))
        } else if let Some(block) = self.parse_block()? {
            // PRIMARY-METHOD : FNAME BLOCK
            Ok(Node::new_send(
                Node::new_self(loc),
                node.as_method_name().unwrap(),
                ArgList::with_block(block),
                false,
                loc,
            ))
        } else {
            Ok(node)
        }
    }

    /// Parse method name.
    /// In primary method call, assign-like method name(cf. foo= or Bar=) is not allowed.
    fn parse_method_name(&mut self) -> Result<(IdentId, Loc), RubyError> {
        let tok = self.get()?;
        let loc = tok.loc();
        let id = match &tok.kind {
            TokenKind::Ident(s) | TokenKind::Const(s) => self.get_ident_id(s),
            TokenKind::Reserved(r) => {
                let s = self.lexer.get_string_from_reserved(*r).to_owned();
                self.get_ident_id(&s)
            }
            TokenKind::Punct(p) => self.parse_op_definable(p)?,
            _ => return Err(self.error_unexpected(tok.loc(), "method name must be an identifier.")),
        };
        Ok((id, loc.merge(self.prev_loc())))
    }

    /// Parse primary method call.
    fn parse_primary_method(&mut self, receiver: Node, safe_nav: bool) -> Result<Node, RubyError> {
        // 一次式メソッド呼出し : 省略可能実引数付きsuper
        //      ｜ 添字メソッド呼出し
        //      ｜ メソッド専用識別子
        //      ｜ メソッド識別子 ブロック
        //      ｜ メソッド識別子 括弧付き実引数 ブロック?
        //      ｜ 一次式 ［行終端子禁止］ "." メソッド名 括弧付き実引数? ブロック?
        //      ｜ 一次式 ［行終端子禁止］ "::" メソッド名 括弧付き実引数 ブロック?
        //      ｜ 一次式 ［行終端子禁止］ "::" 定数以外のメソッド名 ブロック?
        let (id, loc) = self.parse_method_name()?;
        let mut arglist = if !self.consume_punct_no_term(Punct::LParen)? {
            if self.is_command() {
                return Ok(Node::new_send(
                    receiver,
                    id,
                    self.parse_arglist_block()?,
                    false,
                    loc,
                ));
            }
            ArgList::default()
        } else {
            self.parse_argument_list(Punct::RParen)?
        };
        if let Some(block) = self.parse_block()? {
            if arglist.block.is_some() {
                return Err(
                    self.error_unexpected(block.loc(), "Both block arg and actual block given.")
                );
            }
            arglist.block = Some(block);
        };
        let node = match receiver.kind {
            NodeKind::Ident(id) => Node::new_send_noarg(Node::new_self(loc), id, false, loc),
            _ => receiver,
        };
        Ok(Node::new_send(node, id, arglist, safe_nav, loc))
    }

    fn parse_yield(&mut self) -> Result<Node, RubyError> {
        let loc = self.prev_loc();
        let tok = self.peek_no_term()?;
        // TODO: This is not correct.
        if tok.is_term()
            || tok.kind == TokenKind::Reserved(Reserved::Unless)
            || tok.kind == TokenKind::Reserved(Reserved::If)
            || tok.check_stmt_end()
        {
            return Ok(Node::new_yield(ArgList::default(), loc));
        };
        let args = if self.consume_punct(Punct::LParen)? {
            let args = self.parse_arglist_block()?;
            self.expect_punct(Punct::RParen)?;
            args
        } else {
            self.parse_arglist_block()?
        };
        return Ok(Node::new_yield(args, loc));
    }

    fn parse_accesory_assign(&mut self, lhs: &Node) -> Result<Option<Node>, RubyError> {
        if !self.supress_acc_assign {
            if self.consume_punct_no_term(Punct::Assign)? {
                let mrhs = self.parse_mul_assign_rhs()?;
                self.check_lhs(&lhs)?;
                return Ok(Some(Node::new_mul_assign(vec![lhs.clone()], mrhs)));
            } else if let Some(op) = self.consume_assign_op_no_term()? {
                return Ok(Some(self.parse_assign_op(lhs.clone(), op)?));
            }
        };
        Ok(None)
    }

    /// Parse argument list.
    /// arg, *splat_arg, kw: kw_arg, **double_splat_arg, &block <punct>
    /// punct: punctuator for terminating arg list. Set None for unparenthesized argument list.
    fn parse_argument_list(
        &mut self,
        punct: impl Into<Option<Punct>>,
    ) -> Result<ArgList, RubyError> {
        let punct = punct.into();
        let mut arglist = ArgList::default();
        loop {
            if let Some(punct) = punct {
                if self.consume_punct(punct)? {
                    return Ok(arglist);
                }
            }
            if self.consume_punct(Punct::Mul)? {
                // splat argument
                let loc = self.prev_loc();
                let array = self.parse_arg()?;
                arglist.args.push(Node::new_splat(array, loc));
            } else if self.consume_punct(Punct::DMul)? {
                // double splat argument
                arglist.kw_rest.push(self.parse_arg()?);
            } else if self.consume_punct(Punct::BitAnd)? {
                // block argument
                arglist.block = Some(Box::new(self.parse_arg()?));
            } else {
                let node = self.parse_arg()?;
                let loc = node.loc();
                if self.consume_punct(Punct::FatArrow)? {
                    let value = self.parse_arg()?;
                    let mut kvp = vec![(node, value)];
                    if self.consume_punct(Punct::Comma)? {
                        loop {
                            let key = self.parse_arg()?;
                            self.expect_punct(Punct::FatArrow)?;
                            let value = self.parse_arg()?;
                            kvp.push((key, value));
                            if !self.consume_punct(Punct::Comma)? {
                                break;
                            }
                        }
                    }
                    if let Some(punct) = punct {
                        self.consume_punct(punct)?;
                    };
                    let node = Node::new_hash(kvp, loc);
                    arglist.args.push(node);
                    return Ok(arglist);
                }
                match node.kind {
                    NodeKind::Ident(id, ..) | NodeKind::LocalVar(id) => {
                        if self.consume_punct_no_term(Punct::Colon)? {
                            // keyword args
                            arglist.kw_args.push((id, self.parse_arg()?));
                        } else {
                            // positional args
                            arglist.args.push(node);
                        }
                    }
                    _ => {
                        arglist.args.push(node);
                    }
                }
            }
            if !self.consume_punct(Punct::Comma)? {
                break;
            } else {
                let loc = self.prev_loc();
                if arglist.block.is_some() {
                    return Err(self.error_unexpected(loc, "unexpected ','."));
                };
            }
        }
        if let Some(punct) = punct {
            self.consume_punct(punct)?;
        };
        Ok(arglist)
    }

    fn parse_block(&mut self) -> Result<Option<Box<Node>>, RubyError> {
        let old = self.supress_mul_assign;
        self.supress_mul_assign = false;
        let do_flag = if self.consume_reserved_no_skip_line_term(Reserved::Do)? {
            true
        } else {
            if self.consume_punct_no_term(Punct::LBrace)? {
                false
            } else {
                self.supress_mul_assign = old;
                return Ok(None);
            }
        };
        // BLOCK: do [`|' [BLOCK_VAR] `|'] COMPSTMT end
        let loc = self.prev_loc();
        self.context_stack.push(ParseContext::new_block());

        let params = if self.consume_punct(Punct::BitOr)? {
            if self.consume_punct(Punct::BitOr)? {
                vec![]
            } else {
                let params = self.parse_formal_params(TokenKind::Punct(Punct::BitOr))?;
                self.consume_punct(Punct::BitOr)?;
                params
            }
        } else {
            self.consume_punct(Punct::LOr)?;
            vec![]
        };

        let body = self.parse_comp_stmt()?;
        if do_flag {
            self.expect_reserved(Reserved::End)?;
        } else {
            self.expect_punct(Punct::RBrace)?;
        };
        let lvar = self.context_stack.pop().unwrap().lvar;
        let loc = loc.merge(self.prev_loc());
        let node = Node::new_proc(params, body, lvar, loc);
        self.supress_mul_assign = old;
        Ok(Some(Box::new(node)))
    }

    fn alias_name(&mut self) -> Result<Node, RubyError> {
        if self.consume_punct_no_term(Punct::Colon)? {
            self.parse_symbol()
        } else if let TokenKind::GlobalVar(_) = self.peek_no_term()?.kind {
            let tok = self.get()?;
            match &tok.kind {
                TokenKind::GlobalVar(name) => Ok(Node::new_symbol(IdentId::get_id(name), tok.loc)),
                _ => unreachable!(),
            }
        } else {
            Ok(Node::new_symbol(
                self.parse_method_def_name()?,
                self.prev_loc(),
            ))
        }
    }

    fn parse_primary(&mut self, suppress_unparen_call: bool) -> Result<Node, RubyError> {
        let tok = self.get()?;
        let loc = tok.loc();
        match &tok.kind {
            TokenKind::Ident(name) => {
                if self.lexer.trailing_lparen() {
                    let node = Node::new_identifier(name, loc);
                    return Ok(self.parse_function_args(node)?);
                };
                let id = self.get_ident_id(name);
                if self.is_local_var(id) {
                    Ok(Node::new_lvar(id, loc))
                } else {
                    // FUNCTION or COMMAND or LHS for assignment
                    let node = Node::new_identifier(name, loc);
                    if let Ok(tok) = self.peek_no_term() {
                        match tok.kind {
                            // Multiple assignment
                            TokenKind::Punct(Punct::Comma) => return Ok(node),
                            // Method call with block and no args
                            TokenKind::Punct(Punct::LBrace) | TokenKind::Reserved(Reserved::Do) => {
                                return Ok(self.parse_function_args(node)?)
                            }
                            _ => {}
                        }
                    };

                    if !suppress_unparen_call && self.is_command() {
                        Ok(self.parse_command(id, loc)?)
                    } else {
                        Ok(node)
                    }
                }
            }
            TokenKind::InstanceVar(name) => Ok(Node::new_instance_var(name, loc)),
            TokenKind::ClassVar(name) => Ok(Node::new_class_var(name, loc)),
            TokenKind::GlobalVar(name) => Ok(Node::new_global_var(name, loc)),
            TokenKind::Const(name) => {
                if self.lexer.trailing_lparen() {
                    let node = Node::new_identifier(name, loc);
                    self.parse_function_args(node)
                } else if !suppress_unparen_call && self.is_command() {
                    let id = self.get_ident_id(name);
                    Ok(self.parse_command(id, loc)?)
                } else {
                    Ok(Node::new_const(name, false, loc))
                }
            }
            TokenKind::IntegerLit(num) => Ok(Node::new_integer(*num, loc)),
            TokenKind::FloatLit(num) => Ok(Node::new_float(*num, loc)),
            TokenKind::ImaginaryLit(num) => Ok(Node::new_imaginary(*num, loc)),
            TokenKind::StringLit(s) => Ok(self.parse_string_literal(s)?),
            TokenKind::CommandLit(s) => {
                let content = Node::new_string(s.to_owned(), loc);
                Ok(Node::new_command(content))
            }
            TokenKind::OpenString(s, term, level) => {
                Ok(self.parse_interporated_string_literal(s, *term, *level)?)
            }
            TokenKind::OpenCommand(s, term, level) => {
                let content = self.parse_interporated_string_literal(s, *term, *level)?;
                Ok(Node::new_command(content))
            }
            TokenKind::Punct(punct) => match punct {
                Punct::Minus => match self.get()?.kind {
                    TokenKind::IntegerLit(num) => Ok(Node::new_integer(-num, loc)),
                    TokenKind::FloatLit(num) => Ok(Node::new_float(-num, loc)),
                    _ => unreachable!(),
                },
                Punct::LParen => {
                    let node = self.parse_comp_stmt()?;
                    self.expect_punct(Punct::RParen)?;
                    Ok(node)
                }
                Punct::LBracket => {
                    // Array literal
                    let nodes = self.parse_arg_list(Punct::RBracket)?;
                    let loc = loc.merge(self.prev_loc());
                    Ok(Node::new_array(nodes, loc))
                }
                Punct::LBrace => self.parse_hash_literal(),
                Punct::Colon => self.parse_symbol(),
                Punct::Arrow => {
                    // Lambda literal
                    let mut params = vec![];
                    self.context_stack.push(ParseContext::new_block());
                    if self.consume_punct(Punct::LParen)? {
                        if !self.consume_punct(Punct::RParen)? {
                            loop {
                                let id = self.expect_ident()?;
                                params.push(FormalParam::req_param(id, self.prev_loc()));
                                self.new_param(id, self.prev_loc())?;
                                if !self.consume_punct(Punct::Comma)? {
                                    break;
                                }
                            }
                            self.expect_punct(Punct::RParen)?;
                        }
                    } else if let TokenKind::Ident(_) = self.peek()?.kind {
                        let id = self.expect_ident()?;
                        self.new_param(id, self.prev_loc())?;
                        params.push(FormalParam::req_param(id, self.prev_loc()));
                    };
                    self.expect_punct(Punct::LBrace)?;
                    let body = self.parse_comp_stmt()?;
                    self.expect_punct(Punct::RBrace)?;
                    let lvar = self.context_stack.pop().unwrap().lvar;
                    Ok(Node::new_proc(params, body, lvar, loc))
                }
                Punct::Scope => {
                    let name = self.expect_const()?;
                    Ok(Node::new_const(&name, true, loc))
                }
                Punct::Div => self.parse_regexp(),
                Punct::Rem => self.parse_percent_notation(),
                Punct::Question => self.parse_char_literal(),
                Punct::Shl => {
                    if self.lexer.trailing_space() {
                        return Err(
                            self.error_unexpected(loc, r#"Expected '-', '~', '"', '`', or '\''."#)
                        );
                    }
                    let string = self.lexer.read_heredocument()?;
                    let node = Node::new_string(string, loc);
                    Ok(node)
                }
                _ => {
                    return Err(
                        self.error_unexpected(loc, format!("Unexpected token: {:?}", tok.kind))
                    )
                }
            },
            TokenKind::Reserved(reserved) => {
                match reserved {
                    Reserved::If => {
                        let node = self.parse_if_then()?;
                        self.expect_reserved(Reserved::End)?;
                        Ok(node)
                    }
                    Reserved::Unless => {
                        let node = self.parse_unless()?;
                        self.expect_reserved(Reserved::End)?;
                        Ok(node)
                    }
                    Reserved::For => {
                        // for <ident>, .. in <iter>
                        //   COMP_STMT
                        // end
                        //
                        // for <ident>, .. in <iter> do
                        //   COMP_STMT
                        // end
                        //let loc = self.prev_loc();
                        let mut vars = vec![];
                        loop {
                            let var_id = self.expect_ident()?;
                            self.add_local_var_if_new(var_id);
                            vars.push(var_id);
                            if !self.consume_punct(Punct::Comma)? {
                                break;
                            }
                        }
                        self.expect_reserved(Reserved::In)?;
                        let iter = self.parse_expr()?;
                        self.parse_do()?;
                        let loc = self.prev_loc();

                        self.context_stack.push(ParseContext::new_for());
                        let body = self.parse_comp_stmt()?;
                        let mut formal_params = vec![];
                        for (i, _var) in vars.iter().enumerate() {
                            let dummy_var = IdentId::get_id(format!("_{}", i));
                            self.new_param(dummy_var, loc)?;
                            formal_params.push(FormalParam::req_param(dummy_var, loc));
                        }
                        let lvar = self.context_stack.pop().unwrap().lvar;

                        let loc = loc.merge(self.prev_loc());
                        let body = Node::new_proc(formal_params, body, lvar, loc);

                        self.expect_reserved(Reserved::End)?;
                        let node = Node::new(
                            NodeKind::For {
                                param: vars,
                                iter: Box::new(iter),
                                body: Box::new(body),
                            },
                            loc.merge(self.prev_loc()),
                        );
                        Ok(node)
                    }
                    Reserved::While => {
                        let loc = self.prev_loc();
                        let cond = self.parse_expr()?;
                        self.parse_do()?;
                        let body = self.parse_comp_stmt()?;
                        self.expect_reserved(Reserved::End)?;
                        let loc = loc.merge(self.prev_loc());
                        Ok(Node::new_while(cond, body, true, loc))
                    }
                    Reserved::Until => {
                        let loc = self.prev_loc();
                        let cond = self.parse_expr()?;
                        self.parse_do()?;
                        let body = self.parse_comp_stmt()?;
                        self.expect_reserved(Reserved::End)?;
                        let loc = loc.merge(self.prev_loc());
                        Ok(Node::new_while(cond, body, false, loc))
                    }
                    Reserved::Case => {
                        let loc = self.prev_loc();
                        let cond = if self.peek()?.kind != TokenKind::Reserved(Reserved::When) {
                            Some(self.parse_expr()?)
                        } else {
                            None
                        };
                        self.consume_term()?;
                        let mut when_ = vec![];
                        while self.consume_reserved(Reserved::When)? {
                            let arg = self.parse_arg_list(None)?;
                            self.parse_then()?;
                            let body = self.parse_comp_stmt()?;
                            when_.push(CaseBranch::new(arg, body));
                        }
                        let else_ = if self.consume_reserved(Reserved::Else)? {
                            self.parse_comp_stmt()?
                        } else {
                            Node::new_comp_stmt(vec![], self.loc())
                        };
                        self.expect_reserved(Reserved::End)?;
                        Ok(Node::new_case(cond, when_, else_, loc))
                    }
                    Reserved::Def => self.parse_def(),
                    Reserved::Class => {
                        if self.is_method_context() {
                            return Err(self.error_unexpected(
                                loc,
                                "SyntaxError: class definition in method body.",
                            ));
                        }
                        let loc = self.prev_loc();
                        if self.consume_punct(Punct::Shl)? {
                            self.parse_singleton_class(loc)
                        } else {
                            self.parse_class(false)
                        }
                    }
                    Reserved::Module => {
                        if self.is_method_context() {
                            return Err(self.error_unexpected(
                                loc,
                                "SyntaxError: module definition in method body.",
                            ));
                        }
                        self.parse_class(true)
                    }
                    Reserved::Return => {
                        let tok = self.peek_no_term()?;
                        // TODO: This is not correct.
                        if tok.is_term()
                            || tok.kind == TokenKind::Reserved(Reserved::Unless)
                            || tok.kind == TokenKind::Reserved(Reserved::If)
                            || tok.check_stmt_end()
                        {
                            let val = Node::new_nil(loc);
                            return Ok(Node::new_return(val, loc));
                        };
                        let val = self.parse_arg()?;
                        let ret_loc = val.loc();
                        if self.consume_punct_no_term(Punct::Comma)? {
                            let mut vec = vec![val, self.parse_arg()?];
                            while self.consume_punct_no_term(Punct::Comma)? {
                                vec.push(self.parse_arg()?);
                            }
                            let val = Node::new_array(vec, ret_loc);
                            Ok(Node::new_return(val, loc))
                        } else {
                            Ok(Node::new_return(val, loc))
                        }
                    }
                    Reserved::Break => {
                        let tok = self.peek_no_term()?;
                        // TODO: This is not correct.
                        if tok.is_term()
                            || tok.kind == TokenKind::Reserved(Reserved::Unless)
                            || tok.kind == TokenKind::Reserved(Reserved::If)
                            || tok.check_stmt_end()
                        {
                            let val = Node::new_nil(loc);
                            return Ok(Node::new_break(val, loc));
                        };
                        let val = self.parse_arg()?;
                        let ret_loc = val.loc();
                        if self.consume_punct_no_term(Punct::Comma)? {
                            let mut vec = vec![val, self.parse_arg()?];
                            while self.consume_punct_no_term(Punct::Comma)? {
                                vec.push(self.parse_arg()?);
                            }
                            let val = Node::new_array(vec, ret_loc);
                            Ok(Node::new_break(val, loc))
                        } else {
                            Ok(Node::new_break(val, loc))
                        }
                    }
                    Reserved::Next => {
                        let tok = self.peek_no_term()?;
                        // TODO: This is not correct.
                        if tok.is_term()
                            || tok.kind == TokenKind::Reserved(Reserved::Unless)
                            || tok.kind == TokenKind::Reserved(Reserved::If)
                            || tok.check_stmt_end()
                        {
                            let val = Node::new_nil(loc);
                            return Ok(Node::new_next(val, loc));
                        };
                        let val = self.parse_arg()?;
                        let ret_loc = val.loc();
                        if self.consume_punct_no_term(Punct::Comma)? {
                            let mut vec = vec![val, self.parse_arg()?];
                            while self.consume_punct_no_term(Punct::Comma)? {
                                vec.push(self.parse_arg()?);
                            }
                            let val = Node::new_array(vec, ret_loc);
                            Ok(Node::new_next(val, loc))
                        } else {
                            Ok(Node::new_next(val, loc))
                        }
                    }
                    Reserved::True => Ok(Node::new_bool(true, loc)),
                    Reserved::False => Ok(Node::new_bool(false, loc)),
                    Reserved::Nil => Ok(Node::new_nil(loc)),
                    Reserved::Self_ => Ok(Node::new_self(loc)),
                    Reserved::Begin => self.parse_begin(),
                    Reserved::Defined => {
                        if self.consume_punct_no_term(Punct::LParen)? {
                            let node = self.parse_expr()?;
                            self.expect_punct(Punct::RParen)?;
                            Ok(Node::new_defined(node))
                        } else {
                            let tok = self.get()?;
                            Err(self.error_unexpected(tok.loc, format!("expected '('.")))
                        }
                    }
                    Reserved::Alias => {
                        let new_name = self.alias_name()?;
                        let old_name = self.alias_name()?;
                        let loc = loc.merge(self.prev_loc());
                        Ok(Node::new_alias(new_name, old_name, loc))
                    }
                    _ => {
                        Err(self.error_unexpected(loc, format!("Unexpected token: {:?}", tok.kind)))
                    }
                }
            }
            TokenKind::EOF => return Err(self.error_eof(loc)),
            _ => {
                return Err(self.error_unexpected(loc, format!("Unexpected token: {:?}", tok.kind)))
            }
        }
    }
}

impl Parser {
    fn is_command(&mut self) -> bool {
        let tok = match self.peek_no_term() {
            Ok(tok) => tok,
            _ => return false,
        };
        if self.lexer.trailing_space() {
            match tok.kind {
                TokenKind::LineTerm => false,
                TokenKind::Punct(p) => match p {
                    Punct::LParen | Punct::LBracket | Punct::Scope | Punct::Arrow => true,
                    Punct::Colon
                    | Punct::Plus
                    | Punct::Minus
                    | Punct::Mul
                    | Punct::Div
                    | Punct::Rem
                    | Punct::Shl => !self.lexer.has_trailing_space(&tok),
                    _ => false,
                },
                TokenKind::Reserved(r) => match r {
                    Reserved::Do
                    | Reserved::If
                    | Reserved::Unless
                    | Reserved::While
                    | Reserved::Until
                    | Reserved::And
                    | Reserved::Or
                    | Reserved::Then
                    | Reserved::End => false,
                    _ => true,
                },
                _ => true,
            }
        } else {
            match tok.kind {
                TokenKind::GlobalVar(_) => true,
                TokenKind::InstanceVar(_) => true,
                TokenKind::StringLit(_) => true,
                _ => false,
            }
        }
    }

    /// Parse operator which can be defined as a method.
    /// Return IdentId of the operator.
    fn parse_op_definable(&mut self, punct: &Punct) -> Result<IdentId, RubyError> {
        match punct {
            Punct::Plus => Ok(IdentId::_ADD),
            Punct::Minus => Ok(IdentId::_SUB),
            Punct::Mul => Ok(IdentId::_MUL),
            Punct::Shl => Ok(IdentId::_SHL),
            Punct::Shr => Ok(IdentId::_SHR),
            Punct::Cmp => Ok(IdentId::_CMP),
            Punct::Eq => Ok(IdentId::_EQ),
            Punct::Ne => Ok(IdentId::_NEQ),
            Punct::Lt => Ok(IdentId::_LT),
            Punct::Le => Ok(IdentId::_LE),
            Punct::Gt => Ok(IdentId::_GT),
            Punct::Ge => Ok(IdentId::_GE),
            Punct::TEq => Ok(IdentId::_TEQ),
            Punct::Match => Ok(IdentId::get_id("=~")),
            Punct::LBracket => {
                if self.consume_punct_no_term(Punct::RBracket)? {
                    if self.consume_punct_no_term(Punct::Assign)? {
                        Ok(IdentId::_INDEX_ASSIGN)
                    } else {
                        Ok(IdentId::_INDEX)
                    }
                } else {
                    let loc = self.loc();
                    Err(self.error_unexpected(loc, "Invalid operator."))
                }
            }
            _ => Err(self.error_unexpected(self.prev_loc(), "Invalid operator.")),
        }
    }

    /// Parse string literals.
    /// Adjacent string literals are to be combined.
    fn parse_string_literal(&mut self, s: &str) -> Result<Node, RubyError> {
        let loc = self.prev_loc();
        let mut s = s.to_string();
        while let TokenKind::StringLit(next_s) = self.peek_no_term()?.kind {
            self.get()?;
            s = format!("{}{}", s, next_s);
        }
        Ok(Node::new_string(s, loc))
    }

    /// Parse char literals.
    fn parse_char_literal(&mut self) -> Result<Node, RubyError> {
        let loc = self.loc();
        match self.lexer.read_char_literal()?.kind {
            TokenKind::StringLit(s) => Ok(Node::new_string(s, loc.merge(self.prev_loc))),
            _ => unreachable!(),
        }
    }

    /// Parse template (#{..}, #$s, #@a).
    fn parse_template(&mut self, nodes: &mut Vec<Node>) -> Result<(), RubyError> {
        if self.consume_punct(Punct::LBrace)? {
            nodes.push(self.parse_comp_stmt()?);
            if !self.consume_punct(Punct::RBrace)? {
                let loc = self.prev_loc();
                return Err(self.error_unexpected(loc, "Expect '}'"));
            }
        } else {
            let tok = self.get()?;
            let loc = tok.loc();
            let node = match &tok.kind {
                TokenKind::GlobalVar(s) => Node::new_global_var(s, loc),
                TokenKind::InstanceVar(s) => Node::new_instance_var(s, loc),
                _ => unreachable!(format!("{:?}", tok)),
            };
            nodes.push(node);
        };
        Ok(())
    }

    fn parse_interporated_string_literal(
        &mut self,
        s: &str,
        delimiter: char,
        level: usize,
    ) -> Result<Node, RubyError> {
        let start_loc = self.prev_loc();
        let mut nodes = vec![Node::new_string(s.to_string(), start_loc)];
        loop {
            self.parse_template(&mut nodes)?;
            let tok = self
                .lexer
                .read_string_literal_double(None, delimiter, level)?;
            let loc = tok.loc();
            match &tok.kind {
                TokenKind::StringLit(s) => {
                    nodes.push(Node::new_string(s.clone(), loc));
                    return Ok(Node::new_interporated_string(nodes, start_loc.merge(loc)));
                }
                TokenKind::OpenString(s, _, _) => {
                    nodes.push(Node::new_string(s.clone(), loc));
                }
                _ => unreachable!(format!("{:?}", tok)),
            }
        }
    }

    fn parse_regexp(&mut self) -> Result<Node, RubyError> {
        let start_loc = self.prev_loc();
        let tok = self.lexer.get_regexp()?;
        let mut nodes = match tok.kind {
            TokenKind::StringLit(s) => {
                return Ok(Node::new_regexp(
                    vec![Node::new_string(s, tok.loc)],
                    tok.loc,
                ));
            }
            TokenKind::OpenRegex(s) => vec![Node::new_string(s, tok.loc)],
            _ => unreachable!(),
        };
        loop {
            self.parse_template(&mut nodes)?;
            let tok = self.lexer.get_regexp()?;
            let loc = tok.loc();
            match tok.kind {
                TokenKind::StringLit(s) => {
                    nodes.push(Node::new_string(s, loc));
                    return Ok(Node::new_regexp(nodes, start_loc.merge(loc)));
                }
                TokenKind::OpenRegex(s) => {
                    nodes.push(Node::new_string(s, loc));
                }
                _ => unreachable!(),
            }
        }
    }

    fn parse_percent_notation(&mut self) -> Result<Node, RubyError> {
        let tok = self.lexer.get_percent_notation()?;
        let loc = tok.loc;
        if let TokenKind::PercentNotation(kind, content) = tok.kind {
            match kind {
                // TODO: backslash-space must be valid in %w and %i.
                // e.g. "foo\ bar" => "foo bar"
                'w' => {
                    let ary = content
                        .split(|c| c == ' ' || c == '\n')
                        .map(|x| Node::new_string(x.to_string(), loc))
                        .collect();
                    Ok(Node::new_array(ary, tok.loc))
                }
                'i' => {
                    let ary = content
                        .split(|c| c == ' ' || c == '\n')
                        .map(|x| Node::new_symbol(IdentId::get_id(x), loc))
                        .collect();
                    Ok(Node::new_array(ary, tok.loc))
                }
                'r' => {
                    let ary = vec![Node::new_string(content + "-", loc)];
                    Ok(Node::new_regexp(ary, tok.loc))
                }
                _ => return Err(self.error_unexpected(loc, "Unsupported % notation.")),
            }
        } else if let TokenKind::StringLit(s) = tok.kind {
            return Ok(Node::new_string(s, loc));
        } else if let TokenKind::OpenString(s, term, level) = tok.kind {
            let node = self.parse_interporated_string_literal(&s, term, level)?;
            return Ok(node);
        } else {
            unreachable!(format!("parse_percent_notation(): {:?}", tok.kind));
        }
    }

    fn parse_hash_literal(&mut self) -> Result<Node, RubyError> {
        let mut kvp = vec![];
        let loc = self.prev_loc();
        loop {
            if self.consume_punct(Punct::RBrace)? {
                return Ok(Node::new_hash(kvp, loc.merge(self.prev_loc())));
            };
            let ident_loc = self.loc();
            let mut symbol_flag = false;
            let key = if self.peek()?.can_be_symbol() {
                self.save_state();
                let token = self.get()?.clone();
                let ident = self.token_as_symbol(&token);
                if self.consume_punct(Punct::Colon)? {
                    self.discard_state();
                    let id = self.get_ident_id(&ident);
                    symbol_flag = true;
                    Node::new_symbol(id, ident_loc)
                } else {
                    self.restore_state();
                    self.parse_arg()?
                }
            } else {
                self.parse_arg()?
            };
            if !symbol_flag {
                self.expect_punct(Punct::FatArrow)?
            };
            let value = self.parse_arg()?;
            kvp.push((key, value));
            if !self.consume_punct(Punct::Comma)? {
                break;
            };
        }
        self.expect_punct(Punct::RBrace)?;
        Ok(Node::new_hash(kvp, loc.merge(self.prev_loc())))
    }

    fn parse_symbol(&mut self) -> Result<Node, RubyError> {
        let loc = self.prev_loc();
        if self.lexer.trailing_space() {
            return Err(self.error_unexpected(loc, "Unexpected ':'."));
        }
        // Symbol literal
        let token = self.get()?;
        let symbol_loc = self.prev_loc();
        let id = match &token.kind {
            TokenKind::Punct(punct) => self.parse_op_definable(punct)?,
            TokenKind::Const(s) | TokenKind::Ident(s) => self.method_def_ext(s)?,
            _ if token.can_be_symbol() => {
                let ident = self.token_as_symbol(&token);
                self.get_ident_id(&ident)
            }
            TokenKind::OpenString(s, term, level) => {
                let node = self.parse_interporated_string_literal(&s, *term, *level)?;
                let method = self.get_ident_id("to_sym");
                let loc = symbol_loc.merge(node.loc());
                return Ok(Node::new_send_noarg(node, method, false, loc));
            }
            _ => {
                return Err(self.error_unexpected(symbol_loc, "Expect identifier or string."));
            }
        };
        Ok(Node::new_symbol(id, loc.merge(self.prev_loc())))
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
        let else_ = if self.consume_reserved(Reserved::Elsif)? {
            self.parse_if_then()?
        } else if self.consume_reserved(Reserved::Else)? {
            self.parse_comp_stmt()?
        } else {
            Node::new_comp_stmt(vec![], self.loc())
        };
        Ok(Node::new_if(cond, then_, else_, loc))
    }

    fn parse_unless(&mut self) -> Result<Node, RubyError> {
        //  unless EXPR THEN
        //      COMPSTMT
        //      [else COMPSTMT]
        //  end
        let loc = self.prev_loc();
        let cond = self.parse_expr()?;
        self.parse_then()?;
        let then_ = self.parse_comp_stmt()?;
        let else_ = if self.consume_reserved(Reserved::Else)? {
            self.parse_comp_stmt()?
        } else {
            Node::new_comp_stmt(vec![], self.loc())
        };
        Ok(Node::new_if(cond, else_, then_, loc))
    }

    fn parse_then(&mut self) -> Result<(), RubyError> {
        if self.consume_term()? {
            self.consume_reserved(Reserved::Then)?;
            return Ok(());
        }
        self.expect_reserved(Reserved::Then)?;
        Ok(())
    }

    fn parse_do(&mut self) -> Result<(), RubyError> {
        if self.consume_term()? {
            return Ok(());
        }
        self.expect_reserved(Reserved::Do)?;
        Ok(())
    }

    /// Check method name extension.
    /// Parse "xxxx!" as a valid mathod name.
    /// "xxxx!=" or "xxxx?=" is invalid.
    fn method_def_ext(&mut self, s: &str) -> Result<IdentId, RubyError> {
        let id = if !self.lexer.trailing_space()
            && !(s.ends_with(&['!', '?'][..]))
            && self.consume_punct_no_term(Punct::Assign)?
        {
            self.get_ident_id(&format!("{}=", s))
        } else {
            self.get_ident_id(s)
        };
        Ok(id)
    }

    /// Parse method definition name.
    fn parse_method_def_name(&mut self) -> Result<IdentId, RubyError> {
        // メソッド定義名 : メソッド名 ｜ ( 定数識別子 | 局所変数識別子 ) "="
        // メソッド名 : 局所変数識別子
        //      | 定数識別子
        //      | ( 定数識別子 | 局所変数識別子 ) ( "!" | "?" )
        //      | 演算子メソッド名
        //      | キーワード
        // 演算子メソッド名 : “^” | “&” | “|” | “<=>” | “==” | “===” | “=~” | “>” | “>=” | “<” | “<=”
        //      | “<<” | “>>” | “+” | “-” | “*” | “/” | “%” | “**” | “~” | “+@” | “-@” | “[]” | “[]=” | “ʻ”
        let tok = self.get()?;
        let id = match tok.kind {
            TokenKind::Reserved(r) => {
                let s = self.lexer.get_string_from_reserved(r).to_owned();
                self.method_def_ext(&s)?
            }
            TokenKind::Ident(name) | TokenKind::Const(name) => self.method_def_ext(&name)?,
            TokenKind::Punct(p) => self.parse_op_definable(&p)?,
            _ => {
                let loc = tok.loc.merge(self.prev_loc());
                return Err(self.error_unexpected(loc, "Expected identifier or operator."));
            }
        };
        Ok(id)
    }

    /// Parse method definition.
    fn parse_def(&mut self) -> Result<Node, RubyError> {
        // メソッド定義

        // 特異メソッド定義
        // ( 変数参照 | "(" 式 ")" ) ( "." | "::" ) メソッド定義名
        // 変数参照 : 定数識別子 | 大域変数識別子 | クラス変数識別子 | インスタンス変数識別子 | 局所変数識別子 | 擬似変数

        let tok = self.get()?;
        let loc = tok.loc;
        let (singleton, name) = match &tok.kind {
            TokenKind::GlobalVar(name) => {
                self.consume_punct_no_term(Punct::Dot)?;
                (
                    Some(Node::new_global_var(name, loc)),
                    self.parse_method_def_name()?,
                )
            }
            TokenKind::InstanceVar(name) => {
                self.consume_punct_no_term(Punct::Dot)?;
                (
                    Some(Node::new_instance_var(name, loc)),
                    self.parse_method_def_name()?,
                )
            }
            TokenKind::Reserved(Reserved::Self_) => {
                self.consume_punct_no_term(Punct::Dot)?;
                (Some(Node::new_self(loc)), self.parse_method_def_name()?)
            }
            TokenKind::Reserved(r) => {
                let s = self.lexer.get_string_from_reserved(*r).to_owned();
                (None, self.method_def_ext(&s)?)
            }
            TokenKind::Ident(s) => {
                if self.consume_punct_no_term(Punct::Dot)? {
                    let id = IdentId::get_id(s);
                    (Some(Node::new_lvar(id, loc)), self.parse_method_def_name()?)
                } else {
                    (None, self.method_def_ext(s)?)
                }
            }
            TokenKind::Const(s) => {
                if self.consume_punct_no_term(Punct::Dot)? {
                    (
                        Some(Node::new_const(s, false, loc)),
                        self.parse_method_def_name()?,
                    )
                } else {
                    (None, self.method_def_ext(s)?)
                }
            }
            TokenKind::Punct(p) => (None, self.parse_op_definable(p)?),
            _ => return Err(self.error_unexpected(loc, "Invalid method name.")),
        };

        self.context_stack.push(ParseContext::new_method());
        let args = self.parse_def_params()?;
        let body = self.parse_begin()?;
        let lvar = self.context_stack.pop().unwrap().lvar;
        #[cfg(feature = "verbose")]
        {
            match &singleton {
                Some(singleton) => {
                    eprintln!("parsed: method {:?} singleton {:?}", name, singleton.kind)
                }
                None => eprintln!("parsed: method {:?}", name),
            }
        }
        match singleton {
            Some(singleton) => Ok(Node::new_singleton_method_decl(
                singleton, name, args, body, lvar,
            )),
            None => Ok(Node::new_method_decl(name, args, body, lvar)),
        }
    }

    /// Parse formal parameters.
    /// required, optional = defaule, *rest, post_required, kw: default, **rest_kw, &block
    fn parse_formal_params(
        &mut self,
        terminator: TokenKind,
    ) -> Result<Vec<FormalParam>, RubyError> {
        #[derive(Debug, Clone, PartialEq, PartialOrd)]
        enum Kind {
            Reqired,
            Optional,
            Rest,
            PostReq,
            KeyWord,
            KWRest,
        }

        let mut args = vec![];
        let mut state = Kind::Reqired;
        loop {
            let mut loc = self.loc();
            if self.consume_punct(Punct::BitAnd)? {
                // Block param
                let id = self.expect_ident()?;
                loc = loc.merge(self.prev_loc());
                args.push(FormalParam::block(id, loc));
                self.new_block_param(id, loc)?;
                break;
            } else if self.consume_punct(Punct::Mul)? {
                // Splat(Rest) param
                loc = loc.merge(self.prev_loc());
                if state >= Kind::Rest {
                    return Err(self
                        .error_unexpected(loc, "Splat parameter is not allowed in ths position."));
                } else {
                    state = Kind::Rest;
                };
                match self.consume_ident()? {
                    Some(id) => {
                        args.push(FormalParam::rest(id, loc));
                        self.new_param(id, self.prev_loc())?;
                    }
                    None => args.push(FormalParam::rest_discard(loc)),
                }
            } else if self.consume_punct(Punct::DMul)? {
                // Keyword rest param
                let id = self.expect_ident()?;
                loc = loc.merge(self.prev_loc());
                if state >= Kind::KWRest {
                    return Err(self.error_unexpected(
                        loc,
                        "Keyword rest parameter is not allowed in ths position.",
                    ));
                } else {
                    state = Kind::KWRest;
                }

                args.push(FormalParam::kwrest(id, loc));
                self.new_kwrest_param(id, self.prev_loc())?;
            } else {
                let id = self.expect_ident()?;
                if self.consume_punct(Punct::Assign)? {
                    // Optional param
                    let default = self.parse_arg()?;
                    loc = loc.merge(self.prev_loc());
                    match state {
                        Kind::Reqired => state = Kind::Optional,
                        Kind::Optional => {}
                        _ => {
                            return Err(self.error_unexpected(
                                loc,
                                "Optional parameter is not allowed in ths position.",
                            ))
                        }
                    };
                    args.push(FormalParam::optional(id, default, loc));
                    self.new_param(id, loc)?;
                } else if self.consume_punct_no_term(Punct::Colon)? {
                    // Keyword param
                    let next = self.peek_no_term()?.kind;
                    let default = if next == TokenKind::Punct(Punct::Comma)
                        || next == terminator
                        || next == TokenKind::LineTerm
                    {
                        None
                    } else {
                        Some(self.parse_arg()?)
                    };
                    loc = loc.merge(self.prev_loc());
                    if state == Kind::KWRest {
                        return Err(self.error_unexpected(
                            loc,
                            "Keyword parameter is not allowed in ths position.",
                        ));
                    } else {
                        state = Kind::KeyWord;
                    };
                    args.push(FormalParam::keyword(id, default, loc));
                    self.new_param(id, loc)?;
                } else {
                    // Required param
                    loc = self.prev_loc();
                    match state {
                        Kind::Reqired => {
                            args.push(FormalParam::req_param(id, loc));
                            self.new_param(id, loc)?;
                        }
                        Kind::PostReq | Kind::Optional | Kind::Rest => {
                            args.push(FormalParam::post(id, loc));
                            self.new_param(id, loc)?;
                            state = Kind::PostReq;
                        }
                        _ => {
                            return Err(self.error_unexpected(
                                loc,
                                "Required parameter is not allowed in ths position.",
                            ))
                        }
                    }
                };
            }
            if !self.consume_punct_no_term(Punct::Comma)? {
                break;
            }
        }
        Ok(args)
    }

    // ( )
    // ( ident [, ident]* )
    fn parse_def_params(&mut self) -> Result<Vec<FormalParam>, RubyError> {
        if self.consume_term()? {
            return Ok(vec![]);
        };
        let paren_flag = self.consume_punct(Punct::LParen)?;

        if paren_flag && self.consume_punct(Punct::RParen)? {
            self.consume_term()?;
            return Ok(vec![]);
        }

        let args = self.parse_formal_params(TokenKind::Punct(Punct::RParen))?;

        if paren_flag {
            self.expect_punct(Punct::RParen)?
        };
        self.consume_term()?;
        Ok(args)
    }

    fn parse_class_def_name(&mut self) -> Result<Node, RubyError> {
        // クラスパス : "::" 定数識別子
        //      ｜ 定数識別子
        //      ｜ 一次式 [行終端子禁止] "::" 定数識別子
        let mut node = self.parse_primary(true)?;
        loop {
            node = if self.consume_punct(Punct::Dot)? {
                self.parse_primary_method(node, false)?
            } else if self.consume_punct_no_term(Punct::Scope)? {
                let loc = self.prev_loc();
                let name = self.expect_const()?;
                Node::new_scope(node, &name, loc)
            } else {
                return Ok(node);
            };
        }
    }

    /// Parse class definition.
    fn parse_class(&mut self, is_module: bool) -> Result<Node, RubyError> {
        // クラス定義 : "class" クラスパス [行終端子禁止] ("<" 式)? 分離子 本体文 "end"
        // クラスパス : "::" 定数識別子
        //      ｜ 定数識別子
        //      ｜ 一次式 [行終端子禁止] "::" 定数識別子
        let loc = self.prev_loc();
        let prim = self.parse_class_def_name()?;
        let (base, name) = match prim.kind {
            NodeKind::Const { toplevel: true, id } if !self.peek_punct_no_term(Punct::Scope) => {
                (Node::new_nil(loc), id)
            }
            NodeKind::Const {
                toplevel: false,
                id,
                ..
            } if !self.peek_punct_no_term(Punct::Scope) => (Node::new_nil(loc), id),
            NodeKind::Scope(base, id) => (*base, id),
            _ => return Err(self.error_unexpected(prim.loc, "Invalid Class/Module name.")),
        };
        //eprintln!("base:{:?} name:{:?}", base, name);

        #[cfg(feature = "verbose")]
        eprintln!(
            "***parsing.. {} {:?}",
            if is_module { "module" } else { "class" },
            name
        );
        let superclass = if self.consume_punct_no_term(Punct::Lt)? {
            if is_module {
                return Err(self.error_unexpected(self.prev_loc(), "Unexpected '<'."));
            };
            self.parse_expr()?
        } else {
            let loc = loc.merge(self.prev_loc());
            Node::new_nil(loc)
        };
        let loc = loc.merge(self.prev_loc());
        self.consume_term()?;
        self.context_stack.push(ParseContext::new_class(None));
        let body = self.parse_begin()?;
        let lvar = self.context_stack.pop().unwrap().lvar;
        #[cfg(feature = "verbose")]
        eprintln!("***parsed");
        Ok(Node::new_class_decl(
            base, name, superclass, body, lvar, is_module, loc,
        ))
    }

    /// Parse singleton class definition.
    fn parse_singleton_class(&mut self, loc: Loc) -> Result<Node, RubyError> {
        // class "<<" EXPR <term>
        //      COMPSTMT
        // end
        let singleton = self.parse_expr()?;
        #[cfg(feature = "verbose")]
        eprintln!("***parsing.. singleton class {:?}", singleton.kind);
        let loc = loc.merge(self.prev_loc());
        self.consume_term()?;
        self.context_stack.push(ParseContext::new_class(None));
        let body = self.parse_begin()?;
        let lvar = self.context_stack.pop().unwrap().lvar;
        #[cfg(feature = "verbose")]
        eprintln!("***parsed");
        Ok(Node::new_singleton_class_decl(singleton, body, lvar, loc))
    }

    fn parse_begin(&mut self) -> Result<Node, RubyError> {
        // begin式 :: "begin"  複合文  rescue節*  else節?  ensure節?  "end"
        // rescue節 :: "rescue" [行終端子禁止] 例外クラスリスト?  例外変数代入?  then節
        // 例外クラスリスト :: 演算子式 | 多重代入右辺
        // 例外変数代入 :: "=>" 左辺
        // ensure節 :: "ensure" 複合文
        let body = self.parse_comp_stmt()?;
        let mut rescue = vec![];
        loop {
            if !self.consume_reserved(Reserved::Rescue)? {
                break;
            };
            let mut exception = vec![];
            let mut assign = None;
            if !self.consume_term()? {
                loop {
                    if self.peek_punct_no_term(Punct::FatArrow) {
                        break;
                    }
                    exception.push(self.parse_arg()?);
                    if !self.consume_punct_no_term(Punct::Comma)? {
                        break;
                    };
                }
                if self.consume_punct_no_term(Punct::FatArrow)? {
                    let lhs = self.parse_primary(true)?;
                    self.check_lhs(&lhs)?;
                    assign = Some(lhs);
                }
                self.parse_then()?;
            };
            let rescue_body = self.parse_comp_stmt()?;
            rescue.push(RescueEntry::new(exception, assign, rescue_body));
        }
        let else_ = if self.consume_reserved(Reserved::Else)? {
            Some(self.parse_comp_stmt()?)
        } else {
            None
        };
        let ensure = if self.consume_reserved(Reserved::Ensure)? {
            Some(self.parse_comp_stmt()?)
        } else {
            None
        };
        self.expect_reserved(Reserved::End)?;
        let loc = body.loc();
        Ok(Node::new_begin(body, rescue, else_, ensure, loc))
    }
}
