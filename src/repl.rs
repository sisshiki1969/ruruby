use ansi_term::Colour::Red;
use ruruby::error::*;
use ruruby::*;
use rustyline::{error::ReadlineError, Editor};

pub fn repl_vm() {
    assert_eq!(8, std::mem::size_of::<Value>());
    assert_eq!(56, std::mem::size_of::<RValue>());
    #[cfg(debug_assertions)]
    {
        println!("VMResult: {}", std::mem::size_of::<VMResult>());
        println!("Context: {}", std::mem::size_of::<Context>());
        println!("ErrorInfo: {}", std::mem::size_of::<ErrorInfo>());
        //println!("RV: {}", std::mem::size_of::<RV>());
        //println!("Value: {}", std::mem::size_of::<Value>());
        //println!("Option<Value>: {}", std::mem::size_of::<Option<Value>>());
        println!("ObjKind: {}", std::mem::size_of::<ObjKind>());
        /*
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
        );*/
    }
    let mut editor = Editor::<()>::new();
    let prompt_body = if cfg!(not(unix)) {
        // In Windows, it seems that ansi_term does not work well with rustyline.
        format!("irb:")
    } else {
        format!("{}", Red.bold().paint("irb:"))
    };
    let mut temp_script = String::new();
    let mut parser = Parser::new(&String::new());
    let source_info = SourceInfoRef::new(SourceInfo::new("REPL", ""));
    let mut parser_save = parser.clone();
    let mut globals = GlobalsRef::new_globals();
    let mut vm = globals.create_main_fiber();
    vm.set_global_var(IdentId::get_id("$0"), Value::string("irb"));
    let context = ContextRef::new_heap(
        vm.globals.main_object,
        Block::None,
        ISeqRef::default(),
        None,
    );
    loop {
        let prompt = if temp_script.len() == 0 { ">" } else { "*" };
        let readline = editor.readline(&format!("{}{} ", prompt_body, prompt,));
        let line = match readline {
            Ok(line) => {
                editor.add_history_entry(&line);
                line + "\n"
            }
            Err(err) => match err {
                ReadlineError::Interrupted => {
                    temp_script = String::new();
                    continue;
                }
                ReadlineError::Eof => return,
                _ => continue,
            },
        };

        temp_script += &line;
        parser = parser_save.clone();
        parser.lexer.append(&temp_script);

        match parser.parse_program_repl(context) {
            Ok(parse_result) => match vm.run_repl(parse_result, source_info, context) {
                Ok(result) => {
                    parser_save.lexer.set_source_info(source_info);
                    println!("=> {:?}", result);
                }
                Err(err) => {
                    for (info, loc) in &err.info {
                        info.show_loc(loc);
                    }
                    err.show_err();
                    vm.clear();
                }
            },
            Err(err) => {
                if ParseErrKind::UnexpectedEOF == err.0 {
                    continue;
                }
                let err = RubyError::new_parse_err(err.0, source_info, err.1);
                err.show_loc(0);
                err.show_err();
            }
        }
        temp_script = String::new();
    }
}
