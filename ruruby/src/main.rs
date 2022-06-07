#![feature(test)]
extern crate ansi_term;
extern crate clap;
extern crate dirs;
extern crate ruruby;
extern crate rustyline;

use clap::*;
use ruruby::*;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None, trailing_var_arg = true)]
struct Cli {
    /// one line of script. Several -e's allowed. Omit [programfile]
    #[clap(short, multiple_occurrences = true)]
    exec: Option<String>,

    /// print the version number, then turn on verbose mode
    #[clap(short)]
    verbose: bool,

    /// program file and arguments
    args: Vec<String>,
}

#[cfg(not(tarpaulin_include))]
fn main() {
    let cli = Cli::parse();
    if cli.verbose {
        println!("{} {}", crate_name!(), crate_version!());
    }
    match cli.exec {
        Some(command) => {
            let mut vm = VM::new();
            vm.set_global_var(IdentId::get_id("$0"), Value::string("-e"));
            execute(&mut vm, std::path::PathBuf::default(), command);
            return;
        }
        None => {}
    }
    let mut vm = VM::new();

    let file = if cli.args.is_empty() {
        let argv = Value::array_from(vec![]);
        vm.globals.set_toplevel_constant("ARGV", argv);
        vm.globals.set_global_var_by_str("$*", argv);
        let context = HeapCtxRef::new_binding(vm.globals.main_object, ISeqRef::default(), None);
        vm.invoke_repl(context).unwrap();
        return;
    } else {
        &cli.args[0]
    };

    let args = cli.args[1..].iter().map(|x| Value::string(x)).collect();
    let argv = Value::array_from(args);
    vm.globals.set_toplevel_constant("ARGV", argv);
    vm.globals.set_global_var_by_str("$*", argv);

    let absolute_path = match std::path::Path::new(file).canonicalize() {
        Ok(path) => path,
        Err(ioerr) => {
            eprintln!("ruruby: {} -- {} (LoadError)", ioerr, file);
            return;
        }
    };

    let program = match vm.load_file(&absolute_path) {
        Ok(program) => program,
        Err(err) => {
            vm.globals.show_err(&err);
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
