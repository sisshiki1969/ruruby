use core::ptr::NonNull;
use std::path::PathBuf;

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
pub struct Ref<T>(NonNull<T>);

impl<T> Ref<T> {
    pub fn new(info: T) -> Self {
        let boxed = Box::into_raw(Box::new(info));
        Ref(NonNull::new(boxed).unwrap_or_else(|| panic!("Ref::new(): the pointer is NULL.")))
    }

    pub fn free(self) {
        unsafe { Box::from_raw(self.as_ptr()) };
    }

    pub fn from_ref(info: &T) -> Self {
        Ref(NonNull::new(info as *const T as *mut T)
            .unwrap_or_else(|| panic!("Ref::from_ref(): the pointer is NULL.")))
    }

    pub fn from_ptr(info: *mut T) -> Self {
        Ref(NonNull::new(info).unwrap_or_else(|| panic!("Ref::from_ptr(): the pointer is NULL.")))
    }

    pub fn as_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }

    pub fn inner(&self) -> &T {
        unsafe { &*self.0.as_ptr() }
    }

    pub fn inner_mut(&self) -> &mut T {
        unsafe { &mut *self.0.as_ptr() }
    }

    pub fn id(&self) -> u64 {
        self.0.as_ptr() as u64
    }
}

impl<T: Clone> Ref<T> {
    /// Allocates a copy of `self<T>` on the heap, returning `Ref`.
    pub fn dup(&self) -> Self {
        Self::new(self.inner().clone())
    }
}

unsafe impl<T> Send for Ref<T> {}

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
    pub path: PathBuf,
    pub code: Vec<char>,
}

impl SourceInfoRef {
    pub fn empty() -> Self {
        SourceInfoRef::new(SourceInfo::new(PathBuf::default()))
    }
}

impl SourceInfo {
    pub fn new(path: PathBuf) -> Self {
        SourceInfo {
            path: path,
            code: vec![],
        }
    }
    pub fn show_file_name(&self) {
        eprintln!("{}", self.path.to_string_lossy());
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
                eprintln!("{}:{}", self.path.to_string_lossy(), line.0);
            };
            found = true;
            let start = line.1 as usize;
            let mut end = line.2 as usize;
            if self.code[end] == '\n' {
                end -= 1
            }
            eprintln!("{}", self.code[start..=end].iter().collect::<String>());
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
            let is_cr = loc.1 as usize >= self.code.len() || self.code[loc.1 as usize] == '\n';
            eprintln!("{}:{}", self.path.to_string_lossy(), line.0);
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
