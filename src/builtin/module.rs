use crate::*;
use std::path::PathBuf;

pub fn init(globals: &mut Globals) {
    let module_class = globals.builtins.module.as_mut_class();
    module_class.add_builtin_method_by_str("===", teq);
    module_class.add_builtin_method_by_str("constants", constants);
    module_class.add_builtin_method_by_str("class_variables", class_variables);
    module_class.add_builtin_method_by_str("const_defined?", const_defined);
    module_class.add_builtin_method_by_str("instance_methods", instance_methods);
    module_class.add_builtin_method_by_str("attr_accessor", attr_accessor);
    module_class.add_builtin_method_by_str("attr", attr_reader);
    module_class.add_builtin_method_by_str("attr_reader", attr_reader);
    module_class.add_builtin_method_by_str("attr_writer", attr_writer);
    module_class.add_builtin_method_by_str("module_function", module_function);
    module_class.add_builtin_method_by_str("singleton_class?", singleton_class);
    module_class.add_builtin_method_by_str("const_get", const_get);
    module_class.add_builtin_method_by_str("include", include);
    module_class.add_builtin_method_by_str("prepend", prepend);
    module_class.add_builtin_method_by_str("included_modules", included_modules);
    module_class.add_builtin_method_by_str("ancestors", ancestors);
    module_class.add_builtin_method_by_str("module_eval", module_eval);
    module_class.add_builtin_method_by_str("class_eval", module_eval);
    module_class.add_builtin_method_by_str("alias_method", module_alias_method);
    module_class.add_builtin_method_by_str("public", public);
    module_class.add_builtin_method_by_str("private", private);
    module_class.add_builtin_method_by_str("protected", protected);
}

fn teq(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let mut module = args[0].get_class();
    loop {
        let minfo = match module.if_mod_class() {
            Some(info) => info,
            None => return Err(RubyError::argument("Must be module or class.")),
        };
        let true_module = if minfo.is_included() {
            minfo.origin()
        } else {
            module
        };
        if true_module.id() == self_val.id() {
            return Ok(Value::true_val());
        };
        match module.upper() {
            Some(upper) => module = upper,
            None => break,
        }
    }
    Ok(Value::false_val())
}

pub fn set_attr_accessor(globals: &mut Globals, self_val: Value, args: &Args) -> VMResult {
    for arg in args.iter() {
        if arg.is_packed_symbol() {
            let id = arg.as_packed_symbol();
            define_reader(globals, self_val, id);
            define_writer(globals, self_val, id);
        } else {
            return Err(RubyError::name(
                "Each of args for attr_accessor must be a symbol.",
            ));
        }
    }
    Ok(Value::nil())
}

fn constants(_vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let mut v: Vec<Value> = vec![];
    let mut class = self_val;
    loop {
        v.append(
            &mut class
                .as_module()
                .const_table()
                .keys()
                .filter(|x| {
                    IdentId::get_ident_name(**x)
                        .chars()
                        .nth(0)
                        .unwrap()
                        .is_ascii_uppercase()
                })
                .map(|k| Value::symbol(*k))
                .collect::<Vec<Value>>(),
        );
        match class.upper() {
            Some(superclass) => {
                if superclass == BuiltinClass::object() {
                    break;
                } else {
                    class = superclass
                };
            }
            None => break,
        }
    }
    Ok(Value::array_from(v))
}

fn class_variables(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let inherit = args[0].to_bool();
    assert_eq!(inherit, false);
    let receiver = self_val.rvalue();
    let res = match receiver.var_table() {
        Some(table) => table
            .keys()
            .filter(|x| IdentId::starts_with(**x, "@@"))
            .map(|x| Value::symbol(*x))
            .collect(),
        None => vec![],
    };
    Ok(Value::array_from(res))
}

fn const_defined(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 2)?;
    let name = args[0].expect_string_or_symbol("1st arg")?;
    Ok(Value::bool(VM::get_super_const(self_val, name).is_ok()))
}

fn const_get(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let name = match args[0].as_symbol() {
        Some(symbol) => symbol,
        None => return Err(RubyError::typeerr("1st arg must be Symbol.")),
    };
    let val = VM::get_super_const(self_val, name)?;
    Ok(val)
}

fn instance_methods(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let mut module = self_val.expect_mod_class(vm)?;
    let inherited_too = args.len() == 0 || args[0].to_bool();
    match inherited_too {
        false => {
            let v = module
                .method_table()
                .keys()
                .map(|k| Value::symbol(*k))
                .collect();
            Ok(Value::array_from(v))
        }
        true => {
            let mut v = std::collections::HashSet::new();
            loop {
                v = v
                    .union(
                        &module
                            .method_table()
                            .keys()
                            .map(|k| Value::symbol(*k))
                            .collect(),
                    )
                    .cloned()
                    .collect();
                if module.upper().is_nil() {
                    break;
                } else {
                    self_val = module.upper();
                    module = self_val.as_mut_module();
                }
            }
            Ok(Value::array_from(v.iter().cloned().collect()))
        }
    }
}

fn attr_accessor(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    set_attr_accessor(&mut vm.globals, self_val, args)
}

fn attr_reader(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    for arg in args.iter() {
        if arg.is_packed_symbol() {
            let id = arg.as_packed_symbol();
            define_reader(&mut vm.globals, self_val, id);
        } else {
            return Err(RubyError::name(
                "Each of args for attr_accessor must be a symbol.",
            ));
        }
    }
    Ok(Value::nil())
}

fn attr_writer(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    for arg in args.iter() {
        if arg.is_packed_symbol() {
            let id = arg.as_packed_symbol();
            define_writer(&mut vm.globals, self_val, id);
        } else {
            return Err(RubyError::name(
                "Each of args for attr_accessor must be a symbol.",
            ));
        }
    }
    Ok(Value::nil())
}

fn define_reader(globals: &mut Globals, mut class: Value, id: IdentId) {
    let instance_var_id = IdentId::add_prefix(id, "@");
    let info = MethodInfo::AttrReader {
        id: instance_var_id,
    };
    let methodref = MethodRef::new(info);
    class
        .if_mut_mod_class()
        .unwrap()
        .add_method(globals, id, methodref);
}

fn define_writer(globals: &mut Globals, mut class: Value, id: IdentId) {
    let instance_var_id = IdentId::add_prefix(id, "@");
    let assign_id = IdentId::add_postfix(id, "=");
    let info = MethodInfo::AttrWriter {
        id: instance_var_id,
    };
    let methodref = MethodRef::new(info);
    class
        .if_mut_mod_class()
        .unwrap()
        .add_method(globals, assign_id, methodref);
}

fn module_function(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    if args.len() == 0 {
        vm.module_function(true);
    } else {
        let mut class = vm.class();
        let mut singleton = class.get_singleton_class().unwrap();
        let classinfo = singleton.as_mut_class();
        for arg in args.iter() {
            let name = arg.expect_string_or_symbol("Args")?;
            let method = vm.get_method(class.clone(), name)?;
            classinfo.add_method(&mut vm.globals, name, method);
        }
    }
    Ok(Value::nil())
}

fn singleton_class(vm: &mut VM, mut self_val: Value, _: &Args) -> VMResult {
    let class = self_val.expect_mod_class(vm)?;
    Ok(Value::bool(class.is_singleton()))
}

fn include(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let cinfo = self_val.expect_mod_class(vm)?;
    let mut module = args[0];
    module.expect_module(vm, "1st arg")?;
    cinfo.append_include(module, &mut vm.globals);
    Ok(Value::nil())
}

fn prepend(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let self_val2 = self_val.clone();
    let cinfo = self_val.expect_mod_class(vm)?;
    let mut module = args[0];
    module.expect_module(vm, "1st arg")?;
    cinfo.append_prepend(self_val2, module, &mut vm.globals);
    Ok(Value::nil())
}

fn included_modules(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let mut module = self_val;
    let mut ary = vec![];
    loop {
        if module.is_nil() {
            break;
        }
        let cinfo = module.as_module();
        if cinfo.is_included() {
            ary.push(cinfo.origin())
        };
        module = cinfo.upper();
    }
    Ok(Value::array_from(ary))
}

fn ancestors(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let mut module = self_val;
    let mut ary = vec![];
    loop {
        if module.is_nil() {
            break;
        }
        let cinfo = module.as_module();
        if cinfo.is_included() {
            ary.push(cinfo.origin())
        } else {
            ary.push(module)
        };
        let cinfo = module.as_module();
        module = cinfo.upper();
    }
    Ok(Value::array_from(ary))
}

fn module_eval(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let context = vm.current_context();
    match &args.block {
        Block::None => {
            args.check_args_num(1)?;
            let mut arg0 = args[0];
            let program = arg0.expect_string("1st arg")?;
            let method = vm.parse_program_eval(PathBuf::from("(eval)"), program)?;
            let args = Args::new0();
            // The scopes of constants and class variables are same as module definition of `self_val`.
            vm.class_push(self_val);
            let res = vm.invoke_method(method, self_val, Some(context), &args);
            vm.class_pop();
            res
        }
        block => {
            args.check_args_num(0)?;
            let args = Args::new0();
            // The scopes of constants and class variables are outer of the block.
            vm.eval_block_self(block, self_val, &args)
        }
    }
}

fn module_alias_method(vm: &mut VM, mut self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(2)?;
    let new = args[0].expect_string_or_symbol("1st arg")?;
    let org = args[1].expect_string_or_symbol("2nd arg")?;
    let method = vm.get_method(self_val, org)?;
    self_val
        .as_mut_class()
        .add_method(&mut vm.globals, new, method);
    Ok(self_val)
}

fn public(_vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    Ok(self_val)
}

fn private(_vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    Ok(self_val)
}

fn protected(_vm: &mut VM, self_val: Value, _args: &Args) -> VMResult {
    Ok(self_val)
}

#[cfg(test)]
mod test {
    use crate::test::*;

    #[test]
    fn module_op() {
        let program = r#"
        assert(true, Integer === 3)
        assert(false, Integer === "a")
        assert(false, Integer === [])
        assert(false, Array === 3)
        assert(false, Array === "a")
        assert(true, Array === [])

        module M1; end
        module M2; end
        class A; end
        class B < A
          include M1
        end
        class C < B
          include M2
        end

        c = C.new
        assert(true, C === c)
        assert(true, B === c)
        assert(true, A === c)
        assert(true, M1 === c)
        assert(true, M2 === c)
        assert(true, Object === c)
        assert(false, Integer === c)
        "#;
        assert_script(program);
    }

    #[test]
    fn module_visibility() {
        let program = r#"
        class A
            public
            private
            protected
        end
        "#;
        assert_script(program);
    }

    #[test]
    fn module_function() {
        let program = r#"
    class Foo
        module_function
        def bar
            123
        end
    end
    assert(123, Foo.bar)
    assert(123, Foo.new.bar)

    class Bar
        def foo
            456
        end
        def bar
            789
        end
        module_function :foo, "bar"
    end
    assert(456, Bar.new.foo)
    assert(789, Bar.new.bar)
    assert(456, Bar.foo)
    assert(789, Bar.bar)
    "#;
        assert_script(program);
    }

    #[test]
    fn constants() {
        let program = r#"
    class Foo
        Bar = 100
        Ker = 777
    end
    
    class Bar < Foo
        Doo = 555
    end
    
    def ary_cmp(a,b)
        return false if a - b != []
        return false if b - a != []
        true
    end

    assert(100, Foo.const_get(:Bar))
    assert(100, Bar.const_get(:Bar))
    assert_error { Bar.const_get([]) }
    assert(true, ary_cmp(Foo.constants, [:Bar, :Ker]))
    assert(true, ary_cmp(Bar.constants, [:Doo, :Bar, :Ker]))
    "#;
        assert_script(program);
    }

    #[test]
    fn class_variables() {
        let program = r##"
        class One
            @@var1 = 1
        end
        class Two < One
            @@var2 = 2
        end
        assert([:"@@var2"], Two.class_variables(false))
        "##;
        assert_script(program);
    }

    #[test]
    fn attr_accessor() {
        let program = r#"
    class Foo
        attr_accessor :car, :cdr
        attr_reader :bar
        attr_writer :boo
        assert_error { attr_accessor 100 }
        assert_error { attr_reader 100 }
        assert_error { attr_writer 100 }
        def set_bar(x)
            @bar = x
        end
        def get_boo
            @boo
        end
    end
    bar = Foo.new
    assert nil, bar.car
    assert nil, bar.cdr
    assert nil, bar.bar
    assert_error { bar.boo }
    bar.car = 1000
    bar.cdr = :something
    assert_error { bar.bar = 4.7 }
    bar.set_bar(9.55)
    bar.boo = "Ruby"
    assert 1000, bar.car
    assert :something, bar.cdr
    assert 9.55, bar.bar
    assert "Ruby", bar.get_boo
    "#;
        assert_script(program);
    }

    #[test]
    fn module_methods() {
        let program = r#"
    class A
        Foo = 100
        Bar = 200
        def fn
            puts "fn"
        end
        def fo
            puts "fo"
        end
    end
    def ary_cmp(a,b)
        puts a,b
        return false if a - b != []
        return false if b - a != []
        true
    end
    assert(true, ary_cmp(A.constants, [:Bar, :Foo]))
    assert(true, ary_cmp(A.instance_methods - Class.instance_methods, [:fn, :fo]))
    assert(true, ary_cmp(A.instance_methods(false), [:fn, :fo]))
    "#;
        assert_script(program);
    }

    #[test]
    fn ancestors() {
        let program = r#"
        assert([Class, Module, Object, Kernel, BasicObject], Class.ancestors)
        assert([Kernel], Object.included_modules)
        assert([Kernel], Class.included_modules)
        assert(true, Class.singleton_class.singleton_class?)
        "#;
        assert_script(program);
    }

    #[test]
    fn module_eval() {
        let program = r##"
        class C; D = 777; end;
        D = 111
        x = "bow"
        C.module_eval "def foo; \"#{x}\"; end"
        assert("bow", C.new.foo)
        assert(777, C.module_eval("D"))
        C.module_eval do
            x = "view"  # you can capture or manipulate local variables in outer scope of the block.
            def bar
                "mew"
            end
        end
        assert("mew", C.new.bar)
        assert("view", x)
        assert(111, C.module_eval { D })
        "##;
        assert_script(program);
    }

    #[test]
    fn alias_method() {
        let program = r##"
        class Foo
          def foo
            55
          end
          alias_method :bar1, :foo
          alias_method "bar2", :foo
          alias_method :bar3, "foo"
          alias_method "bar4", "foo"
          assert_error { alias_method 124, :foo }
          assert_error { alias_method :bar5, [] }
        end
        f = Foo.new
        assert(55, f.bar1)
        assert(55, f.bar2)
        assert(55, f.bar3)
        assert(55, f.bar4)
        "##;
        assert_script(program);
    }

    #[test]
    fn const_defined() {
        let program = r#"
        assert(true, Object.const_defined?(:Kernel))
        assert(false, Object.const_defined?(:Kernels))
        assert(true, Object.const_defined? "Array")
        assert(false, Object.const_defined? "Arrays")
        "#;
        assert_script(program);
    }

    #[test]
    fn include1() {
        let program = r#"
        class C; end
        module M1
          def f; "M1"; end
        end
        module M2
          def f; "M2"; end
        end
        class C
          include M1
        end
        assert "M1", C.new.f
        class C
          include M2
        end
        assert "M2", C.new.f
        "#;
        assert_script(program);
    }

    #[test]
    fn include2() {
        let program = r#"
    module M2; end

    module M1
      include M2
    end

    class S; end

    class C < S
      include M1
    end

    assert C, C.ancestors[0]
    assert M1, C.ancestors[1]
    assert M2, C.ancestors[2]
    assert S, C.ancestors[3]
        "#;
        assert_script(program);
    }

    #[test]
    fn prepend1() {
        let program = r#"
        module M0
          def f; "M0"; end
        end
        class S
          include M0
        end
        class C < S; end
        module M1
          def f; "M1"; end
        end
        module M2
          def f; "M2"; end
        end
        assert "M0", C.new.f
        class S
          def f; "S"; end
        end
        assert "S", C.new.f
        class S
          prepend M1
        end
        assert "M1", C.new.f
        class S
          prepend M2
        end
        assert "M2", C.new.f
        assert [C, M2, M1, S, M0, Object, Kernel, BasicObject], C.ancestors
        "#;
        assert_script(program);
    }
}
