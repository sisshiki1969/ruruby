use super::*;
use num::BigInt;
use ruruby_common::*;
use std::path::PathBuf;

mod define;
mod expression;
mod flow_control;
mod lexer;
mod literals;
use lexer::*;

pub trait LocalsContext: Copy + Sized {
    fn outer(&self) -> Option<Self>;

    fn get_lvarid(&self, id: IdentId) -> Option<LvarId>;

    fn lvar_collector(&self) -> LvarCollector;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parser<'a, OuterContext: LocalsContext> {
    lexer: Lexer<'a>,
    path: PathBuf,
    prev_loc: Loc,
    context_stack: Vec<ParseContext>,
    extern_context: Option<OuterContext>,
    /// this flag suppress accesory assignment. e.g. x=3
    suppress_acc_assign: bool,
    /// this flag suppress accesory multiple assignment. e.g. x = 2,3
    suppress_mul_assign: bool,
    /// this flag suppress parse do-end style block.
    suppress_do_block: bool,
}

impl<'a, A: LocalsContext> Parser<'a, A> {
    pub fn parse_program(
        code: String,
        path: impl Into<PathBuf>,
        context_name: &str,
        extern_context: Option<impl LocalsContext>,
    ) -> Result<ParseResult, RubyError> {
        let path = path.into();
        let parse_ctx =
            ParseContext::new_eval(context_name, extern_context.map(|ctx| ctx.lvar_collector()));
        parse(code, path, extern_context, parse_ctx)
    }

    pub fn parse_program_binding(
        code: String,
        path: PathBuf,
        context: Option<impl LocalsContext>,
        extern_context: Option<impl LocalsContext>,
    ) -> Result<ParseResult, RubyError> {
        let parse_ctx = ParseContext::new_block(context.map(|ctx| ctx.lvar_collector()));
        parse(code, path, extern_context, parse_ctx)
    }
}

impl<'a, A: LocalsContext> Parser<'a, A> {
    fn new(
        code: &'a str,
        path: PathBuf,
        extern_context: Option<A>,
        parse_context: ParseContext,
    ) -> Result<(Node, LvarCollector, Token), ParseErr> {
        let lexer = Lexer::new(code);
        let mut parser = Parser {
            lexer,
            path,
            prev_loc: Loc(0, 0),
            context_stack: vec![parse_context],
            extern_context,
            suppress_acc_assign: false,
            suppress_mul_assign: false,
            suppress_do_block: false,
        };
        let node = parser.parse_comp_stmt()?;
        let lvar = parser.context_stack.pop().unwrap().lvar;
        let tok = parser.peek()?;
        Ok((node, lvar, tok))
    }

    fn save_state(&self) -> (usize, usize) {
        self.lexer.save_state()
    }

    fn restore_state(&mut self, state: (usize, usize)) {
        self.lexer.restore_state(state);
    }

    fn context_mut(&mut self) -> &mut ParseContext {
        self.context_stack.last_mut().unwrap()
    }

    fn is_method_context(&self) -> bool {
        self.context_stack.last().unwrap().kind == ParseContextKind::Method
    }

    /// Check whether parameter delegation exists or not in the method def of current context.
    /// If not, return ParseErr.
    fn check_delegate(&self) -> Result<(), ParseErr> {
        for ctx in self.context_stack.iter().rev() {
            if ctx.kind == ParseContextKind::Method {
                if ctx.lvar.delegate_param.is_some() {
                    return Ok(());
                } else {
                    break;
                }
            }
        }
        Err(error_unexpected(self.prev_loc(), "Unexpected ..."))
    }

    /// If the `id` does not exist in the scope chain,
    /// add `id` as a local variable in the current context.
    fn add_local_var_if_new(&mut self, id: IdentId) {
        if !self.is_local_var(id) {
            for c in self.context_stack.iter_mut().rev() {
                match c.kind {
                    ParseContextKind::For => {}
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
            None => Err(error_unexpected(loc, "Duplicated argument name.")),
        }
    }

    fn add_kw_param(&mut self, lvar: LvarId) {
        self.context_mut().lvar.kw.push(lvar);
    }

    /// Add the `id` as a new parameter in the current context.
    /// If a parameter with the same name already exists, return error.
    fn new_kwrest_param(&mut self, id: IdentId, loc: Loc) -> Result<(), ParseErr> {
        if self.context_mut().lvar.insert_kwrest_param(id).is_none() {
            return Err(error_unexpected(loc, "Duplicated argument name."));
        }
        Ok(())
    }

    /// Add the `id` as a new block parameter in the current context.
    /// If a parameter with the same name already exists, return error.
    fn new_block_param(&mut self, id: IdentId, loc: Loc) -> Result<(), ParseErr> {
        if self.context_mut().lvar.insert_block_param(id).is_none() {
            return Err(error_unexpected(loc, "Duplicated argument name."));
        }
        Ok(())
    }

    /// Add the `id` as a new block parameter in the current context.
    /// If a parameter with the same name already exists, return error.
    fn new_delegate_param(&mut self, loc: Loc) -> Result<(), ParseErr> {
        if self.context_mut().lvar.insert_delegate_param().is_none() {
            return Err(error_unexpected(loc, "Duplicated argument name."));
        }
        Ok(())
    }

    /// Examine whether `id` exists in the scope chain.
    /// If exiets, return true.
    fn is_local_var(&mut self, id: IdentId) -> bool {
        for c in self.context_stack.iter().rev() {
            if c.lvar.table.get_lvarid(id).is_some() {
                return true;
            }
            match c.kind {
                ParseContextKind::Block | ParseContextKind::For => {}
                _ => return false,
            }
        }
        let mut ctx = self.extern_context;
        while let Some(a) = ctx {
            if a.get_lvarid(id).is_some() {
                return true;
            };
            ctx = a.outer();
        }
        false
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
                return Err(error_eof(tok.loc()));
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
        Ok(true)
    }

    /// Get the next token and examine whether it is an expected Reserved.
    /// If not, return RubyError.
    fn expect_reserved(&mut self, expect: Reserved) -> Result<(), ParseErr> {
        match &self.get()?.kind {
            TokenKind::Reserved(reserved) if *reserved == expect => Ok(()),
            t => Err(error_unexpected(
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
            t => Err(error_unexpected(
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
            _ => Err(error_unexpected(self.prev_loc(), "Expect identifier.")),
        }
    }

    /// Get the next token and examine whether it is Const.
    /// Return IdentId of the Const.
    /// If not, return RubyError.
    fn expect_const(&mut self) -> Result<String, ParseErr> {
        match self.get()?.kind {
            TokenKind::Const(s) => Ok(s),
            _ => Err(error_unexpected(self.prev_loc(), "Expect constant.")),
        }
    }
}

impl<'a, A: LocalsContext> Parser<'a, A> {
    /// Parse block.
    ///     do |x| stmt end
    ///     { |x| stmt }
    fn parse_block(&mut self) -> Result<Option<Box<Node>>, ParseErr> {
        let old_suppress_mul_flag = self.suppress_mul_assign;
        self.suppress_mul_assign = false;
        let do_flag =
            if !self.suppress_do_block && self.consume_reserved_no_skip_line_term(Reserved::Do)? {
                true
            } else if self.consume_punct_no_term(Punct::LBrace)? {
                false
            } else {
                self.suppress_mul_assign = old_suppress_mul_flag;
                return Ok(None);
            };
        // BLOCK: do [`|' [BLOCK_VAR] `|'] COMPSTMT end
        let loc = self.prev_loc();
        self.context_stack.push(ParseContext::new_block(None));

        let params = if self.consume_punct(Punct::BitOr)? {
            self.parse_formal_params(Punct::BitOr)?
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
        let node = Node::new_lambda(params, body, lvar, loc);
        self.suppress_mul_assign = old_suppress_mul_flag;
        Ok(Some(Box::new(node)))
    }

    /// Parse operator which can be defined as a method.
    /// Return IdentId of the operator.
    fn parse_op_definable(&mut self, punct: &Punct) -> Result<IdentId, ParseErr> {
        // TODO: must support
        // ^
        // **   ~   +@  -@   ` !  !~
        match punct {
            Punct::Plus => Ok(IdentId::_ADD),
            Punct::Minus => Ok(IdentId::_SUB),
            Punct::Mul => Ok(IdentId::_MUL),
            Punct::Div => Ok(IdentId::_DIV),
            Punct::Rem => Ok(IdentId::_REM),
            Punct::Shl => Ok(IdentId::_SHL),
            Punct::Shr => Ok(IdentId::_SHR),
            Punct::BitAnd => Ok(IdentId::get_id("&")),
            Punct::BitOr => Ok(IdentId::get_id("|")),

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
                    Err(error_unexpected(loc, "Invalid operator."))
                }
            }
            _ => Err(error_unexpected(self.prev_loc(), "Invalid operator.")),
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

    /// Parse formal parameters.
    /// required, optional = defaule, *rest, post_required, kw: default, **rest_kw, &block
    fn parse_formal_params(
        &mut self,
        terminator: impl Into<Option<Punct>>,
    ) -> Result<Vec<FormalParam>, ParseErr> {
        #[derive(Debug, Clone, PartialEq, PartialOrd)]
        enum Kind {
            Required,
            Optional,
            Rest,
            PostReq,
            KeyWord,
            KWRest,
        }

        let terminator = terminator.into();
        let mut args = vec![];
        let mut state = Kind::Required;
        if let Some(term) = terminator {
            if self.consume_punct(term)? {
                return Ok(args);
            }
        }
        loop {
            let mut loc = self.loc();
            if self.consume_punct(Punct::Range3)? {
                // Argument delegation
                if state > Kind::Required {
                    return Err(error_unexpected(
                        loc,
                        "parameter delegate is not allowed in ths position.",
                    ));
                }
                args.push(FormalParam::delegeate(loc));
                self.new_delegate_param(loc)?;
                break;
            } else if self.consume_punct(Punct::BitAnd)? {
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
                    return Err(error_unexpected(
                        loc,
                        "Rest parameter is not allowed in ths position.",
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
                    return Err(error_unexpected(
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
                        Kind::Required => state = Kind::Optional,
                        Kind::Optional => {}
                        _ => {
                            return Err(error_unexpected(
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
                    let default =
                        if next == TokenKind::Punct(Punct::Comma) || next == TokenKind::LineTerm {
                            None
                        } else if let Some(term) = terminator {
                            if next == TokenKind::Punct(term) {
                                None
                            } else {
                                Some(self.parse_arg()?)
                            }
                        } else {
                            Some(self.parse_arg()?)
                        };
                    loc = loc.merge(self.prev_loc());
                    if state == Kind::KWRest {
                        return Err(error_unexpected(
                            loc,
                            "Keyword parameter is not allowed in ths position.",
                        ));
                    } else {
                        state = Kind::KeyWord;
                    };
                    args.push(FormalParam::keyword(id, default, loc));
                    let lvar = self.new_param(id, loc)?;
                    self.add_kw_param(lvar);
                } else {
                    // Required param
                    loc = self.prev_loc();
                    match state {
                        Kind::Required => {
                            args.push(FormalParam::req_param(id, loc));
                            self.new_param(id, loc)?;
                        }
                        Kind::PostReq | Kind::Optional | Kind::Rest => {
                            args.push(FormalParam::post(id, loc));
                            self.new_param(id, loc)?;
                            state = Kind::PostReq;
                        }
                        _ => {
                            return Err(error_unexpected(
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
        if let Some(term) = terminator {
            self.expect_punct(term)?;
        }
        Ok(args)
    }
}

fn error_unexpected(loc: Loc, msg: impl Into<String>) -> ParseErr {
    ParseErr(ParseErrKind::SyntaxError(msg.into()), loc)
}

fn error_eof(loc: Loc) -> ParseErr {
    ParseErr(ParseErrKind::UnexpectedEOF, loc)
}

fn parse(
    code: String,
    path: PathBuf,
    extern_context: Option<impl LocalsContext>,
    parse_context: ParseContext,
) -> Result<ParseResult, RubyError> {
    match Parser::new(&code, path.clone(), extern_context, parse_context) {
        Ok((node, lvar_collector, tok)) => {
            let source_info = SourceInfoRef::new(SourceInfo::new(path, code));
            if tok.is_eof() {
                let result = ParseResult {
                    node,
                    lvar_collector,
                    source_info,
                };
                Ok(result)
            } else {
                let err = error_unexpected(tok.loc(), "Expected end-of-input.");
                Err(RubyError::new_parse_err(err.0, source_info, err.1))
            }
        }
        Err(err) => {
            let source_info = SourceInfoRef::new(SourceInfo::new(path, code));
            return Err(RubyError::new_parse_err(err.0, source_info, err.1));
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseResult {
    pub node: Node,
    pub lvar_collector: LvarCollector,
    pub source_info: SourceInfoRef,
}

#[derive(Debug, Clone, PartialEq)]
enum ParseContextKind {
    Eval,
    Class,
    Method,
    Block,
    For,
}

#[derive(Debug, Clone, PartialEq)]
struct ParseContext {
    lvar: LvarCollector,
    kind: ParseContextKind,
    name: Option<IdentId>,
}

impl ParseContext {
    fn new_method(name: IdentId) -> Self {
        ParseContext {
            lvar: LvarCollector::new(),
            kind: ParseContextKind::Method,
            name: Some(name),
        }
    }

    fn new_eval(name: &str, lvar_collector: Option<LvarCollector>) -> Self {
        ParseContext {
            lvar: lvar_collector.unwrap_or_default(),
            kind: ParseContextKind::Eval,
            name: Some(IdentId::get_id(name)),
        }
    }

    fn new_class(name: IdentId, lvar_collector: Option<LvarCollector>) -> Self {
        ParseContext {
            lvar: lvar_collector.unwrap_or_default(),
            kind: ParseContextKind::Class,
            name: Some(name),
        }
    }

    fn new_block(lvar_collector: Option<LvarCollector>) -> Self {
        ParseContext {
            lvar: lvar_collector.unwrap_or_default(),
            kind: ParseContextKind::Block,
            name: None,
        }
    }

    fn new_for() -> Self {
        ParseContext {
            lvar: LvarCollector::new(),
            kind: ParseContextKind::For,
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
    fn new(exception_list: Vec<Node>, assign: Option<Node>, body: Node) -> Self {
        Self {
            exception_list,
            assign: assign.map(Box::new),
            body: Box::new(body),
        }
    }

    fn new_postfix(body: Node) -> Self {
        Self {
            exception_list: vec![],
            assign: None,
            body: Box::new(body),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum NReal {
    Integer(i64),
    Bignum(BigInt),
    Float(f64),
}
