use ansi_term::Colour::Red;
use ruruby::*;
use rustyline::{error::ReadlineError, Editor};
use std::path::PathBuf;

pub(crate) fn repl_vm(mut vm: VMRef) {
    assert_eq!(8, std::mem::size_of::<Value>());
    assert_eq!(56, std::mem::size_of::<RValue>());
    #[cfg(debug_assertions)]
    {
        println!("VMResult: {}", std::mem::size_of::<VMResult>());
        println!("Context: {}", std::mem::size_of::<HeapContext>());
        println!("RubyError: {}", std::mem::size_of::<RubyError>());
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
    let mut script = String::new();
    //let mut globals = GlobalsRef::new_globals();
    //let mut vm = globals.create_main_fiber();
    vm.set_global_var(IdentId::get_id("$0"), Value::string("irb"));
    let context = HeapCtxRef::new_heap(vm.globals.main_object, ISeqRef::default(), None, None);
    loop {
        let prompt = if script.len() == 0 { ">" } else { "*" };
        let readline = editor.readline(&format!("{}{} ", prompt_body, prompt,));
        let line = match readline {
            Ok(line) => {
                editor.add_history_entry(&line);
                line + "\n"
            }
            Err(err) => match err {
                ReadlineError::Interrupted => {
                    script = String::new();
                    continue;
                }
                ReadlineError::Eof => return,
                _ => continue,
            },
        };

        script += &line;
        {
            match Parser::<DynamicFrame>::parse_program_repl(
                script.clone(),
                PathBuf::from("REPL"),
                context.as_dfp(),
            ) {
                Ok(parse_result) => match vm.run_repl(parse_result, context) {
                    Ok(result) => {
                        println!("=> {:?}", result);
                    }
                    Err(err) => {
                        vm.show_err(&err);
                        err.show_loc(0);
                        vm.clear();
                    }
                },
                Err(err) => {
                    match &err.kind {
                        RubyErrorKind::ParseErr(kind) => match kind {
                            ParseErrKind::UnexpectedEOF => continue,
                            _ => {}
                        },
                        _ => {}
                    };
                    vm.globals.show_err(&err);
                    err.show_loc(0);
                }
            }
        }
        script = String::new();
    }
}
