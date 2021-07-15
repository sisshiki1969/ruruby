use crate::loader::*;
use crate::*;
use rand;
use std::path::PathBuf;

pub fn init() -> Module {
    let class = Module::module();
    BuiltinClass::set_toplevel_constant("Kernel", class);
    class.add_builtin_module_func("puts", puts);
    class.add_builtin_module_func("p", p);
    class.add_builtin_module_func("print", print);
    class.add_builtin_module_func("assert", assert);
    class.add_builtin_module_func("assert_error", assert_error);
    class.add_builtin_module_func("require", require);
    class.add_builtin_module_func("require_relative", require_relative);
    class.add_builtin_module_func("load", load);
    class.add_builtin_module_func("block_given?", block_given);
    class.add_builtin_module_func("is_a?", isa);
    class.add_builtin_module_func("kind_of?", isa);
    class.add_builtin_module_func("__dir__", dir);
    class.add_builtin_module_func("__FILE__", file_);
    class.add_builtin_module_func("raise", raise);
    class.add_builtin_module_func("rand", rand_);
    class.add_builtin_module_func("loop", loop_);
    class.add_builtin_module_func("exit", exit);
    class.add_builtin_module_func("abort", abort);
    class.add_builtin_module_func("sleep", sleep);
    class.add_builtin_module_func("proc", proc);
    class.add_builtin_module_func("lambda", lambda);
    class.add_builtin_module_func("Integer", kernel_integer);
    class.add_builtin_module_func("Complex", kernel_complex);
    class.add_builtin_module_func("Array", kernel_array);
    class.add_builtin_module_func("at_exit", at_exit);
    class.add_builtin_module_func("`", command);
    class.add_builtin_module_func("eval", eval);
    class
}
/// Built-in function "puts".
fn puts(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    fn flatten(vm: &mut VM, val: Value) -> Result<(), RubyError> {
        match val.as_array() {
            None => println!("{}", val.val_to_s(vm)?),
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
            None => print!("{}", arg.val_to_s(vm)?),
        }
    }
    Ok(Value::nil())
}

/// Built-in function "assert".
fn assert(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(2)?;
    if !vm.eval_eq(args[0], args[1])? {
        let res = format!(
            "Assertion error: Expected: {:?} Actual: {:?}",
            args[0], args[1],
        );
        Err(RubyError::argument(res))
    } else {
        println!("Assert OK: {:?}", args[0]);
        Ok(Value::nil())
    }
}

fn assert_error(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let method = args.expect_block()?;
    match vm.eval_block(method, &Args::new0()) {
        Ok(val) => Err(RubyError::argument(format!(
            "Assertion error: No error occured. returned {:?}",
            val
        ))),
        Err(err) => {
            match err.kind {
                RubyErrorKind::BlockReturn | RubyErrorKind::MethodReturn => {
                    vm.stack_pop();
                }
                _ => {}
            }
            println!("Assert_error OK:");
            err.show_err();
            err.show_loc(0);
            Ok(Value::nil())
        }
    }
}

fn require(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let file_name = match args[0].as_string() {
        Some(string) => string,
        None => return Err(RubyError::argument("file name must be a string.")),
    };
    Ok(Value::bool(vm.require(file_name)?))
}

fn require_relative(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let mut path = vm.get_source_path();
    let file_name = match args[0].as_string() {
        Some(string) => PathBuf::from(string),
        None => return Err(RubyError::argument("file name must be a string.")),
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
    args.check_args_num(1)?;
    let file_name = match args[0].as_string() {
        Some(string) => string,
        None => return Err(RubyError::argument("file name must be a string.")),
    };
    let path = PathBuf::from(file_name);
    if path.exists() {
        load_exec(vm, &path, true)?;
        return Ok(Value::true_val());
    }

    let mut load_path = match vm.get_global_var(IdentId::get_id("$:")) {
        Some(path) => path,
        None => return Err(RubyError::internal("Load path not found.")),
    };

    let mut load_ary = load_path.expect_array("LOAD_PATH($:)")?.elements.clone();
    for path in load_ary.iter_mut() {
        let mut base_path = PathBuf::from(path.expect_string("LOAD_PATH($:)")?);
        base_path.push(file_name);
        if base_path.exists() {
            load_exec(vm, &base_path, true)?;
            return Ok(Value::true_val());
        }
    }
    Err(RubyError::load(format!(
        "Can not load such file -- {:?}",
        file_name
    )))
}

/// Built-in function "block_given?".
fn block_given(vm: &mut VM, _: Value, _args: &Args) -> VMResult {
    Ok(Value::bool(vm.context().block.is_some()))
}

fn isa(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    Ok(Value::bool(self_val.kind_of(args[0])))
}

fn dir(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let mut path = vm.get_source_path();
    if path.as_os_str().to_string_lossy() == "REPL" {
        return Ok(Value::string(conv_pathbuf(
            &std::env::current_dir().unwrap(),
        )));
    }
    path.pop();
    Ok(Value::string(conv_pathbuf(&path)))
}

fn file_(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let path = vm.get_source_path();
    Ok(Value::string(conv_pathbuf(&path)))
}

/// raise -> ()
/// fail -> ()
/// raise(message, cause: $!) -> ()
/// fail(message, cause: $!) -> ()
/// raise(error_type, message = nil, backtrace = caller(0), cause: $!) -> ()
/// fail(error_type, message = nil, backtrace = caller(0), cause: $!) -> ()
fn raise(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 2)?;
    match args.len() {
        0 => Err(RubyError::none("")),
        1 => {
            if let Some(s) = args[0].as_string() {
                Err(RubyError::none(s))
            } else if args[0].is_class() {
                if args[0].is_exception_class() {
                    let method = args[0].get_method_or_nomethod(IdentId::NEW)?;
                    let val = vm.eval_method(method, args[0], &Args::new0())?;
                    Err(RubyError::value(val))
                } else {
                    Err(RubyError::typeerr("Exception class/object expected."))
                }
            } else if args[0].if_exception().is_some() {
                Err(RubyError::value(args[0]))
            } else {
                Err(RubyError::typeerr("Exception class/object expected."))
            }
        }
        _ => Err(RubyError::none(args[1].clone().expect_string("2nd arg")?)),
    }
}

fn rand_(_vm: &mut VM, _: Value, _args: &Args) -> VMResult {
    let num = rand::random();
    Ok(Value::float(num))
}

fn loop_(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let block = args.expect_block()?;
    let arg = Args::new0();
    loop {
        let res = vm.eval_block(&block, &arg);
        match res {
            Ok(_) => {}
            Err(err) => match &err.kind {
                RubyErrorKind::RuntimeErr {
                    kind: RuntimeErrKind::StopIteration,
                    ..
                } => return Ok(Value::nil()),
                RubyErrorKind::Value(val) if val.get_class_name() == "StopIteration" => {
                    return Ok(Value::nil())
                }
                _ => return Err(err),
            },
        }
    }
}

fn exit(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let code = if args.len() == 0 {
        0
    } else {
        args[0].expect_integer("Expect Integer.")?
    };
    std::process::exit(code as i32);
}

fn abort(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let msg = if args.len() == 0 {
        "".to_string()
    } else {
        let mut msg = args[0];
        msg.expect_string("1st")?.to_owned()
    };
    eprintln!("{}", msg);
    std::process::exit(1);
}

fn sleep(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let secs = if args.len() == 0 {
        0.0
    } else {
        let secs = match args[0].unpack() {
            RV::Integer(i) => i as f64,
            RV::Float(f) => f,
            _ => return Err(RubyError::argument("Arg must be Integer or Float.")),
        };
        if secs < 0.0 {
            return Err(RubyError::argument("Negative number."));
        }
        secs
    };
    let start = std::time::Instant::now();
    std::thread::sleep(std::time::Duration::from_secs_f64(secs));
    let duration = start.elapsed().as_secs() as u64 as i64;
    Ok(Value::integer(duration))
}

fn proc(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let block = args.expect_block()?;
    let procobj = vm.create_proc(block)?;
    Ok(procobj)
}

fn lambda(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let block = args.expect_block()?;
    let procobj = vm.create_lambda(block)?;
    Ok(procobj)
}

fn kernel_integer(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let val = match args[0].unpack() {
        RV::Integer(num) => num,
        RV::Float(num) => num as i64,
        RV::Object(obj) => match &obj.kind {
            ObjKind::String(s) => match s.parse::<i64>() {
                Some(num) => num,
                None => {
                    let inspect = vm.val_inspect(args[0])?;
                    return Err(RubyError::typeerr(format!(
                        "Invalid value for Integer(): {}",
                        inspect
                    )));
                }
            },
            _ => {
                return Err(RubyError::no_implicit_conv(args[0], "Integer"));
            }
        },
        _ => {
            return Err(RubyError::no_implicit_conv(args[0], "Integer"));
        }
    };
    Ok(Value::integer(val))
}

fn kernel_complex(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 3)?;
    let (r, i, ex) = match args.len() {
        1 => (args[0], Value::integer(0), true),
        2 => (args[0], args[1], true),
        3 => (args[0], args[1], args[2].to_bool()),
        _ => unreachable!(),
    };
    if !r.is_real() || !i.is_real() {
        if ex {
            return Err(RubyError::argument("Not a real."));
        } else {
            return Ok(Value::nil());
        }
    }

    Ok(Value::complex(r, i))
}

/// Array(arg) -> Array
fn kernel_array(vm: &mut VM, _self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let arg = args[0];
    let arg_class = arg.get_class_for_method();
    match MethodRepo::find_method(arg_class, IdentId::get_id("to_a")) {
        Some(method) => return vm.eval_method(method, arg, &Args::new0()),
        None => {}
    };
    match MethodRepo::find_method(arg_class, IdentId::get_id("to_ary")) {
        Some(method) => return vm.eval_method(method, arg, &Args::new0()),
        None => {}
    };
    let res = Value::array_from(vec![arg]);
    Ok(res)
}

fn at_exit(_vm: &mut VM, _self_val: Value, _args: &Args) -> VMResult {
    Ok(_self_val)
}

/// TODO: Can not handle command args including ' or ".
fn command(_: &mut VM, _: Value, args: &Args) -> VMResult {
    use std::process::Command;
    args.check_args_num(1)?;
    let mut arg = args[0];
    let input = if cfg!(windows) {
        format!("cmd /C {}", arg.expect_string("Arg")?)
    } else {
        arg.expect_string("Arg")?.to_string()
    };
    let mut input = input.split_ascii_whitespace();
    let command = input.next().unwrap();
    let output = match Command::new(command.clone()).args(input).output() {
        Ok(ok) => ok,
        Err(err) => {
            return Err(RubyError::runtime(format!(
                "Command failed. {} {:?}",
                command,
                err.kind()
            )))
        }
    };
    Ok(Value::bytes(output.stdout))
}

fn eval(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    let mut args = args.clone();
    args.check_args_range(1, 4)?;
    let mut arg0 = args[0];
    let program = arg0.expect_string("1st arg")?;
    if args.len() > 1 {
        if !args[1].is_nil() {
            return Err(RubyError::argument("Currently, 2nd arg must be Nil."));
        }
    }
    let path = if args.len() > 2 {
        args[2].expect_string("3rd arg")?
    } else {
        "(eval)"
    };

    let method = vm.parse_program_eval(path, program)?;
    let args = Args::new0();
    let block = vm.new_block(method);
    let res = vm.eval_block(&block, &args)?;
    Ok(res)
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
        assert_error { assert_error }
        require "#{Dir.pwd}/tests/kernel_test"
        require_relative "../../tests/kernel_test"
        load "#{Dir.pwd}/tests/kernel_test.rb"
        assert_error { require 100 }
        assert_error { require "kernel_test" }
        assert_error { require_relative 100 }
        assert_error { require_relative "kernel_test" }
        assert_error { load 100 }
        assert_error { load "kernel_test" }
        assert_error { assert rand, rand }
        sleep(0.1)
        print "Ruby"
        print 3
        puts
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
    fn kernel_complex() {
        let program = r#"
        assert(Complex.rect(5.2, -99), Complex(5.2, -99))
        assert(Complex.rect(5.2, -99), Complex(5.2, -99, true))
        assert_error { Complex("s","k",true) }
        assert nil, Complex("s","k",false) 
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
        #[cfg(not(windows))]
        let program = r#"
        assert("Cargo.toml\n", `ls Cargo.toml`)
        a = "toml"
        assert("Cargo.toml\n", `ls Cargo.#{a}`)
        assert_error { `wooo` }
        "#;
        #[cfg(windows)]
        let program = r#"
        assert("Cargo.toml\r\n", `dir /B Cargo.toml`)
        a = "toml"
        assert("Cargo.toml\r\n", `dir /B Cargo.#{a}`)
        #assert_error { `wooo` }
        "#;
        assert_script(program);
    }

    #[test]
    fn kernel_eval() {
        let program = r#"
        a = 100
        eval("b = 100; assert(100, b);")
        assert(77, eval("a = 77"))
        assert(77, a)
        "#;
        assert_script(program);
    }

    #[test]
    fn kernel_eval2() {
        let program = r#"
        n = 2
        assert("n", %w{n}*"")
        assert(2, eval(%w{n}*""))
        assert("eval(%w{n}*\"\")", %q{eval(%w{n}*"")})
        assert(2, eval(%q{eval(%w{n}*"")}))
        "#;
        assert_script(program);
    }
}
