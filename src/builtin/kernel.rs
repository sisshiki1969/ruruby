use crate::*;
use std::path::PathBuf;

pub(crate) fn init(globals: &mut Globals) -> Module {
    let class = Module::module();
    BuiltinClass::set_toplevel_constant("Kernel", class);
    class.add_builtin_module_func(globals, "puts", puts);
    class.add_builtin_module_func(globals, "p", p);
    class.add_builtin_module_func(globals, "print", print);
    class.add_builtin_module_func(globals, "assert", assert);
    class.add_builtin_module_func(globals, "assert_error", assert_error);
    class.add_builtin_module_func(globals, "require", require);
    class.add_builtin_module_func(globals, "require_relative", require_relative);
    class.add_builtin_module_func(globals, "load", load);
    class.add_builtin_module_func(globals, "block_given?", block_given);
    class.add_builtin_module_func(globals, "is_a?", isa);
    class.add_builtin_module_func(globals, "kind_of?", isa);
    class.add_builtin_module_func(globals, "__dir__", dir);
    class.add_builtin_module_func(globals, "raise", raise);
    class.add_builtin_module_func(globals, "rand", rand_);
    class.add_builtin_module_func(globals, "loop", loop_);
    class.add_builtin_module_func(globals, "exit", exit);
    class.add_builtin_module_func(globals, "abort", abort);
    class.add_builtin_module_func(globals, "sleep", sleep);
    class.add_builtin_module_func(globals, "proc", proc);
    class.add_builtin_module_func(globals, "lambda", lambda);
    class.add_builtin_module_func(globals, "Integer", kernel_integer);
    class.add_builtin_module_func(globals, "Complex", kernel_complex);
    class.add_builtin_module_func(globals, "Array", kernel_array);
    class.add_builtin_module_func(globals, "at_exit", at_exit);
    class.add_builtin_module_func(globals, "`", command);
    class.add_builtin_module_func(globals, "eval", eval);
    class.add_builtin_module_func(globals, "binding", binding);
    class
}
/// Built-in function "puts".
fn puts(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
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
    for i in 0..vm.args().len() {
        flatten(vm, vm[i])?;
    }
    Ok(Value::nil())
}

fn p(vm: &mut VM, _: Value, _: &Args2) -> VMResult {
    for i in 0..vm.args_len() {
        println!("{}", vm.val_inspect(vm[i])?);
    }
    match vm.args_len() {
        0 => Ok(Value::nil()),
        1 => Ok(vm[0]),
        _ => Ok(Value::array_from(vm.args().to_vec())),
    }
}

/// Built-in function "print".
fn print(vm: &mut VM, _: Value, _args: &Args2) -> VMResult {
    for i in 0..vm.args().len() {
        let arg = vm[i];
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
fn assert(vm: &mut VM, _: Value, _: &Args2) -> VMResult {
    vm.check_args_num(2)?;
    if !vm.eval_eq2(vm[0], vm[1])? {
        let res = format!("Assertion error: Expected: {:?} Actual: {:?}", vm[0], vm[1],);
        Err(RubyError::argument(res))
    } else {
        println!("Assert OK: {:?}", vm[0]);
        Ok(Value::nil())
    }
}

fn assert_error(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let method = args.expect_block()?;
    match vm.eval_block0(method) {
        Ok(val) => Err(RubyError::argument(format!(
            "Assertion error: No error occured. returned {:?}",
            val
        ))),
        Err(err) => {
            println!("Assert_error OK:");
            vm.show_err(&err);
            err.show_loc(0);
            Ok(Value::nil())
        }
    }
}

fn require(vm: &mut VM, _: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let arg0 = vm[0];
    let file_name = match arg0.as_string() {
        Some(string) => string,
        None => return Err(RubyError::argument("file name must be a string.")),
    };
    Ok(Value::bool(vm.require(file_name)?))
}

fn require_relative(vm: &mut VM, _: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let mut path = vm.caller_iseq().source_info.path.clone();
    let file_name = match vm[0].as_string() {
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
    Ok(Value::bool(vm.load_exec(&path, false)?))
}

fn load(vm: &mut VM, _: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let file_name = match vm[0].as_string() {
        Some(string) => string,
        None => return Err(RubyError::argument("file name must be a string.")),
    };
    let path = PathBuf::from(file_name);
    if path.exists() {
        vm.load_exec(&path, true)?;
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
            vm.load_exec(&base_path, true)?;
            return Ok(Value::true_val());
        }
    }
    Err(RubyError::load(format!(
        "Can not load such file -- {:?}",
        file_name
    )))
}

/// Built-in function "block_given?".
fn block_given(vm: &mut VM, _: Value, _args: &Args2) -> VMResult {
    let block = vm.caller_method_block();
    Ok(Value::bool(block.is_some()))
}

fn isa(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    Ok(Value::bool(self_val.kind_of(vm[0])))
}

fn dir(vm: &mut VM, _: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let mut path = vm.caller_iseq().source_info.path.clone();
    if path.as_os_str().to_string_lossy() == "REPL" {
        return Ok(Value::string(conv_pathbuf(
            &std::env::current_dir().unwrap(),
        )));
    }
    path.pop();
    Ok(Value::string(conv_pathbuf(&path)))
}

/// raise -> ()
/// fail -> ()
/// raise(message, cause: $!) -> ()
/// fail(message, cause: $!) -> ()
/// raise(error_type, message = nil, backtrace = caller(0), cause: $!) -> ()
/// fail(error_type, message = nil, backtrace = caller(0), cause: $!) -> ()
fn raise(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    vm.check_args_range(0, 2)?;
    match args.len() {
        0 => Err(RubyError::none("")),
        1 => {
            let arg0 = vm[0];
            if let Some(s) = arg0.as_string() {
                Err(RubyError::none(s))
            } else if arg0.is_class() {
                if arg0.is_exception_class() {
                    let method = arg0.get_method_or_nomethod(&mut vm.globals, IdentId::NEW)?;
                    vm.globals.val = vm.eval_method0(method, arg0)?;
                    Err(RubyError::value())
                } else {
                    Err(RubyError::typeerr("Exception class/object expected."))
                }
            } else if arg0.if_exception().is_some() {
                vm.globals.val = arg0;
                Err(RubyError::value())
            } else {
                Err(RubyError::typeerr("Exception class/object expected."))
            }
        }
        _ => Err(RubyError::none(vm[1].clone().expect_string("2nd arg")?)),
    }
}

/// rand(max = 0) -> Integer | Float
/// rand(range) -> Integer | Float | nil
/// https://docs.ruby-lang.org/ja/latest/method/Kernel/m/rand.html
fn rand_(vm: &mut VM, _: Value, _: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let num = rand::random();
    Ok(Value::float(num))
}

fn loop_(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    let block = args.expect_block()?;
    loop {
        match vm.eval_block0(&block) {
            Ok(_) => {}
            Err(err) => match &err.kind {
                RubyErrorKind::BlockReturn => return Ok(vm.globals.val),
                RubyErrorKind::RuntimeErr {
                    kind: RuntimeErrKind::StopIteration,
                    ..
                } => return Ok(Value::nil()),
                RubyErrorKind::Exception if vm.globals.val.get_class_name() == "StopIteration" => {
                    return Ok(Value::nil())
                }
                _ => return Err(err),
            },
        }
    }
}

fn exit(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    vm.check_args_range(0, 1)?;
    let code = if args.len() == 0 {
        0
    } else {
        vm[0].coerce_to_fixnum("Expect Integer.")?
    };
    std::process::exit(code as i32);
}

fn abort(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    vm.check_args_range(0, 1)?;
    if args.len() != 0 {
        let mut msg = vm[0];
        eprintln!("{}", msg.expect_string("1st")?);
    };
    std::process::exit(1);
}

fn sleep(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    vm.check_args_range(0, 1)?;
    let secs = if args.len() == 0 {
        0.0
    } else {
        let secs = match vm[0].unpack() {
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

fn proc(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let block = args.expect_block()?;
    Ok(vm.create_proc(block))
}

fn lambda(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    vm.check_args_num(0)?;
    let block = args.expect_block()?;
    vm.create_lambda(block)
}

fn kernel_integer(vm: &mut VM, _: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let arg0 = vm[0];
    let val = match arg0.unpack() {
        RV::Integer(num) => num,
        RV::Float(num) => num as i64,
        RV::Object(obj) => match &obj.kind {
            ObjKind::String(s) => match s.parse::<i64>() {
                Some(num) => num,
                None => {
                    let inspect = vm.val_inspect(arg0)?;
                    return Err(RubyError::argument(format!(
                        "Invalid value for Integer(): {}",
                        inspect
                    )));
                }
            },
            _ => {
                return Err(RubyError::no_implicit_conv(arg0, "Integer"));
            }
        },
        _ => {
            return Err(RubyError::no_implicit_conv(arg0, "Integer"));
        }
    };
    Ok(Value::integer(val))
}

fn kernel_complex(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    vm.check_args_range(1, 3)?;
    let arg0 = vm[0];
    let (r, i, ex) = match args.len() {
        1 => (arg0, Value::integer(0), true),
        2 => (arg0, vm[1], true),
        3 => (arg0, vm[1], vm[2].to_bool()),
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
fn kernel_array(vm: &mut VM, _self_val: Value, _: &Args2) -> VMResult {
    vm.check_args_num(1)?;
    let arg = vm[0];
    let arg_class = arg.get_class_for_method();
    match vm
        .globals
        .methods
        .find_method(arg_class, IdentId::get_id("to_a"))
    {
        Some(method) => return vm.eval_method0(method, arg),
        None => {}
    };
    match vm
        .globals
        .methods
        .find_method(arg_class, IdentId::get_id("to_ary"))
    {
        Some(method) => return vm.eval_method0(method, arg),
        None => {}
    };
    let res = Value::array_from(vec![arg]);
    Ok(res)
}

fn at_exit(_vm: &mut VM, _self_val: Value, _args: &Args2) -> VMResult {
    Ok(_self_val)
}

fn command(vm: &mut VM, _: Value, _: &Args2) -> VMResult {
    use std::process::Command;
    vm.check_args_num(1)?;
    let mut arg = vm[0];
    let opt = if cfg!(windows) { "/C" } else { "-c" };
    let input = arg.expect_string("Arg")?;
    let command = if cfg!(windows) { "cmd" } else { "sh" };
    let output = match Command::new(command).args(&[opt, input]).output() {
        Ok(ok) => ok,
        Err(err) => {
            return Err(RubyError::runtime(format!(
                "Command failed. {} {:?}",
                command,
                err.kind()
            )))
        }
    };
    let status = output.status;
    if status.success() {
        Ok(Value::bytes(output.stdout))
    } else {
        let err = format!(
            "{} exit status:{:?}",
            String::from_utf8_lossy(&output.stderr),
            status.code()
        );
        Err(RubyError::runtime(err))
    }
}

fn eval(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    vm.check_args_range(1, 4)?;
    let mut arg0 = vm[0];
    let program = arg0.expect_string("1st arg")?.to_string();
    let path = if args.len() > 2 {
        let mut arg2 = vm[2];
        arg2.expect_string("3rd arg")?.to_string()
    } else {
        "(eval)".to_string()
    };

    if args.len() == 1 || vm[1].is_nil() {
        let method = vm.parse_program_eval(path, program)?;
        let p = vm.create_proc_from_block(method, vm.cur_outer_frame());
        vm.eval_block0(&p.into())
    } else {
        let ctx = vm[1].expect_binding("2nd arg must be Binding.")?;
        vm.eval_binding(path, program, ctx)
    }
}

fn binding(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    let ctx = vm.create_block_context(MethodId::default(), vm.cur_outer_frame());
    Ok(Value::binding(ctx))
}

#[cfg(test)]
mod test {
    use crate::tests::*;

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

        def bar
          1.times { return block_given? }
        end

        assert true, foo {}
        assert false, foo
        assert true, bar {}
        assert false, bar
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

    #[cfg(unix)]
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

    #[cfg(windows)]
    #[test]
    fn kernel_etc() {
        let program = r###"
        assert_error { assert 2, 3 }
        assert_error { assert_error { true } }
        assert_error { raise }
        assert_error { assert_error }
        require "#{Dir.pwd}/tests/kernel_test_win"
        require_relative "../../tests/kernel_test_win"
        load "#{Dir.pwd}/tests/kernel_test_win.rb"
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
    fn kernel_loop1() {
        let program = r#"
        a = loop do break end
        assert nil, a
        a = loop do break 42 end
        assert 42, a
        a = loop do
          raise StopIteration
        end
        assert nil, a
        "#;
        assert_script(program);
    }

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
