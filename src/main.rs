#![feature(test)]
extern crate clap;
extern crate rustyline;
pub mod class;
pub mod eval;
pub mod instance;
pub mod lexer;
pub mod node;
pub mod parser;
pub mod token;
pub mod util;
pub mod value;
use crate::eval::Evaluator;
use crate::parser::{ParseErrorKind, Parser};
use clap::{App, Arg};

fn main() {
    let app = App::new("ruruby")
        .version("0.0.1")
        .author("monochrome")
        .about("A toy Ruby interpreter")
        .arg(
            Arg::with_name("debug")
                .help("Show useful information for debugging")
                .long("debug"),
        )
        .arg(Arg::with_name("file").help("Input file name").index(1));
    let app_matches = app.get_matches();
    match app_matches.value_of("file") {
        Some(file_name) => {
            file_read(file_name);
            return;
        }
        None => {
            repl();
            return;
        }
    };
}

fn repl() {
    let mut rl = rustyline::Editor::<()>::new();
    let mut program = String::new();
    let mut parser = Parser::new();
    let mut eval = Evaluator::new();
    eval.repl_init(parser.lexer.source_info.clone(), parser.ident_table.clone());
    eval.repl_set_main();
    let mut level = parser.get_context_depth();
    loop {
        let prompt = if program.len() == 0 { ">" } else { "*" };
        let readline = rl.readline(&format!("irb:{:1}{} ", level, prompt).to_string());
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
                eval.repl_init(parser.lexer.source_info.clone(), parser.ident_table.clone());
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
                level = parser.get_context_depth();
                parser.show_loc(&err.loc());
                println!("ParseError: {:?}", err.kind);
                parser = parser_save;
                program = String::new();
            }
        }
    }
}

fn file_read(file_name: impl Into<String>) {
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
    match res {
        Ok(node) => {
            let mut eval = Evaluator::new();
            eval.init(parser.lexer.source_info, parser.ident_table);
            match eval.eval(&node) {
                Ok(result) => println!("=> {:?}", &result),
                Err(_) => {}
            }
        }
        Err(err) => println!("ParseError: {:?}", err.kind),
    }
}
