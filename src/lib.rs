#![feature(box_patterns)]
#![feature(assoc_char_funcs)]
#![feature(pattern)]
extern crate arraystring;
extern crate fancy_regex;
extern crate fxhash;
extern crate once_cell;
extern crate smallvec;
pub use fxhash::FxHashMap;
pub mod alloc;
pub mod builtin;
pub mod error;
pub mod globals;
pub mod id_table;
pub mod loader;
pub mod parse;
pub mod test;
pub mod util;
pub mod value;
pub mod vm;
pub use crate::alloc::*;
pub use crate::builtin::enumerator::*;
pub use crate::builtin::fiber::*;
pub use crate::builtin::procobj::*;
pub use crate::builtin::range::*;
pub use crate::builtin::regexp::*;
pub use crate::builtin::time::*;
pub use crate::error::*;
pub use crate::globals::*;
pub use crate::id_table::*;
pub use crate::parse::parser::{LvarCollector, LvarId, ParseResult, Parser};
pub use crate::util::*;
pub use crate::value::*;
pub use crate::vm::*;
pub use smallvec::{smallvec, SmallVec};
