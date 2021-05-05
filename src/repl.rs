use ansi_term::Colour::Red;
use ruruby::error::*;
use ruruby::*;
use rustyline::{error::ReadlineError, Editor};

pub fn repl_vm() {
    assert_eq!(8, std::mem::size_of::<Value>());
    assert_eq!(56, std::mem::size_of::<RValue>());
    #[cfg(debug_assertions)]
    {
        println!("RV: {}", std::mem::size_of::<RV>());
        println!("Value: {}", std::mem::size_of::<Value>());
        println!("Option<Value>: {}", std::mem::size_of::<Option<Value>>());
        println!("ObjKind: {}", std::mem::size_of::<ObjKind>());
        println!("HashInfo: {}", std::mem::size_of::<HashInfo>());
        println!("RangeInfo: {}", std::mem::size_of::<RangeInfo>());
        println!("RString: {}", std::mem::size_of::<RString>());
        println!("ClassInfo: {}", std::mem::size_of::<ClassInfo>());
        println!(
            "FiberContext: {}",
            std::mem::size_of::<crate::coroutine::FiberContext>()
        );
        println!("RegexpInfo: {}", std::mem::size_of::<RegexpInfo>());
        println!("MethodObjInfo: {}", std::mem::size_of::<MethodObjInfo>());
        println!("ArrayInfo: {}", std::mem::size_of::<ArrayInfo>());
        println!("MethodInfo: {}", std::mem::size_of::<MethodInfo>());
        println!(
            "Option<MethodId>: {}",
            std::mem::size_of::<Option<MethodId>>()
        );
    }
    let mut rl = Editor::<()>::new();
    let prompt_body = if cfg!(not(unix)) {
        // In Windows, it seems that ansi_term does not work well in rustyline.
        format!("irb:")
    } else {
        format!("{}", Red.bold().paint("irb:"))
    };
    let mut program = String::new();
    let mut parser = Parser::new();
    let mut globals = GlobalsRef::new_globals();
    let mut vm = globals.create_main_fiber();
    vm.set_global_var(IdentId::get_id("$0"), Value::string("irb"));
    let mut level = parser.get_context_depth();
    let context = ContextRef::new_heap(
        vm.globals.main_object,
        Block::None,
        ISeqRef::default(),
        None,
    );
    rustyline::config::Builder::new().tab_stop(0).indent_size(0);
    loop {
        let prompt = if program.len() == 0 { ">" } else { "*" };
        let readline = rl.readline(&format!(
            "{}{:1}{} {}",
            prompt_body,
            level,
            prompt,
            " ".repeat(level * 2)
        ));
        let line = match readline {
            Ok(line) => {
                rl.add_history_entry(&line);
                line
            }
            Err(err) => match err {
                ReadlineError::Interrupted => {
                    program = String::new();
                    level = 0;
                    continue;
                }
                ReadlineError::Eof => return,
                _ => continue,
            },
        };

        program = format!("{}{}\n", program, line);

        match parser.clone().parse_program_repl(
            std::path::PathBuf::from("REPL"),
            &program,
            Some(context),
        ) {
            Ok(parse_result) => {
                let source_info = parse_result.source_info;
                match vm.run_repl(parse_result, context) {
                    Ok(result) => {
                        parser.lexer.source_info = source_info;
                        println!("=> {:?}", result);
                    }
                    Err(err) => {
                        for (info, loc) in &err.info {
                            info.show_loc(loc);
                        }
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
