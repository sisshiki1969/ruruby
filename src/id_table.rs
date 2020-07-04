use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::RwLock;

lazy_static! {
    pub static ref ID: RwLock<IdentifierTable> = {
        let id = IdentifierTable::new();
        RwLock::new(id)
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentId(NonZeroU32);

impl std::hash::Hash for IdentId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Into<usize> for IdentId {
    fn into(self) -> usize {
        self.0.get() as usize
    }
}

impl Into<u32> for IdentId {
    fn into(self) -> u32 {
        self.0.get()
    }
}

impl From<u32> for IdentId {
    fn from(id: u32) -> Self {
        let id = unsafe { NonZeroU32::new_unchecked(id) };
        IdentId(id)
    }
}

macro_rules! id {
    ($constant:expr) => {
        IdentId(unsafe { std::num::NonZeroU32::new_unchecked($constant) })
    };
}

impl IdentId {
    pub const INITIALIZE: IdentId = id!(1);
    pub const OBJECT: IdentId = id!(2);
    pub const NEW: IdentId = id!(3);
    pub const NAME: IdentId = id!(4);
    pub const _ADD: IdentId = id!(5);
    pub const _SUB: IdentId = id!(6);
    pub const _MUL: IdentId = id!(7);
    pub const _POW: IdentId = id!(8);
    pub const _SHL: IdentId = id!(9);
    pub const _REM: IdentId = id!(10);
    pub const _EQ: IdentId = id!(11);
    pub const _NEQ: IdentId = id!(12);
    pub const _GT: IdentId = id!(13);
    pub const _GE: IdentId = id!(14);
    pub const _DIV: IdentId = id!(15);
    pub const _LT: IdentId = id!(16);
    pub const _LE: IdentId = id!(17);
}

impl IdentId {
    pub fn get_ident_id(name: impl Into<String>) -> Self {
        ID.write().unwrap().get_ident_id(name)
    }

    pub fn get_name(id: IdentId) -> String {
        ID.read().unwrap().get_name(id).to_string()
    }

    pub fn get_ident_name(id: impl Into<Option<IdentId>>) -> String {
        let id = id.into();
        match id {
            Some(id) => IdentId::get_name(id).to_string(),
            None => "".to_string(),
        }
    }

    pub fn add_postfix(id: IdentId, postfix: &str) -> IdentId {
        let new_name = IdentId::get_name(id) + postfix;
        IdentId::get_ident_id(new_name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdentifierTable {
    table: HashMap<String, u32>,
    table_rev: HashMap<u32, String>,
    ident_id: u32,
}

impl IdentifierTable {
    pub fn new() -> Self {
        let mut table = IdentifierTable {
            table: HashMap::new(),
            table_rev: HashMap::new(),
            ident_id: 20,
        };
        table.set_ident_id("<null>", IdentId::from(0));
        table.set_ident_id("initialize", IdentId::INITIALIZE);
        table.set_ident_id("Object", IdentId::OBJECT);
        table.set_ident_id("new", IdentId::NEW);
        table.set_ident_id("name", IdentId::NAME);
        table.set_ident_id("+", IdentId::_ADD);
        table.set_ident_id("-", IdentId::_SUB);
        table.set_ident_id("*", IdentId::_MUL);
        table.set_ident_id("**", IdentId::_POW);
        table.set_ident_id("<<", IdentId::_SHL);
        table.set_ident_id("%", IdentId::_REM);
        table.set_ident_id("==", IdentId::_EQ);
        table.set_ident_id("!=", IdentId::_NEQ);
        table.set_ident_id(">", IdentId::_GT);
        table.set_ident_id(">=", IdentId::_GE);
        table.set_ident_id("/", IdentId::_DIV);
        table.set_ident_id("<", IdentId::_LT);
        table.set_ident_id("<=", IdentId::_LE);
        table
    }

    fn set_ident_id(&mut self, name: impl Into<String>, id: IdentId) {
        let name = name.into();
        self.table.insert(name.clone(), id.into());
        self.table_rev.insert(id.into(), name);
    }

    fn get_ident_id(&mut self, name: impl Into<String>) -> IdentId {
        let name = name.into();
        match self.table.get(&name) {
            Some(id) => IdentId::from(*id),
            None => {
                let id = self.ident_id;
                self.table.insert(name.clone(), id);
                self.table_rev.insert(id, name.clone());
                self.ident_id += 1;
                IdentId::from(id)
            }
        }
    }

    pub fn get_name(&self, id: IdentId) -> &str {
        self.table_rev.get(&id.0.get()).unwrap()
    }

    pub fn get_ident_name(&self, id: impl Into<Option<IdentId>>) -> &str {
        let id = id.into();
        match id {
            Some(id) => self.get_name(id),
            None => &"",
        }
    }
}
