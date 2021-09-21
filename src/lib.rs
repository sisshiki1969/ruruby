#![feature(box_patterns)]
#![feature(pattern)]
#![feature(asm, naked_functions)]
#![feature(once_cell)]
extern crate arraystring;
extern crate fancy_regex;
extern crate fxhash;
extern crate indexmap;
extern crate num;
extern crate num_bigint;
extern crate region;
extern crate smallvec;
pub use fxhash::FxHashMap;
pub use fxhash::FxHashSet;
mod alloc;
pub mod arith;
mod builtin;
mod coroutine;
pub mod error;
mod globals;
mod id_table;
pub mod parse;
pub mod tests;
mod util;
mod value;
mod vm;
pub use crate::alloc::*;
pub use crate::builtin::enumerator::*;
pub use crate::builtin::fiber::*;
pub use crate::builtin::procobj::*;
pub use crate::builtin::range::*;
pub use crate::builtin::regexp::*;
pub use crate::builtin::time::*;
pub use crate::builtin::*;
pub use crate::error::*;
pub use crate::globals::*;
pub use crate::id_table::*;
pub use crate::parse::codegen::{ArgFlag, Codegen, ExceptionEntry};
pub use crate::parse::parser::{LvarCollector, LvarId, ParseResult, Parser};
pub use crate::util::*;
pub use crate::value::*;
pub use crate::vm::*;
pub use smallvec::{smallvec, SmallVec};

pub type FxIndexMap<K, V> = indexmap::IndexMap<K, V, fxhash::FxBuildHasher>;
