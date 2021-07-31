use super::*;

impl<'a> Parser<'a> {
    pub fn parse_comp_stmt(&mut self) -> Result<Node, ParseErr> {
        // COMP_STMT : (STMT (TERM STMT)*)? (TERM+)?
        self.peek()?;
        let loc = self.loc();
        let mut nodes = vec![];

        loop {
            if self.peek()?.check_stmt_end() {
                let node = Node::new_comp_stmt(nodes, loc);
                return Ok(node);
            }

            let node = self.parse_stmt()?;
            nodes.push(node);
            if !self.consume_term()? {
                break;
            }
        }
        let node = Node::new_comp_stmt(nodes, loc);
        Ok(node)
    }

    fn parse_stmt(&mut self) -> Result<Node, ParseErr> {
        // STMT : EXPR
        // | ALIAS-STMT
        // | UNDEF-STMT
        // | STMT [no-term] if EXPR
        // | STMT [no-term] unless EXPR
        // | STMT [no-term] while EXPR
        // | STMT [no-term] until EXPR
        // | STMT [no-term] rescue EXPR
        // | STMT - NORET-STMT [no-term] rescue EXPR
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
            } else if self.consume_reserved_no_skip_line_term(Reserved::Rescue)? {
                // STMT : STMT rescue EXPR
                let rescue = self.parse_expr()?;
                node = Node::new_begin(node, vec![RescueEntry::new_postfix(rescue)], None, None);
            } else {
                break;
            }
        }
        // STMT : EXPR
        Ok(node)
    }

    pub fn parse_expr(&mut self) -> Result<Node, ParseErr> {
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

    fn parse_not(&mut self) -> Result<Node, ParseErr> {
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

    fn parse_mul_assign(&mut self, node: Node) -> Result<Node, ParseErr> {
        // EXPR : MLHS `=' MRHS
        let mut mlhs = vec![node];
        let old = self.suppress_acc_assign;
        self.suppress_acc_assign = true;
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
        self.suppress_acc_assign = old;
        if !self.consume_punct_no_term(Punct::Assign)? {
            let loc = self.loc();
            return Err(self.error_unexpected(loc, "Expected '='."));
        }

        let mrhs = self.parse_mul_assign_rhs_if_allowed()?;
        for lhs in &mlhs {
            self.check_lhs(lhs)?;
        }

        return Ok(Node::new_mul_assign(mlhs, mrhs));
    }

    /// Parse rhs of multiple assignment.
    /// If Parser.mul_assign_rhs is true, only a single assignment is allowed.
    pub fn parse_mul_assign_rhs_if_allowed(&mut self) -> Result<Vec<Node>, ParseErr> {
        if self.suppress_mul_assign {
            let node = vec![self.parse_arg()?];
            Ok(node)
        } else {
            let mrhs = self.parse_mul_assign_rhs(None)?;
            Ok(mrhs)
        }
    }

    /// Parse rhs of multiple assignment. cf: a,b,*c,d
    pub fn parse_mul_assign_rhs(
        &mut self,
        term: impl Into<Option<Punct>>,
    ) -> Result<Vec<Node>, ParseErr> {
        let term = term.into();
        let old = self.suppress_mul_assign;
        // multiple assignment must be suppressed in parsing arg list.
        self.suppress_mul_assign = true;

        let mut args = vec![];
        loop {
            if let Some(term) = term {
                if self.consume_punct(term)? {
                    self.suppress_mul_assign = old;
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
        self.suppress_mul_assign = old;
        match term {
            Some(term) => self.expect_punct(term)?,
            None => {}
        };
        Ok(args)
    }

    pub fn parse_arg(&mut self) -> Result<Node, ParseErr> {
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

    fn parse_arg_assign(&mut self) -> Result<Node, ParseErr> {
        let lhs = self.parse_arg_ternary()?;
        if self.is_line_term()? {
            return Ok(lhs);
        }
        if self.consume_punct_no_term(Punct::Assign)? {
            self.check_lhs(&lhs)?;
            let mrhs = self.parse_mul_assign_rhs(None)?;
            Ok(Node::new_mul_assign(vec![lhs], mrhs))
        } else if let Some(op) = self.consume_assign_op_no_term()? {
            // <lhs> <assign_op> <arg>
            self.parse_assign_op(lhs, op)
        } else {
            Ok(lhs)
        }
    }

    fn parse_arg_ternary(&mut self) -> Result<Node, ParseErr> {
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

    fn parse_arg_range(&mut self) -> Result<Node, ParseErr> {
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

    fn parse_arg_logical_or(&mut self) -> Result<Node, ParseErr> {
        let mut lhs = self.parse_arg_logical_and()?;
        while self.consume_punct_no_term(Punct::LOr)? {
            let rhs = self.parse_arg_logical_and()?;
            lhs = Node::new_binop(BinOp::LOr, lhs, rhs);
        }
        Ok(lhs)
    }

    fn parse_arg_logical_and(&mut self) -> Result<Node, ParseErr> {
        let mut lhs = self.parse_arg_eq()?;
        while self.consume_punct_no_term(Punct::LAnd)? {
            let rhs = self.parse_arg_eq()?;
            lhs = Node::new_binop(BinOp::LAnd, lhs, rhs);
        }
        Ok(lhs)
    }

    // 4==4==4 => SyntaxError
    fn parse_arg_eq(&mut self) -> Result<Node, ParseErr> {
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

    fn parse_arg_comp(&mut self) -> Result<Node, ParseErr> {
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

    fn parse_arg_bitor(&mut self) -> Result<Node, ParseErr> {
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

    fn parse_arg_bitand(&mut self) -> Result<Node, ParseErr> {
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

    fn parse_arg_shift(&mut self) -> Result<Node, ParseErr> {
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

    fn parse_arg_add(&mut self) -> Result<Node, ParseErr> {
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

    fn parse_arg_mul(&mut self) -> Result<Node, ParseErr> {
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

    fn parse_unary_minus(&mut self) -> Result<Node, ParseErr> {
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

    fn parse_accesory_assign(&mut self, lhs: &Node) -> Result<Option<Node>, ParseErr> {
        if !self.suppress_acc_assign {
            if self.consume_punct_no_term(Punct::Assign)? {
                self.check_lhs(&lhs)?;
                let mrhs = self.parse_mul_assign_rhs_if_allowed()?;
                return Ok(Some(Node::new_mul_assign(vec![lhs.clone()], mrhs)));
            } else if let Some(op) = self.consume_assign_op_no_term()? {
                return Ok(Some(self.parse_assign_op(lhs.clone(), op)?));
            }
        };
        Ok(None)
    }

    fn parse_exponent(&mut self) -> Result<Node, ParseErr> {
        let lhs = self.parse_unary()?;
        if self.consume_punct_no_term(Punct::DMul)? {
            let rhs = self.parse_exponent()?;
            Ok(Node::new_binop(BinOp::Exp, lhs, rhs))
        } else {
            Ok(lhs)
        }
    }

    fn parse_unary(&mut self) -> Result<Node, ParseErr> {
        if self.consume_punct(Punct::BitNot)? {
            let loc = self.prev_loc();
            let lhs = Node::new_unop(UnOp::BitNot, self.parse_unary()?, loc);
            Ok(lhs)
        } else if self.consume_punct(Punct::Not)? {
            let loc = self.prev_loc();
            let lhs = Node::new_unop(UnOp::Not, self.parse_unary()?, loc);
            Ok(lhs)
        } else if self.consume_punct(Punct::Plus)? {
            let loc = self.prev_loc();
            let lhs = Node::new_unop(UnOp::Pos, self.parse_unary()?, loc);
            Ok(lhs)
        } else {
            self.parse_method_call()
        }
    }

    fn parse_method_call(&mut self) -> Result<Node, ParseErr> {
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
                    Node::new_scope(node, &name, self.prev_loc().merge(loc))
                } else {
                    self.parse_primary_method(node, false)?
                }
            } else if self.consume_punct_no_term(Punct::LBracket)? {
                let member_loc = self.prev_loc();
                let args = self.parse_mul_assign_rhs(Punct::RBracket)?;
                let member_loc = member_loc.merge(self.prev_loc());
                Node::new_array_member(node, args, member_loc)
            } else {
                return Ok(node);
            };
        }
    }

    fn parse_yield(&mut self) -> Result<Node, ParseErr> {
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
            self.parse_arglist_block(Punct::RParen)?
        } else {
            self.parse_arglist_block(None)?
        };
        return Ok(Node::new_yield(args, loc));
    }

    pub fn parse_super(&mut self) -> Result<Node, ParseErr> {
        let loc = self.prev_loc();
        let arglist = if self.consume_punct_no_term(Punct::LParen)? {
            self.parse_arglist_block(Punct::RParen)?
        } else if self.is_command() {
            self.parse_arglist_block(None)?
        } else {
            return Ok(Node::new_super(None, loc));
        };
        let loc = self.prev_loc().merge(loc);
        Ok(Node::new_super(arglist, loc))
    }
}
