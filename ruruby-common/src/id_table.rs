use fxhash::FxHashMap;
use std::fmt;
use std::num::NonZeroU32;
use std::sync::LazyLock;
use std::sync::{Arc, Mutex};

static ID: LazyLock<Arc<Mutex<IdentifierTable>>> =
    LazyLock::new(|| Arc::new(Mutex::new(IdentifierTable::new())));

///
/// Wrapper of ID for strings.
///
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdentId(NonZeroU32);

impl fmt::Debug for IdentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_name())
    }
}

impl From<IdentId> for usize {
    #[inline(always)]
    fn from(id: IdentId) -> usize {
        id.0.get() as usize
    }
}

impl From<IdentId> for u32 {
    #[inline(always)]
    fn from(id: IdentId) -> u32 {
        id.0.get()
    }
}

impl From<u32> for IdentId {
    #[inline(always)]
    fn from(id: u32) -> Self {
        let id = NonZeroU32::new(id).unwrap();
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
    pub const _CMP: IdentId = id!(18);
    pub const _TEQ: IdentId = id!(19);
    pub const _ENUM_FUNC: IdentId = id!(20);
    pub const _INDEX: IdentId = id!(21);
    pub const _INDEX_ASSIGN: IdentId = id!(22);
    pub const TO_S: IdentId = id!(23);
    pub const _SHR: IdentId = id!(24);
    pub const _ALIAS_METHOD: IdentId = id!(25);
    pub const _METHOD_MISSING: IdentId = id!(26);
    pub const EACH: IdentId = id!(27);
    pub const MAP: IdentId = id!(28);
    pub const _NAME: IdentId = id!(29);
}

impl IdentId {
    fn to_usize(&self) -> usize {
        self.0.get() as usize
    }

    #[inline(always)]
    pub fn get_id<'a>(name: &str) -> Self {
        ID.lock().unwrap().get_ident_id(name)
    }

    #[inline(always)]
    pub fn get_id_from_string<'a>(name: String) -> Self {
        ID.lock().unwrap().get_ident_id_from_string(name)
    }

    #[inline(always)]
    pub fn get_name(&self) -> String {
        ID.lock().unwrap().get_name(*self).to_string()
    }

    #[inline(always)]
    pub fn get_ident_name(id: impl Into<Option<IdentId>>) -> String {
        match id.into() {
            Some(id) => id.get_name(),
            None => "".to_string(),
        }
    }

    #[inline(always)]
    pub fn starts_with(&self, pat: &str) -> bool {
        ID.lock().unwrap().get_name(*self).starts_with(pat)
    }

    #[inline(always)]
    pub fn is_constant(&self) -> bool {
        ID.lock()
            .unwrap()
            .get_name(*self)
            .starts_with(|c: char| c.is_ascii_uppercase())
    }

    #[inline(always)]
    pub fn is_global_var(&self) -> bool {
        ID.lock().unwrap().get_name(*self).starts_with('$')
    }

    #[inline(always)]
    pub fn is_class_var(&self) -> bool {
        ID.lock().unwrap().get_name(*self).starts_with("@@")
    }

    #[inline(always)]
    pub fn add_postfix(&self, postfix: &str) -> IdentId {
        let new_name = format!("{:?}{}", *self, postfix);
        IdentId::get_id_from_string(new_name)
    }

    #[inline(always)]
    pub fn add_prefix(&self, prefix: &str) -> IdentId {
        let new_name = format!("{}{:?}", prefix, *self);
        IdentId::get_id_from_string(new_name)
    }
}

#[derive(Debug, Clone, PartialEq)]
struct IdentifierTable {
    rev_table: FxHashMap<String, IdentId>,
    table: Vec<String>,
}

impl IdentifierTable {
    pub(crate) fn new() -> Self {
        let mut table = IdentifierTable {
            rev_table: FxHashMap::default(),
            table: vec![String::new(); 40],
        };
        table.table[0] = "<null>".to_string();
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
        table.set_ident_id("<=>", IdentId::_CMP);
        table.set_ident_id("===", IdentId::_TEQ);
        table.set_ident_id("/enum", IdentId::_ENUM_FUNC);
        table.set_ident_id("[]", IdentId::_INDEX);
        table.set_ident_id("[]=", IdentId::_INDEX_ASSIGN);
        table.set_ident_id("to_s", IdentId::TO_S);
        table.set_ident_id(">>", IdentId::_SHR);
        table.set_ident_id("/alias_method", IdentId::_ALIAS_METHOD);
        table.set_ident_id("method_missing", IdentId::_METHOD_MISSING);
        table.set_ident_id("each", IdentId::EACH);
        table.set_ident_id("map", IdentId::MAP);
        table.set_ident_id("/name", IdentId::_NAME);
        table
    }

    fn set_ident_id(&mut self, name: &str, id: IdentId) {
        self.rev_table.insert(name.to_string(), id);
        self.table[id.to_usize()] = name.to_string();
    }

    fn get_ident_id<'a>(&mut self, name: &str) -> IdentId {
        match self.rev_table.get(name) {
            Some(id) => (*id).into(),
            None => {
                let id = IdentId::from(self.table.len() as u32);
                self.rev_table.insert(name.to_string(), id);
                self.table.push(name.to_string());
                id.into()
            }
        }
    }

    fn get_ident_id_from_string<'a>(&mut self, name: String) -> IdentId {
        match self.rev_table.get(&name) {
            Some(id) => (*id).into(),
            None => {
                let id = IdentId::from(self.table.len() as u32);
                self.rev_table.insert(name.clone(), id);
                self.table.push(name);
                id.into()
            }
        }
    }

    fn get_name(&self, id: IdentId) -> &str {
        &self.table[id.to_usize()]
    }
}
