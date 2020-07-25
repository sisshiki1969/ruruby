use crate::*;
use std::path::PathBuf;

pub fn eval_script(script: impl Into<String>, expected: Value) {
    let mut globals = GlobalsRef::new_globals();
    let mut vm = globals.new_vm();
    let res = vm.run(PathBuf::from(""), &script.into());
    #[cfg(feature = "perf")]
    vm.perf.print_perf();
    #[cfg(feature = "gc-debug")]
    globals.print_mark();
    match res {
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
    let mut globals = GlobalsRef::new_globals();
    let mut vm = globals.new_vm();
    let res = vm.run(PathBuf::from(""), &script.into());
    #[cfg(feature = "perf")]
    vm.perf.print_perf();
    #[cfg(feature = "gc-debug")]
    globals.print_mark();
    match res {
        Ok(_) => {}
        Err(err) => {
            err.show_err();
            err.show_loc(0);
            panic!("Got error: {:?}", err);
        }
    }
}

pub fn assert_error(script: impl Into<String>) {
    let mut globals = GlobalsRef::new_globals();
    let mut vm = globals.new_vm();
    let program = script.into();
    let res = vm.run(PathBuf::from(""), &program);
    #[cfg(feature = "perf")]
    vm.perf.print_perf();

    #[cfg(feature = "gc-debug")]
    globals.print_mark();
    match res {
        Ok(_) => panic!("Must be an error:{}", program),
        Err(err) => {
            err.show_err();
            err.show_loc(0);
        }
    }
}
