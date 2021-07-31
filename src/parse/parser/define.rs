use super::*;

impl<'a> Parser<'a> {
    /// Parse method definition.
    pub fn parse_def(&mut self) -> Result<Node, ParseErr> {
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
                let s = Lexer::get_string_from_reserved(r);
                (None, self.method_def_ext(&s)?)
            }
            TokenKind::Ident(s) => {
                if self.consume_punct_no_term(Punct::Dot)?
                    || self.consume_punct_no_term(Punct::Scope)?
                {
                    let id = IdentId::get_id(s);
                    (Some(Node::new_lvar(id, loc)), self.parse_method_def_name()?)
                } else {
                    (None, self.method_def_ext(s)?)
                }
            }
            TokenKind::Const(s) => {
                if self.consume_punct_no_term(Punct::Dot)?
                    || self.consume_punct_no_term(Punct::Scope)?
                {
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

        self.context_stack.push(ParseContext::new_method(name));
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
        let decl = match singleton {
            Some(singleton) => Node::new_singleton_method_decl(singleton, name, args, body, lvar),
            None => Node::new_method_decl(name, args, body, lvar),
        };
        Ok(decl)
    }
    /// Parse class definition.
    pub fn parse_class(&mut self, is_module: bool) -> Result<Node, ParseErr> {
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
        self.context_stack.push(ParseContext::new_class(name, None));
        let body = self.parse_begin()?;
        let lvar = self.context_stack.pop().unwrap().lvar;
        #[cfg(feature = "verbose")]
        eprintln!("***parsed");
        Ok(Node::new_class_decl(
            base, name, superclass, body, lvar, is_module, loc,
        ))
    }

    /// Parse singleton class definition.
    pub fn parse_singleton_class(&mut self, loc: Loc) -> Result<Node, ParseErr> {
        // class "<<" EXPR <term>
        //      COMPSTMT
        // end
        let singleton = self.parse_expr()?;
        #[cfg(feature = "verbose")]
        eprintln!("***parsing.. singleton class {:?}", singleton.kind);
        let loc = loc.merge(self.prev_loc());
        self.consume_term()?;
        self.context_stack
            .push(ParseContext::new_class(IdentId::get_id("Singleton"), None));
        let body = self.parse_begin()?;
        let lvar = self.context_stack.pop().unwrap().lvar;
        #[cfg(feature = "verbose")]
        eprintln!("***parsed");
        Ok(Node::new_singleton_class_decl(singleton, body, lvar, loc))
    }

    fn parse_class_def_name(&mut self) -> Result<Node, ParseErr> {
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

    pub fn alias_name(&mut self) -> Result<Node, ParseErr> {
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

    /// Parse method definition name.
    fn parse_method_def_name(&mut self) -> Result<IdentId, ParseErr> {
        // メソッド定義名 : メソッド名 ｜ ( 定数識別子 | 局所変数識別子 ) "="
        // メソッド名 : 局所変数識別子
        //      | 定数識別子
        //      | ( 定数識別子 | 局所変数識別子 ) ( "!" | "?" )
        //      | 演算子メソッド名
        //      | キーワード
        // 演算子メソッド名 : “^” | “&” | “|” | “<=>” | “==” | “===” | “=~” | “>” | “>=” | “<” | “<=”
        //      | “<<” | “>>” | “+” | “-” | “*” | “/” | “%” | “**” | “~” | “+@” | “-@” | “[]” | “[]=” | “ʻ”
        let tok = self.get()?;
        let id = match &tok.kind {
            TokenKind::Reserved(r) => {
                let s = Lexer::get_string_from_reserved(r);
                self.method_def_ext(&s)?
            }
            TokenKind::Ident(name) | TokenKind::Const(name) => self.method_def_ext(name)?,
            TokenKind::Punct(p) => self.parse_op_definable(p)?,
            _ => {
                let loc = tok.loc.merge(self.prev_loc());
                return Err(self.error_unexpected(loc, "Expected identifier or operator."));
            }
        };
        Ok(id)
    }

    // ( )
    // ( ident [, ident]* )
    fn parse_def_params(&mut self) -> Result<Vec<FormalParam>, ParseErr> {
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
}
