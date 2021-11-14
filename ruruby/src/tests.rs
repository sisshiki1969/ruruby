use crate::*;

fn run(script: &str) -> (VMRef, VMResult) {
    let mut vm = VM::new();
    let res = vm.run("", script.to_string());
    #[cfg(feature = "perf")]
    vm.globals.perf.print_perf();
    #[cfg(feature = "gc-debug")]
    vm.globals.print_mark();
    #[cfg(feature = "perf-method")]
    {
        vm.globals.methods.print_stats();
        vm.globals.print_constant_cache_stats();
        vm.globals.methods.print_stats();
    }
    (vm, res)
}

pub fn eval_script(script: &str, expected: Value) {
    let (mut vm, res) = run(script);
    match res {
        Ok(res) => {
            if !vm.eval_eq2(res, expected).unwrap() {
                panic!("Expected:{:?} Got:{:?}", expected, res);
            }
        }
        Err(err) => {
            vm.show_err(&err);
            err.show_loc(0);
            panic!("Got error: {:?}", err);
        }
    }
}

pub fn assert_script(script: &str) {
    let (vm, res) = run(script);
    match res {
        Ok(_) => {}
        Err(err) => {
            vm.show_err(&err);
            err.show_loc(0);
            panic!("Got error: {:?}", err);
        }
    }
}

pub fn assert_error(script: impl Into<String>) {
    let script = script.into();
    let (vm, res) = run(&script);
    match res {
        Ok(_) => panic!("Must be an error:{}", script),
        Err(err) => {
            vm.show_err(&err);
            err.show_loc(0);
        }
    }
}
