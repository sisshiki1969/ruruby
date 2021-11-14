#![feature(test)]
extern crate ansi_term;
extern crate clap;
extern crate dirs;
extern crate ruruby;
extern crate rustyline;

use clap::*;
#[cfg(not(tarpaulin_include))]
mod repl;
use repl::*;
use ruruby::*;

#[cfg(not(tarpaulin_include))]
fn main() {
    let app = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .setting(AppSettings::TrailingVarArg)
        .arg(Arg::from_usage("[exec] -e 'Eval string as program'").takes_value(true))
        .arg(Arg::from_usage("[verbose] -v 'Show version'"))
        .arg(Arg::from_usage("[file]... 'Input file name'").multiple(true));
    let m = app.get_matches();
    if m.is_present("verbose") {
        println!("{} {}", crate_name!(), crate_version!());
    }
    match m.value_of("exec") {
        Some(command) => {
            let mut vm = VM::new();
            vm.set_global_var(IdentId::get_id("$0"), Value::string("-e"));
            execute(&mut vm, std::path::PathBuf::default(), command);
            return;
        }
        None => {}
    }
    let (args, repl_flag): (Vec<&str>, _) = match m.values_of("file") {
        Some(val) => (val.collect(), false),
        None => (vec![], true),
    };
    let mut vm = VM::new();
    let res: Vec<Value> = if args.len() == 0 {
        vec![]
    } else {
        args[1..].iter().map(|x| Value::string(*x)).collect()
    };
    let argv = Value::array_from(res);
    vm.globals.set_toplevel_constant("ARGV", argv);
    vm.globals.set_global_var_by_str("$*", argv);

    if repl_flag {
        repl_vm(vm);
        return;
    }

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
            VMError::show_err(&err);
            return;
        }
    };
    let file = absolute_path
        .file_name()
        .map(|x| x.to_string_lossy())
        .unwrap_or(std::borrow::Cow::Borrowed(""));
    vm.globals.set_global_var_by_str("$0", Value::string(file));
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
                vm.globals.methods.print_stats();
                vm.globals.print_constant_cache_stats();
                vm.globals.methods.print_cache_stats();
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
