#![feature(test)]
extern crate ansi_term;
extern crate clap;
extern crate ruruby;
extern crate rustyline;

use ansi_term::Colour::Red;
use clap::{App, AppSettings, Arg};
use ruruby::error::*;
use ruruby::loader::*;
use ruruby::parser::{LvarCollector, Parser};
use ruruby::vm::*;

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
    let mut vm = VM::new();
    let id = vm.globals.get_ident_id("ARGV");
    let mut res: Vec<Value> = args.iter().map(|x| Value::string(x.to_string())).collect();
    res.remove(0);
    let argv = Value::array_from(&vm.globals, res);
    vm.globals.builtins.object.set_var(id, argv);
    exec_file(&mut vm, "struct.rb");
    exec_file(&mut vm, args[0]);
    return;
}

fn exec_file(vm: &mut VM, file_name: impl Into<String>) {
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
    eprintln!("load file: {:?}", root_path);
    vm.root_path.push(root_path);
    match vm.run(absolute_path, program, None) {
        Ok(_) => {}
        Err(err) => {
            err.show_err();
            for i in 0..err.info.len() {
                eprint!("{}:", i);
                err.show_file_name(i);
                err.show_loc(i);
            }
        }
    };
    vm.root_path.pop();
}

fn repl_vm() {
    println!("RValue: {}", std::mem::size_of::<RValue>());
    println!("ObjectInfo: {}", std::mem::size_of::<ObjectInfo>());
    println!("ClassInfo: {}", std::mem::size_of::<ClassInfo>());
    let mut rl = rustyline::Editor::<()>::new();
    let mut program = String::new();
    let mut parser = Parser::new();
    let mut vm = VM::new();
    parser.ident_table = vm.globals.ident_table.clone();
    let mut level = parser.get_context_depth();
    let mut lvar_collector = LvarCollector::new();
    let method = vm.globals.new_method();
    let info = ISeqInfo::default(method);
    let context = ContextRef::from(vm.globals.main_object, None, ISeqRef::new(info), None);
    loop {
        let prompt = if program.len() == 0 { ">" } else { "*" };
        let readline =
            rl.readline(&format!("{}{:1}{} ", Red.bold().paint("irb:"), level, prompt).to_string());
        let mut line = match readline {
            Ok(line) => line,
            Err(_) => return,
        };
        rl.add_history_entry(line.clone());
        line.push('\n');

        program = format!("{}{}", program, line);

        match parser.clone().parse_program_repl(
            std::path::PathBuf::from("REPL"),
            program.clone(),
            Some(lvar_collector.clone()),
        ) {
            Ok(parse_result) => {
                //println!("{:?}", node);
                match vm.run_repl(&parse_result, context) {
                    Ok(result) => {
                        parser.ident_table = vm.globals.ident_table.clone();
                        parser.lexer.source_info = parse_result.source_info;
                        lvar_collector = parse_result.lvar_collector;
                        let res_str = vm.val_pp(result);
                        println!("=> {}", res_str);
                    }
                    Err(err) => {
                        err.show_loc(0);
                        err.show_err();
                        vm.clear();
                    }
                }
                level = 0;
                program = String::new();
            }
            Err(err) => {
                level = err.level();
                if RubyErrorKind::ParseErr(ParseErrKind::UnexpectedEOF) == err.kind {
                    continue;
                }
                err.show_loc(0);
                err.show_err();
                program = String::new();
            }
        }
    }
}
