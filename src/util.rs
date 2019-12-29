use core::ptr::NonNull;
use std::collections::HashMap;

const INITIALIZE: usize = 0;
const OBJECT: usize = 1;
const NEW: usize = 2;
const _ADD: usize = 3;
const _SUB: usize = 4;
const _MUL: usize = 5;

#[derive(Debug, Clone, PartialEq)]
pub struct Annot<T> {
    pub kind: T,
    pub loc: Loc,
}

impl<T> Annot<T> {
    pub fn new(kind: T, loc: Loc) -> Self {
        Annot { kind, loc }
    }

    pub fn loc(&self) -> Loc {
        self.loc
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Loc(pub usize, pub usize);

impl Loc {
    pub fn new(loc: Loc) -> Self {
        loc
    }

    pub fn dec(&self) -> Self {
        use std::cmp::*;
        Loc(min(self.0, self.1 - 1), self.1 - 1)
    }

    pub fn merge(&self, loc: Loc) -> Self {
        use std::cmp::*;
        Loc(min(self.0, loc.0), max(self.1, loc.1))
    }
}

//------------------------------------------------------------

#[derive(Debug)]
pub struct Ref<T>(pub NonNull<T>);

impl<T> Ref<T> {
    pub fn new(info: T) -> Self {
        let boxed = Box::into_raw(Box::new(info));
        Ref(unsafe { NonNull::new_unchecked(boxed) })
    }
}

impl<T> Copy for Ref<T> {}

impl<T> Clone for Ref<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for Ref<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for Ref<T> {}

impl<T> std::hash::Hash for Ref<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T> std::ops::Deref for Ref<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.as_ptr() }
    }
}

impl<T> std::ops::DerefMut for Ref<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0.as_ptr() }
    }
}

//------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct SourceInfo {
    pub code: Vec<char>,
    pub line_pos: Vec<(usize, usize, usize)>, // (line_no, line_top_pos, line_end_pos)
}

impl SourceInfo {
    pub fn new() -> Self {
        SourceInfo {
            code: vec![],
            line_pos: vec![],
        }
    }

    /// Show the location of the Loc in the source code using '^^^'.
    pub fn show_loc(&self, loc: &Loc) {
        let mut found = false;
        for line in &self.line_pos {
            if line.2 < loc.0 || line.1 > loc.1 {
                continue;
            }
            found = true;
            eprintln!(
                "{}",
                self.code[(line.1)..(line.2)].iter().collect::<String>()
            );
            use std::cmp::*;
            let read = if loc.0 <= line.1 {
                0
            } else {
                self.code[(line.1)..(loc.0)]
                    .iter()
                    .map(|x| calc_width(x))
                    .sum()
            };
            let length: usize = self.code[max(loc.0, line.1)..min(loc.1, line.2)]
                .iter()
                .map(|x| calc_width(x))
                .sum();
            eprintln!("{}{}", " ".repeat(read), "^".repeat(length + 1));
        }

        if !found {
            let line = match self.line_pos.last() {
                Some(line) => (line.0 + 1, line.2 + 1, loc.1),
                None => (1, 0, loc.1),
            };
            let read = self.code[(line.1)..(loc.0)]
                .iter()
                .map(|x| calc_width(x))
                .sum();
            let length: usize = self.code[loc.0..loc.1].iter().map(|x| calc_width(x)).sum();
            let is_cr = self.code[loc.1] == '\n';
            eprintln!(
                "{}",
                if !is_cr {
                    self.code[(line.1)..=(loc.1)].iter().collect::<String>()
                } else {
                    self.code[(line.1)..(loc.1)].iter().collect::<String>()
                }
            );
            eprintln!("{}{}", " ".repeat(read), "^".repeat(length + 1));
        }

        fn calc_width(ch: &char) -> usize {
            if ch.is_ascii() {
                1
            } else {
                2
            }
        }
    }
}

//------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IdentId(usize);

impl std::ops::Deref for IdentId {
    type Target = usize;
    fn deref(&self) -> &usize {
        &self.0
    }
}

impl std::hash::Hash for IdentId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Into<usize> for IdentId {
    fn into(self) -> usize {
        self.0
    }
}

impl Into<u32> for IdentId {
    fn into(self) -> u32 {
        self.0 as u32
    }
}

impl From<u32> for IdentId {
    fn from(id: u32) -> Self {
        IdentId(id as usize)
    }
}

impl IdentId {
    pub const INITIALIZE: IdentId = IdentId(INITIALIZE);
    pub const OBJECT: IdentId = IdentId(OBJECT);
    pub const NEW: IdentId = IdentId(NEW);
    pub const _ADD: IdentId = IdentId(_ADD);
    pub const _SUB: IdentId = IdentId(_SUB);
    pub const _MUL: IdentId = IdentId(_MUL);
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdentifierTable {
    table: HashMap<String, usize>,
    table_rev: HashMap<usize, String>,
    ident_id: usize,
}

impl IdentifierTable {
    pub fn new() -> Self {
        let mut table = IdentifierTable {
            table: HashMap::new(),
            table_rev: HashMap::new(),
            ident_id: 20,
        };
        table.set_ident_id("initialize", INITIALIZE);
        table.set_ident_id("Object", OBJECT);
        table.set_ident_id("new", NEW);
        table.set_ident_id("@add", _ADD);
        table.set_ident_id("@sub", _SUB);
        table.set_ident_id("@mul", _MUL);
        table
    }

    fn set_ident_id(&mut self, name: impl Into<String>, id: usize) {
        let name = name.into();
        self.table.insert(name.clone(), id);
        self.table_rev.insert(id, name);
    }

    pub fn get_ident_id(&mut self, name: impl Into<String>) -> IdentId {
        let name = name.into();
        match self.table.get(&name) {
            Some(id) => IdentId(*id),
            None => {
                let id = self.ident_id;
                self.table.insert(name.clone(), id);
                self.table_rev.insert(id, name.clone());
                self.ident_id += 1;
                IdentId(id)
            }
        }
    }

    pub fn get_name(&self, id: IdentId) -> &String {
        self.table_rev.get(&id).unwrap()
    }
}
