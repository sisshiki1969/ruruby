use crate::*;
use std::path::PathBuf;

pub fn eval_script(script: impl Into<String>, expected: Value) {
    let mut globals = GlobalsRef::new_globals();
    //globals.gc_enabled = false;
    let mut vm = globals.new_vm();
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
    //Allocator::init();
}

pub fn assert_script(script: impl Into<String>) {
    let mut globals = GlobalsRef::new_globals();
    //globals.gc_enabled = false;
    let mut vm = globals.new_vm();
    match vm.run(PathBuf::from(""), &script.into(), None) {
        Ok(_) => {}
        Err(err) => {
            err.show_err();
            err.show_loc(0);
            panic!("Got error: {:?}", err);
        }
    }
    //Allocator::init();
}

pub fn assert_error(script: impl Into<String>) {
    let mut globals = GlobalsRef::new_globals();
    //globals.gc_enabled = false;
    let mut vm = globals.new_vm();
    let program = script.into();
    match vm.run(PathBuf::from(""), &program, None) {
        Ok(_) => panic!("Must be an error:{}", program),
        Err(_) => {}
    }
    //Allocator::init();
}
