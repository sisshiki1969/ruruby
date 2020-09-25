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
        .about("A toy Ruby interpreter")
        .setting(AppSettings::TrailingVarArg)
        .arg(Arg::from_usage("[command] -e 'Eval string as program'").takes_value(true))
        .arg(Arg::from_usage("[file]... 'Input file name'").multiple(true));
    let m = app.get_matches();
    match m.value_of("command") {
        Some(command) => {
            let mut globals = GlobalsRef::new_globals();
            let mut vm = globals.new_vm();
            vm.exec_program(std::path::PathBuf::default(), command.to_string());
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
    let mut vm = globals.new_vm();
    let res: Vec<Value> = args[1..]
        .iter()
        .map(|x| Value::string(x.to_string()))
        .collect();
    let argv = Value::array_from(res);
    let mut object = globals.builtins.object;
    object.set_var_by_str("ARGV", argv);
    object.set_var_by_str("RUBY_VERSION", Value::string("2.5.0".to_string()));
    object.set_var_by_str("RUBY_ENGINE", Value::string("ruruby".to_string()));

    vm.exec_file(args[0]);
}
