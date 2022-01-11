use super::*;
use ansi_term::Colour::Red;
use rustyline::{error::ReadlineError, Editor};

impl VM {
    pub fn invoke_repl(&mut self, context: EnvFrame) -> VMResult {
        assert_eq!(8, std::mem::size_of::<Value>());
        assert_eq!(64, std::mem::size_of::<RValue>());
        /*#[cfg(debug_assertions)]
        {
            println!("VMResult: {}", std::mem::size_of::<VMResult>());
            println!("RubyError: {}", std::mem::size_of::<RubyError>());
            println!("ObjKind: {}", std::mem::size_of::<ObjKind>());
            println!("HashInfo: {}", std::mem::size_of::<HashInfo>());
            println!("RangeInfo: {}", std::mem::size_of::<RangeInfo>());
            println!("RString: {}", std::mem::size_of::<RString>());
            println!("ClassInfo: {}", std::mem::size_of::<ClassInfo>());
            println!(
                "FiberContext: {}",
                std::mem::size_of::<ruruby::coroutine::FiberContext>()
            );
            println!("RegexpInfo: {}", std::mem::size_of::<RegexpInfo>());
            println!("MethodObjInfo: {}", std::mem::size_of::<MethodObjInfo>());
            println!("ArrayInfo: {}", std::mem::size_of::<ArrayInfo>());
            println!("MethodInfo: {}", std::mem::size_of::<MethodInfo>());
            println!("TimeInfo: {}", std::mem::size_of::<TimeInfo>());
            println!("Option<MethodId>: {}", std::mem::size_of::<Option<FnId>>());
        }*/
        let mut editor = Editor::<()>::new();
        let prompt_body = if cfg!(not(unix)) {
            // In Windows, it seems that ansi_term does not work well with rustyline.
            format!("irrb:")
        } else {
            format!("{}", Red.bold().paint("irrb:"))
        };
        let mut script = String::new();
        self.set_global_var(IdentId::get_id("$0"), Value::string("irrb"));

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
                    ReadlineError::Eof => return Ok(Value::nil()),
                    _ => continue,
                },
            };

            script += &line;
            {
                match self.eval_binding("REPL".to_string(), script.clone(), context) {
                    Ok(res) => println!("=> {:?}", res),
                    Err(err) => match &err.kind {
                        RubyErrorKind::ParseErr(kind) => match kind {
                            ParseErrKind::UnexpectedEOF => continue,
                            _ => {
                                self.show_err(&err);
                                err.show_loc(0);
                            }
                        },
                        RubyErrorKind::SystemExit(code) => {
                            eprintln!("exited with code {}", *code as i32);
                            return Ok(Value::fixnum(*code));
                            //std::process::exit(*code as i32);
                        }
                        _ => {
                            self.show_err(&err);
                            err.show_loc(0);
                            self.clear();
                        }
                    },
                }
            }
            script = String::new();
        }
    }
}
