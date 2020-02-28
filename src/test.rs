pub use crate::vm::value::RValue;
use crate::vm::*;
use std::path::PathBuf;

pub fn eval_script(script: impl Into<String>, expected: RValue) {
    let mut vm = VM::new();
    match vm.run(PathBuf::from(""), &script.into(), None) {
        Ok(res) => {
            let res = res.unpack();
            if res != expected {
                panic!("Expected:{:?} Got:{:?}", expected, res);
            }
        }
        Err(err) => {
            err.show_err();
            err.show_loc(0);
            panic!("Got error: {:?}", err);
        }
    }
}
