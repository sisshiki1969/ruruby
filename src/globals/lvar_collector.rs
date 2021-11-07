use crate::*;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LvarCollector {
    pub kw: Vec<LvarId>,
    pub table: LvarTable,
    kwrest: Option<LvarId>,
    block: Option<LvarId>,
    pub delegate_param: Option<LvarId>,
}

impl LvarCollector {
    pub(crate) fn from(id: IdentId) -> Self {
        let mut table = LvarTable::new();
        table.push(id);
        Self {
            kw: vec![],
            table,
            kwrest: None,
            block: None,
            delegate_param: None,
        }
    }
}

impl LvarCollector {
    /// Create new `LvarCollector`.
    pub(crate) fn new() -> Self {
        LvarCollector {
            kw: vec![],
            table: LvarTable::new(),
            kwrest: None,
            block: None,
            delegate_param: None,
        }
    }

    /// Check whether `val` exists in `LvarCollector` or not, and return `LvarId` if exists.
    /// If not, add new variable `val` to the `LvarCollector`.
    pub fn insert(&mut self, val: IdentId) -> LvarId {
        match self.table.get_lvarid(val) {
            Some(id) => id,
            None => {
                self.table.push(val);
                LvarId::from(self.len() - 1)
            }
        }
    }

    /// Add a new variable `val` to the `LvarCollector`.
    /// Return None if `val` already exists.
    pub fn insert_new(&mut self, val: IdentId) -> Option<LvarId> {
        match self.table.get_lvarid(val) {
            Some(_) => None,
            None => {
                self.table.push(val);
                Some(LvarId::from(self.len() - 1))
            }
        }
    }

    pub fn insert_block_param(&mut self, val: IdentId) -> Option<LvarId> {
        let lvar = self.insert_new(val)?;
        self.block = Some(lvar);
        Some(lvar)
    }

    pub fn insert_kwrest_param(&mut self, val: IdentId) -> Option<LvarId> {
        let lvar = self.insert_new(val)?;
        self.kwrest = Some(lvar);
        Some(lvar)
    }

    pub fn insert_delegate_param(&mut self) -> Option<LvarId> {
        let lvar = self.insert_new(IdentId::get_id("..."))?;
        self.delegate_param = Some(lvar);
        Some(lvar)
    }

    pub(crate) fn get_name_id(&self, id: LvarId) -> Option<IdentId> {
        self.table.get(id.as_usize())
    }

    pub(crate) fn get_name(&self, id: LvarId) -> String {
        match self.get_name_id(id) {
            Some(id) => format!("{:?}", id),
            None => "<unnamed>".to_string(),
        }
    }

    pub(crate) fn kwrest_param(&self) -> Option<LvarId> {
        self.kwrest
    }

    pub(crate) fn block_param(&self) -> Option<LvarId> {
        self.block
    }

    pub(crate) fn len(&self) -> usize {
        self.table.0.len()
    }

    pub(crate) fn table(&self) -> &Vec<IdentId> {
        &self.table.0
    }

    #[cfg(feature = "emit-iseq")]
    pub(crate) fn block(&self) -> &Option<LvarId> {
        &self.block
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LvarTable(Vec<IdentId>);

impl LvarTable {
    pub(crate) fn new() -> Self {
        Self(vec![])
    }

    pub(crate) fn get_lvarid(&self, id: IdentId) -> Option<LvarId> {
        self.0
            .iter()
            .position(|i| *i == id)
            .map(|i| LvarId::from(i))
    }

    fn push(&mut self, id: IdentId) {
        self.0.push(id)
    }

    fn get(&self, i: usize) -> Option<IdentId> {
        self.0.get(i).cloned()
    }
}
