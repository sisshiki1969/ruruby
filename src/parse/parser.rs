use super::lexer::ParseErr;
use super::*;
use crate::error::ParseErrKind;
use crate::error::RubyError;
use crate::id_table::IdentId;
use crate::util::*;
use crate::vm::context::{ContextRef, ISeqKind};
use fxhash::FxHashMap;
use std::path::PathBuf;

mod define;
mod flow_control;
mod literals;
mod statement;

#[derive(Debug, Clone, PartialEq)]
pub struct Parser<'a> {
    pub lexer: Lexer<'a>,
    pub path: PathBuf,
    prev_loc: Loc,
    context_stack: Vec<ParseContext>,
    extern_context: Option<ContextRef>,
    /// this flag suppress accesory assignment. e.g. x=3
    suppress_acc_assign: bool,
    /// this flag suppress accesory multiple assignment. e.g. x = 2,3
    suppress_mul_assign: bool,
    /// this flag suppress parse do-end style block.
    suppress_do_block: bool,
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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LvarCollector {
    id: usize,
    pub optkw: Vec<LvarId>,
    table: FxHashMap<IdentId, LvarId>,
    kwrest: Option<LvarId>,
    block: Option<LvarId>,
}

impl LvarCollector {
    /// Create new `LvarCollector`.
    pub fn new() -> Self {
        LvarCollector {
            id: 0,
            optkw: vec![],
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
        let lvar = self.insert_new(val)?;
        self.block = Some(lvar);
        Some(lvar)
    }

    fn insert_kwrest_param(&mut self, val: IdentId) -> Option<LvarId> {
        let lvar = self.insert_new(val)?;
        self.kwrest = Some(lvar);
        Some(lvar)
    }

    pub fn get(&self, val: &IdentId) -> Option<&LvarId> {
        self.table.get(val)
    }

    pub fn get_name_id(&self, id: LvarId) -> Option<IdentId> {
        for (k, v) in self.table.iter() {
            if *v == id {
                return Some(*k);
            }
        }
        None
    }

    pub fn get_name(&self, id: LvarId) -> String {
        match self.get_name_id(id) {
            Some(id) => format!("{:?}", id),
            None => "<unnamed>".to_string(),
        }
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
    name: Option<IdentId>,
}

impl ParseContext {
    fn new_method(name: IdentId) -> Self {
        ParseContext {
            lvar: LvarCollector::new(),
            kind: ContextKind::Method,
            name: Some(name),
        }
    }
    fn new_class(name: IdentId, lvar_collector: Option<LvarCollector>) -> Self {
        ParseContext {
            lvar: lvar_collector.unwrap_or(LvarCollector::new()),
            kind: ContextKind::Class,
            name: Some(name),
        }
    }
    fn new_block() -> Self {
        ParseContext {
            lvar: LvarCollector::new(),
            kind: ContextKind::Block,
            name: None,
        }
    }
    fn new_for() -> Self {
        ParseContext {
            lvar: LvarCollector::new(),
            kind: ContextKind::For,
            name: None,
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

    pub fn new_postfix(body: Node) -> Self {
        Self {
            exception_list: vec![],
            assign: None,
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

impl<'a> Parser<'a> {
    pub fn new(code: &'a str, path: PathBuf) -> Self {
        let lexer = Lexer::new(code);
        Parser {
            lexer,
            path,
            prev_loc: Loc(0, 0),
            context_stack: vec![],
            extern_context: None,
            suppress_acc_assign: false,
            suppress_mul_assign: false,
            suppress_do_block: false,
        }
    }

    pub fn new_with_range(&self, pos: usize, end: usize) -> Self {
        let lexer = self.lexer.new_with_range(pos, end);
        Parser {
            lexer,
            path: self.path.clone(),
            prev_loc: Loc(0, 0),
            context_stack: vec![],
            extern_context: None,
            suppress_acc_assign: false,
            suppress_mul_assign: false,
            suppress_do_block: false,
        }
    }

    fn save_state(&self) -> (usize, usize) {
        self.lexer.save_state()
    }

    fn restore_state(&mut self, state: (usize, usize)) {
        self.lexer.restore_state(state);
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
                        return;
                    }
                };
            }
        }
    }

    /// Add the `id` as a new parameter in the current context.
    /// If a parameter with the same name already exists, return error.
    fn new_param(&mut self, id: IdentId, loc: Loc) -> Result<LvarId, ParseErr> {
        match self.context_mut().lvar.insert_new(id) {
            Some(lvar) => Ok(lvar),
            None => Err(Self::error_unexpected(loc, "Duplicated argument name.")),
        }
    }

    fn add_kwopt_param(&mut self, lvar: LvarId) {
        self.context_mut().lvar.optkw.push(lvar);
    }

    /// Add the `id` as a new parameter in the current context.
    /// If a parameter with the same name already exists, return error.
    fn new_kwrest_param(&mut self, id: IdentId, loc: Loc) -> Result<(), ParseErr> {
        if self.context_mut().lvar.insert_kwrest_param(id).is_none() {
            return Err(Self::error_unexpected(loc, "Duplicated argument name."));
        }
        Ok(())
    }

    /// Add the `id` as a new block parameter in the current context.
    /// If a parameter with the same name already exists, return error.
    fn new_block_param(&mut self, id: IdentId, loc: Loc) -> Result<(), ParseErr> {
        if self.context_mut().lvar.insert_block_param(id).is_none() {
            return Err(Self::error_unexpected(loc, "Duplicated argument name."));
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
            let iseq = ctx.iseq_ref.unwrap();
            if iseq.lvar.table.contains_key(&id) {
                return true;
            };
            if let ISeqKind::Method(_) = iseq.kind {
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
    fn peek(&mut self) -> Result<Token, ParseErr> {
        self.lexer.peek_token_skip_lt()
    }

    /// Peek next token (no skipping line terminators).
    fn peek_no_term(&mut self) -> Result<Token, ParseErr> {
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
    fn is_line_term(&mut self) -> Result<bool, ParseErr> {
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
    fn get(&mut self) -> Result<Token, ParseErr> {
        loop {
            let tok = self.lexer.get_token()?;
            if tok.is_eof() {
                return Err(Self::error_eof(tok.loc()));
            }
            if !tok.is_line_term() {
                self.prev_loc = tok.loc;
                return Ok(tok);
            }
        }
    }

    /// Get next token (no skipping line terminators).
    fn get_no_skip_line_term(&mut self) -> Result<Token, ParseErr> {
        let tok = self.lexer.get_token()?;
        self.prev_loc = tok.loc;
        Ok(tok)
    }

    /// If the next token is Ident, consume and return Some(it).
    /// If not, return None.
    fn consume_ident(&mut self) -> Result<Option<IdentId>, ParseErr> {
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
    fn consume_punct(&mut self, expect: Punct) -> Result<bool, ParseErr> {
        match self.peek()?.kind {
            TokenKind::Punct(punct) if punct == expect => {
                self.get()?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn consume_punct_no_term(&mut self, expect: Punct) -> Result<bool, ParseErr> {
        if TokenKind::Punct(expect) == self.peek_no_term()?.kind {
            self.get()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn consume_assign_op_no_term(&mut self) -> Result<Option<BinOp>, ParseErr> {
        if let TokenKind::Punct(Punct::AssignOp(op)) = self.peek_no_term()?.kind {
            Ok(Some(op))
        } else {
            Ok(None)
        }
    }

    /// If next token is an expected kind of Reserved keyeord, get it and return true.
    /// Otherwise, return false.
    fn consume_reserved(&mut self, expect: Reserved) -> Result<bool, ParseErr> {
        match self.peek()?.kind {
            TokenKind::Reserved(reserved) if reserved == expect => {
                self.get()?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn consume_reserved_no_skip_line_term(&mut self, expect: Reserved) -> Result<bool, ParseErr> {
        if TokenKind::Reserved(expect) == self.peek_no_term()?.kind {
            self.get()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get the next token if it is a line terminator or ';' or EOF, and return true,
    /// Otherwise, return false.
    fn consume_term(&mut self) -> Result<bool, ParseErr> {
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
    fn expect_reserved(&mut self, expect: Reserved) -> Result<(), ParseErr> {
        match &self.get()?.kind {
            TokenKind::Reserved(reserved) if *reserved == expect => Ok(()),
            t => Err(Self::error_unexpected(
                self.prev_loc(),
                format!("Expect {:?} Got {:?}", expect, t),
            )),
        }
    }

    /// Get the next token and examine whether it is an expected Punct.
    /// If not, return RubyError.
    fn expect_punct(&mut self, expect: Punct) -> Result<(), ParseErr> {
        match &self.get()?.kind {
            TokenKind::Punct(punct) if *punct == expect => Ok(()),
            t => Err(Self::error_unexpected(
                self.prev_loc(),
                format!("Expect {:?} Got {:?}", expect, t),
            )),
        }
    }

    /// Get the next token and examine whether it is Ident.
    /// Return IdentId of the Ident.
    /// If not, return RubyError.
    fn expect_ident(&mut self) -> Result<IdentId, ParseErr> {
        match &self.get()?.kind {
            TokenKind::Ident(s) => Ok(self.get_ident_id(s)),
            _ => Err(Self::error_unexpected(
                self.prev_loc(),
                "Expect identifier.",
            )),
        }
    }

    /// Get the next token and examine whether it is Const.
    /// Return IdentId of the Const.
    /// If not, return RubyError.
    fn expect_const(&mut self) -> Result<String, ParseErr> {
        match self.get()?.kind {
            TokenKind::Const(s) => Ok(s),
            _ => Err(Self::error_unexpected(self.prev_loc(), "Expect constant.")),
        }
    }
}

impl<'a> Parser<'a> {
    fn error_unexpected(loc: Loc, msg: impl Into<String>) -> ParseErr {
        ParseErr(ParseErrKind::SyntaxError(msg.into()), loc)
    }

    fn error_eof(loc: Loc) -> ParseErr {
        ParseErr(ParseErrKind::UnexpectedEOF, loc)
    }
}

impl<'a> Parser<'a> {
    pub fn parse_program(code: String, path: PathBuf) -> Result<ParseResult, RubyError> {
        let parse_ctx = ParseContext::new_class(IdentId::get_id("Top"), None);
        Self::parse(code, path, None, parse_ctx)
    }

    pub fn parse_program_repl(
        code: String,
        path: PathBuf,
        extern_context: ContextRef,
    ) -> Result<ParseResult, RubyError> {
        let parse_ctx = ParseContext::new_class(
            IdentId::get_id("REPL"),
            Some(extern_context.iseq_ref.unwrap().lvar.clone()),
        );
        Self::parse(code, path, Some(extern_context), parse_ctx)
    }

    pub fn parse_program_eval(
        code: String,
        path: PathBuf,
        extern_context: Option<ContextRef>,
    ) -> Result<ParseResult, RubyError> {
        Self::parse(code, path, extern_context, ParseContext::new_block())
    }

    fn parse(
        code: String,
        path: PathBuf,
        extern_context: Option<ContextRef>,
        parse_context: ParseContext,
    ) -> Result<ParseResult, RubyError> {
        let (node, lvar, tok) =
            match Self::parse_sub(&code, path.clone(), extern_context, parse_context) {
                Ok(ok) => ok,
                Err(err) => {
                    let source_info = SourceInfoRef::from_code(path, code);
                    return Err(RubyError::new_parse_err(err.0, source_info, err.1));
                }
            };
        #[cfg(feature = "emit-ast")]
        eprintln!("{:#?}", node);
        let source_info = SourceInfoRef::from_code(path, code);
        if tok.is_eof() {
            let result = ParseResult::default(node, lvar, source_info);
            Ok(result)
        } else {
            let err = Self::error_unexpected(tok.loc(), "Expected end-of-input.");
            Err(RubyError::new_parse_err(err.0, source_info, err.1))
        }
    }

    fn parse_sub(
        code: &str,
        path: PathBuf,
        extern_context: Option<ContextRef>,
        parse_context: ParseContext,
    ) -> Result<(Node, LvarCollector, Token), ParseErr> {
        let mut parser = Parser::new(&code, path);
        parser.extern_context = extern_context;
        parser.context_stack.push(parse_context);
        let node = parser.parse_comp_stmt()?;
        let lvar = parser.context_stack.pop().unwrap().lvar;
        let tok = parser.peek()?;
        Ok((node, lvar, tok))
    }

    fn parse_arglist_block(
        &mut self,
        delimiter: impl Into<Option<Punct>>,
    ) -> Result<ArgList, ParseErr> {
        let mut arglist = self.parse_argument_list(delimiter)?;
        match self.parse_block()? {
            Some(actual_block) => {
                if arglist.block.is_some() {
                    return Err(Self::error_unexpected(
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

    /// Check whether `lhs` is a local variable or not.
    fn check_lhs(&mut self, lhs: &Node) -> Result<(), ParseErr> {
        if let NodeKind::Ident(id) = lhs.kind {
            self.add_local_var_if_new(id);
        } else if let NodeKind::Const { .. } = lhs.kind {
            for c in self.context_stack.iter().rev() {
                match c.kind {
                    ContextKind::Class => return Ok(()),
                    ContextKind::Method => {
                        return Err(Self::error_unexpected(
                            lhs.loc(),
                            "Dynamic constant assignment.",
                        ))
                    }
                    _ => {}
                }
            }
        };
        Ok(())
    }

    fn parse_function_args(&mut self, node: Node) -> Result<Node, ParseErr> {
        let loc = node.loc();
        if self.consume_punct_no_term(Punct::LParen)? {
            // PRIMARY-METHOD : FNAME ( ARGS ) BLOCK?
            let send_args = self.parse_arglist_block(Punct::RParen)?;

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

    /// Parse argument list.
    /// arg, *splat_arg, kw: kw_arg, **double_splat_arg, &block <punct>
    /// punct: punctuator for terminating arg list. Set None for unparenthesized argument list.
    fn parse_argument_list(
        &mut self,
        punct: impl Into<Option<Punct>>,
    ) -> Result<ArgList, ParseErr> {
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
                    return Err(Self::error_unexpected(loc, "unexpected ','."));
                };
            }
        }
        if let Some(punct) = punct {
            self.consume_punct(punct)?;
        };
        Ok(arglist)
    }

    /// Parse block.
    ///     do |x| stmt end
    ///     { |x| stmt }
    fn parse_block(&mut self) -> Result<Option<Box<Node>>, ParseErr> {
        let old_suppress_mul_flag = self.suppress_mul_assign;
        self.suppress_mul_assign = false;
        let do_flag =
            if !self.suppress_do_block && self.consume_reserved_no_skip_line_term(Reserved::Do)? {
                true
            } else {
                if self.consume_punct_no_term(Punct::LBrace)? {
                    false
                } else {
                    self.suppress_mul_assign = old_suppress_mul_flag;
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
        self.suppress_mul_assign = old_suppress_mul_flag;
        Ok(Some(Box::new(node)))
    }
}

impl<'a> Parser<'a> {
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
    fn parse_op_definable(&mut self, punct: &Punct) -> Result<IdentId, ParseErr> {
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
                    Err(Self::error_unexpected(loc, "Invalid operator."))
                }
            }
            _ => Err(Self::error_unexpected(self.prev_loc(), "Invalid operator.")),
        }
    }

    fn parse_then(&mut self) -> Result<(), ParseErr> {
        if self.consume_term()? {
            self.consume_reserved(Reserved::Then)?;
            return Ok(());
        }
        self.expect_reserved(Reserved::Then)?;
        Ok(())
    }

    fn parse_do(&mut self) -> Result<(), ParseErr> {
        if self.consume_term()? {
            return Ok(());
        }
        self.expect_reserved(Reserved::Do)?;
        Ok(())
    }

    /// Check method name extension.
    /// Parse "xxxx!" as a valid mathod name.
    /// "xxxx!=" or "xxxx?=" is invalid.
    fn method_def_ext(&mut self, s: &str) -> Result<IdentId, ParseErr> {
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

    /// Parse formal parameters.
    /// required, optional = defaule, *rest, post_required, kw: default, **rest_kw, &block
    fn parse_formal_params(&mut self, terminator: TokenKind) -> Result<Vec<FormalParam>, ParseErr> {
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
                    return Err(Self::error_unexpected(
                        loc,
                        "Splat parameter is not allowed in ths position.",
                    ));
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
                    return Err(Self::error_unexpected(
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
                            return Err(Self::error_unexpected(
                                loc,
                                "Optional parameter is not allowed in ths position.",
                            ))
                        }
                    };
                    args.push(FormalParam::optional(id, default, loc));
                    let lvar = self.new_param(id, loc)?;
                    self.add_kwopt_param(lvar);
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
                        return Err(Self::error_unexpected(
                            loc,
                            "Keyword parameter is not allowed in ths position.",
                        ));
                    } else {
                        state = Kind::KeyWord;
                    };
                    args.push(FormalParam::keyword(id, default, loc));
                    let lvar = self.new_param(id, loc)?;
                    self.add_kwopt_param(lvar);
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
                            return Err(Self::error_unexpected(
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
}
