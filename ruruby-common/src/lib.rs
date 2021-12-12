#![feature(once_cell)]
#![feature(pattern)]
pub mod arg_flag;
pub mod code_context;
pub mod error;
pub mod func_id;
pub mod id_table;
pub mod iseq;
pub mod lvar_collector;
pub mod source_info;
pub mod vm_inst;
pub use arg_flag::ArgFlag;
pub use code_context::*;
pub use error::*;
pub use func_id::FnId;
pub use id_table::IdentId;
pub use iseq::*;
pub use lvar_collector::*;
pub use source_info::*;
pub use vm_inst::Inst;
