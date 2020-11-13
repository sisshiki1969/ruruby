use crate::*;
use rand;
use std::path::PathBuf;

pub fn init(_globals: &mut Globals) -> Value {
    let mut kernel = Value::module();
    kernel.add_builtin_module_func("puts", puts);
    kernel.add_builtin_module_func("p", p);
    kernel.add_builtin_module_func("print", print);
    kernel.add_builtin_module_func("assert", assert);
    kernel.add_builtin_module_func("assert_error", assert_error);
    kernel.add_builtin_module_func("require", require);
    kernel.add_builtin_module_func("require_relative", require_relative);
    kernel.add_builtin_module_func("load", load);
    kernel.add_builtin_module_func("block_given?", block_given);
    kernel.add_builtin_module_func("method", method);
    kernel.add_builtin_module_func("is_a?", isa);
    kernel.add_builtin_module_func("__dir__", dir);
    kernel.add_builtin_module_func("__FILE__", file_);
    kernel.add_builtin_module_func("raise", raise);
    kernel.add_builtin_module_func("rand", rand_);
    kernel.add_builtin_module_func("loop", loop_);
    kernel.add_builtin_module_func("exit", exit);
    kernel.add_builtin_module_func("abort", abort);
    kernel.add_builtin_module_func("sleep", sleep);
    kernel.add_builtin_module_func("proc", proc);
    kernel.add_builtin_module_func("lambda", lambda);
    kernel.add_builtin_module_func("Integer", kernel_integer);
    kernel.add_builtin_module_func("Complex", kernel_complex);
    kernel.add_builtin_module_func("Array", kernel_array);
    kernel.add_builtin_module_func("at_exit", at_exit);
    kernel.add_builtin_module_func("`", command);
    return kernel;
}
/// Built-in function "puts".
fn puts(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    fn flatten(vm: &mut VM, val: Value) -> Result<(), RubyError> {
        match val.as_array() {
            None => println!("{}", vm.val_to_s(val)?),
            Some(aref) => {
                for val in &aref.elements {
                    flatten(vm, val.clone())?;
                }
            }
        }
        Ok(())
    }
    if args.len() == 0 {
        println!();
    }
    for arg in args.iter() {
        flatten(vm, *arg)?;
    }
    Ok(Value::nil())
}

fn p(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    for arg in args.iter() {
        println!("{}", vm.val_inspect(*arg)?);
    }
    match args.len() {
        0 => Ok(Value::nil()),
        1 => Ok(args[0]),
        _ => Ok(Value::array_from(args.to_vec())),
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
            None => print!("{}", vm.val_to_s(*arg)?),
        }
    }
    Ok(Value::nil())
}

/// Built-in function "assert".
fn assert(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 2)?;
    if !vm.eval_eq(args[0], args[1])? {
        let res = format!(
            "Assertion error: Expected: {:?} Actual: {:?}",
            args[0], args[1],
        );
        Err(vm.error_argument(res))
    } else {
        println!("Assert OK: {:?}", args[0]);
        Ok(Value::nil())
    }
}

fn assert_error(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let method = match &args.block {
        Some(block) => block,
        None => return Err(vm.error_argument("assert_error(): Block not given.")),
    };
    match vm.eval_block(method, &Args::new0()) {
        Ok(val) => Err(vm.error_argument(format!(
            "Assertion error: No error occured. returned {:?}",
            val
        ))),
        Err(err) => {
            println!("Assert_error OK:");
            err.show_err();
            err.show_loc(0);
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
    let mut load_path = match vm.get_global_var(IdentId::get_id("$:")) {
        Some(path) => path,
        None => return Ok(Value::false_val()),
    };
    let ainfo = load_path.expect_array(vm, "LOAD_PATH($:)")?;
    for path in ainfo.elements.iter_mut() {
        let mut base_path = PathBuf::from(path.expect_string(vm, "LOAD_PATH($:)")?);
        base_path.push(file_name);
        base_path.set_extension("rb");
        if base_path.exists() {
            return Ok(Value::bool(load_exec(vm, &base_path, false)?));
        }
    }
    Ok(Value::false_val())
    // TODO: This is not correct. Work around for load error in requiring .so files.
    //Err(vm.error_load(format!("Can not load such file -- {:?}", file_name)))
}

fn require_relative(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let mut path = vm.get_source_path();
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
    Ok(Value::bool(load_exec(vm, &path, false)?))
}

fn load(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let file_name = match args[0].as_string() {
        Some(string) => string,
        None => return Err(vm.error_argument("file name must be a string.")),
    };
    let path = PathBuf::from(file_name);
    if path.exists() {
        load_exec(vm, &path, true)?;
        return Ok(Value::true_val());
    }

    let mut load_path = match vm.get_global_var(IdentId::get_id("$:")) {
        Some(path) => path,
        None => return Err(vm.error_internal("Load path not found.")),
    };

    let mut load_ary = load_path
        .expect_array(vm, "LOAD_PATH($:)")?
        .elements
        .clone();
    for path in load_ary.iter_mut() {
        let mut base_path = PathBuf::from(path.expect_string(vm, "LOAD_PATH($:)")?);
        base_path.push(file_name);
        if base_path.exists() {
            load_exec(vm, &base_path, true)?;
            return Ok(Value::true_val());
        }
    }
    Err(vm.error_load(format!("Can not load such file -- {:?}", file_name)))
}

/// Load file and execute.
fn load_exec(vm: &mut VM, path: &PathBuf, allow_repeat: bool) -> Result<bool, RubyError> {
    let absolute_path = vm.canonicalize_path(path)?;
    let res = vm.globals.add_source_file(&absolute_path);
    if !allow_repeat && res.is_none() {
        return Ok(false);
    }
    let program = vm.load_file(&absolute_path)?;
    #[cfg(feature = "verbose")]
    eprintln!("reading:{}", absolute_path.to_string_lossy());
    //vm.root_path.push(absolute_path.clone());
    vm.class_push(BuiltinClass::object());
    vm.run(absolute_path, &program)?;
    vm.class_pop();
    //vm.root_path.pop().unwrap();
    Ok(true)
}

/// Built-in function "block_given?".
fn block_given(vm: &mut VM, _: Value, _args: &Args) -> VMResult {
    Ok(Value::bool(vm.current_context().block.is_some()))
}

fn method(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let name = match args[0].as_symbol() {
        Some(id) => id,
        None => return Err(vm.error_type("An argument must be a Symbol.")),
    };
    let method = vm.get_method_from_receiver(self_val, name)?;
    let val = Value::method(name, self_val, method);
    Ok(val)
}

fn isa(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let mut module = self_val.get_class();
    loop {
        let cinfo = module.as_module();
        let real_module = if cinfo.is_included() {
            cinfo.origin()
        } else {
            module
        };
        if real_module.id() == args[0].id() {
            return Ok(Value::true_val());
        }
        module = cinfo.upper();
        if module.is_nil() {
            return Ok(Value::false_val());
        };
    }
}

fn dir(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let mut path = vm.get_source_path();
    path.pop();
    Ok(Value::string(path.to_string_lossy().to_string()))
}

fn file_(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let path = vm.get_source_path();
    Ok(Value::string(path.to_string_lossy().to_string()))
}

/// raise -> ()
/// fail -> ()
/// raise(message, cause: $!) -> ()
/// fail(message, cause: $!) -> ()
/// raise(error_type, message = nil, backtrace = caller(0), cause: $!) -> ()
/// fail(error_type, message = nil, backtrace = caller(0), cause: $!) -> ()
fn raise(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_range(args.len(), 0, 2)?;
    /*for arg in args.iter() {
        eprintln!("{}", vm.val_inspect(*arg));
    }*/
    if args.len() == 1 && args[0].is_class() {
        if Some(IdentId::get_id("StopIteration")) == args[0].as_class().name() {
            return Err(vm.error_stop_iteration(""));
        };
    }
    Err(vm.error_unimplemented("error"))
}

fn rand_(_vm: &mut VM, _: Value, _args: &Args) -> VMResult {
    let num = rand::random();
    Ok(Value::float(num))
}

fn loop_(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let block = vm.expect_block(&args.block)?;
    let arg = Args::new0();
    loop {
        let res = vm.eval_block(&block, &arg);
        match res {
            Ok(_) => {}
            Err(err) => match &err.kind {
                RubyErrorKind::RuntimeErr {
                    kind: RuntimeErrKind::StopIteration,
                    ..
                } => {
                    return Ok(Value::nil());
                }

                _ => return Err(err),
            },
        }
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

fn abort(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_range(args.len(), 0, 1)?;
    let msg = if args.len() == 0 {
        "".to_string()
    } else {
        let mut msg = args[0];
        msg.expect_string(vm, "1st")?.to_owned()
    };
    eprintln!("{}", msg);
    std::process::exit(1);
}

fn sleep(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_range(args.len(), 0, 1)?;
    let secs = if args.len() == 0 {
        0.0
    } else {
        let secs = match args[0].unpack() {
            RV::Integer(i) => i as f64,
            RV::Float(f) => f,
            _ => return Err(vm.error_argument("Arg must be Integer or Float.")),
        };
        if secs < 0.0 {
            return Err(vm.error_argument("Negative number."));
        }
        secs
    };
    let start = std::time::Instant::now();
    std::thread::sleep(std::time::Duration::from_secs_f64(secs));
    let duration = start.elapsed().as_secs() as u64 as i64;
    Ok(Value::integer(duration))
}

fn proc(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let block = vm.expect_block(&args.block)?;
    let procobj = vm.create_proc(block)?;
    Ok(procobj)
}

fn lambda(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let block = vm.expect_block(&args.block)?;
    let procobj = vm.create_lambda(block)?;
    Ok(procobj)
}

fn kernel_integer(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let val = match args[0].unpack() {
        RV::Integer(num) => num,
        RV::Float(num) => num as i64,
        RV::Object(obj) => match &obj.kind {
            ObjKind::String(s) => match s.parse::<i64>() {
                Some(num) => num,
                None => {
                    let inspect = vm.val_inspect(args[0])?;
                    return Err(vm.error_type(format!("Invalid value for Integer(): {}", inspect)));
                }
            },
            _ => {
                let inspect = vm.val_inspect(args[0])?;
                return Err(vm.error_type(format!("Can not convert {} into Integer.", inspect)));
            }
        },
        _ => {
            let inspect = vm.val_inspect(args[0])?;
            return Err(vm.error_type(format!("Can not convert {} into Integer.", inspect)));
        }
    };
    Ok(Value::integer(val))
}

fn kernel_complex(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_range(args.len(), 1, 3)?;
    let (r, i, ex) = match args.len() {
        1 => (args[0], Value::integer(0), true),
        2 => (args[0], args[1], true),
        3 => (args[0], args[1], args[2].to_bool()),
        _ => unreachable!(),
    };
    if !r.is_real() || !i.is_real() {
        if ex {
            return Err(vm.error_argument("Not a real."));
        } else {
            return Ok(Value::nil());
        }
    }

    Ok(Value::complex(r, i))
}

/// Array(arg) -> Array
fn kernel_array(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let arg = args[0];
    let arg_class = arg.get_class_for_method();
    match vm.globals.find_method(arg_class, IdentId::get_id("to_a")) {
        Some(method) => return vm.eval_send(method, arg, &Args::new0()),
        None => {}
    };
    match vm.globals.find_method(arg_class, IdentId::get_id("to_ary")) {
        Some(method) => return vm.eval_send(method, arg, &Args::new0()),
        None => {}
    };
    let res = Value::array_from(vec![arg]);
    Ok(res)
}

fn at_exit(_vm: &mut VM, _self_val: Value, _args: &Args) -> VMResult {
    Ok(_self_val)
}

fn command(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    use std::process::Command;
    vm.check_args_num(args.len(), 1)?;
    let mut arg = args[0].to_owned();
    let mut input = arg.expect_string(vm, "Arg")?.split_whitespace();
    let command = input.next().unwrap();
    let output = match Command::new(command).args(input).output() {
        Ok(ok) => ok,
        Err(err) => return Err(vm.error_internal(format!("Command failed. {:?}", err.kind()))),
    };
    Ok(Value::bytes(output.stdout))
}

#[cfg(test)]
mod test {
    use crate::test::*;

    #[test]
    fn is_a() {
        let program = "
        module M
        end
        class C < Object
          include M
        end
        class S < C
        end

        obj = S.new
        assert true, obj.is_a?(S)
        assert true, obj.is_a?(C)
        assert true, obj.is_a?(Object)
        #assert true, obj.is_a?(M)
        assert false, obj.is_a?(Integer)
        assert false, obj.is_a?(Array)
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
        assert -4, Integer(-4.2)
        assert 9, Integer(9.02)
        assert 10, Integer("10")
        assert 4, Integer 4
        assert -4, Integer -4.2
        assert 10, Integer"10"
        assert_error { Integer("13.55") }
        assert_error { Integer([1,3,6]) }
        assert_error { Integer(:"2") }
        "#;
        assert_script(program);
    }

    #[test]
    fn kernel_etc() {
        let program = r###"
        assert_error { assert 2, 3 }
        assert_error { assert_error { true } }
        assert_error { raise }
        require "#{Dir.pwd}/tests/kernel_test"
        require_relative "../../tests/kernel_test"
        load "#{Dir.pwd}/tests/kernel_test.rb"
        assert_error { require_relative "kernel_test" }
        assert_error { assert rand, rand }
        sleep(0.1)
        print "Ruby"
        at_exit
        "###;
        assert_script(program);
    }
    /*
        #[test]
        fn kernel_exit() {
            let program = r###"
            exit(0)
            "###;
            assert_script(program);
        }
    */
    #[test]
    fn kernel_loop() {
        let program = r#"
      class Enum
        def initialize(receiver, method = :each, *args)
          @fiber = Fiber.new do
            receiver.send(method, *args) do |x|
              Fiber.yield(x)
            end
            raise StopIteration
          end
        end
        def each
          if block_given?
            loop do
              yield @fiber.resume
            end
          else
            loop do
              @fiber.resume
            end
          end
        end
      end

      str = "Yet Another Ruby Hacker"
      e = Enum.new(str, :scan, /\w+/)
      res = []
      e.each { |x| res << x }
      assert(["Yet", "Another", "Ruby", "Hacker"], res)
        "#;
        assert_script(program);
    }

    #[test]
    fn kernel_eval() {
        let program = r#"
        n = 2
        assert("n", %w{n}*"")
        assert(2, eval(%w{n}*""))
        assert("eval(%w{n}*\"\")", %q{eval(%w{n}*"")})
        assert(2, eval(%q{eval(%w{n}*"")}))
        "#;
        assert_script(program);
    }

    #[test]
    fn kernel_complex() {
        let program = r#"
        assert(Complex.rect(5.2, -99), Complex(5.2, -99))
        "#;
        assert_script(program);
    }

    #[test]
    fn kernel_array() {
        let program = r#"
        assert([1,2,3], Array([1,2,3]))
        assert([1], Array(1))
        assert([1,2,3], Array(1..3))
        "#;
        assert_script(program);
    }

    #[test]
    fn kernel_command() {
        let program = r#"
        assert("Cargo.toml\n", `ls Cargo.toml`)
        a = "toml"
        assert("Cargo.toml\n", `ls Cargo.#{a}`)
        "#;
        assert_script(program);
    }
}
