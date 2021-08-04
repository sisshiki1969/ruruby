use super::*;

// Parse
impl<'a> Parser<'a> {
    pub(super) fn parse_if(&mut self) -> Result<Node, ParseErr> {
        //  if EXPR THEN
        //      COMPSTMT
        //      (elsif EXPR THEN COMPSTMT)*
        //      [else COMPSTMT]
        //  end
        let node = self.parse_if_then()?;
        self.expect_reserved(Reserved::End)?;
        Ok(node)
    }

    pub(super) fn parse_if_then(&mut self) -> Result<Node, ParseErr> {
        //  EXPR THEN
        //      COMPSTMT
        //      (elsif EXPR THEN COMPSTMT)*
        //      [else COMPSTMT]
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

    pub(super) fn parse_unless(&mut self) -> Result<Node, ParseErr> {
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
        self.expect_reserved(Reserved::End)?;
        Ok(Node::new_if(cond, else_, then_, loc))
    }

    pub(super) fn parse_while(&mut self, is_while: bool) -> Result<Node, ParseErr> {
        let old_suppress_do_flag = self.suppress_do_block;
        self.suppress_do_block = true;
        let loc = self.prev_loc();
        let cond = self.parse_expr()?;
        self.suppress_do_block = old_suppress_do_flag;
        self.parse_do()?;
        let body = self.parse_comp_stmt()?;
        self.expect_reserved(Reserved::End)?;
        let loc = loc.merge(self.prev_loc());
        Ok(Node::new_while(cond, body, is_while, loc))
    }

    pub(super) fn parse_for(&mut self) -> Result<Node, ParseErr> {
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
        let body = Block::new(formal_params, body, lvar);

        self.expect_reserved(Reserved::End)?;
        let node = Node::new(
            NodeKind::For {
                param: vars,
                iter: Box::new(iter),
                body: body,
            },
            loc.merge(self.prev_loc()),
        );
        Ok(node)
    }

    pub(super) fn parse_case(&mut self) -> Result<Node, ParseErr> {
        let loc = self.prev_loc();
        let old = self.suppress_mul_assign;
        self.suppress_mul_assign = false;
        let cond = if self.peek()?.kind != TokenKind::Reserved(Reserved::When) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.consume_term()?;
        let mut when_ = vec![];
        while self.consume_reserved(Reserved::When)? {
            let arg = self.parse_mul_assign_rhs(None)?;
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
        self.suppress_mul_assign = old;
        Ok(Node::new_case(cond, when_, else_, loc))
    }

    pub(super) fn parse_return(&mut self) -> Result<Node, ParseErr> {
        let (node, loc) = self.parse_break_sub()?;
        Ok(Node::new_return(node, loc))
    }

    pub(super) fn parse_break(&mut self) -> Result<Node, ParseErr> {
        let (node, loc) = self.parse_break_sub()?;
        Ok(Node::new_break(node, loc))
    }

    pub(super) fn parse_next(&mut self) -> Result<Node, ParseErr> {
        let (node, loc) = self.parse_break_sub()?;
        Ok(Node::new_next(node, loc))
    }

    fn parse_break_sub(&mut self) -> Result<(Node, Loc), ParseErr> {
        let loc = self.prev_loc();
        let tok = self.peek_no_term()?;
        // TODO: This is not correct.
        if tok.is_term()
            || tok.kind == TokenKind::Reserved(Reserved::Unless)
            || tok.kind == TokenKind::Reserved(Reserved::If)
            || tok.check_stmt_end()
        {
            let val = Node::new_nil(loc);
            return Ok((val, loc));
        };
        let val = self.parse_arg()?;
        let ret_loc = val.loc();
        if self.consume_punct_no_term(Punct::Comma)? {
            let mut vec = vec![val, self.parse_arg()?];
            while self.consume_punct_no_term(Punct::Comma)? {
                vec.push(self.parse_arg()?);
            }
            let val = Node::new_array(vec, ret_loc);
            Ok((val, loc))
        } else {
            Ok((val, loc))
        }
    }
}
