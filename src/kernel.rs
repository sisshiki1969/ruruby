use crate::loader::*;
use crate::*;
use rand;
use std::path::PathBuf;

pub struct Kernel {}

impl Kernel {
    pub fn init_kernel(globals: &mut Globals) -> Value {
        let id = globals.get_ident_id("Kernel");
        let kernel_class = ClassRef::from(id, None);
        globals.add_builtin_instance_method(kernel_class, "puts", puts);
        globals.add_builtin_instance_method(kernel_class, "p", p);
        globals.add_builtin_instance_method(kernel_class, "print", print);
        globals.add_builtin_instance_method(kernel_class, "assert", assert);
        globals.add_builtin_instance_method(kernel_class, "require", require);
        globals.add_builtin_instance_method(kernel_class, "require_relative", require_relative);
        globals.add_builtin_instance_method(kernel_class, "block_given?", block_given);
        globals.add_builtin_instance_method(kernel_class, "method", method);
        globals.add_builtin_instance_method(kernel_class, "is_a?", isa);
        globals.add_builtin_instance_method(kernel_class, "to_s", tos);
        globals.add_builtin_instance_method(kernel_class, "Integer", integer);
        globals.add_builtin_instance_method(kernel_class, "__dir__", dir);
        globals.add_builtin_instance_method(kernel_class, "__FILE__", file_);
        globals.add_builtin_instance_method(kernel_class, "raise", raise);
        globals.add_builtin_instance_method(kernel_class, "rand", rand);
        globals.add_builtin_instance_method(kernel_class, "loop", loop_);
        globals.add_builtin_instance_method(kernel_class, "exit", exit);
        let kernel = Value::class(globals, kernel_class);
        return kernel;

        /// Built-in function "puts".
        fn puts(vm: &mut VM, _: Value, args: &Args) -> VMResult {
            fn flatten(vm: &mut VM, val: Value) {
                match val.as_array() {
                    None => println!("{}", vm.val_to_s(val)),
                    Some(aref) => {
                        for val in &aref.elements {
                            flatten(vm, val.clone());
                        }
                    }
                }
            }
            for i in 0..args.len() {
                flatten(vm, args[i]);
            }
            Ok(Value::nil())
        }

        fn p(vm: &mut VM, _: Value, args: &Args) -> VMResult {
            for i in 0..args.len() {
                println!("{}", vm.val_inspect(args[i]));
            }
            if args.len() == 1 {
                Ok(args[0])
            } else {
                Ok(Value::array_from(&vm.globals, args.to_vec()))
            }
        }

        /// Built-in function "print".
        fn print(vm: &mut VM, _: Value, args: &Args) -> VMResult {
            for i in 0..args.len() {
                match args[i].as_bytes() {
                    Some(bytes) => {
                        use std::io::{self, Write};
                        io::stdout().write(bytes).unwrap();
                    }
                    None => print!("{}", vm.val_to_s(args[i])),
                }
            }
            Ok(Value::nil())
        }

        /// Built-in function "assert".
        fn assert(vm: &mut VM, _: Value, args: &Args) -> VMResult {
            vm.check_args_num(args.len(), 2, 2)?;
            if !vm.eval_eq(args[0], args[1])? {
                panic!(
                    "Assertion error: Expected: {} Actual: {}",
                    vm.val_inspect(args[0]),
                    vm.val_inspect(args[1]),
                );
            } else {
                println!("Assert OK: {:?}", vm.val_inspect(args[0]));
                Ok(Value::nil())
            }
        }

        fn require(vm: &mut VM, _: Value, args: &Args) -> VMResult {
            vm.check_args_num(args.len(), 1, 1)?;
            let file_name = match args[0].as_string() {
                Some(string) => string,
                None => return Err(vm.error_argument("file name must be a string.")),
            };
            let mut path = std::env::current_dir().unwrap();
            path.push(file_name);
            require_main(vm, path)?;
            Ok(Value::bool(true))
        }

        fn require_relative(vm: &mut VM, _: Value, args: &Args) -> VMResult {
            vm.check_args_num(args.len(), 1, 1)?;
            let context = vm.context();
            let mut path = std::path::PathBuf::from(context.iseq_ref.source_info.path.clone());

            let file_name = match args[0].as_string() {
                Some(string) => PathBuf::from(string),
                None => return Err(vm.error_argument("file name must be a string.")),
            };
            path.pop();
            //path.push(file_name);
            for p in file_name.iter() {
                if p == ".." {
                    path.pop();
                } else {
                    path.push(p);
                }
            }
            path.set_extension("rb");
            require_main(vm, path)?;
            Ok(Value::bool(true))
        }

        fn require_main(vm: &mut VM, path: PathBuf) -> Result<(), RubyError> {
            let file_name = path.to_string_lossy().to_string();
            let (absolute_path, program) = match load_file(file_name.clone()) {
                Ok((path, program)) => (path, program),
                Err(err) => {
                    match err {
                        LoadError::NotFound(msg) => {
                            eprintln!("No such file or directory --- {} (LoadError)", &file_name);
                            eprintln!("{}", msg);
                        }
                        LoadError::CouldntOpen(msg) => {
                            eprintln!("Cannot open file. '{}'", &file_name);
                            eprintln!("{}", msg);
                        }
                    }
                    return Err(vm.error_internal("LoadError"));
                }
            };
            #[cfg(feature = "verbose")]
            eprintln!("reading:{}", absolute_path.to_string_lossy());
            vm.root_path.push(path);
            vm.class_push(vm.globals.builtins.object);
            vm.run(absolute_path, &program, None)?;
            vm.class_pop();
            vm.root_path.pop().unwrap();
            Ok(())
        }

        /// Built-in function "block_given?".
        fn block_given(vm: &mut VM, _: Value, _args: &Args) -> VMResult {
            Ok(Value::bool(vm.context().block.is_some()))
        }

        fn method(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
            vm.check_args_num(args.len(), 1, 1)?;
            let name = match args[0].as_symbol() {
                Some(id) => id,
                None => return Err(vm.error_type("An argument must be a Symbol.")),
            };
            let method = vm.get_method(self_val, name)?;
            let val = Value::method(&vm.globals, name, self_val, method);
            Ok(val)
        }

        fn isa(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
            vm.check_args_num(args.len(), 1, 1)?;
            let mut recv_class = self_val.get_class_object(&vm.globals);
            loop {
                if recv_class.id() == args[0].id() {
                    return Ok(Value::true_val());
                }
                recv_class = recv_class.as_class().superclass;
                if recv_class.is_nil() {
                    return Ok(Value::false_val());
                }
            }
        }

        fn tos(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
            vm.check_args_num(args.len(), 0, 0)?;
            let s = vm.val_to_s(self_val);
            Ok(Value::string(&vm.globals, s))
        }

        fn integer(vm: &mut VM, _: Value, args: &Args) -> VMResult {
            vm.check_args_num(args.len(), 1, 1)?;
            let self_ = args[0];
            let val = if self_.is_packed_value() {
                if self_.is_packed_fixnum() {
                    self_.as_packed_fixnum()
                } else if self_.is_packed_num() {
                    self_.as_packed_flonum().trunc() as i64
                } else {
                    let inspect = vm.val_inspect(self_);
                    return Err(vm.error_type(format!("Can not convert {} into Integer.", inspect)));
                }
            } else {
                match self_.unpack() {
                    RV::Integer(num) => num,
                    RV::Float(num) => num as i64,
                    RV::Object(obj) => match &obj.kind {
                        ObjKind::String(s) => match s.parse::<i64>() {
                            Some(num) => num,
                            None => {
                                let inspect = vm.val_inspect(self_);
                                return Err(vm.error_type(format!(
                                    "Invalid value for Integer(): {}",
                                    inspect
                                )));
                            }
                        },
                        _ => {
                            let inspect = vm.val_inspect(self_);
                            return Err(
                                vm.error_type(format!("Can not convert {} into Integer.", inspect))
                            );
                        }
                    },
                    _ => {
                        let inspect = vm.val_inspect(self_);
                        return Err(
                            vm.error_type(format!("Can not convert {} into Integer.", inspect))
                        );
                    }
                }
            };
            Ok(Value::fixnum(val))
        }

        fn dir(vm: &mut VM, _: Value, args: &Args) -> VMResult {
            vm.check_args_num(args.len(), 0, 0)?;
            let mut path = vm.root_path.last().unwrap().clone();
            path.pop();
            Ok(Value::string(
                &vm.globals,
                path.to_string_lossy().to_string(),
            ))
        }

        fn file_(vm: &mut VM, _: Value, args: &Args) -> VMResult {
            vm.check_args_num(args.len(), 0, 0)?;
            //let path = vm.root_path.last().unwrap().clone();
            let path = vm.context().iseq_ref.source_info.path.clone();
            Ok(Value::string(
                &vm.globals,
                path.to_string_lossy().to_string(),
            ))
        }

        fn raise(vm: &mut VM, _: Value, args: &Args) -> VMResult {
            vm.check_args_num(args.len(), 0, 2)?;
            for i in 0..args.len() {
                eprintln!("{}", vm.val_inspect(args[i]));
            }
            Err(vm.error_unimplemented("error"))
        }

        fn rand(_vm: &mut VM, _: Value, _args: &Args) -> VMResult {
            let num = rand::random();
            Ok(Value::flonum(num))
        }

        fn loop_(vm: &mut VM, _: Value, args: &Args) -> VMResult {
            let method = vm.expect_block(args.block)?;
            let arg = Args::new0();
            loop {
                vm.eval_block(method, &arg)?;
            }
        }

        fn exit(vm: &mut VM, _: Value, args: &Args) -> VMResult {
            vm.check_args_num(args.len(), 0, 1)?;
            let code = if args.len() == 0 {
                0
            } else {
                args[0].expect_integer(vm, "Expect Integer.")?
            };
            std::process::exit(code as i32);
        }
    }
}
