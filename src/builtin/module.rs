use crate::*;
use std::path::PathBuf;

pub fn init(globals: &mut Globals) {
    let class = globals.module_class;
    globals.add_builtin_instance_method(class, "constants", constants);
    globals.add_builtin_instance_method(class, "instance_methods", instance_methods);
    globals.add_builtin_instance_method(class, "attr_accessor", attr_accessor);
    globals.add_builtin_instance_method(class, "attr", attr_reader);
    globals.add_builtin_instance_method(class, "attr_reader", attr_reader);
    globals.add_builtin_instance_method(class, "attr_writer", attr_writer);
    globals.add_builtin_instance_method(class, "module_function", module_function);
    globals.add_builtin_instance_method(class, "singleton_class?", singleton_class);
    globals.add_builtin_instance_method(class, "const_get", const_get);
    globals.add_builtin_instance_method(class, "include", include);
    globals.add_builtin_instance_method(class, "included_modules", included_modules);
    globals.add_builtin_instance_method(class, "ancestors", ancestors);
    globals.add_builtin_instance_method(class, "module_eval", module_eval);
    globals.add_builtin_instance_method(class, "class_eval", module_eval);
}

fn constants(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let mut v: Vec<Value> = vec![];
    let mut class = self_val;
    loop {
        match &mut class.rvalue().var_table() {
            Some(table) => v.append(
                &mut table
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
            ),
            None => {}
        };
        match class.superclass() {
            Some(superclass) => {
                if superclass == vm.globals.builtins.object {
                    break;
                } else {
                    class = superclass
                };
            }
            None => break,
        }
    }
    Ok(Value::array_from(&vm.globals, v))
}

fn const_get(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let name = match args[0].as_symbol() {
        Some(symbol) => symbol,
        None => return Err(vm.error_type("1st arg must be Symbol.")),
    };
    let val = vm.get_super_const(self_val, name)?;
    Ok(val)
}

fn instance_methods(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let mut class = vm.expect_module(self_val)?;
    vm.check_args_range(args.len(), 0, 1)?;
    let inherited_too = args.len() == 0 || vm.val_to_bool(args[0]);
    match inherited_too {
        false => {
            let v = class
                .method_table
                .keys()
                .map(|k| Value::symbol(*k))
                .collect();
            Ok(Value::array_from(&vm.globals, v))
        }
        true => {
            let mut v = std::collections::HashSet::new();
            loop {
                v = v
                    .union(
                        &class
                            .method_table
                            .keys()
                            .map(|k| Value::symbol(*k))
                            .collect(),
                    )
                    .cloned()
                    .collect();
                match class.superclass() {
                    Some(superclass) => class = superclass,
                    None => break,
                };
            }
            Ok(Value::array_from(&vm.globals, v.iter().cloned().collect()))
        }
    }
}

pub fn attr_accessor(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    for arg in args.iter() {
        if arg.is_packed_symbol() {
            let id = arg.as_packed_symbol();
            define_reader(vm, self_val, id);
            define_writer(vm, self_val, id);
        } else {
            return Err(vm.error_name("Each of args for attr_accessor must be a symbol."));
        }
    }
    Ok(Value::nil())
}

fn attr_reader(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    for arg in args.iter() {
        if arg.is_packed_symbol() {
            let id = arg.as_packed_symbol();
            define_reader(vm, self_val, id);
        } else {
            return Err(vm.error_name("Each of args for attr_accessor must be a symbol."));
        }
    }
    Ok(Value::nil())
}

fn attr_writer(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    for arg in args.iter() {
        if arg.is_packed_symbol() {
            let id = arg.as_packed_symbol();
            define_writer(vm, self_val, id);
        } else {
            return Err(vm.error_name("Each of args for attr_accessor must be a symbol."));
        }
    }
    Ok(Value::nil())
}

fn define_reader(vm: &mut VM, class: Value, id: IdentId) {
    let instance_var_id = get_instance_var(vm, id);
    let info = MethodInfo::AttrReader {
        id: instance_var_id,
    };
    let methodref = vm.globals.add_method(info);
    vm.add_instance_method(class, id, methodref);
}

fn define_writer(vm: &mut VM, class: Value, id: IdentId) {
    let instance_var_id = get_instance_var(vm, id);
    let assign_id = IdentId::add_postfix(id, "=");
    let info = MethodInfo::AttrWriter {
        id: instance_var_id,
    };
    let methodref = vm.globals.add_method(info);
    vm.add_instance_method(class, assign_id, methodref);
}

fn get_instance_var(_vm: &VM, id: IdentId) -> IdentId {
    let s = IdentId::get_ident_name(id);
    IdentId::get_id(format!("@{}", s))
}

fn module_function(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    vm.module_function(true);
    Ok(Value::nil())
}

fn singleton_class(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let class = vm.expect_module(self_val)?;
    Ok(Value::bool(class.is_singleton))
}

fn include(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 1)?;
    let mut class = vm.expect_module(self_val)?;
    let module = args[0];
    class.include.push(module);
    Ok(Value::nil())
}

fn included_modules(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let mut class = self_val;
    let mut ary = vec![];
    loop {
        if class.is_nil() {
            break;
        }
        class = match class.as_module() {
            Some(cref) => {
                for included in &cref.include {
                    ary.push(*included);
                }
                cref.superclass
            }
            None => {
                let inspect = vm.val_inspect(class);
                return Err(
                    vm.error_internal(format!("Illegal value in superclass chain. {}", inspect))
                );
            }
        };
    }
    Ok(Value::array_from(&vm.globals, ary))
}

fn ancestors(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    vm.check_args_num(args.len(), 0)?;
    let mut superclass = self_val;
    let mut ary = vec![];
    loop {
        if superclass.is_nil() {
            break;
        }
        ary.push(superclass);
        superclass = match superclass.as_module() {
            Some(cref) => {
                for included in &cref.include {
                    ary.push(*included);
                }
                cref.superclass
            }
            None => {
                let inspect = vm.val_inspect(superclass);
                return Err(
                    vm.error_internal(format!("Illegal value in superclass chain. {}", inspect))
                );
            }
        };
    }
    Ok(Value::array_from(&vm.globals, ary))
}

fn module_eval(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let method = match args.block {
        Some(method) => {
            vm.check_args_num(args.len(), 0)?;
            method
        }
        None => {
            vm.check_args_num(args.len(), 1)?;
            let mut arg0 = args[0];
            let program = arg0.expect_string(vm, "1st arg")?;
            vm.parse_program_eval(PathBuf::from("(eval)"), program)?
        }
    };
    let args = Args::new0();
    let context = vm.current_context();
    vm.class_push(self_val);
    let res = vm.eval_method(method, self_val, Some(context), &args);
    vm.class_pop();
    res
}

#[cfg(test)]
mod test {
    use crate::test::*;

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
    assert(true, ary_cmp(Foo.constants, [:Bar, :Ker]))
    assert(true, ary_cmp(Bar.constants, [:Doo, :Bar, :Ker]))
    "#;
        assert_script(program);
    }

    #[test]
    fn attr_accessor() {
        let program = r#"
    class Foo
        attr_accessor :car, :cdr
        attr_reader :bar
        attr_writer :boo
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
    "#;
        assert_script(program);
    }

    #[test]
    fn ancestors() {
        let program = r#"
        assert([Class, Module, Object, Kernel], Class.ancestors)
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
        # assert(111, C.module_eval { D })
        "##;
        assert_script(program);
    }
}
