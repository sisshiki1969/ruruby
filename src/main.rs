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
    let id = IdentId::get_id("ARGV");
    let mut res: Vec<Value> = args
        .iter()
        .map(|x| Value::string(&vm.globals, x.to_string()))
        .collect();
    res.remove(0);
    let argv = Value::array_from(&vm.globals, res);
    vm.globals.builtins.object.set_var(id, argv);
    vm.exec_file(args[0]);
}
