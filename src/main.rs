#![feature(test)]
extern crate ansi_term;
extern crate clap;
extern crate dirs;
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
        .about("An alternative Ruby interpreter")
        .setting(AppSettings::TrailingVarArg)
        .arg(Arg::from_usage("[command] -e 'Eval string as program'").takes_value(true))
        .arg(Arg::from_usage("[file]... 'Input file name'").multiple(true));
    let m = app.get_matches();
    match m.value_of("command") {
        Some(command) => {
            let mut globals = GlobalsRef::new_globals();
            let mut vm = globals.create_main_fiber();
            vm.set_global_var(IdentId::get_id("$0"), Value::string("-e"));
            execute(&mut vm, std::path::PathBuf::default(), command);
            return;
        }
        None => {}
    }
    let args: Vec<&str> = match m.values_of("file") {
        Some(val) => val.collect(),
        None => {
            repl_vm();
            return;
        }
    };
    let mut globals = GlobalsRef::new_globals();
    let mut vm = globals.create_main_fiber();
    let res: Vec<Value> = args[1..].iter().map(|x| Value::string(*x)).collect();
    let argv = Value::array_from(res);
    globals.set_toplevel_constant("ARGV", argv);

    let absolute_path = match std::path::Path::new(args[0]).canonicalize() {
        Ok(path) => path,
        Err(ioerr) => {
            eprintln!("ruruby: {} -- {} (LoadError)", ioerr, args[0]);
            return;
        }
    };

    let program = match vm.load_file(&absolute_path) {
        Ok(program) => program,
        Err(err) => {
            err.show_err();
            return;
        }
    };
    let file = absolute_path
        .file_name()
        .map(|x| x.to_string_lossy())
        .unwrap_or(std::borrow::Cow::Borrowed(""));
    vm.set_global_var(IdentId::get_id("$0"), Value::string(file));
    #[cfg(feature = "verbose")]
    eprintln!("load file: {:?}", &absolute_path);
    execute(&mut vm, absolute_path, program.to_string());
}

#[cfg(not(tarpaulin_include))]
fn execute(vm: &mut VM, absolute_path: std::path::PathBuf, program: impl Into<String>) {
    let program = program.into();
    match vm.run(absolute_path, program) {
        Ok(_) => {
            #[cfg(feature = "perf")]
            vm.globals.perf.print_perf();
            #[cfg(feature = "perf-method")]
            {
                MethodRepo::print_stats();
                vm.globals.print_constant_cache_stats();
                MethodPerf::print_stats();
            }
            #[cfg(feature = "gc-debug")]
            vm.globals.print_mark();
        }
        Err(err) => {
            vm.show_err(&err);
            err.show_all_loc();
        }
    };
}
