mod node;
pub use node::*;
pub mod parser;
pub use parser::*;
mod token;
use ruruby_common::*;
pub use token::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Annot<T: PartialEq> {
    pub kind: T,
    pub loc: Loc,
}

impl<T: PartialEq> Annot<T> {
    pub fn new(kind: T, loc: Loc) -> Self {
        Annot { kind, loc }
    }

    pub fn loc(&self) -> Loc {
        self.loc
    }
}

use enum_iterator::all;
use fxhash::FxHashMap;
use once_cell::sync::Lazy;
use std::sync::Mutex;

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
    for r in all::<Reserved>() {
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
