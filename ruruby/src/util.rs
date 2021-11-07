use core::ptr::NonNull;
use ruruby_common::*;
use std::fmt;
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
