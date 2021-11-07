#![feature(once_cell)]
#![feature(pattern)]
pub mod arg_flag;
pub mod id_table;
pub mod iseq;
pub mod loc;
pub mod lvar_collector;
pub mod lvar_id;
pub mod method_id;
pub use arg_flag::ArgFlag;
pub use id_table::IdentId;
pub use iseq::*;
pub use loc::Loc;
pub use lvar_collector::*;
pub use lvar_id::LvarId;
pub use method_id::MethodId;
