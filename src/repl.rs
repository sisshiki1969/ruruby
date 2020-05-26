use ansi_term::Colour::Red;
use ruruby::error::*;
//use ruruby::parser::{LvarCollector, Parser};
//#[macro_use]
use ruruby::*;

pub fn repl_vm() {
    println!("RV: {}", std::mem::size_of::<RV>());
    println!("Value: {}", std::mem::size_of::<Value>());
    println!(
        "HashMap: {}",
        std::mem::size_of::<std::collections::HashMap<Value, Value>>()
    );
    println!(
        "Option<Box<HashMap>>: {}",
        std::mem::size_of::<Option<Box<std::collections::HashMap<Value, Value>>>>()
    );
    println!("RValue: {}", std::mem::size_of::<RValue>());
    println!("ObjKind: {}", std::mem::size_of::<ObjKind>());
    println!("RString: {}", std::mem::size_of::<RString>());
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
            &program,
            Some(lvar_collector.clone()),
        ) {
            Ok(parse_result) => {
                match vm.run_repl(&parse_result, context) {
                    Ok(result) => {
                        parser.ident_table = vm.globals.ident_table.clone();
                        parser.lexer.source_info = parse_result.source_info;
                        lvar_collector = parse_result.lvar_collector;
                        println!("=> {}", vm.val_inspect(result));
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
