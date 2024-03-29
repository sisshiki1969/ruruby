use crate::*;

pub(crate) fn init(globals: &mut Globals) {
    let class = BuiltinClass::module();
    globals.set_toplevel_constant("Module", class);
    class.add_builtin_class_method(globals, "new", module_new);
    class.add_builtin_class_method(globals, "constants", module_constants);

    class.add_builtin_method_by_str(globals, "===", teq);
    class.add_builtin_method_by_str(globals, "<=>", cmp);
    class.add_builtin_method_by_str(globals, "name", name);
    class.add_builtin_method_by_str(globals, "to_s", inspect);
    class.add_builtin_method_by_str(globals, "inspect", inspect);
    class.add_builtin_method_by_str(globals, "constants", constants);
    class.add_builtin_method_by_str(globals, "autoload", autoload);
    class.add_builtin_method_by_str(globals, "class_variables", class_variables);
    class.add_builtin_method_by_str(globals, "const_defined?", const_defined);
    class.add_builtin_method_by_str(globals, "instance_methods", instance_methods);
    class.add_builtin_method_by_str(globals, "instance_method", instance_method);
    class.add_builtin_method_by_str(globals, "method_defined?", method_defined);
    class.add_builtin_method_by_str(globals, "attr_accessor", attr_accessor);
    class.add_builtin_method_by_str(globals, "attr", attr_reader);
    class.add_builtin_method_by_str(globals, "attr_reader", attr_reader);
    class.add_builtin_method_by_str(globals, "attr_writer", attr_writer);
    class.add_builtin_method_by_str(globals, "module_function", module_function);
    class.add_builtin_method_by_str(globals, "singleton_class?", singleton_class);
    class.add_builtin_method_by_str(globals, "const_get", const_get);
    class.add_builtin_method_by_str(globals, "include", include);
    class.add_builtin_method_by_str(globals, "prepend", prepend);
    class.add_builtin_method_by_str(globals, "included_modules", included_modules);
    class.add_builtin_method_by_str(globals, "ancestors", ancestors);
    class.add_builtin_method_by_str(globals, "module_eval", module_eval);
    class.add_builtin_method_by_str(globals, "class_eval", module_eval);
    class.add_builtin_method_by_str(globals, "alias_method", module_alias_method);
    class.add_builtin_method_by_str(globals, "public", public);
    class.add_builtin_method_by_str(globals, "private", private);
    class.add_builtin_method_by_str(globals, "protected", protected);
    class.add_builtin_method_by_str(globals, "private_constant", private_constant);
    class.add_builtin_method_by_str(globals, "include?", include_);
    class.add_builtin_method_by_str(globals, "deprecate_constant", deprecate_constant);
    class.add_builtin_method_by_str(globals, "private_class_method", private_class_method);
}

/// ## singleton method Module.new
/// - new -> Module
/// - new {|mod| ... } -> Module
///
/// https://docs.ruby-lang.org/ja/latest/method/Module/s/new.html
fn module_new(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    let module = Module::module();
    let val = module.into();
    match &args.block {
        None => {}
        Some(block) => {
            let arg = Args::new1(val);
            let _ = vm.eval_block_self(block, val, &arg)?;
        }
    };
    Ok(val)
}

/// ## singleton method Module.constants
/// - constants -> [[Symbol]]
///
/// https://docs.ruby-lang.org/ja/latest/method/Module/s/constants.html
fn module_constants(vm: &mut VM, _: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    let v = vm
        .enumerate_const()
        .into_iter()
        .map(|id| Value::symbol(id))
        .collect();
    Ok(Value::array_from(v))
}

/// ## instance method Module#===
/// - self === obj -> bool
///
/// https://docs.ruby-lang.org/ja/latest/method/Module/i/=3d=3d=3d.html
fn teq(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(1)?;
    let class = vm.globals.get_class(vm[0]);
    let self_val = self_val.into_module();
    Ok(Value::bool(class.include_module(self_val)))
}

/// ## instance method Module#<=>
/// - self <=> other -> Integer | nil
///
/// https://docs.ruby-lang.org/ja/latest/method/Module/i/=3c=3d=3e.html
fn cmp(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(1)?;
    if self_val.id() == vm[0].id() {
        return Ok(Value::fixnum(0));
    }
    let self_val = self_val.into_module();
    let other = match vm[0].if_mod_class() {
        Some(m) => m,
        None => return Ok(Value::nil()),
    };
    if other.include_module(self_val) {
        Ok(Value::fixnum(1))
    } else if self_val.include_module(other) {
        Ok(Value::fixnum(-1))
    } else {
        Ok(Value::nil())
    }
}

/// ## instance method Module#inspect
/// - name -> String | nil
///
///https://docs.ruby-lang.org/ja/latest/method/Module/i/inspect.html
fn name(_vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    let val = match self_val.into_module().op_name() {
        Some(name) => Value::string(name.to_owned()),
        None => Value::nil(),
    };
    Ok(val)
}

/// ## instance method Module#inspect
/// - to_s -> String
/// - inspect -> String
///
/// https://docs.ruby-lang.org/ja/latest/method/Module/i/inspect.html
fn inspect(_: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    Ok(Value::string(self_val.into_module().inspect()))
}

pub(crate) fn set_attr_accessor(
    globals: &mut Globals,
    self_val: Module,
    args: &[Value],
) -> VMResult {
    for arg in args {
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

/// ## instance method Module#constants
/// - constants(inherit = true) -> [[Symbol]]
///
/// https://docs.ruby-lang.org/ja/latest/method/Module/i/constants.html
fn constants(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_range(0, 1)?;
    let inherit = if args.len() == 0 {
        true
    } else {
        vm[0].to_bool()
    };
    let mut v: Vec<Value> = vec![];
    let mut class = self_val.into_module();
    let object = vm.globals.classes.object;
    loop {
        class
            .const_table()
            .keys()
            .filter(|x| x.is_constant())
            .map(|k| Value::symbol(*k))
            .for_each(|e| v.push(e));
        if !inherit {
            break;
        }
        match class.upper() {
            Some(superclass) => {
                if superclass.id() == object.id() {
                    break;
                } else {
                    class = superclass;
                };
            }
            None => break,
        }
    }
    Ok(Value::array_from(v))
}

/// Module#autoload(const_name, feature) -> nil
/// https://docs.ruby-lang.org/ja/latest/method/Module/i/autoload.html
fn autoload(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(2)?;
    let const_name = vm[0].expect_string_or_symbol("1st arg")?;
    let mut arg1 = vm[1];
    let feature = arg1.expect_string("2nd")?.to_string();
    let mut parent = Module::new(self_val);
    parent.set_autoload(const_name, feature);
    Ok(Value::nil())
}

fn class_variables(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(1)?;
    let inherit = vm[0].to_bool();
    assert_eq!(inherit, false);
    let receiver = self_val.rvalue();
    let res = match receiver.var_table() {
        Some(table) => table
            .keys()
            .filter(|x| x.is_class_var())
            .map(|x| Value::symbol(*x))
            .collect(),
        None => vec![],
    };
    Ok(Value::array_from(res))
}

fn const_defined(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_range(1, 2)?;
    let name = vm[0].expect_string_or_symbol("1st arg")?;
    Ok(Value::bool(
        vm.get_super_const(Module::new(self_val), name).is_ok(),
    ))
}

fn const_get(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(1)?;
    let name = match vm[0].as_symbol() {
        Some(symbol) => symbol,
        None => return Err(VMError::wrong_type("1st arg", "Symbol", vm[0])),
    };
    let val = vm.get_super_const(Module::new(self_val), name)?;
    Ok(val)
}

pub(crate) fn instance_methods(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_range(0, 1)?;
    let mut module = self_val.into_module();
    let inherited_too = args.len() == 0 || vm[0].to_bool();
    match inherited_too {
        false => {
            let v = module.method_names().map(|k| Value::symbol(*k)).collect();
            Ok(Value::array_from(v))
        }
        true => {
            let mut v = FxIndexSet::default();
            loop {
                for k in module.method_names() {
                    v.insert(*k);
                }
                match module.upper() {
                    None => break,
                    Some(upper) => {
                        module = upper;
                    }
                }
            }
            Ok(Value::array_from(
                v.into_iter().map(|id| Value::symbol(id)).collect(),
            ))
        }
    }
}

fn instance_method(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(1)?;
    let name = vm[0].expect_symbol_or_string("1st arg")?;
    let info = match self_val.into_module().search_method(name) {
        Some(m) => m,
        None => {
            return Err(RubyError::name(format!(
                "undefined method `{:?}' for class `{}'",
                name,
                self_val.into_module().name()
            )))
        }
    };
    Ok(Value::unbound_method(name, info.fid(), info.owner()))
}

/// ## instance method Module#method_defined?
/// - method_defined?(name, inherit=true) -> bool
///
/// https://docs.ruby-lang.org/ja/latest/method/Module/i/method_defined=3f.html
fn method_defined(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_range(1, 2)?;
    let name = vm[0].expect_symbol_or_string("1st arg")?;
    let b = if args.len() == 2 && !vm[1].to_bool() {
        self_val
            .into_module()
            .search_method_no_inherit(name)
            .is_some()
    } else {
        self_val.into_module().search_method(name).is_some()
    };
    Ok(Value::bool(b))
}

fn attr_accessor(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    let args = vm.args().to_owned();
    set_attr_accessor(&mut vm.globals, self_val.into_module(), &args)
}

fn attr_reader(vm: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    for arg in vm.args().to_owned() {
        if arg.is_packed_symbol() {
            let id = arg.as_packed_symbol();
            define_reader(&mut vm.globals, self_val.into_module(), id);
        } else {
            return Err(RubyError::name(
                "Each of args for attr_accessor must be a symbol.",
            ));
        }
    }
    Ok(Value::nil())
}

fn attr_writer(vm: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    for arg in vm.args().to_owned() {
        if arg.is_packed_symbol() {
            let id = arg.as_packed_symbol();
            define_writer(&mut vm.globals, self_val.into_module(), id);
        } else {
            return Err(RubyError::name(
                "Each of args for attr_accessor must be a symbol.",
            ));
        }
    }
    Ok(Value::nil())
}

fn define_reader(globals: &mut Globals, mut class: Module, id: IdentId) {
    let instance_var_id = id.add_prefix("@");
    let info = MethodInfo::AttrReader {
        id: instance_var_id,
    };
    let methodref = globals.methods.add(info);
    class.add_method(globals, id, methodref);
}

fn define_writer(globals: &mut Globals, mut class: Module, id: IdentId) {
    let instance_var_id = id.add_prefix("@");
    let assign_id = id.add_postfix("=");
    let info = MethodInfo::AttrWriter {
        id: instance_var_id,
    };
    let methodref = globals.methods.add(info);
    class.add_method(globals, assign_id, methodref);
}

fn module_function(vm: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    if vm.args().len() == 0 {
        vm.set_module_function();
    } else {
        let class = self_val.into_module();
        let mut singleton = class.get_singleton_class();
        for arg in vm.args().to_owned() {
            let name = arg.expect_string_or_symbol("Args")?;
            let method = class.get_method_or_nomethod(&mut vm.globals, name)?;
            singleton.add_method(&mut vm.globals, name, method);
        }
    }
    Ok(Value::nil())
}

fn singleton_class(_: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    Ok(Value::bool(self_val.into_module().is_singleton()))
}

/// Module#include(*modules) -> self
/// https://docs.ruby-lang.org/ja/latest/method/Module/i/include.html
fn include(vm: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    for arg in vm.args().to_owned() {
        let module = arg.expect_module("arg")?;
        self_val
            .into_module()
            .append_include(&mut vm.globals, module);
    }
    Ok(self_val)
}

/// Module#prepend(*modules) -> self
/// https://docs.ruby-lang.org/ja/latest/method/Module/i/prepend.html
fn prepend(vm: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    let self_mod = self_val.into_module();
    for arg in vm.args().to_owned() {
        let module = arg.expect_module("arg")?;
        self_val
            .into_module()
            .append_prepend(&mut vm.globals, self_mod, module);
    }
    Ok(self_val)
}

fn included_modules(_: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    let mut module = Some(self_val.into_module());
    let mut ary = vec![];
    loop {
        match module {
            None => break,
            Some(m) => {
                if m.is_included() {
                    ary.push(m.origin().unwrap().into())
                };
                module = m.upper();
            }
        }
    }
    Ok(Value::array_from(ary))
}

fn ancestors(_: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    let mut module = Some(self_val.into_module());
    let mut ary = vec![];
    loop {
        match module {
            None => break,
            Some(m) => {
                ary.push(m.real_module().into());
                module = m.upper();
            }
        }
    }
    Ok(Value::array_from(ary))
}

/// module_eval(expr, fname = "(eval)", lineno = 1) -> object
/// module_eval {|mod| ... } -> object
/// class_eval(expr, fname = "(eval)", lineno = 1) -> object
/// class_eval {|mod| ... } -> object
/// https://docs.ruby-lang.org/ja/latest/method/Module/i/class_eval.html
fn module_eval(vm: &mut VM, self_value: Value, args: &Args2) -> VMResult {
    let self_val = self_value.into_module();
    match &args.block {
        None => {
            args.check_args_min(1)?;
            let mut arg0 = vm[0];
            let program = arg0.expect_string("1st arg")?;
            let method = vm.parse_program_eval("(eval)", program.to_string())?;
            // The scopes of constants and class variables are same as module definition of `self_val`.
            let mut iseq = vm.caller_method_iseq();
            iseq.class_defined.push(self_val);
            let res =
                vm.eval_block_with_methodid(method, self_val, vm.caller_cfp().ep(), &Args::new0());
            iseq.class_defined.pop().unwrap();
            res
        }
        Some(block) => {
            args.check_args_min(0)?;
            // The scopes of constants and class variables are outer of the block.
            let res = vm.eval_block_self(block, self_val, &Args::new1(self_value));
            res
        }
    }
}

fn module_alias_method(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(2)?;
    let new = vm[0].expect_string_or_symbol("1st arg")?;
    let org = vm[1].expect_string_or_symbol("2nd arg")?;
    let method = Module::new(self_val).get_method_or_nomethod(&mut vm.globals, org)?;
    self_val
        .into_module()
        .add_method(&mut vm.globals, new, method);
    Ok(self_val)
}

fn public(_vm: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    Ok(self_val)
}

fn private(_vm: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    Ok(self_val)
}

fn protected(_vm: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    Ok(self_val)
}

fn private_constant(_vm: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    Ok(self_val)
}

fn include_(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(1)?;
    let val = Module::new(self_val);
    let module = vm[0].expect_module("1st arg")?;
    Ok(Value::bool(val.include_module(module)))
}

fn deprecate_constant(_vm: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    Ok(self_val)
}

fn private_class_method(_vm: &mut VM, self_val: Value, _args: &Args2) -> VMResult {
    Ok(self_val)
}

#[cfg(test)]
mod test {
    use crate::tests::*;

    #[test]
    fn module_teq() {
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
    fn module_cmp() {
        let program = r#"
        module Foo; end
        class Bar
          include Foo
        end
        class Baz < Bar; end
        class Qux; end
        assert 0, Foo <=> Foo
        assert 0, Bar <=> Bar
        assert 0, Baz <=> Baz
        assert 1, Foo <=> Bar
        assert 1, Bar <=> Baz
        assert 1, Foo <=> Baz
        assert -1, Bar <=> Foo     # => -1
        assert -1, Baz <=> Bar     # => -1
        assert -1, Baz <=> Foo     # => -1
        assert nil, Baz <=> Qux     # => nil
        assert nil, Qux <=> Baz     # => nil
        assert nil, Baz <=> Object.new  # => nil
        "#;
        assert_script(program);
    }

    #[test]
    fn name() {
        let program = r#"
        assert("Integer", Integer.name)
        assert("Class", Class.name)
        assert("Module", Module.name)
        module M
            class A; end
        end
        assert("M::A", M::A.name)
        assert("M", M.name)
        a = Class.new()
        assert(nil, a.name)
        B = a
        assert("B", a.name)
        assert(0, Module.new.inspect =~ /#<Module:0x.{16}>/)
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
    fn autoload() {
        let program = r##"
        load "./tests/autoload_test.rb"
    "##;
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

    assert 100, Foo.const_get(:Bar)
    assert 100, Bar.const_get(:Bar)
    assert_error { Bar.const_get([]) }
    assert true, ary_cmp(Foo.constants, [:Bar, :Ker])
    assert true, ary_cmp(Bar.constants, [:Doo, :Bar, :Ker])

    assert [], Module.constants - Object.constants
    class Z
        CONST = 100
        assert [:CONST], Module.constants - Object.constants
    end
    "#;
        assert_script(program);
    }

    #[test]
    fn constants2() {
        // https://docs.ruby-lang.org/ja/latest/method/Module/i/constants.html
        let program = r#"
        $clist = Module.constants

        def ary_cmp(a,b)
          return false if a - b != []
          return false if b - a != []
          true
        end

        class Foo
          FOO = 1
        end

        class Bar
          BAR = 1

        # Bar は BAR を含む
        assert true, ary_cmp(constants, [:BAR])
        # 出力に FOO は含まれない
        assert true, ary_cmp(Module.constants - $clist, [:BAR, :Bar, :Foo])

          class Baz
            # Baz は定数を含まない
            assert [], constants
          
            # ネストしたクラスでは、外側のクラスで定義した定数は
            # 参照可能なので、BAR は、Module.constants には含まれる
            # (クラス Baz も Bar の定数なので同様)
            assert true, ary_cmp(Module.constants - $clist, [:BAR, :Baz, :Foo, :Bar])
          end
        end
        "#;
        assert_script(program);
    }

    #[test]
    fn constants3() {
        let program = r##"
        class S
          S1 = 1
          assert [:S1], constants
        end

        class C < S
          C1 = 2
          assert [:C1, :S1], constants
          assert [:C1], constants(false)
        end

        "##;
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
    fn method_defined() {
        let program = r##"
        module A
          def method1()  end
          def protected_method1()  end
          protected :protected_method1
        end

        class B
          def method2()  end
          def private_method2()  end
          private :private_method2
        end

        class C < B
          include A
          def method3()  end
        end
      
        assert true, A.method_defined? :method1 
        assert false, A.method_defined? :method2 
        assert true, C.method_defined? "method1"
        assert true, C.method_defined? "method2"
        assert true, C.method_defined? "method2", true
        assert false, C.method_defined? "method2", false
        assert true, C.method_defined? "method3"
        assert true, C.method_defined? "protected_method1"
        assert false, C.method_defined? "method4"
        #assert false, C.method_defined? "private_method2"
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
    bar.send(:car=, 500) {}
    assert 500, bar.send(:car) {}
    assert_error { bar.send(:car=) {} }
    assert_error { bar.send(:car, 100) {} }
    assert_error { bar.send(:car=) }
    assert_error { bar.send(:car, 100) }
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
    fn instance_method() {
        let program = r#"
    class A
      def foo
        "foo"
      end
    end
    assert UnboundMethod, A.instance_method(:foo).class
    assert "foo", A.instance_method(:foo).bind_call(A.new)
    begin
      A.instance_method(:poo)
    rescue => ex
      assert NameError, ex.class
    end
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
        assert(C, C.module_eval { |mod| mod })
        assert(777, C.module_eval { |mod| mod::D })
        "##;
        assert_script(program);
    }

    #[test]
    fn module_eval2() {
        let program = r##"
        class C; end
        D = 0
        C.class_eval "def fn; 77; end; D = 1"
        assert 77, C.new.fn
        assert 1, C::D
        assert 0, D
        C.class_eval do
          def gn
            99
          end
          D = 2
        end
        assert 99, C.new.gn
        assert 1, C::D
        assert 2, D
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

        class D; end
        module M3
          def f; "M3"; end
        end
        module M4
          def f; "M4"; end
        end
        class D
          include M3, M4
        end
        assert "M4", D.new.f
        assert_error { class D; include 100; end }
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

    #[test]
    fn prepend2() {
        let program = r#"
        module M0
          def f; "M0"; end
        end
        module M1
          def f; "M1"; end
        end
        module M2
          def f; "M2"; end
        end
        
        class S
          include M0
        end
        class C < S; end

        assert "M0", C.new.f
        class S
          def f; "S"; end
        end
        assert "S", C.new.f
        class S
          prepend M1, M2
        end
        assert "M2", C.new.f
        assert [C, M2, M1, S, M0, Object, Kernel, BasicObject], C.ancestors
        "#;
        assert_script(program);
    }

    #[test]
    fn include_() {
        let program = "
        assert true, Integer.include?(Kernel)
        assert true, Integer.include?(Comparable)
        assert_error { Integer.include?(Numeric) }
        ";
        assert_script(program);
    }
}
