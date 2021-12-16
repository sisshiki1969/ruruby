#![feature(box_patterns)]
#![feature(pattern)]
#![feature(naked_functions)]
#![feature(once_cell)]
#![feature(int_roundings)]
#![feature(new_uninit)]
extern crate arrayvec;
extern crate fancy_regex;
extern crate fxhash;
extern crate indexmap;
extern crate num;
extern crate num_bigint;
extern crate region;
extern crate ruruby_common;
pub use fxhash::FxHashMap;
pub use fxhash::FxHashSet;
mod alloc;
pub mod arith;
mod builtin;
pub mod codegen;
pub mod coroutine;
pub mod error;
mod globals;
pub mod tests;
mod value;
mod vm;
pub use crate::alloc::*;
use crate::builtin::enumerator::*;
pub use crate::builtin::procobj::*;
pub use crate::builtin::range::*;
pub use crate::builtin::regexp::*;
pub use crate::builtin::time::*;
pub use crate::builtin::*;
pub use crate::codegen::Codegen;
pub use crate::error::*;
pub use crate::globals::*;
pub use crate::value::*;
pub use crate::vm::*;
pub use ruruby_common::*;
pub use ruruby_parse::*;

pub type FxIndexMap<K, V> = indexmap::IndexMap<K, V, fxhash::FxBuildHasher>;
pub type FxIndexSet<T> = indexmap::IndexSet<T, fxhash::FxBuildHasher>;

use core::ptr::NonNull;

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

    #[inline(always)]
    pub(crate) fn decode(i: i64) -> Self {
        let u = (i << 3) as u64;
        Self::from_ptr(u as *const T as *mut _)
    }
}

impl<T> From<u64> for Ref<T> {
    #[inline(always)]
    fn from(val: u64) -> Ref<T> {
        Ref(NonNull::new(val as *mut T).expect("new(): the pointer is NULL."))
    }
}

unsafe impl<T> Send for Ref<T> {}
unsafe impl<T> Sync for Ref<T> {}

impl<T> Copy for Ref<T> {}

impl<T> Clone for Ref<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for Ref<T> {
    #[inline(always)]
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
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0.as_ptr() }
    }
}

impl<T> std::ops::DerefMut for Ref<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.0.as_ptr() }
    }
}

//------------------------------------------------------------
