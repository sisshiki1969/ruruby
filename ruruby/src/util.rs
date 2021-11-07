use core::ptr::NonNull;
use ruruby_common::*;
use std::path::PathBuf;

pub type FxIndexSet<T> = indexmap::IndexSet<T, fxhash::FxBuildHasher>;

#[cfg(not(windows))]
pub(crate) fn conv_pathbuf(dir: &PathBuf) -> String {
    dir.to_string_lossy().to_string()
}
#[cfg(windows)]
pub(crate) fn conv_pathbuf(dir: &PathBuf) -> String {
    dir.to_string_lossy()
        .replace("\\\\?\\", "")
        .replace('\\', "/")
}

#[derive(Clone, PartialEq)]
pub struct ExceptionEntry {
    pub ty: ExceptionType,
    /// start position in ISeq.
    pub start: ISeqPos,
    /// end position in ISeq.
    pub end: ISeqPos,
    pub dest: ISeqPos,
}

/// Type of each exception.
#[derive(Debug, Clone, PartialEq)]
pub enum ExceptionType {
    /// When raised, exec stack is cleared.
    Rescue,
    /// When raised, exec stack does not change.
    Continue,
}

use std::fmt;

impl fmt::Debug for ExceptionEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!(
            "ExceptionEntry {:?} ({:?}, {:?}) => {:?}",
            self.ty, self.start, self.end, self.dest,
        ))
    }
}

impl ExceptionEntry {
    pub fn new_rescue(start: ISeqPos, end: ISeqPos, dest: ISeqPos) -> Self {
        Self {
            ty: ExceptionType::Rescue,
            start,
            end,
            dest,
        }
    }

    pub fn new_continue(start: ISeqPos, end: ISeqPos, dest: ISeqPos) -> Self {
        Self {
            ty: ExceptionType::Continue,
            start,
            end,
            dest,
        }
    }

    pub(crate) fn include(&self, pc: usize) -> bool {
        self.start.into_usize() < pc && pc <= self.end.into_usize()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ISeqPos(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ISeqDisp(i32);

impl ISeqDisp {
    pub(crate) fn from_i32(disp: i32) -> Self {
        Self(disp)
    }

    pub(crate) fn to_i32(self) -> i32 {
        self.0
    }
}

impl fmt::Debug for ISeqPos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("ISeqPos({})", self.0))
    }
}

impl std::convert::From<ISeqPos> for usize {
    fn from(pos: ISeqPos) -> usize {
        pos.0
    }
}

impl std::ops::Add<ISeqDisp> for ISeqPos {
    type Output = Self;
    fn add(self, other: ISeqDisp) -> Self {
        Self(((self.0) as i64 + other.0 as i64) as usize)
    }
}

impl std::ops::AddAssign<ISeqDisp> for ISeqPos {
    fn add_assign(&mut self, other: ISeqDisp) {
        *self = *self + other
    }
}

impl std::ops::Add<usize> for ISeqPos {
    type Output = Self;
    fn add(self, other: usize) -> Self {
        Self(((self.0) as i64 + other as i64) as usize)
    }
}

impl std::ops::AddAssign<usize> for ISeqPos {
    fn add_assign(&mut self, other: usize) {
        *self = *self + other
    }
}

impl std::ops::Sub<usize> for ISeqPos {
    type Output = Self;
    fn sub(self, other: usize) -> Self {
        Self(((self.0) as i64 - other as i64) as usize)
    }
}

impl std::ops::SubAssign<usize> for ISeqPos {
    fn sub_assign(&mut self, other: usize) {
        *self = *self - other
    }
}

impl std::ops::Sub<ISeqPos> for ISeqPos {
    type Output = ISeqDisp;
    fn sub(self, other: ISeqPos) -> Self::Output {
        ISeqDisp((other.0 as i64 - self.0 as i64) as i32)
    }
}

impl ISeqPos {
    pub(crate) fn from(pos: usize) -> Self {
        ISeqPos(pos)
    }

    pub(crate) fn into_usize(self) -> usize {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContextKind {
    Method(Option<IdentId>),
    Class(IdentId),
    Block,
    Eval,
}

impl ContextKind {
    pub fn is_method(&self) -> bool {
        if let Self::Method(_) = self {
            true
        } else {
            false
        }
    }
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
    pub(crate) fn new(kind: T, loc: Loc) -> Self {
        Annot { kind, loc }
    }

    pub(crate) fn loc(&self) -> Loc {
        self.loc
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Loc(pub usize, pub usize);

impl Loc {
    /*pub(crate) fn new(loc: Loc) -> Self {
        loc
    }*/

    pub(crate) fn merge(&self, loc: Loc) -> Self {
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
    pub(crate) fn new(info: T) -> Self {
        let boxed = Box::into_raw(Box::new(info));
        Ref(NonNull::new(boxed).expect("Ref::new(): the pointer is NULL."))
    }

    pub(crate) fn free(self) {
        unsafe { Box::from_raw(self.as_ptr()) };
    }

    #[inline(always)]
    pub(crate) fn from_ref(info: &T) -> Self {
        Ref(NonNull::new(info as *const T as *mut T).expect("from_ref(): the pointer is NULL."))
    }

    #[inline(always)]
    pub(crate) fn from_ptr(info: *mut T) -> Self {
        Ref(NonNull::new(info).expect("from_ptr(): the pointer is NULL."))
    }

    #[inline(always)]
    pub(crate) fn as_ptr(&self) -> *mut T {
        self.0.as_ptr()
    }

    #[inline(always)]
    pub(crate) fn id(&self) -> u64 {
        self.0.as_ptr() as u64
    }

    #[inline(always)]
    pub(crate) fn encode(&self) -> i64 {
        self.id() as i64 >> 3
    }

    pub(crate) fn decode(i: i64) -> Self {
        let u = (i << 3) as u64;
        Self::from_ptr(u as *const T as *mut _)
    }
}

impl<T> From<u64> for Ref<T> {
    fn from(val: u64) -> Ref<T> {
        Ref(NonNull::new(val as *mut T).expect("new(): the pointer is NULL."))
    }
}

/*impl<T: Clone> Ref<T> {
    /// Allocates a copy of `self<T>` on the heap, returning `Ref`.
    pub(crate) fn dup(&self) -> Self {
        Self::new((**self).clone())
    }
}*/

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
        self.as_ptr() == other.as_ptr()
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

pub type SourceInfoRef = std::rc::Rc<SourceInfo>;

/// This struct holds infomation of a certain line in the code.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Line {
    /// line number. (the first line is 1)
    pub no: usize,
    /// byte position of the line top in the code.
    pub top: usize,
    /// byte position of the line end in the code.
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
    /// directory path of the source code.
    pub path: PathBuf,
    /// source code text.
    pub code: String,
}

impl Default for SourceInfo {
    fn default() -> Self {
        SourceInfo::new(PathBuf::default(), "")
    }
}
impl SourceInfo {
    pub(crate) fn new(path: impl Into<PathBuf>, code: impl Into<String>) -> Self {
        SourceInfo {
            path: path.into(),
            code: code.into(),
        }
    }

    #[cfg(feature = "emit-iseq")]
    pub(crate) fn get_file_name(&self) -> String {
        self.path.to_string_lossy().to_string()
    }

    pub(crate) fn show_loc(&self, loc: &Loc) {
        eprint!("{}", self.get_location(loc));
    }

    pub(crate) fn get_next_char(&self, pos: usize) -> Option<char> {
        self.code[pos..].chars().next()
    }

    pub(crate) fn get_lines(&self, loc: &Loc) -> Vec<Line> {
        let mut line_top = 0;
        let code_len = self.code.len();
        let mut lines: Vec<_> = self
            .code
            .char_indices()
            .filter(|(_, ch)| *ch == '\n')
            .map(|(pos, _)| pos)
            .enumerate()
            .map(|(idx, pos)| {
                let top = line_top;
                line_top = pos + 1;
                Line::new(idx + 1, top, pos)
            })
            .filter(|line| line.end >= loc.0 && line.top <= loc.1)
            .collect();
        if line_top < code_len && (code_len - 1) >= loc.0 && line_top <= loc.1 {
            lines.push(Line::new(lines.len() + 1, line_top, code_len - 1));
        }
        lines
    }

    /// Return a string represents the location of `loc` in the source code using '^^^'.
    pub(crate) fn get_location(&self, loc: &Loc) -> String {
        if self.code.len() == 0 {
            return "(internal)".to_string();
        }
        let mut res_string = String::new();
        let lines = self.get_lines(loc);
        let mut found = false;
        for line in &lines {
            if !found {
                res_string += &format!("{}:{}\n", self.path.to_string_lossy(), line.no);
                found = true;
            };

            let start = line.top;
            let mut end = line.end;
            if self.get_next_char(end) == Some('\n') && end > 0 {
                end -= 1
            }
            res_string += &self.code[start..=end];
            res_string.push('\n');
            use std::cmp::*;
            let lead = if loc.0 <= line.top {
                0
            } else {
                console::measure_text_width(&self.code[start..loc.0])
            };
            let range_start = max(loc.0, line.top);
            let range_end = min(loc.1, line.end);
            let length = console::measure_text_width(&self.code[range_start..=range_end]);
            res_string += &" ".repeat(lead);
            res_string += &"^".repeat(length);
            res_string += "\n";
        }

        if !found {
            res_string += "NOT FOUND\n";
            let line = match lines.last() {
                Some(line) => (line.no + 1, line.end + 1, loc.1),
                None => (1, 0, loc.1),
            };
            let lead = console::measure_text_width(&self.code[line.1..loc.0]);
            let length = console::measure_text_width(&self.code[loc.0..loc.1]);
            let is_cr = loc.1 >= self.code.len() || self.get_next_char(loc.1) == Some('\n');
            res_string += &format!("{}:{}\n", self.path.to_string_lossy(), line.0);
            res_string += if !is_cr {
                &self.code[line.1..=loc.1]
            } else {
                &self.code[line.1..loc.1]
            };
            res_string += &" ".repeat(lead);
            res_string += &"^".repeat(length + 1);
            res_string += "\n";
        }
        return res_string;
    }
}
