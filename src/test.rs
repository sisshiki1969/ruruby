pub use crate::vm::value::Value;
use crate::vm::*;

pub fn eval_script(script: impl Into<String>, expected: Value) {
    let mut vm = VM::new();
    match vm.run("", script.into()) {
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
