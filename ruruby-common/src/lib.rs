#![feature(once_cell)]
#![feature(pattern)]
pub mod arg_flag;
pub mod id_table;
pub mod lvar_collector;
pub mod lvar_id;
pub use arg_flag::ArgFlag;
pub use id_table::IdentId;
pub use lvar_collector::*;
pub use lvar_id::LvarId;
