use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Annot<T> {
    pub kind: T,
    loc: Loc,
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
        for line in &self.line_pos {
            if line.2 < loc.0 || line.1 > loc.1 {
                continue;
            }
            println!("line {:?}", line);
            println!("loc {:?}", loc);
            println!(
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
            println!("{}{}", " ".repeat(read), "^".repeat(length + 1));
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

impl IdentId {
    pub fn from_usize(id: usize) -> Self {
        IdentId(id)
    }
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdentifierTable {
    table: HashMap<String, usize>,
    table_rev: HashMap<usize, String>,
    ident_id: usize,
}

impl IdentifierTable {
    pub fn new() -> Self {
        IdentifierTable {
            table: HashMap::new(),
            table_rev: HashMap::new(),
            ident_id: 0,
        }
    }

    pub fn get_ident_id(&mut self, name: &String) -> IdentId {
        match self.table.get(name) {
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

    pub fn get_name(&mut self, id: IdentId) -> &String {
        self.table_rev.get(&id).unwrap()
    }
}
