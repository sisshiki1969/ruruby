//#![feature(test)]
#![feature(box_patterns)]
#![feature(cow_is_borrowed)]
extern crate fancy_regex;
pub mod builtin;
pub mod error;
pub mod globals;
pub mod lexer;
pub mod loader;
pub mod node;
pub mod parser;
pub mod test;
pub mod token;
pub mod util;
pub mod value;
pub mod vm;
pub use crate::builtin::*;
pub use crate::error::*;
pub use crate::globals::*;
pub use crate::parser::{LvarCollector, LvarId, ParseResult};
pub use crate::util::*;
pub use crate::value::*;
pub use crate::vm::*;
