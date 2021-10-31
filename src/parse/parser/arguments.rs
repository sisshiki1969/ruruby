use super::*;

impl<'a> Parser<'a> {
    pub(crate) fn parse_arglist_block(
        &mut self,
        delimiter: impl Into<Option<Punct>>,
    ) -> Result<ArgList, ParseErr> {
        let mut arglist = self.parse_argument_list(delimiter)?;
        if let Some(actual_block) = self.parse_block()? {
            if arglist.block.is_some() {
                return Err(Self::error_unexpected(
                    actual_block.loc(),
                    "Both block arg and actual block given.",
                ));
            }
            arglist.block = Some(actual_block);
        };
        Ok(arglist)
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
            if self.consume_punct(Punct::Range3)? {
                self.check_delegate()?;
                arglist.delegate = true;
            } else if self.consume_punct(Punct::Mul)? {
                // splat argument
                let loc = self.prev_loc();
                let array = self.parse_arg()?;
                arglist.args.push(Node::new_splat(array, loc));
            } else if self.consume_punct(Punct::DMul)? {
                // double splat argument
                arglist.hash_splat.push(self.parse_arg()?);
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
}
