use crate::loader::*;
use crate::*;
use rand;
use std::path::PathBuf;

pub fn init(globals: &mut Globals) -> Value {
    let id = globals.get_ident_id("Kernel");
    let kernel_class = ClassRef::from(id, None);
    globals.add_builtin_instance_method(kernel_class, "puts", puts);
    globals.add_builtin_instance_method(kernel_class, "p", p);
    globals.add_builtin_instance_method(kernel_class, "print", print);
    globals.add_builtin_instance_method(kernel_class, "assert", assert);
    globals.add_builtin_instance_method(kernel_class, "assert_error", assert_error);
    globals.add_builtin_instance_method(kernel_class, "require", require);
    globals.add_builtin_instance_method(kernel_class, "require_relative", require_relative);
    globals.add_builtin_instance_method(kernel_class, "block_given?", block_given);
    globals.add_builtin_instance_method(kernel_class, "method", method);
    globals.add_builtin_instance_method(kernel_class, "is_a?", isa);
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
        for arg in args.iter() {
            flatten(vm, *arg);
        }
        Ok(Value::nil())
    }

    fn p(vm: &mut VM, _: Value, args: &Args) -> VMResult {
        for arg in args.iter() {
            println!("{}", vm.val_inspect(*arg));
        }
        if args.len() == 1 {
            Ok(args[0])
        } else {
            Ok(Value::array_from(&vm.globals, args.to_vec()))
        }
    }

    /// Built-in function "print".
    fn print(vm: &mut VM, _: Value, args: &Args) -> VMResult {
        for arg in args.iter() {
            match arg.as_bytes() {
                Some(bytes) => {
                    use std::io::{self, Write};
                    io::stdout().write(bytes).unwrap();
                }
                None => print!("{}", vm.val_to_s(*arg)),
            }
        }
        Ok(Value::nil())
    }

    /// Built-in function "assert".
    fn assert(vm: &mut VM, _: Value, args: &Args) -> VMResult {
        vm.check_args_num(args.len(), 2)?;
        if !vm.eval_eq(args[0], args[1])? {
            let res = format!(
                "Assertion error: Expected: {} Actual: {}",
                vm.val_inspect(args[0]),
                vm.val_inspect(args[1]),
            );
            Err(vm.error_argument(res))
        } else {
            println!("Assert OK: {:?}", vm.val_inspect(args[0]));
            Ok(Value::nil())
        }
    }

    fn assert_error(vm: &mut VM, _: Value, args: &Args) -> VMResult {
        vm.check_args_num(args.len(), 0)?;
        let method = match args.block {
            Some(block) => block,
            None => return Err(vm.error_argument("assert_error(): Block not given.")),
        };
        match vm.eval_block(method, &Args::new0()) {
            Ok(val) => {
                let res = format!(
                    "Assertion error: No error occured. returned {}",
                    vm.val_inspect(val)
                );
                Err(vm.error_argument(res))
            }
            Err(err) => {
                println!("Assert_error OK: {:?}", err.kind);
                Ok(Value::nil())
            }
        }
    }

    fn require(vm: &mut VM, _: Value, args: &Args) -> VMResult {
        vm.check_args_num(args.len(), 1)?;
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
        vm.check_args_num(args.len(), 1)?;
        let context = vm.context();
        let mut path = std::path::PathBuf::from(context.iseq_ref.source_info.path.clone());

        let file_name = match args[0].as_string() {
            Some(string) => PathBuf::from(string),
            None => return Err(vm.error_argument("file name must be a string.")),
        };
        path.pop();
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
        #[cfg_attr(tarpaulin, skip)]
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
        vm.check_args_num(args.len(), 1)?;
        let name = match args[0].as_symbol() {
            Some(id) => id,
            None => return Err(vm.error_type("An argument must be a Symbol.")),
        };
        let method = vm.get_method(self_val, name)?;
        let val = Value::method(&vm.globals, name, self_val, method);
        Ok(val)
    }

    fn isa(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
        vm.check_args_num(args.len(), 1)?;
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

    fn integer(vm: &mut VM, _: Value, args: &Args) -> VMResult {
        vm.check_args_num(args.len(), 1)?;
        let val = match args[0].unpack() {
            RV::Integer(num) => num,
            RV::Float(num) => num as i64,
            RV::Object(obj) => match &obj.kind {
                ObjKind::String(s) => match s.parse::<i64>() {
                    Some(num) => num,
                    None => {
                        let inspect = vm.val_inspect(args[0]);
                        return Err(
                            vm.error_type(format!("Invalid value for Integer(): {}", inspect))
                        );
                    }
                },
                _ => {
                    let inspect = vm.val_inspect(args[0]);
                    return Err(vm.error_type(format!("Can not convert {} into Integer.", inspect)));
                }
            },
            _ => {
                let inspect = vm.val_inspect(args[0]);
                return Err(vm.error_type(format!("Can not convert {} into Integer.", inspect)));
            }
        };
        Ok(Value::fixnum(val))
    }

    fn dir(vm: &mut VM, _: Value, args: &Args) -> VMResult {
        vm.check_args_num(args.len(), 0)?;
        let mut path = match vm.root_path.last() {
            Some(path) => path,
            None => return Ok(Value::nil()),
        }
        .clone();
        path.pop();
        Ok(Value::string(
            &vm.globals,
            path.to_string_lossy().to_string(),
        ))
    }

    fn file_(vm: &mut VM, _: Value, args: &Args) -> VMResult {
        vm.check_args_num(args.len(), 0)?;
        let path = vm.context().iseq_ref.source_info.path.clone();
        Ok(Value::string(
            &vm.globals,
            path.to_string_lossy().to_string(),
        ))
    }

    fn raise(vm: &mut VM, _: Value, args: &Args) -> VMResult {
        vm.check_args_range(args.len(), 0, 2)?;
        for arg in args.iter() {
            eprintln!("{}", vm.val_inspect(*arg));
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
        vm.check_args_range(args.len(), 0, 1)?;
        let code = if args.len() == 0 {
            0
        } else {
            args[0].expect_integer(vm, "Expect Integer.")?
        };
        std::process::exit(code as i32);
    }
}

#[cfg(test)]
mod test {
    use crate::test::*;

    #[test]
    fn is_a() {
        let program = "
        module M
        end
        class C
        end
        class S < C
        end

        obj = S.new
        assert true, obj.is_a?(S)
        assert true, obj.is_a?(C)
        assert true, obj.is_a?(Object)
        assert false, obj.is_a?(Integer)
        assert false, obj.is_a?(Array)
        assert false, obj.is_a?(M)
        ";
        assert_script(program);
    }

    #[test]
    fn block_given() {
        let program = "
        def foo
            return block_given?
        end

        assert true, foo {|x| x}
        assert false, foo
        ";
        assert_script(program);
    }

    #[test]
    fn integer() {
        let program = r#"
        assert 4, Integer(4)
        assert 9, Integer(9.88)
        assert 9, Integer(9.02)
        assert 10, Integer("10")
        assert_error { Integer("13.55") }
        assert_error { Integer([1,3,6]) }
        assert_error { Integer(:"2") }
        "#;
        assert_script(program);
    }

    #[test]
    fn kernel_etc() {
        let program = r#"
        assert_error { assert 2, 3 }
        assert_error { assert_error { true } }
        assert_error { raise }
        require_relative "../../tests/kernel_test.rb"
        assert_error { require_relative "kernel_test.rb" }
        assert_error { assert rand. rand }
        "#;
        assert_script(program);
    }
}
