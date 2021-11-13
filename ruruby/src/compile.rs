mod node;
pub mod parser;
mod token;
use enum_iterator::IntoEnumIterator;
use node::*;
use token::*;
pub mod codegen;
use ruruby_common::*;

use fxhash::FxHashMap;
use once_cell::sync::Lazy;
use std::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Annot<T> {
    pub kind: T,
    pub loc: Loc,
}

impl<T: PartialEq> std::cmp::PartialEq for Annot<T> {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && self.loc == other.loc
    }
}

impl<T> Annot<T> {
    fn new(kind: T, loc: Loc) -> Self {
        Annot { kind, loc }
    }

    fn loc(&self) -> Loc {
        self.loc
    }
}

fn get_string_from_reserved(reserved: &Reserved) -> String {
    RESERVED
        .lock()
        .unwrap()
        .reserved_rev
        .get(reserved)
        .unwrap()
        .clone()
}

fn check_reserved(reserved: &str) -> Option<Reserved> {
    RESERVED.lock().unwrap().reserved.get(reserved).cloned()
}

static RESERVED: Lazy<Mutex<ReservedChecker>> = Lazy::new(|| {
    let mut reserved = FxHashMap::default();
    let mut reserved_rev = FxHashMap::default();
    for r in Reserved::into_enum_iter() {
        reserved.insert(format!("{:?}", r), r);
        reserved_rev.insert(r, format!("{:?}", r));
    }

    Mutex::new(ReservedChecker {
        reserved,
        reserved_rev,
    })
});
pub struct ReservedChecker {
    reserved: FxHashMap<String, Reserved>,
    reserved_rev: FxHashMap<Reserved, String>,
}
