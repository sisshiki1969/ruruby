use super::value::*;
use crate::loader::*;
use crate::vm::*;

pub struct Builtin {}

impl Builtin {
    pub fn init_builtin(globals: &mut Globals) {
        globals.add_builtin_method("puts", builtin_puts);
        globals.add_builtin_method("p", builtin_p);
        globals.add_builtin_method("print", builtin_print);
        globals.add_builtin_method("assert", builtin_assert);
        globals.add_builtin_method("require", builtin_require);
        globals.add_builtin_method("require_relative", builtin_require_relative);
        globals.add_builtin_method("block_given?", builtin_block_given);
        globals.add_builtin_method("method", builtin_method);

        /// Built-in function "puts".
        fn builtin_puts(
            vm: &mut VM,
            _receiver: PackedValue,
            args: VecArray,
            _block: Option<MethodRef>,
        ) -> VMResult {
            fn flatten(vm: &VM, val: PackedValue) {
                match val.as_array() {
                    None => println!("{}", vm.val_to_s(val)),
                    Some(aref) => {
                        for val in &aref.elements {
                            flatten(vm, val.clone());
                        }
                    }
                }
            }
            for arg in args.iter() {
                flatten(vm, arg.clone());
            }
            Ok(PackedValue::nil())
        }

        fn builtin_p(
            vm: &mut VM,
            _receiver: PackedValue,
            args: VecArray,
            _block: Option<MethodRef>,
        ) -> VMResult {
            for arg in args.iter() {
                println!("{}", vm.val_pp(*arg));
            }
            Ok(if args.len() == 1 {
                args[0]
            } else {
                PackedValue::array(&vm.globals, ArrayRef::from(args.to_vec()))
            })
        }

        /// Built-in function "print".
        fn builtin_print(
            vm: &mut VM,
            _receiver: PackedValue,
            args: VecArray,
            _block: Option<MethodRef>,
        ) -> VMResult {
            for arg in args.iter() {
                if let Value::Char(ch) = arg.unpack() {
                    let v = [ch];
                    use std::io::{self, Write};
                    io::stdout().write(&v).unwrap();
                } else {
                    print!("{}", vm.val_to_s(arg.clone()));
                }
            }
            Ok(PackedValue::nil())
        }

        /// Built-in function "assert".
        fn builtin_assert(
            vm: &mut VM,
            _receiver: PackedValue,
            args: VecArray,
            _block: Option<MethodRef>,
        ) -> VMResult {
            if args.len() != 2 {
                panic!("Invalid number of arguments.");
            }
            if !vm.eval_eq(args[0].clone(), args[1].clone())? {
                panic!(
                    "Assertion error: Expected: {} Actual: {}",
                    vm.val_pp(args[0]),
                    vm.val_pp(args[1]),
                );
            } else {
                println!("Assert OK: {:?}", vm.val_pp(args[0]));
                Ok(PackedValue::nil())
            }
        }

        fn builtin_require(
            vm: &mut VM,
            _receiver: PackedValue,
            args: VecArray,
            _block: Option<MethodRef>,
        ) -> VMResult {
            vm.check_args_num(args.len(), 1, 1)?;
            let file_name = match args[0].as_string() {
                Some(string) => string,
                None => return Err(vm.error_argument("file name must be a string.")),
            };
            let mut path = std::env::current_dir().unwrap();
            path.push(file_name);
            require(vm, path)?;
            Ok(PackedValue::bool(true))
        }

        fn builtin_require_relative(
            vm: &mut VM,
            _receiver: PackedValue,
            args: VecArray,
            _block: Option<MethodRef>,
        ) -> VMResult {
            vm.check_args_num(args.len(), 1, 1)?;
            let mut path = vm.root_path.last().unwrap().clone();

            let file_name = match args[0].as_string() {
                Some(string) => string,
                None => return Err(vm.error_argument("file name must be a string.")),
            };
            path.pop();
            path.push(file_name);
            require(vm, path)?;
            Ok(PackedValue::bool(true))
        }

        fn require(vm: &mut VM, path: PathBuf) -> Result<(), RubyError> {
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
            vm.class_stack.push(vm.globals.object_class);
            vm.run(absolute_path.to_str().unwrap(), program)?;
            vm.class_stack.pop().unwrap();
            vm.root_path.pop().unwrap();
            Ok(())
        }

        /// Built-in function "block_given?".
        fn builtin_block_given(
            vm: &mut VM,
            _receiver: PackedValue,
            _args: VecArray,
            _block: Option<MethodRef>,
        ) -> VMResult {
            Ok(PackedValue::bool(vm.context().block.is_some()))
        }

        fn builtin_method(
            vm: &mut VM,
            receiver: PackedValue,
            args: VecArray,
            _block: Option<MethodRef>,
        ) -> VMResult {
            eprintln!("method:{:?}", vm.val_pp(receiver));
            let name = match args[0].as_symbol() {
                Some(id) => id,
                None => return Err(vm.error_type("An argument must be a Symbol.")),
            };
            let method = match receiver.as_class() {
                Some(class) => match class.get_class_method(name) {
                    Some(method) => method.clone(),
                    None => return Err(vm.error_type("Method not found.")),
                },
                None => match receiver.get_class(&vm.globals).get_instance_method(name) {
                    Some(method) => method.clone(),
                    None => return Err(vm.error_type("Method not found.")),
                },
            };

            let val = PackedValue::method(&vm.globals, name, receiver, method);
            Ok(val)
        }
    }
}
