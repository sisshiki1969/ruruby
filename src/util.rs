use core::ptr::NonNull;
use std::collections::HashMap;

const INITIALIZE: u32 = 1;
const OBJECT: u32 = 2;
const NEW: u32 = 3;
const _ADD: u32 = 4;
const _SUB: u32 = 5;
const _MUL: u32 = 6;
const _POW: u32 = 7;

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
pub struct Loc(pub u32, pub u32);

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

    pub fn inner(&self) -> &T {
        unsafe { &*self.0.as_ptr() }
    }

    pub fn id(&self) -> u64 {
        self.0.as_ptr() as u64
    }
}

impl<T: Clone> Ref<T> {
    pub fn dup(&self) -> Self {
        Self::new(self.inner().clone())
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

pub type SourceInfoRef = Ref<SourceInfo>;

#[derive(Debug, Clone, PartialEq)]
pub struct SourceInfo {
    pub path: String,
    pub code: Vec<char>,
}

impl SourceInfoRef {
    pub fn empty() -> Self {
        SourceInfoRef::new(SourceInfo::new(""))
    }
}

impl SourceInfo {
    pub fn new(path: impl Into<String>) -> Self {
        SourceInfo {
            path: path.into(),
            code: vec![],
        }
    }
    pub fn show_file_name(&self) {
        eprintln!("{}", self.path);
    }

    /// Show the location of the Loc in the source code using '^^^'.
    pub fn show_loc(&self, loc: &Loc) {
        let mut line: u32 = 1;
        let mut line_top_pos: u32 = 0;
        let mut line_pos = vec![];
        for (pos, ch) in self.code.iter().enumerate() {
            if *ch == '\n' {
                line_pos.push((line, line_top_pos, pos as u32));
                line += 1;
                line_top_pos = pos as u32 + 1;
            }
        }
        if line_top_pos as usize <= self.code.len() - 1 {
            line_pos.push((line, line_top_pos, self.code.len() as u32 - 1));
        }

        let mut found = false;
        for line in &line_pos {
            if line.2 < loc.0 || line.1 > loc.1 {
                continue;
            }
            if !found {
                eprintln!("line: {}", line.0)
            };
            found = true;
            eprintln!(
                "{}",
                self.code[(line.1 as usize)..(line.2 as usize)]
                    .iter()
                    .collect::<String>()
            );
            use std::cmp::*;
            let read = if loc.0 <= line.1 {
                0
            } else {
                self.code[(line.1 as usize)..(loc.0 as usize)]
                    .iter()
                    .map(|x| calc_width(x))
                    .sum()
            };
            let length: usize = self.code[max(loc.0, line.1) as usize..min(loc.1, line.2) as usize]
                .iter()
                .map(|x| calc_width(x))
                .sum();
            eprintln!("{}{}", " ".repeat(read), "^".repeat(length + 1));
        }

        if !found {
            let line = match line_pos.last() {
                Some(line) => (line.0 + 1, line.2 + 1, loc.1),
                None => (1, 0, loc.1),
            };
            let read = self.code[(line.1 as usize)..(loc.0 as usize)]
                .iter()
                .map(|x| calc_width(x))
                .sum();
            let length: usize = self.code[loc.0 as usize..loc.1 as usize]
                .iter()
                .map(|x| calc_width(x))
                .sum();
            let is_cr = self.code[loc.1 as usize] == '\n';
            eprintln!("line: {}", line.0);
            eprintln!(
                "{}",
                if !is_cr {
                    self.code[(line.1 as usize)..=(loc.1 as usize)]
                        .iter()
                        .collect::<String>()
                } else {
                    self.code[(line.1 as usize)..(loc.1 as usize)]
                        .iter()
                        .collect::<String>()
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
pub struct IdentId(std::num::NonZeroU32);

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
        let id = unsafe { std::num::NonZeroU32::new_unchecked(id) };
        IdentId(id)
    }
}

pub struct OptionalId(Option<IdentId>);

impl OptionalId {
    pub fn new(id: impl Into<Option<IdentId>>) -> Self {
        OptionalId(id.into())
    }
}

impl std::ops::Deref for OptionalId {
    type Target = Option<IdentId>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

macro_rules! to {
    ($constant:ident) => {
        unsafe { std::num::NonZeroU32::new_unchecked($constant) }
    };
}

impl IdentId {
    pub const INITIALIZE: IdentId = IdentId(to!(INITIALIZE));
    pub const OBJECT: IdentId = IdentId(to!(OBJECT));
    pub const NEW: IdentId = IdentId(to!(NEW));
    pub const _ADD: IdentId = IdentId(to!(_ADD));
    pub const _SUB: IdentId = IdentId(to!(_SUB));
    pub const _MUL: IdentId = IdentId(to!(_MUL));
    pub const _POW: IdentId = IdentId(to!(_POW));
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
        table.set_ident_id("<null>", 0);
        table.set_ident_id("initialize", INITIALIZE);
        table.set_ident_id("Object", OBJECT);
        table.set_ident_id("new", NEW);
        table.set_ident_id("+", _ADD);
        table.set_ident_id("-", _SUB);
        table.set_ident_id("*", _MUL);
        table.set_ident_id("**", _POW);
        table
    }

    fn set_ident_id(&mut self, name: impl Into<String>, id: u32) {
        let name = name.into();
        self.table.insert(name.clone(), id);
        self.table_rev.insert(id, name);
    }

    pub fn get_ident_id(&mut self, name: impl Into<String>) -> IdentId {
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

    pub fn get_name(&self, id: IdentId) -> &String {
        self.table_rev.get(&id.0.get()).unwrap()
    }

    pub fn add_postfix(&mut self, id: IdentId, postfix: &str) -> IdentId {
        let new_name = self.get_name(id).to_string() + postfix;
        self.get_ident_id(new_name)
    }
}
