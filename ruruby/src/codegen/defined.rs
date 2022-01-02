use super::*;

impl Codegen {
    /// Generate code for `defined?`.
    pub(super) fn gen_defined(
        &mut self,
        globals: &mut Globals,
        iseq: &mut ISeq,
        content: Node,
    ) -> Result<(), RubyError> {
        let mut nil_labels = vec![];
        let mut exceptions = vec![];
        let s = defined_str(&content);
        self.check_defined(globals, &content, iseq, &mut nil_labels, &mut exceptions)?;
        iseq.gen_string(globals, s);
        let end = iseq.gen_jmp();
        let pos = iseq.current();
        for ex in exceptions {
            self.push_ex_continue(ex.0, ex.1, pos);
        }
        iseq.push(Inst::POP);
        nil_labels
            .iter()
            .for_each(|label| iseq.write_disp_from_cur(*label));
        iseq.gen_push_nil();
        iseq.write_disp_from_cur(end);

        Ok(())
    }

    /// Helper for `defined?`.
    /// Check `node`, and generate bytecode into `iseq`.
    /// Collect destinations for returning nil into `labels`.
    pub(super) fn check_defined(
        &mut self,
        globals: &mut Globals,
        node: &Node,
        iseq: &mut ISeq,
        nil_labels: &mut Vec<ISeqPos>,
        exceptions: &mut Vec<(ISeqPos, ISeqPos)>,
    ) -> Result<(), RubyError> {
        match &node.kind {
            NodeKind::LocalVar(id) => {
                match self.get_local_var(*id) {
                    Some((outer, lvar_id)) => {
                        iseq.push(Inst::CHECK_LOCAL);
                        iseq.push32(lvar_id.as_u32());
                        iseq.push32(outer);
                        nil_labels.push(iseq.gen_jmp_if_t());
                    }
                    None => {
                        nil_labels.push(iseq.gen_jmp());
                    }
                };
                Ok(())
            }
            NodeKind::Ident(id) => {
                iseq.push(Inst::PUSH_SELF);
                iseq.push(Inst::CHECK_METHOD);
                iseq.push32((*id).into());
                nil_labels.push(iseq.gen_jmp_if_t());
                Ok(())
            }
            NodeKind::GlobalVar(id) => {
                iseq.push(Inst::CHECK_GVAR);
                iseq.push32((*id).into());
                nil_labels.push(iseq.gen_jmp_if_t());
                Ok(())
            }
            NodeKind::InstanceVar(id) => {
                iseq.push(Inst::CHECK_IVAR);
                iseq.push32((*id).into());
                nil_labels.push(iseq.gen_jmp_if_t());
                Ok(())
            }
            NodeKind::Const {
                toplevel: false,
                id,
            } => {
                iseq.push(Inst::CHECK_CONST);
                iseq.push32((*id).into());
                nil_labels.push(iseq.gen_jmp_if_t());
                Ok(())
            }
            NodeKind::Const { toplevel: true, id } => {
                self.emit_get_const(globals, iseq, IdentId::get_id("Object"));
                iseq.push(Inst::CHECK_SCOPE);
                iseq.push32((*id).into());
                nil_labels.push(iseq.gen_jmp_if_t());
                Ok(())
            }
            NodeKind::Scope(box parent, id) => {
                self.check_defined(globals, parent, iseq, nil_labels, exceptions)?;
                self.gen(globals, iseq, parent.clone(), true)?;
                iseq.push(Inst::CHECK_SCOPE);
                iseq.push32((*id).into());
                nil_labels.push(iseq.gen_jmp_if_t());
                Ok(())
            }
            NodeKind::Array(elems, ..) => {
                for n in elems {
                    self.check_defined(globals, n, iseq, nil_labels, exceptions)?
                }
                Ok(())
            }
            NodeKind::AssignOp(_, box lhs, _) => match lhs.kind {
                NodeKind::LocalVar(id) | NodeKind::Ident(id) => {
                    iseq.gen_push_nil();
                    self.emit_set_local(iseq, id);
                    Ok(())
                }
                _ => Ok(()),
            },
            NodeKind::MulAssign(mlhs, _) => {
                for lhs in mlhs {
                    match lhs.kind {
                        NodeKind::LocalVar(id) | NodeKind::Ident(id) => {
                            iseq.gen_push_nil();
                            self.emit_set_local(iseq, id);
                        }
                        _ => {}
                    }
                }
                Ok(())
            }
            NodeKind::BinOp(op, box lhs, box rhs) => {
                self.check_defined(globals, lhs, iseq, nil_labels, exceptions)?;
                self.check_defined(globals, rhs, iseq, nil_labels, exceptions)?;
                self.gen(globals, iseq, lhs.clone(), true)?;
                iseq.push(Inst::CHECK_METHOD);
                iseq.push32((op.to_method()).into());
                nil_labels.push(iseq.gen_jmp_if_t());
                Ok(())
            }
            NodeKind::UnOp(op, box node) => {
                self.check_defined(globals, node, iseq, nil_labels, exceptions)?;
                self.gen(globals, iseq, node.clone(), true)?;
                iseq.push(Inst::CHECK_METHOD);
                iseq.push32((op.to_method()).into());
                nil_labels.push(iseq.gen_jmp_if_t());
                Ok(())
            }
            NodeKind::Index { box base, index } => {
                self.check_defined(globals, base, iseq, nil_labels, exceptions)?;
                for i in index {
                    self.check_defined(globals, i, iseq, nil_labels, exceptions)?
                }
                self.gen(globals, iseq, base.clone(), true)?;
                iseq.push(Inst::CHECK_METHOD);
                iseq.push32(IdentId::get_id("[]").into());
                nil_labels.push(iseq.gen_jmp_if_t());
                Ok(())
            }
            NodeKind::Send {
                box receiver,
                arglist,
                method,
                ..
            } => {
                self.check_defined(globals, receiver, iseq, nil_labels, exceptions)?;
                for n in &arglist.args {
                    self.check_defined(globals, n, iseq, nil_labels, exceptions)?
                }
                let start = iseq.current();
                self.gen(globals, iseq, receiver.clone(), true)?;
                let end = iseq.current();
                exceptions.push((start, end));
                iseq.push(Inst::CHECK_METHOD);
                iseq.push32((*method).into());
                nil_labels.push(iseq.gen_jmp_if_t());
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

/// This method returns str corresponding to the type of `self` for `defined?`.
fn defined_str(node: &Node) -> &'static str {
    match &node.kind {
        NodeKind::LocalVar(..) => "local-variable",
        NodeKind::Ident(_) => "method",
        NodeKind::GlobalVar(..) => "global-variable",
        NodeKind::ClassVar(..) => "class variable",
        NodeKind::Const { .. } | NodeKind::Scope(..) => "constant",
        NodeKind::InstanceVar(..) => "instance-variable",
        NodeKind::MulAssign(mlhs, _) => {
            if mlhs.len() != 1 {
                "assignment"
            } else if let NodeKind::Index { .. } = mlhs[0].kind {
                "method"
            } else {
                "assignment"
            }
        }
        NodeKind::AssignOp(..) => "assignment",
        NodeKind::BinOp(..)
        | NodeKind::UnOp(..)
        | NodeKind::Index { .. }
        | NodeKind::Send { .. } => "method",
        NodeKind::Bool(true) => "true",
        NodeKind::Bool(false) => "false",
        NodeKind::Nil => "nil",
        NodeKind::SelfValue => "self",
        _ => "expression",
    }
}
