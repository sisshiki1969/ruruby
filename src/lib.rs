//#![feature(test)]
#![feature(box_patterns)]
extern crate fancy_regex;
//#[macro_use]
extern crate lazy_static;
pub mod alloc;
pub mod builtin;
pub mod error;
pub mod globals;
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
pub use crate::builtin::string::RString;
pub use crate::error::*;
pub use crate::globals::*;
pub use crate::parse::parser::{LvarCollector, LvarId, ParseResult, Parser};
pub use crate::util::*;
pub use crate::value::*;
pub use crate::vm::*;
