use crate::vm::value::Value;
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
            err.show_loc();
            eprintln!("{:?}", err.kind);
            panic!("Got error: {:?}", err);
        }
    }
}
