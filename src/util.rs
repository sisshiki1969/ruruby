use console;
use core::ptr::NonNull;
use std::path::PathBuf;
use terminal_size::{terminal_size, Height, Width};

pub type FxIndexSet<T> = indexmap::IndexSet<T, fxhash::FxBuildHasher>;

#[cfg(not(windows))]
pub fn conv_pathbuf(dir: &PathBuf) -> String {
    dir.to_string_lossy().to_string()
}
#[cfg(windows)]
pub fn conv_pathbuf(dir: &PathBuf) -> String {
    dir.to_string_lossy()
        .replace("\\\\?\\", "")
        .replace('\\', "/")
}

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

    pub fn merge(&self, loc: Loc) -> Self {
        use std::cmp::*;
        Loc(min(self.0, loc.0), max(self.1, loc.1))
    }
}

//------------------------------------------------------------

#[derive(Debug)]
#[repr(transparent)]
pub struct Ref<T>(NonNull<T>);

impl<T: Default> Default for Ref<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

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

    pub fn id(&self) -> u64 {
        self.0.as_ptr() as u64
    }
}

impl<T> From<u64> for Ref<T> {
    fn from(val: u64) -> Ref<T> {
        Ref(NonNull::new(val as *mut T)
            .unwrap_or_else(|| panic!("Ref::new(): the pointer is NULL.")))
    }
}

impl<T: Clone> Ref<T> {
    /// Allocates a copy of `self<T>` on the heap, returning `Ref`.
    pub fn dup(&self) -> Self {
        Self::new((**self).clone())
    }
}

unsafe impl<T> Send for Ref<T> {}
unsafe impl<T> Sync for Ref<T> {}

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

/// This struct holds infomation of a certain line in the code.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// line number. (the first line is 1)
    pub no: usize,
    /// an index of the line top in Vec<char>.
    pub top: usize,
    /// an index of the line end in Vec<char>.
    pub end: usize,
}

impl Line {
    fn new(line_no: usize, top_pos: usize, end_pos: usize) -> Self {
        Line {
            no: line_no,
            top: top_pos,
            end: end_pos,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceInfo {
    pub path: PathBuf,
    pub code: String,
}

impl Default for SourceInfo {
    fn default() -> Self {
        SourceInfo::new(PathBuf::default(), "")
    }
}
impl SourceInfo {
    pub fn new(path: impl Into<PathBuf>, code: impl Into<String>) -> Self {
        SourceInfo {
            path: path.into(),
            code: code.into(),
        }
    }

    pub fn get_file_name(&self) -> String {
        self.path.to_string_lossy().to_string()
    }

    pub fn show_loc(&self, loc: &Loc) {
        eprint!("{}", self.get_location(loc));
    }

    /// Return a string represents the location of `loc` in the source code using '^^^'.
    pub fn get_location(&self, loc: &Loc) -> String {
        if self.code.len() == 0 {
            return "(internal)".to_string();
        }
        let mut res_string = String::new();
        let term_width = terminal_size().unwrap_or((Width(80), Height(25))).0 .0 as usize;
        let mut line_top = 0;
        let code: Vec<char> = self.code.chars().collect();
        let mut lines: Vec<Line> = code
            .iter()
            .enumerate()
            .filter(|(_, ch)| **ch == '\n')
            .map(|(pos, _)| pos)
            .enumerate()
            .map(|(idx, pos)| {
                let top = line_top;
                line_top = pos + 1;
                Line::new(idx + 1, top, pos)
            })
            .collect();
        if line_top < code.len() {
            let line_no = lines.len() + 1;
            lines.push(Line::new(line_no, line_top, code.len() - 1));
        }

        let mut found = false;
        for line in lines
            .iter()
            .filter(|line| line.end >= loc.0 && line.top <= loc.1)
        {
            if !found {
                res_string += &format!("{}:{}\n", self.path.to_string_lossy(), line.no);
                found = true;
            };

            let mut start = line.top;
            let mut end = line.end;
            if code[end] == '\n' && end > 0 {
                end -= 1
            }
            start += (if loc.0 >= start { loc.0 - start } else { 0 }) / term_width * term_width;
            if calc_width(&code[start..=end]) >= term_width {
                for e in loc.1..=end {
                    if calc_width(&code[start..=e]) < term_width {
                        end = e;
                    } else {
                        break;
                    }
                }
            }
            res_string += &(code[start..=end].iter().collect::<String>() + "\n");
            use std::cmp::*;
            let lead = if loc.0 <= line.top {
                0
            } else {
                calc_width(&code[start..loc.0])
            };
            let range_start = max(loc.0, line.top);
            let range_end = min(loc.1, line.end);
            let length = calc_width(&code[range_start..range_end]);
            res_string += &" ".repeat(lead);
            res_string += &"^".repeat(length + 1);
            res_string += "\n";
        }

        if !found {
            res_string += "NOT FOUND\n";
            let line = match lines.last() {
                Some(line) => (line.no + 1, line.end + 1, loc.1),
                None => (1, 0, loc.1),
            };
            let lead = calc_width(&code[line.1..loc.0]);
            let length = calc_width(&code[loc.0..loc.1]);
            let is_cr = loc.1 >= code.len() || code[loc.1] == '\n';
            res_string += &format!("{}:{}\n", self.path.to_string_lossy(), line.0);
            res_string += &(if !is_cr {
                code[line.1..=loc.1].iter().collect::<String>()
            } else {
                code[line.1..loc.1].iter().collect::<String>()
            });
            res_string += &" ".repeat(lead);
            res_string += &"^".repeat(length + 1);
            res_string += "\n";
        }
        return res_string;

        fn calc_width(chars: &[char]) -> usize {
            let str: String = chars.iter().collect();
            console::measure_text_width(&str)
        }
    }
}
