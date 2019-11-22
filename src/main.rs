#![feature(test)]
extern crate ansi_term;
extern crate clap;
extern crate ruruby;
extern crate rustyline;

use ansi_term::Colour::Red;
use clap::{App, Arg};
use ruruby::error::*;
use ruruby::eval::Evaluator;
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
    let eval_flag = app_matches.is_present("eval");
    match app_matches.value_of("file") {
        Some(file_name) => {
            file_read(file_name, !eval_flag);
            return;
        }
        None => {
            if eval_flag {
                repl()
            } else {
                repl_vm();
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
        match parser.parse_program(program.clone(), None) {
            Ok(result) => {
                //println!("{:?}", node);
                eval.init(result.source_info, result.ident_table.clone());
                match eval.eval(&result.node) {
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
                if RubyErrorKind::ParseErr(ParseErrKind::UnexpectedEOF) == err.kind {
                    level = parser.get_context_depth();
                    parser = parser_save;
                    continue;
                }
                parser.show_tokens();
                level = parser.get_context_depth();
                parser.show_loc(&err.loc());
                println!("RubyError: {:?}", err.kind);
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
            eprintln!("No such file or directory --- {} (LoadError)", &file_name);
            eprintln!("{}", msg);
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
            eprintln!("Error: Cannot find module file. '{}'", &file_name);
            eprintln!("{}", msg);
            return;
        }
    };

    let mut parser = Parser::new();
    let res = parser.parse_program(file_body, None);

    match res {
        Ok(result) => {
            if vm_flag {
                let mut eval = VM::new(Some(result.ident_table));
                eval.init_builtin();
                match eval.run(&result.node, &result.lvar_collector) {
                    Ok(_result) => {}
                    Err(err) => {
                        result.source_info.show_loc(&err.loc());
                        eprintln!("{:?}", err.kind);
                    }
                };
            } else {
                let mut eval = Evaluator::new(result.source_info, result.ident_table);
                match eval.eval(&result.node) {
                    Ok(_result) => {}
                    Err(_) => {}
                }
                eprintln!("Executed by AST Evaluator.");
            }
        }
        Err(err) => {
            parser.show_tokens();
            parser.show_loc(&err.loc());
            eprintln!("{:?}", err.kind);
        }
    }
}

fn repl_vm() {
    println!("MethodRef: {}", std::mem::size_of::<MethodRef>());
    println!("PackedValue: {}", std::mem::size_of::<PackedValue>());
    println!("Value: {}", std::mem::size_of::<Value>());
    println!("InstanceInfo: {}", std::mem::size_of::<InstanceInfo>());
    println!("ClassInfo: {}", std::mem::size_of::<ClassInfo>());

    let mut rl = rustyline::Editor::<()>::new();
    let mut program = String::new();
    let mut parser = Parser::new();
    let mut vm = VM::new(None);
    vm.init_builtin();
    parser.ident_table = vm.globals.ident_table.clone();
    let mut level = parser.get_context_depth();
    let mut lvar_collector = LvarCollector::new();
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
        match parser.parse_program(program.clone(), Some(lvar_collector.clone())) {
            Ok(parse_result) => {
                //println!("{:?}", node);
                vm.init(parse_result.ident_table);
                match vm.run_repl(&parse_result.node, &parse_result.lvar_collector) {
                    Ok(result) => {
                        parser.ident_table = vm.globals.ident_table.clone();
                        parser.lexer.source_info = parse_result.source_info;
                        lvar_collector = parse_result.lvar_collector;
                        let res_str = vm.val_pp(result);
                        println!("=> {}", res_str);
                    }
                    Err(err) => {
                        parse_result.source_info.show_loc(&err.loc());
                        match err.kind {
                            RubyErrorKind::ParseErr(e) => {
                                println!("parse error: {:?}", e);
                            }
                            RubyErrorKind::RuntimeErr(e) => match e {
                                RuntimeErrKind::Name(n) => {
                                    println!("runtime error: NoNameError ({})", n)
                                }
                                RuntimeErrKind::NoMethod(n) => {
                                    println!("runtime error: NoMethodError ({})", n)
                                }
                                RuntimeErrKind::Unimplemented(n) => {
                                    println!("runtime error: UnimplementedError ({})", n)
                                }
                                _ => {}
                            },
                        }
                        parser = parser_save;
                    }
                }
                level = parser.get_context_depth();
                program = String::new();
            }
            Err(err) => {
                if RubyErrorKind::ParseErr(ParseErrKind::UnexpectedEOF) == err.kind {
                    level = parser.get_context_depth();
                    parser = parser_save;
                    continue;
                }
                parser.show_tokens();
                level = parser.get_context_depth();
                parser.show_loc(&err.loc());
                eprintln!("RubyError: {:?}", err.kind);
                parser = parser_save;
                program = String::new();
            }
        }
    }
}
