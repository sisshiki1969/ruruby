use super::*;
use crate::error::{ParseErrKind, RubyError};
use crate::id_table::IdentId;
use crate::util::*;
use fxhash::FxHashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct Parser {
    pub lexer: Lexer,
    prev_loc: Loc,
    context_stack: Vec<Context>,
    //pub ident_table: IdentifierTable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseResult {
    pub node: Node,
    //pub ident_table: IdentifierTable,
    pub lvar_collector: LvarCollector,
    pub source_info: SourceInfoRef,
}

impl ParseResult {
    pub fn default(node: Node, lvar_collector: LvarCollector, source_info: SourceInfoRef) -> Self {
        ParseResult {
            node,
            //ident_table,
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
    table: FxHashMap<IdentId, LvarId>,
    block: Option<LvarId>,
}

impl LvarCollector {
    pub fn new() -> Self {
        LvarCollector {
            id: 0,
            table: FxHashMap::default(),
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

    fn insert_new(&mut self, val: IdentId) -> Result<LvarId, ()> {
        let id = self.id;
        if self.table.insert(val, LvarId(id)).is_some() {
            return Err(());
        };
        self.id += 1;
        Ok(LvarId(id))
    }

    fn insert_block_param(&mut self, val: IdentId) -> Result<LvarId, ()> {
        let lvar = self.insert_new(val)?;
        self.block = Some(lvar);
        Ok(lvar)
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

#[derive(Debug, Clone, PartialEq)]
struct ArgList {
    args: Vec<Node>,
    kw_args: Vec<(IdentId, Node)>,
    block: Option<Box<Node>>,
}

impl Parser {
    pub fn new() -> Self {
        let lexer = Lexer::new();
        Parser {
            lexer,
            prev_loc: Loc(0, 0),
            context_stack: vec![],
            //ident_table: IdentifierTable::new(),
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
    /*
        fn context(&self) -> &Context {
            self.context_stack.last().unwrap()
        }
    */
    fn context_mut(&mut self) -> &mut Context {
        self.context_stack.last_mut().unwrap()
    }

    // If the identifier(IdentId) does not exist in the current scope,
    // add the identifier as a local variable in the current context.
    fn add_local_var_if_new(&mut self, id: IdentId) {
        if !self.is_local_var(id) {
            self.context_mut().lvar.insert(id);
        }
    }

    // Add the identifier(IdentId) as a new parameter in the current context.
    // If a parameter with the same name already exists, return error.
    fn new_param(&mut self, id: IdentId, loc: Loc) -> Result<(), RubyError> {
        let res = self.context_mut().lvar.insert_new(id);
        if res.is_err() {
            return Err(self.error_unexpected(loc, "Duplicated argument name."));
        }
        Ok(())
    }

    // Add the identifier(IdentId) as a new block parameter in the current context.
    // If a parameter with the same name already exists, return error.
    fn new_block_param(&mut self, id: IdentId, loc: Loc) -> Result<(), RubyError> {
        let res = self.context_mut().lvar.insert_block_param(id);
        if res.is_err() {
            return Err(self.error_unexpected(loc, "Duplicated argument name."));
        }
        Ok(())
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

    fn get_ident_id(&self, method: impl Into<String>) -> IdentId {
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

    /// If next token is an expected kind of Punctuator, get it and return true.
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

    fn consume_const(&mut self) -> Result<bool, RubyError> {
        match self.peek()?.kind {
            TokenKind::Const(_, _, _) => {
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
            TokenKind::Ident(s, _, _) => s.clone(),
            _ => {
                return Err(self.error_unexpected(self.prev_loc(), "Expect identifier."));
            }
        };
        Ok(self.get_ident_id(name))
    }

    /// Get the next token and examine whether it is Const.
    /// Return IdentId of the Const.
    /// If not, return RubyError.
    fn expect_const(&mut self) -> Result<IdentId, RubyError> {
        let name = match self.get()?.kind {
            TokenKind::Const(s, _, _) => s,
            _ => {
                return Err(self.error_unexpected(self.prev_loc(), "Expect constant."));
            }
        };
        Ok(self.get_ident_id(name))
    }

    fn token_as_symbol(&self, token: &Token) -> String {
        match token.kind.clone() {
            TokenKind::Ident(ident, _, _) => ident,
            TokenKind::Const(ident, _, _) => ident,
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
        lvar_collector: Option<LvarCollector>,
    ) -> Result<ParseResult, RubyError> {
        let (node, lvar) = match self.parse_program_core(path, program, lvar_collector) {
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
        lvar: Option<LvarCollector>,
    ) -> Result<(Node, LvarCollector), RubyError> {
        self.lexer.init(path, program);
        self.context_stack.push(Context::new_class(lvar));
        let node = self.parse_comp_stmt()?;
        let lvar = self.context_stack.pop().unwrap().lvar;
        Ok((node, lvar))
    }

    pub fn parse_program_eval(
        mut self,
        path: PathBuf,
        program: &str,
        ext_lvar: LvarCollector,
    ) -> Result<ParseResult, RubyError> {
        self.lexer.init(path, program);
        self.context_stack.push(Context::new_class(Some(ext_lvar)));
        self.context_stack.push(Context::new_block());
        let node = self.parse_comp_stmt()?;
        let lvar = self.context_stack.pop().unwrap().lvar;
        self.context_stack.pop().unwrap();

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
        // | return ARGS
        // | break ARGS
        // | next ARGS
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
        if self.consume_punct_no_term(Punct::Comma)?
        /*&& node.is_lvar()*/
        {
            // EXPR : MLHS `=' MRHS
            return Ok(self.parse_mul_assign(node)?);
        }
        if node.is_operation() && self.is_command()? {
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
                    mut send_args,
                    completed: false,
                    safe_nav,
                    ..
                },
            loc,
        } = node.clone()
        {
            if self.is_command()? {
                send_args = self.parse_arglist()?;
            } else {
                send_args.block = self.parse_block()?
            };
            let node = Node::new_send(*receiver, method, send_args, true, safe_nav, loc);
            Ok(node)
        } else {
            // EXPR : ARG
            Ok(node)
        }
    }

    fn parse_mul_assign(&mut self, node: Node) -> Result<Node, RubyError> {
        // EXPR : MLHS `=' MRHS
        let mut mlhs = vec![node];
        loop {
            if self.peek_no_term()?.kind == TokenKind::Punct(Punct::Assign) {
                break;
            }
            let node = self.parse_function()?;
            mlhs.push(node);
            if !self.consume_punct_no_term(Punct::Comma)? {
                break;
            }
        }

        if !self.consume_punct_no_term(Punct::Assign)? {
            let loc = self.loc();
            return Err(self.error_unexpected(loc, "Expected '='."));
        }

        let mrhs = self.parse_arg_list(None)?;
        for lhs in &mlhs {
            self.check_lhs(lhs)?;
        }

        return Ok(Node::new_mul_assign(mlhs, mrhs));
    }

    fn parse_arg_list(&mut self, punct: impl Into<Option<Punct>>) -> Result<Vec<Node>, RubyError> {
        let (flag, punct) = match punct.into() {
            Some(punct) => (true, punct),
            None => (false, Punct::Arrow /* dummy */),
        };
        let mut args = vec![];
        loop {
            if flag && self.consume_punct(punct)? {
                return Ok(args);
            }
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
        if flag {
            self.expect_punct(punct)?
        };
        Ok(args)
    }

    fn parse_command(&mut self, operation: IdentId, loc: Loc) -> Result<Node, RubyError> {
        // FNAME ARGS
        // FNAME ARGS DO-BLOCK
        let send_args = self.parse_arglist()?;
        Ok(Node::new_send(
            Node::new_self(loc),
            operation,
            send_args,
            true,
            false,
            loc,
        ))
    }

    fn parse_arglist(&mut self) -> Result<SendArgs, RubyError> {
        let first_arg = self.parse_arg()?;
        if self.is_line_term()? {
            return Ok(SendArgs {
                args: vec![first_arg],
                kw_args: vec![],
                block: None,
            });
        }

        if first_arg.is_operation() && self.is_command()? {
            let args =
                vec![self.parse_command(first_arg.as_method_name().unwrap(), first_arg.loc())?];
            return Ok(SendArgs {
                args,
                kw_args: vec![],
                block: None,
            });
        }

        let mut args = vec![first_arg];
        let mut kw_args = vec![];
        let mut block = None;
        if self.consume_punct_no_term(Punct::Comma)? {
            let res = self.parse_argument_list(None)?;
            let mut new_args = res.args;
            kw_args = res.kw_args;
            block = res.block;
            args.append(&mut new_args);
        }
        match self.parse_block()? {
            Some(actual_block) => {
                if block.is_some() {
                    return Err(self.error_unexpected(
                        actual_block.loc(),
                        "Both block arg and actual block given.",
                    ));
                }
                block = Some(actual_block);
            }
            None => {}
        };
        Ok(SendArgs {
            args,
            kw_args,
            block,
        })
    }

    fn is_command(&mut self) -> Result<bool, RubyError> {
        let tok = self.peek_no_term()?;
        match tok.kind {
            TokenKind::Ident(_, _, _)
            | TokenKind::InstanceVar(_)
            | TokenKind::GlobalVar(_)
            | TokenKind::Const(_, _, _)
            | TokenKind::IntegerLit(_)
            | TokenKind::FloatLit(_)
            | TokenKind::StringLit(_)
            | TokenKind::OpenString(_, _) => Ok(true),
            TokenKind::Punct(p) => match p {
                Punct::LParen
                | Punct::LBracket
                | Punct::LBrace
                | Punct::Colon
                | Punct::Scope
                | Punct::Plus
                | Punct::Minus
                | Punct::Arrow => Ok(true),
                _ => Ok(false),
            },
            TokenKind::Reserved(r) => match r {
                Reserved::False | Reserved::Nil | Reserved::True => Ok(true),
                _ => Ok(false),
            },
            _ => Ok(false),
        }
    }

    fn parse_arg(&mut self) -> Result<Node, RubyError> {
        let node = self.parse_arg_assign()?;
        Ok(node)
    }

    fn parse_arg_assign(&mut self) -> Result<Node, RubyError> {
        let mut lhs = self.parse_arg_ternary()?;
        if self.is_line_term()? {
            return Ok(lhs);
        }
        if self.consume_punct_no_term(Punct::Assign)? {
            let mrhs = self.parse_arg_list(None)?;

            if mrhs.len() == 1 {
                match &lhs.kind {
                    NodeKind::BinOp(op, box op_lhs, box op_rhs) => {
                        if op_rhs.is_variable() {
                            self.check_lhs(&op_rhs)?;
                            let assign = Node::new_mul_assign(vec![op_rhs.clone()], mrhs);
                            return Ok(Node::new_binop(*op, op_lhs.clone(), assign));
                        }
                    }
                    NodeKind::UnOp(op, box node) => {
                        if node.is_variable() {
                            self.check_lhs(node)?;
                            let loc = lhs.loc();
                            let assign = Node::new_mul_assign(vec![node.clone()], mrhs);
                            return Ok(Node::new_unop(*op, assign, loc));
                        }
                    }
                    NodeKind::Range {
                        start: box start,
                        end: box end,
                        exclude_end,
                    } => {
                        if end.is_variable() {
                            self.check_lhs(end)?;
                            let loc = lhs.loc();
                            let assign = Node::new_mul_assign(vec![end.clone()], mrhs);
                            return Ok(Node::new_range(start.clone(), assign, *exclude_end, loc));
                        }
                    }
                    _ => {}
                }
            };

            self.check_lhs(&lhs)?;
            Ok(Node::new_mul_assign(vec![lhs], mrhs))
        } else if let TokenKind::Punct(Punct::AssignOp(op)) = self.peek_no_term()?.kind {
            match op {
                BinOp::LOr => {
                    self.get()?;
                    let rhs = self.parse_arg()?;
                    self.check_lhs(&lhs)?;
                    if let NodeKind::Ident(id) = lhs.kind {
                        lhs = Node::new_lvar(id, lhs.loc());
                    };
                    let node = Node::new_binop(
                        BinOp::LOr,
                        lhs.clone(),
                        Node::new_mul_assign(vec![lhs.clone()], vec![rhs]),
                    );
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
        } else {
            Ok(lhs)
        }
    }

    fn check_lhs(&mut self, lhs: &Node) -> Result<(), RubyError> {
        if let NodeKind::Ident(id) = lhs.kind {
            self.add_local_var_if_new(id);
        } else if let NodeKind::Const { toplevel: _, id: _ } = lhs.kind {
            for c in self.context_stack.iter().rev() {
                match c.kind {
                    ContextKind::Class => return Ok(()),
                    ContextKind::Method => {
                        return Err(self.error_unexpected(lhs.loc(), "Dynamic constant assignment."))
                    }
                    ContextKind::Block => {}
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
        // TODO: Support <==> === !~
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
        if self.consume_punct(Punct::Minus)? {
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
            let lhs = Node::new_unop(UnOp::Neg, lhs, loc);
            Ok(lhs)
        } else {
            self.discard_state();
            let lhs = self.parse_exponent()?;
            Ok(lhs)
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
            let lhs = self.parse_function()?;
            Ok(lhs)
        }
    }

    fn parse_function_args(&mut self, node: Node) -> Result<Node, RubyError> {
        let loc = node.loc();
        if self.consume_punct_no_term(Punct::LParen)? {
            // PRIMARY-METHOD : FNAME ( ARGS ) BLOCK?
            let ArgList {
                args,
                kw_args,
                mut block,
            } = self.parse_argument_list(Punct::RParen)?;
            match self.parse_block()? {
                Some(actual_block) => {
                    if block.is_some() {
                        return Err(self.error_unexpected(
                            actual_block.loc(),
                            "Both block arg and actual block given.",
                        ));
                    }
                    block = Some(actual_block);
                }
                None => {}
            };
            let send_args = SendArgs {
                args,
                kw_args,
                block,
            };

            Ok(Node::new_send(
                Node::new_self(loc),
                node.as_method_name().unwrap(),
                send_args,
                true,
                false,
                loc,
            ))
        } else if let Some(block) = self.parse_block()? {
            // PRIMARY-METHOD : FNAME BLOCK
            let send_args = SendArgs {
                args: vec![],
                kw_args: vec![],
                block: Some(block),
            };
            Ok(Node::new_send(
                Node::new_self(loc),
                node.as_method_name().unwrap(),
                send_args,
                true,
                false,
                loc,
            ))
        } else {
            Ok(node)
        }
    }

    /// PRIMARY-METHOD :
    /// | PRIMARY . FNAME BLOCK => completed: true
    /// | PRIMARY . FNAME ( ARGS ) BLOCK? => completed: true
    /// | PRIMARY . FNAME => completed: false
    fn parse_primary_method(
        &mut self,
        node: Node,
        safe_nav: bool,
        loc: Loc,
    ) -> Result<Node, RubyError> {
        let tok = self.get()?;
        let id = match &tok.kind {
            TokenKind::Ident(s, has_suffix, _) => {
                let name = if *has_suffix {
                    //if self.consume_punct_no_term(Punct::Question)? {
                    //    s.clone() + "?"
                    //} else if self.consume_punct_no_term(Punct::Not)? {
                    //    s.clone() + "!"
                    //} else {
                    s.clone()
                //}
                } else {
                    s.clone()
                };
                self.get_ident_id(name)
            }
            TokenKind::Reserved(r) => {
                let string = self.lexer.get_string_from_reserved(*r).to_owned();
                self.get_ident_id(string)
            }
            TokenKind::Punct(p) => self.parse_op_definable(p)?,
            _ => return Err(self.error_unexpected(tok.loc(), "method name must be an identifier.")),
        };
        let mut args = vec![];
        let mut kw_args = vec![];
        let mut block = None;
        let mut completed = false;
        if self.consume_punct_no_term(Punct::LParen)? {
            let res = self.parse_argument_list(Punct::RParen)?;
            args = res.args;
            kw_args = res.kw_args;
            block = res.block;
            completed = true;
        }
        match self.parse_block()? {
            Some(actual_block) => {
                if block.is_some() {
                    return Err(self.error_unexpected(
                        actual_block.loc(),
                        "Both block arg and actual block given.",
                    ));
                }
                block = Some(actual_block);
            }
            None => {}
        };
        if block.is_some() {
            completed = true;
        };
        let node = match node.kind {
            NodeKind::Ident(id) => Node::new_send_noarg(Node::new_self(loc), id, true, false, loc),
            _ => node,
        };
        let send_args = SendArgs {
            args,
            kw_args,
            block,
        };
        Ok(Node::new_send(
            node,
            id,
            send_args,
            completed,
            safe_nav,
            loc.merge(self.prev_loc()),
        ))
    }

    fn parse_function(&mut self) -> Result<Node, RubyError> {
        if self.consume_reserved(Reserved::Yield)? {
            let loc = self.prev_loc();
            let tok = self.peek_no_term()?;
            // TODO: This is not correct.
            if tok.is_term()
                || tok.kind == TokenKind::Reserved(Reserved::Unless)
                || tok.kind == TokenKind::Reserved(Reserved::If)
                || tok.check_stmt_end()
            {
                return Ok(Node::new_yield(SendArgs::default(), loc));
            };
            let args = if self.consume_punct(Punct::LParen)? {
                let args = self.parse_arglist()?;
                self.expect_punct(Punct::RParen)?;
                args
            } else {
                self.parse_arglist()?
            };
            return Ok(Node::new_yield(args, loc));
        }
        // <一次式メソッド呼び出し>
        let mut node = self.parse_primary()?;
        let loc = node.loc();
        /*if node.is_operation() {
            node = self.parse_function_args(node)?;
        }*/
        loop {
            //let tok = self.peek()?;
            node = if self.consume_punct(Punct::Dot)? {
                self.parse_primary_method(node, false, loc)?
            } else if self.consume_punct_no_term(Punct::SafeNav)? {
                self.parse_primary_method(node, true, loc)?
            } else if self.consume_punct_no_term(Punct::Scope)? {
                let id = self.expect_const()?;
                Node::new_scope(node, id, self.prev_loc())
            } else if node.is_operation() {
                return Ok(node);
            } else if self.consume_punct_no_term(Punct::LBracket)? {
                let member_loc = self.prev_loc();
                let mut args = self.parse_arg_list(Punct::RBracket)?;
                let member_loc = member_loc.merge(self.prev_loc());
                args.reverse();
                Node::new_array_member(node, args, member_loc)
            } else {
                return Ok(node);
            };
        }
    }

    /// Parse argument list.
    /// arg, *splat_arg, kw: kw_arg, &block <punct>
    /// punct: punctuator for terminating arg list. Set None for unparenthesized argument list.
    fn parse_argument_list(
        &mut self,
        punct: impl Into<Option<Punct>>,
    ) -> Result<ArgList, RubyError> {
        let (flag, punct) = match punct.into() {
            Some(punct) => (true, punct),
            None => (false, Punct::Arrow /* dummy */),
        };
        let mut args = vec![];
        let mut kw_args = vec![];
        let mut block = None;
        loop {
            if flag && self.consume_punct(punct)? {
                return Ok(ArgList {
                    args,
                    kw_args,
                    block,
                });
            }
            if self.consume_punct(Punct::Mul)? {
                // splat argument
                let loc = self.prev_loc();
                let array = self.parse_arg()?;
                args.push(Node::new_splat(array, loc));
            } else if self.consume_punct(Punct::BitAnd)? {
                // block argument
                let arg = self.parse_arg()?;
                block = Some(Box::new(arg));
            } else {
                let node = self.parse_arg()?;
                match node.kind {
                    NodeKind::Ident(id, ..) | NodeKind::LocalVar(id) => {
                        if self.consume_punct_no_term(Punct::Colon)? {
                            kw_args.push((id, self.parse_arg()?));
                        } else {
                            args.push(node);
                        }
                    }
                    _ => {
                        args.push(node);
                    }
                }
            }
            if !self.consume_punct(Punct::Comma)? {
                break;
            } else {
                let loc = self.prev_loc();
                if block.is_some() {
                    return Err(self.error_unexpected(loc, "unexpected ','."));
                };
            }
        }
        if flag {
            self.expect_punct(punct)?
        };
        Ok(ArgList {
            args,
            kw_args,
            block,
        })
    }

    fn parse_block(&mut self) -> Result<Option<Box<Node>>, RubyError> {
        let do_flag = if self.consume_reserved_no_skip_line_term(Reserved::Do)? {
            true
        } else {
            if self.consume_punct_no_term(Punct::LBrace)? {
                false
            } else {
                return Ok(None);
            }
        };
        // BLOCK: do [`|' [BLOCK_VAR] `|'] COMPSTMT end
        let loc = self.prev_loc();
        self.context_stack.push(Context::new_block());

        let params = if self.consume_punct(Punct::BitOr)? {
            if self.consume_punct(Punct::BitOr)? {
                vec![]
            } else {
                let params = self.parse_params(TokenKind::Punct(Punct::BitOr))?;
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
        Ok(Some(Box::new(node)))
    }

    fn parse_primary(&mut self) -> Result<Node, RubyError> {
        let tok = self.get()?;
        let loc = tok.loc();
        match &tok.kind {
            TokenKind::Ident(name, has_suffix, trailing_space) => {
                let id = self.get_ident_id(name);
                if *has_suffix {
                    if self.peek_no_term()?.kind == TokenKind::Punct(Punct::LParen) {
                        let node = Node::new_identifier(id, loc);
                        return Ok(self.parse_function_args(node)?);
                    }
                };
                if self.is_local_var(id) {
                    Ok(Node::new_lvar(id, loc))
                } else {
                    // FUNCTION or COMMAND or LHS for assignment
                    let node = Node::new_identifier(id, loc);
                    match self.peek_no_term()?.kind {
                        // Assignment
                        TokenKind::Punct(Punct::AssignOp(_))
                        | TokenKind::Punct(Punct::Assign)
                        // Multiple assignment
                        | TokenKind::Punct(Punct::Comma) => return Ok(node),
                        TokenKind::Punct(Punct::LBrace)
                        | TokenKind::Reserved(Reserved::Do) => return Ok(self.parse_function_args(node)?),
                        _ => {}
                    };
                    if *trailing_space && self.is_command_()? {
                        Ok(self.parse_command(id, loc)?)
                    } else {
                        Ok(node)
                    }
                }
            }
            TokenKind::InstanceVar(name) => {
                let id = self.get_ident_id(name);
                return Ok(Node::new_instance_var(id, loc));
            }
            TokenKind::GlobalVar(name) => {
                let id = self.get_ident_id(name);
                return Ok(Node::new_global_var(id, loc));
            }
            TokenKind::Const(name, has_suffix, _) => {
                let id = self.get_ident_id(name);
                if *has_suffix {
                    if self.peek_no_term()?.kind == TokenKind::Punct(Punct::LParen) {
                        let node = Node::new_identifier(id, loc);
                        return Ok(self.parse_function_args(node)?);
                        //return Ok(Node::new_identifier(id, loc));
                    }
                };
                Ok(Node::new_const(id, false, loc))
            }
            TokenKind::IntegerLit(num) => Ok(Node::new_integer(*num, loc)),
            TokenKind::FloatLit(num) => Ok(Node::new_float(*num, loc)),
            TokenKind::ImaginaryLit(num) => Ok(Node::new_imaginary(*num, loc)),
            TokenKind::StringLit(s) => Ok(self.parse_string_literal(s)?),
            TokenKind::OpenString(s, _) => Ok(self.parse_interporated_string_literal(s)?),
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
                    let mut nodes = self.parse_arg_list(Punct::RBracket)?;
                    nodes.reverse();
                    let loc = loc.merge(self.prev_loc());
                    Ok(Node::new_array(nodes, loc))
                }
                Punct::LBrace => self.parse_hash_literal(),
                Punct::Colon => {
                    // Symbol literal
                    let token = self.get()?;
                    let symbol_loc = self.prev_loc();
                    let id = match &token.kind {
                        TokenKind::Punct(punct) => self.parse_op_definable(punct)?,
                        _ if token.can_be_symbol() => {
                            let ident = self.token_as_symbol(&token);
                            self.get_ident_id(ident)
                        }
                        _ => {
                            if let TokenKind::OpenString(s, _) = token.kind {
                                let node = self.parse_interporated_string_literal(&s)?;
                                let method = self.get_ident_id("to_sym");
                                let loc = symbol_loc.merge(node.loc());
                                return Ok(Node::new_send_noarg(node, method, true, false, loc));
                            }
                            return Err(
                                self.error_unexpected(symbol_loc, "Expect identifier or string.")
                            );
                        }
                    };
                    Ok(Node::new_symbol(id, loc.merge(self.prev_loc())))
                }
                Punct::Arrow => {
                    // Lambda literal
                    let mut params = vec![];
                    self.context_stack.push(Context::new_block());
                    if self.consume_punct(Punct::LParen)? {
                        if !self.consume_punct(Punct::RParen)? {
                            loop {
                                let id = self.expect_ident()?;
                                params.push(Node::new_param(id, self.prev_loc()));
                                self.new_param(id, self.prev_loc())?;
                                if !self.consume_punct(Punct::Comma)? {
                                    break;
                                }
                            }
                            self.expect_punct(Punct::RParen)?;
                        }
                    } else if let TokenKind::Ident(_, _, _) = self.peek()?.kind {
                        let id = self.expect_ident()?;
                        self.new_param(id, self.prev_loc())?;
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
                Punct::Div => {
                    let node = self.parse_regexp()?;
                    Ok(node)
                }
                Punct::Rem => {
                    let node = self.parse_percent_notation()?;
                    Ok(node)
                }
                Punct::Question => {
                    let node = self.parse_char_literal()?;
                    Ok(node)
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
            TokenKind::Reserved(Reserved::Unless) => {
                let node = self.parse_unless()?;
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
            TokenKind::Reserved(Reserved::While) => {
                let loc = self.prev_loc();
                let cond = self.parse_expr()?;
                self.parse_do()?;
                let body = self.parse_comp_stmt()?;
                self.expect_reserved(Reserved::End)?;
                let loc = loc.merge(self.prev_loc());
                Ok(Node::new_while(cond, body, true, loc))
            }
            TokenKind::Reserved(Reserved::Until) => {
                let loc = self.prev_loc();
                let cond = self.parse_expr()?;
                self.parse_do()?;
                let body = self.parse_comp_stmt()?;
                self.expect_reserved(Reserved::End)?;
                let loc = loc.merge(self.prev_loc());
                Ok(Node::new_while(cond, body, false, loc))
            }
            TokenKind::Reserved(Reserved::Case) => {
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
            TokenKind::Reserved(Reserved::Def) => Ok(self.parse_def()?),
            TokenKind::Reserved(Reserved::Class) => {
                if self.context_stack.last().unwrap().kind == ContextKind::Method {
                    return Err(
                        self.error_unexpected(loc, "SyntaxError: class definition in method body.")
                    );
                }
                Ok(self.parse_class(false)?)
            }
            TokenKind::Reserved(Reserved::Module) => {
                if self.context_stack.last().unwrap().kind == ContextKind::Method {
                    return Err(
                        self.error_unexpected(loc, "SyntaxError: class definition in method body.")
                    );
                }
                Ok(self.parse_class(true)?)
            }
            TokenKind::Reserved(Reserved::Return) => {
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
                    vec.reverse();
                    let val = Node::new_array(vec, ret_loc);
                    Ok(Node::new_return(val, loc))
                } else {
                    Ok(Node::new_return(val, loc))
                }
            }
            TokenKind::Reserved(Reserved::Break) => {
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
                    vec.reverse();
                    let val = Node::new_array(vec, ret_loc);
                    Ok(Node::new_break(val, loc))
                } else {
                    Ok(Node::new_break(val, loc))
                }
            }
            TokenKind::Reserved(Reserved::Next) => {
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
                    vec.reverse();
                    let val = Node::new_array(vec, ret_loc);
                    Ok(Node::new_next(val, loc))
                } else {
                    Ok(Node::new_next(val, loc))
                }
            }
            TokenKind::Reserved(Reserved::True) => Ok(Node::new_bool(true, loc)),
            TokenKind::Reserved(Reserved::False) => Ok(Node::new_bool(false, loc)),
            TokenKind::Reserved(Reserved::Nil) => Ok(Node::new_nil(loc)),
            TokenKind::Reserved(Reserved::Self_) => Ok(Node::new_self(loc)),
            TokenKind::Reserved(Reserved::Begin) => Ok(self.parse_begin()?),
            TokenKind::EOF => return Err(self.error_eof(loc)),
            _ => {
                return Err(self.error_unexpected(loc, format!("Unexpected token: {:?}", tok.kind)))
            }
        }
    }

    fn is_command_(&mut self) -> Result<bool, RubyError> {
        let tok = self.peek_no_term()?;
        match tok.kind {
            TokenKind::Ident(_, _, _)
            | TokenKind::InstanceVar(_)
            | TokenKind::Const(_, _, _)
            | TokenKind::IntegerLit(_)
            | TokenKind::FloatLit(_)
            | TokenKind::StringLit(_)
            | TokenKind::OpenString(_, _) => Ok(true),
            TokenKind::Punct(p) => match p {
                Punct::LParen | Punct::LBracket | Punct::Colon | Punct::Scope | Punct::Arrow => {
                    Ok(true)
                }
                _ => Ok(false),
            },
            TokenKind::Reserved(r) => match r {
                Reserved::False | Reserved::Nil | Reserved::True | Reserved::Self_ => Ok(true),
                _ => Ok(false),
            },
            _ => Ok(false),
        }
    }

    fn parse_string_literal(&mut self, s: &str) -> Result<Node, RubyError> {
        let loc = self.prev_loc();
        let mut s = s.to_string();
        while let TokenKind::StringLit(next_s) = self.peek_no_term()?.kind {
            self.get()?;
            s = format!("{}{}", s, next_s);
        }
        Ok(Node::new_string(s, loc))
    }

    fn parse_char_literal(&mut self) -> Result<Node, RubyError> {
        let loc = self.loc();
        match self.lexer.lex_char_literal()?.kind {
            TokenKind::StringLit(s) => Ok(Node::new_string(s, loc.merge(self.prev_loc))),
            _ => unreachable!(),
        }
    }

    fn parse_interporated_string_literal(&mut self, s: &str) -> Result<Node, RubyError> {
        let start_loc = self.prev_loc();
        let mut nodes = vec![Node::new_string(s.to_string(), start_loc)];
        loop {
            match self.peek()?.kind {
                TokenKind::CloseString(s) => {
                    let end_loc = self.loc();
                    nodes.push(Node::new_string(s.clone(), end_loc));
                    self.get()?;
                    return Ok(Node::new_interporated_string(
                        nodes,
                        start_loc.merge(end_loc),
                    ));
                }
                TokenKind::InterString(s) => {
                    nodes.push(Node::new_string(s.clone(), self.loc()));
                    self.get()?;
                }
                TokenKind::OpenString(s, _) => {
                    let s = s.clone();
                    self.get()?;
                    self.parse_interporated_string_literal(&s)?;
                }
                TokenKind::EOF => {
                    let loc = self.loc();
                    return Err(self.error_eof(loc));
                }
                _ => {
                    nodes.push(self.parse_comp_stmt()?);
                }
            }
        }
    }

    fn parse_regexp(&mut self) -> Result<Node, RubyError> {
        let tok = self.lexer.get_regexp()?;
        let mut nodes = match tok.kind {
            TokenKind::StringLit(s) => {
                return Ok(Node::new_regexp(
                    vec![Node::new_string(s, tok.loc)],
                    tok.loc,
                ));
            }
            TokenKind::OpenRegex(s) => vec![Node::new_string(s, tok.loc)],
            _ => panic!(),
        };
        loop {
            match self.peek()?.kind {
                TokenKind::CloseString(s) => {
                    self.get()?;
                    let end_loc = self.prev_loc();
                    nodes.push(Node::new_string(s, end_loc));
                    return Ok(Node::new_regexp(nodes, tok.loc.merge(end_loc)));
                }
                TokenKind::InterString(s) => {
                    self.get()?;
                    nodes.push(Node::new_string(s, self.prev_loc()));
                }
                TokenKind::EOF => {
                    let loc = self.loc();
                    return Err(self.error_eof(loc));
                }
                _ => {
                    nodes.push(self.parse_comp_stmt()?);
                }
            }
        }
    }

    fn parse_percent_notation(&mut self) -> Result<Node, RubyError> {
        let tok = self.lexer.get_percent_notation()?;
        let loc = tok.loc;
        if let TokenKind::PercentNotation(kind, content) = tok.kind {
            match kind {
                'w' => {
                    let ary = content
                        .split(|c| c == ' ' || c == '\n')
                        .map(|x| Node::new_string(x.to_string(), loc))
                        .rev()
                        .collect();
                    Ok(Node::new_array(ary, tok.loc))
                }
                'i' => {
                    let ary = content
                        .split(|c| c == ' ' || c == '\n')
                        .map(|x| Node::new_symbol(IdentId::get_id(x), loc))
                        .rev()
                        .collect();
                    Ok(Node::new_array(ary, tok.loc))
                }
                _ => return Err(self.error_unexpected(loc, "Unsupported % notation.")),
            }
        } else if let TokenKind::StringLit(s) = tok.kind {
            return Ok(Node::new_string(s, loc));
        //} else if let TokenKind::OpenString(s) = tok.kind {
        //    panic!();
        //let node = self.parse_interporated_string_literal(&s)?;
        } else {
            panic!();
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
                    let id = self.get_ident_id(ident);
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

    fn parse_def(&mut self) -> Result<Node, RubyError> {
        //  def FNAME ARGDECL
        //      COMPSTMT
        //      [rescue [ARGS] [`=>' LHS] THEN COMPSTMT]+
        //      [else COMPSTMT]
        //      [ensure COMPSTMT]
        //  end
        let mut is_singleton_method = false;
        let tok = self.get()?;
        let id = match tok.kind {
            TokenKind::Reserved(Reserved::Self_) => {
                is_singleton_method = true;
                self.expect_punct(Punct::Dot)?;
                self.expect_ident()?
            }
            TokenKind::Reserved(r) => {
                let string = self.lexer.get_string_from_reserved(r).to_owned();
                self.get_ident_id(string)
            }
            TokenKind::Ident(name, has_suffix, _) => {
                if has_suffix {
                    match self.peek_no_term()?.kind {
                        TokenKind::Punct(Punct::Assign) => {
                            self.get()?;
                            self.get_ident_id(name + "=")
                        }
                        _ => self.get_ident_id(name),
                    }
                } else {
                    self.get_ident_id(name)
                }
            }
            TokenKind::Punct(Punct::Plus) => self.get_ident_id("+"),
            TokenKind::Punct(Punct::Minus) => self.get_ident_id("-"),
            TokenKind::Punct(Punct::Mul) => self.get_ident_id("*"),
            TokenKind::Punct(Punct::LBracket) => {
                if self.consume_punct_no_term(Punct::RBracket)? {
                    if self.consume_punct_no_term(Punct::Assign)? {
                        IdentId::_INDEX_ASSIGN
                    } else {
                        IdentId::_INDEX
                    }
                } else {
                    let loc = self.loc();
                    return Err(self.error_unexpected(loc, "Expected `]'"));
                }
            }
            _ => {
                let loc = self.loc();
                return Err(self.error_unexpected(loc, "Expected identifier or operator."));
            }
        };
        self.context_stack.push(Context::new_method());
        let args = self.parse_def_params()?;
        let body = self.parse_begin()?;
        let lvar = self.context_stack.pop().unwrap().lvar;
        if is_singleton_method {
            Ok(Node::new_singleton_method_decl(id, args, body, lvar))
        } else {
            Ok(Node::new_method_decl(id, args, body, lvar))
        }
    }

    /// Parse parameters.
    /// required, optional = defaule, *rest, post_required, kw: default, **rest_kw, &block
    fn parse_params(&mut self, terminator: TokenKind) -> Result<Vec<Node>, RubyError> {
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
                args.push(Node::new_block_param(id, loc));
                self.new_block_param(id, loc)?;
                break;
            } else if self.consume_punct(Punct::Mul)? {
                // Splat(Rest) param
                let id = self.expect_ident()?;
                loc = loc.merge(self.prev_loc());
                if state >= Kind::Rest {
                    return Err(self
                        .error_unexpected(loc, "Splat parameter is not allowed in ths position."));
                } else {
                    state = Kind::Rest;
                }

                args.push(Node::new_splat_param(id, loc));
                self.new_param(id, self.prev_loc())?;
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
                    args.push(Node::new_optional_param(id, default, loc));
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
                    args.push(Node::new_keyword_param(id, default, loc));
                    self.new_param(id, loc)?;
                } else {
                    // Required param
                    loc = self.prev_loc();
                    match state {
                        Kind::Reqired => {
                            args.push(Node::new_param(id, loc));
                            self.new_param(id, loc)?;
                        }
                        Kind::PostReq | Kind::Optional | Kind::Rest => {
                            args.push(Node::new_post_param(id, loc));
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
    fn parse_def_params(&mut self) -> Result<Vec<Node>, RubyError> {
        if self.consume_term()? {
            return Ok(vec![]);
        };
        let paren_flag = self.consume_punct(Punct::LParen)?;

        if paren_flag && self.consume_punct(Punct::RParen)? {
            if !self.consume_term()? {
                let loc = self.loc();
                return Err(self.error_unexpected(loc, "Expect terminator"));
            }
            return Ok(vec![]);
        }

        let args = self.parse_params(TokenKind::Punct(Punct::RParen))?;

        if paren_flag {
            self.expect_punct(Punct::RParen)?
        };
        if !self.consume_term()? {
            let loc = self.loc();
            return Err(self.error_unexpected(loc, "Expect terminator."));
        }
        Ok(args)
    }

    fn parse_class(&mut self, is_module: bool) -> Result<Node, RubyError> {
        //  class identifier [`<' EXPR]
        //      COMPSTMT
        //  end
        let loc = self.prev_loc();
        let name = match &self.get()?.kind {
            TokenKind::Const(s, _, _) => s.clone(),
            _ => return Err(self.error_unexpected(loc, "Class/Module name must be CONSTANT.")),
        };
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
        let id = self.get_ident_id(&name);
        self.context_stack.push(Context::new_class(None));
        let body = self.parse_begin()?;
        let lvar = self.context_stack.pop().unwrap().lvar;
        #[cfg(feature = "verbose")]
        eprintln!(
            "Parsed {} name:{}",
            if is_module { "module" } else { "class" },
            name
        );
        Ok(Node::new_class_decl(
            id, superclass, body, lvar, is_module, loc,
        ))
    }

    fn parse_op_definable(&mut self, punct: &Punct) -> Result<IdentId, RubyError> {
        match punct {
            Punct::Plus => Ok(IdentId::_ADD),
            Punct::Minus => Ok(IdentId::_SUB),
            Punct::LBracket => {
                if self.consume_punct_no_term(Punct::RBracket)? {
                    if self.consume_punct_no_term(Punct::Assign)? {
                        Ok(IdentId::_INDEX_ASSIGN)
                    } else {
                        Ok(IdentId::_INDEX)
                    }
                } else {
                    let loc = self.loc();
                    Err(self.error_unexpected(loc, "Invalid symbol literal."))
                }
            }
            _ => Err(self.error_unexpected(self.prev_loc(), "Invalid symbol literal.")),
        }
    }

    fn parse_begin(&mut self) -> Result<Node, RubyError> {
        let body = self.parse_comp_stmt()?;
        if self.consume_reserved(Reserved::Rescue)? {
            if self.consume_const()? {}
            if self.consume_punct_no_term(Punct::FatArrow)? {
                self.expect_ident()?;
            }
            self.parse_comp_stmt()?;
        }
        let ensure = if self.consume_reserved(Reserved::Ensure)? {
            self.parse_comp_stmt()?
        } else {
            Node::new_nop(body.loc())
        };
        self.expect_reserved(Reserved::End)?;
        let loc = body.loc();
        Ok(Node::new_begin(
            body,
            vec![],
            Node::new_nop(loc),
            ensure,
            loc,
        ))
    }
}
