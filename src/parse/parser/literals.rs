use super::*;

// Parse
impl<'a> Parser<'a> {
    /// Parse char literals.
    pub fn parse_char_literal(&mut self) -> Result<Node, ParseErr> {
        let loc = self.loc();
        match self.lexer.read_char_literal()?.kind {
            TokenKind::StringLit(s) => Ok(Node::new_string(s, loc.merge(self.prev_loc))),
            _ => unreachable!(),
        }
    }

    /// Parse string literals.
    /// Adjacent string literals are to be combined.
    pub fn parse_string_literal(&mut self, s: &str) -> Result<Node, ParseErr> {
        let loc = self.prev_loc();
        let mut s = s.to_string();
        loop {
            match self.peek_no_term()?.kind {
                TokenKind::StringLit(next_s) => {
                    self.get()?;
                    s += &next_s;
                }
                TokenKind::OpenString(next_s, delimiter, level) => {
                    self.get()?;
                    s += &next_s;
                    return self.parse_interporated_string_literal(&s, delimiter, level);
                }
                _ => break,
            }
        }
        Ok(Node::new_string(s, loc))
    }

    pub fn parse_interporated_string_literal(
        &mut self,
        s: &str,
        delimiter: char,
        level: usize,
    ) -> Result<Node, ParseErr> {
        let start_loc = self.prev_loc();
        let mut nodes = vec![Node::new_string(s.to_string(), start_loc)];
        loop {
            self.parse_template(&mut nodes)?;
            let tok = self
                .lexer
                .read_string_literal_double(None, delimiter, level)?;
            let mut loc = tok.loc();
            match tok.kind {
                TokenKind::StringLit(mut s) => {
                    loop {
                        match self.peek_no_term()?.kind {
                            TokenKind::StringLit(next_s) => {
                                let t = self.get()?;
                                s += &next_s;
                                loc = loc.merge(t.loc);
                            }
                            TokenKind::OpenString(next_s, _, _) => {
                                let t = self.get()?;
                                s += &next_s;
                                loc = loc.merge(t.loc);
                                break;
                            }
                            _ => {
                                nodes.push(Node::new_string(s, loc));
                                return Ok(Node::new_interporated_string(
                                    nodes,
                                    start_loc.merge(loc),
                                ));
                            }
                        }
                    }
                    nodes.push(Node::new_string(s.clone(), loc));
                }
                TokenKind::OpenString(s, _, _) => {
                    nodes.push(Node::new_string(s.clone(), loc));
                }
                _ => unreachable!(format!("{:?}", tok)),
            }
        }
    }

    /// Parse template (#{..}, #$s, #@a).
    fn parse_template(&mut self, nodes: &mut Vec<Node>) -> Result<(), ParseErr> {
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

    pub fn parse_percent_notation(&mut self) -> Result<Node, ParseErr> {
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

    pub fn parse_heredocument(&mut self) -> Result<Node, ParseErr> {
        if self.lexer.trailing_space() {
            let loc = self.prev_loc();
            return Err(self.error_unexpected(loc, "Unexpectd <<."));
        }
        self.lexer.read_heredocument()
    }

    pub fn parse_hash_literal(&mut self) -> Result<Node, ParseErr> {
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

    pub fn parse_symbol(&mut self) -> Result<Node, ParseErr> {
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

    pub fn parse_regexp(&mut self) -> Result<Node, ParseErr> {
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

    pub fn parse_lambda_literal(&mut self) -> Result<Node, ParseErr> {
        // Lambda literal
        let loc = self.prev_loc();
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
        let body = if self.consume_punct(Punct::LBrace)? {
            let body = self.parse_comp_stmt()?;
            self.expect_punct(Punct::RBrace)?;
            body
        } else if self.consume_reserved(Reserved::Do)? {
            let body = self.parse_comp_stmt()?;
            self.expect_reserved(Reserved::End)?;
            body
        } else {
            let loc = self.loc();
            let tok = self.get()?;
            return Err(
                self.error_unexpected(loc, format!("Expected 'do' or '{{'. Actual:{:?}", tok.kind))
            );
        };
        let lvar = self.context_stack.pop().unwrap().lvar;
        Ok(Node::new_proc(params, body, lvar, loc))
    }
}
