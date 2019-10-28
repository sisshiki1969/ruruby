#![feature(test)]
extern crate ansi_term;
extern crate clap;
extern crate ruruby;
extern crate rustyline;

use ansi_term::Colour::Red;
use clap::{App, Arg};
use ruruby::eval::Evaluator;
use ruruby::parser::{ParseErrorKind, Parser};
use ruruby::vm::VM;

fn main() {
    let app = App::new("ruruby")
        .version("0.0.1")
        .author("monochrome")
        .about("A toy Ruby interpreter")
        .arg(Arg::with_name("vm").help("Execute using VM").long("vm"))
        .arg(Arg::with_name("file").help("Input file name").index(1));
    let app_matches = app.get_matches();
    let vm_flag = app_matches.is_present("vm");
    match app_matches.value_of("file") {
        Some(file_name) => {
            file_read(file_name, vm_flag);
            return;
        }
        None => {
            if vm_flag {
                repl_vm()
            } else {
                repl();
            };
            return;
        }
    };
}

fn repl() {
    let mut rl = rustyline::Editor::<()>::new();
    let mut program = String::new();
    let mut parser = Parser::new();
    let mut eval = Evaluator::new(parser.lexer.source_info.clone(), parser.ident_table.clone());
    let mut level = parser.get_context_depth();
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

        let parser_save = parser.clone();
        match parser.parse_program(program.clone()) {
            Ok(node) => {
                //println!("{:?}", node);
                eval.init(parser.lexer.source_info.clone(), parser.ident_table.clone());
                match eval.eval(&node) {
                    Ok(result) => {
                        parser.lexer.source_info = eval.source_info.clone();
                        parser.ident_table = eval.ident_table.clone();
                        println!("=> {:?}", result);
                    }
                    Err(_) => {
                        parser = parser_save;
                        //println!("{}", program);
                    }
                }
                level = parser.get_context_depth();
                program = String::new();
            }
            Err(err) => {
                if ParseErrorKind::UnexpectedEOF == err.kind {
                    level = parser.get_context_depth();
                    parser = parser_save;
                    continue;
                }
                parser.show_tokens();
                level = parser.get_context_depth();
                parser.show_loc(&err.loc());
                println!("ParseError: {:?}", err.kind);
                parser = parser_save;
                program = String::new();
            }
        }
    }
}

fn file_read(file_name: impl Into<String>, vm_flag: bool) {
    use std::fs::*;
    use std::io::Read;
    let file_name = file_name.into();
    let path = std::path::Path::new(&file_name).with_extension("rb");
    let absolute_path = match path.canonicalize() {
        Ok(path) => path,
        Err(ioerr) => {
            let msg = format!("{}", ioerr);
            println!("No such file or directory --- {} (LoadError)", &file_name);
            println!("{}", msg);
            return;
        }
    };

    let mut file_body = String::new();

    match OpenOptions::new().read(true).open(&absolute_path) {
        Ok(mut ok) => ok
            .read_to_string(&mut file_body)
            .ok()
            .expect("cannot read file"),
        Err(ioerr) => {
            let msg = format!("{}", ioerr);
            println!("Error: Cannot find module file. '{}'", &file_name);
            println!("{}", msg);
            return;
        }
    };

    let mut parser = Parser::new();
    let res = parser.parse_program(file_body);
    if vm_flag {
        match res {
            Ok(node) => {
                let mut eval = VM::new(parser.lexer.source_info, parser.ident_table);
                eval.init_builtin();
                match eval.run(&node) {
                    Ok(result) => println!("=> {:?}", &result),
                    Err(_) => {}
                }
            }
            Err(err) => {
                parser.show_tokens();
                parser.show_loc(&err.loc);
                println!("ParseError: {:?}", err.kind);
            }
        }
        println!("Executed by VM.");
    } else {
        match res {
            Ok(node) => {
                let mut eval = Evaluator::new(parser.lexer.source_info, parser.ident_table);
                match eval.eval(&node) {
                    Ok(result) => println!("=> {:?}", &result),
                    Err(_) => {}
                }
            }
            Err(err) => {
                parser.show_tokens();
                parser.show_loc(&err.loc);
                println!("ParseError: {:?}", err.kind);
            }
        }
    }
}

fn repl_vm() {
    let mut rl = rustyline::Editor::<()>::new();
    let mut program = String::new();
    let mut parser = Parser::new();
    let mut vm = VM::new(parser.lexer.source_info.clone(), parser.ident_table.clone());
    let mut level = parser.get_context_depth();
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

        let parser_save = parser.clone();
        match parser.parse_program(program.clone()) {
            Ok(node) => {
                //println!("{:?}", node);
                vm.init(parser.lexer.source_info.clone(), parser.ident_table.clone());
                vm.init_builtin();
                match vm.run(&node) {
                    Ok(result) => {
                        parser.lexer.source_info = vm.source_info.clone();
                        parser.ident_table = vm.ident_table.clone();
                        println!("=> {:?}", result);
                    }
                    Err(_) => {
                        parser = parser_save;
                        //println!("{}", program);
                    }
                }
                level = parser.get_context_depth();
                program = String::new();
            }
            Err(err) => {
                if ParseErrorKind::UnexpectedEOF == err.kind {
                    level = parser.get_context_depth();
                    parser = parser_save;
                    continue;
                }
                parser.show_tokens();
                level = parser.get_context_depth();
                parser.show_loc(&err.loc());
                println!("ParseError: {:?}", err.kind);
                parser = parser_save;
                program = String::new();
            }
        }
    }
}
