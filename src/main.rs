#![feature(test)]
extern crate ansi_term;
extern crate clap;
extern crate ruruby;
extern crate rustyline;

use ansi_term::Colour::Red;
use clap::{App, Arg};
use ruruby::error::*;
use ruruby::loader::*;
use ruruby::parser::{LvarCollector, Parser};
use ruruby::vm::*;

fn main() {
    let app = App::new("ruruby")
        .version("0.0.1")
        .author("monochrome")
        .about("A toy Ruby interpreter")
        .arg(
            Arg::with_name("eval")
                .help("Execute using AST evaluator")
                .long("eval"),
        )
        .arg(Arg::with_name("file").help("Input file name").index(1));
    let app_matches = app.get_matches();
    match app_matches.value_of("file") {
        Some(file_name) => {
            file_read(file_name);
            return;
        }
        None => {
            repl_vm();
            return;
        }
    };
}

fn file_read(file_name: impl Into<String>) {
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

    let mut vm = VM::new();
    let root_path = absolute_path.clone();
    #[cfg(feature = "verbose")]
    eprintln!("load file: {:?}", root_path);
    vm.root_path.push(root_path);
    match vm.run(absolute_path.to_str().unwrap(), program) {
        Ok(_) => {}
        Err(err) => {
            err.show_file_name();
            err.show_loc();
            err.show_err();
        }
    };
}

fn repl_vm() {
    /*
    println!("MethodRef: {}", std::mem::size_of::<MethodRef>());
    println!("PackedValue: {}", std::mem::size_of::<PackedValue>());
    println!("Value: {}", std::mem::size_of::<Value>());
    println!("ObjectInfo: {}", std::mem::size_of::<ObjectInfo>());
    println!("ClassInfo: {}", std::mem::size_of::<ClassInfo>());
    */
    println!("Value: {}", std::mem::size_of::<Value>());
    println!(
        "Option<PackedValue>: {}",
        std::mem::size_of::<Option<PackedValue>>()
    );
    println!("IdentId: {}", std::mem::size_of::<IdentId>());
    println!("OptionalID: {}", std::mem::size_of::<OptionalId>());
    let mut rl = rustyline::Editor::<()>::new();
    let mut program = String::new();
    let mut parser = Parser::new();
    let mut vm = VM::new();
    parser.ident_table = vm.globals.ident_table.clone();
    let mut level = parser.get_context_depth();
    let mut lvar_collector = LvarCollector::new();
    let main = vm.globals.main_object;
    let main_object = PackedValue::object(main);
    let context = ContextRef::from(
        main_object,
        None,
        ISeqRef::new(ISeqInfo::new(
            0,
            0,
            false,
            0,
            false,
            0,
            0,
            vec![],
            std::collections::HashMap::new(),
            vec![],
            LvarCollector::new(),
            vec![],
            SourceInfoRef::empty(),
        )),
        None,
    );
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
            "REPL",
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
                        err.show_loc();
                        err.show_err();
                        vm.exec_stack.clear();
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
                err.show_loc();
                err.show_err();
                program = String::new();
            }
        }
    }
}
