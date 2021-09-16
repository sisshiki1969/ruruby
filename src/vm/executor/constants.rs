use crate::*;

// Handling constants.

// public API
impl VM {
    /// Search class inheritance chain of `class` for a constant `id`, returning the value.
    /// Returns name error if the constant was not defined.
    pub fn get_super_const(&mut self, mut class: Module, id: IdentId) -> VMResult {
        let is_module = class.is_module();
        loop {
            match self.get_mut_const(class, id)? {
                Some(val) => return Ok(val),
                None => match class.upper() {
                    Some(upper) => class = upper,
                    None => {
                        if is_module {
                            if let Some(v) = self.get_mut_const(BuiltinClass::object(), id)? {
                                return Ok(v);
                            }
                        }
                        return Err(RubyError::uninitialized_constant(id));
                    }
                },
            }
        }
    }

    pub fn enumerate_const(&self) -> Vec<IdentId> {
        let mut map = FxHashSet::default();
        self.enumerate_env_const(&mut map);
        self.enumerate_super_const(&mut map);
        map.into_iter().collect()
    }
}

impl VM {
    /// Search lexical class stack and then, search class inheritance chain for a constant `id`,
    /// returning the value.
    /// Returns error if the constant was not defined, or autoload failed.
    pub(super) fn find_const(&mut self, id: IdentId) -> VMResult {
        match self.get_lexical_const(id)? {
            Some(v) => Ok(v),
            None => {
                let class = self.self_value().get_class();
                self.get_super_const(class, id)
            }
        }
    }

    /// Search constant table of `parent` for a constant `id`.
    /// If the constant was found, returns the value.
    /// Returns error if the constant was not defined or an autoload failed.
    pub(super) fn get_scope(&mut self, parent: Module, id: IdentId) -> VMResult {
        match self.get_mut_const(parent, id)? {
            Some(val) => Ok(val),
            None => Err(RubyError::uninitialized_constant(id)),
        }
    }

    /// Search lexical class stack for a constant `id`.
    /// If the constant was found, returns Ok(Some(Value)), and if not, returns Ok(None).
    /// Returns error if an autoload failed.
    fn get_lexical_const(&mut self, id: IdentId) -> Result<Option<Value>, RubyError> {
        let class_defined = &self.get_method_iseq().class_defined;
        for m in class_defined.iter().rev() {
            match self.get_mut_const(*m, id)? {
                Some(v) => return Ok(Some(v)),
                None => {}
            }
        }
        Ok(None)
    }

    fn enumerate_env_const(&self, map: &mut FxHashSet<IdentId>) {
        let class_defined = &self.get_method_iseq().class_defined;
        class_defined.iter().for_each(|m| {
            m.enumerate_const().for_each(|id| {
                map.insert(*id);
            })
        });
    }

    fn enumerate_super_const(&self, map: &mut FxHashSet<IdentId>) {
        let mut class = self.self_value().get_class();
        let is_module = class.is_module();
        loop {
            class.enumerate_const().into_iter().for_each(|id| {
                map.insert(*id);
            });
            match class.upper() {
                Some(upper) => class = upper,
                None => {
                    if is_module {
                        BuiltinClass::object()
                            .enumerate_const()
                            .into_iter()
                            .for_each(|id| {
                                map.insert(*id);
                            })
                    }
                    break;
                }
            }
        }
    }

    /// Search constant table of `parent` for a constant `id`.
    /// If the constant was found, returns Ok(Some(Value)), and if not, returns Ok(None).
    /// Returns error if an autoload failed.
    fn get_mut_const(
        &mut self,
        mut parent: Module,
        id: IdentId,
    ) -> Result<Option<Value>, RubyError> {
        match parent.get_mut_const(id) {
            Some(ConstEntry::Value(v)) => Ok(Some(*v)),
            Some(ConstEntry::Autoload(file)) => {
                self.require(file)?;
                match parent.get_mut_const(id) {
                    Some(ConstEntry::Value(v)) => Ok(Some(*v)),
                    _ => Ok(None),
                }
            }
            None => Ok(None),
        }
    }
}
