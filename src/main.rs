#![feature(test)]
extern crate ansi_term;
extern crate clap;
extern crate ruruby;
extern crate rustyline;

use clap::{App, AppSettings, Arg};
use ruruby::loader::{load_file, LoadError};
use std::thread;
mod repl;
use repl::*;
use ruruby::*;

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
    let mut vm = VMRef::new(VM::new());
    let id = vm.globals.get_ident_id("ARGV");
    let mut res: Vec<Value> = args
        .iter()
        .map(|x| Value::string(&vm.globals, x.to_string()))
        .collect();
    res.remove(0);
    let argv = Value::array_from(&vm.globals, res);
    vm.globals.builtins.object.set_var(id, argv);
    exec_file(&mut vm, args[0]);
    return;
}

fn exec_file(vm: &mut VMRef, file_name: impl Into<String>) {
    let file_name = file_name.into();
    let (absolute_path, program) = match load_file(file_name.clone()) {
        Ok((path, program)) => (path, program),
        Err(err) => match err {
            LoadError::NotFound(msg) => {
                eprintln!("No such file or directory --- {} (LoadError)", &file_name);
                eprintln!("{}", msg);
                return;
            }
            LoadError::CouldntOpen(msg) => {
                eprintln!("Cannot open file. '{}'", &file_name);
                eprintln!("{}", msg);
                return;
            }
        },
    };

    let root_path = absolute_path.clone();
    #[cfg(feature = "verbose")]
    #[cfg_attr(tarpaulin, skip)]
    eprintln!("load file: {:?}", root_path);
    vm.root_path.push(root_path);
    let mut vm2 = vm.clone();
    let res = thread::spawn(move || vm2.run(absolute_path, &program, None))
        .join()
        .unwrap();
    match res {
        Ok(_) => {}
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
