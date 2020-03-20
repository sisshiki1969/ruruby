//#![feature(test)]
#![feature(box_patterns)]
#![feature(cow_is_borrowed)]
extern crate fancy_regex;
pub mod builtin;
pub mod error;
pub mod lexer;
pub mod loader;
pub mod node;
pub mod parser;
pub mod test;
pub mod token;
pub mod util;
pub mod vm;
