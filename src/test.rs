use crate::*;

pub fn eval_script(script: impl Into<String>, expected: Value) {
    let mut globals = GlobalsRef::new_globals();
    let mut vm = globals.create_main_fiber();
    let res = vm.run("", script);
    #[cfg(feature = "perf")]
    vm.globals.perf.print_perf();
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
    let mut vm = globals.create_main_fiber();
    let res = vm.run("", script);
    #[cfg(feature = "perf")]
    vm.globals.perf.print_perf();
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
    let mut vm = globals.create_main_fiber();
    let script = script.into();
    let res = vm.run("", &script);
    #[cfg(feature = "perf")]
    vm.globals.perf.print_perf();

    #[cfg(feature = "gc-debug")]
    globals.print_mark();
    match res {
        Ok(_) => panic!("Must be an error:{}", script),
        Err(err) => {
            err.show_err();
            err.show_loc(0);
        }
    }
}
