use crate::*;
use std::path::PathBuf;

pub fn eval_script(script: impl Into<String>, expected: Value) {
    let mut vm = VMRef::new(VM::new());
    vm.clone().globals.fibers.push(vm);
    match vm.run(PathBuf::from(""), &script.into(), None) {
        Ok(res) => {
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

pub fn assert_script(script: impl Into<String>) {
    let mut vm = VMRef::new(VM::new());
    vm.clone().globals.fibers.push(vm);
    match vm.run(PathBuf::from(""), &script.into(), None) {
        Ok(_) => {}
        Err(err) => {
            err.show_err();
            err.show_loc(0);
            panic!("Got error: {:?}", err);
        }
    }
}
