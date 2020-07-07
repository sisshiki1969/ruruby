#![feature(test)]
extern crate ansi_term;
extern crate clap;
extern crate ruruby;
extern crate rustyline;

use clap::{App, AppSettings, Arg};
#[cfg(not(tarpaulin_include))]
mod repl;
use repl::*;
use ruruby::*;

#[cfg(not(tarpaulin_include))]
fn main() {
    let app = App::new("ruruby")
        .version("0.0.1")
        .author("monochrome")
        .about("A toy Ruby interpreter")
        .setting(AppSettings::TrailingVarArg)
        .arg(Arg::from_usage("[file]... 'Input file name'").multiple(true));
    let m = app.get_matches();
    let args: Vec<&str> = match m.values_of("file") {
        Some(val) => val.collect(),
        None => {
            repl_vm();
            return;
        }
    };
    let mut globals = GlobalsRef::new_globals();
    let mut vm = globals.new_vm();
    let id = IdentId::get_ident_id("ARGV");
    let mut res: Vec<Value> = args
        .iter()
        .map(|x| Value::string(&vm.globals, x.to_string()))
        .collect();
    res.remove(0);
    let argv = Value::array_from(&vm.globals, res);
    vm.globals.builtins.object.set_var(id, argv);
    exec_file(&mut vm, args[0]);
}

#[cfg(not(tarpaulin_include))]
fn exec_file(vm: &mut VMRef, file_name: impl Into<String>) {
    use crate::loader::*;
    let file_name = file_name.into();
    let (absolute_path, program) = match crate::loader::load_file(file_name.clone()) {
        Ok((path, program)) => (path, program),
        Err(err) => {
            match err {
                LoadError::NotFound(msg) => eprintln!("LoadError: {}\n{}", &file_name, msg),
                LoadError::CouldntOpen(msg) => eprintln!("LoadError: {}\n{}", &file_name, msg),
            };
            return;
        }
    };

    let root_path = absolute_path.clone();
    #[cfg(feature = "verbose")]
    eprintln!("load file: {:?}", root_path);
    vm.root_path.push(root_path);
    let mut vm2 = vm.clone();
    match vm2.run(absolute_path, &program, None) {
        Ok(_) => {
            #[cfg(feature = "perf")]
            vm.globals.print_perf();
            #[cfg(feature = "gc-debug")]
            vm.globals.print_mark();
        }
        Err(err) => {
            err.show_err();
            for i in 0..err.info.len() {
                eprint!("{}:", i);
                err.show_loc(i);
            }
        }
    };
    vm.root_path.pop();
}
