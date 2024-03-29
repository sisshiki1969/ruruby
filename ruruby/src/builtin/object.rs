use crate::*;

pub(crate) fn init(globals: &mut Globals) {
    let mut object = BuiltinClass::object();
    object.append_include_without_increment_version(BuiltinClass::kernel());
    globals.set_toplevel_constant("Object", object);

    object.add_builtin_method_by_str(globals, "initialize", initialize);
    object.add_builtin_method_by_str(globals, "class", class);
    object.add_builtin_method_by_str(globals, "object_id", object_id);
    object.add_builtin_method_by_str(globals, "to_s", to_s);
    object.add_builtin_method_by_str(globals, "inspect", inspect);
    object.add_builtin_method_by_str(globals, "equal?", equal);
    object.add_builtin_method_by_str(globals, "==", equal);
    object.add_builtin_method_by_str(globals, "===", equal);
    object.add_builtin_method_by_str(globals, "=~", match_); // This method is deprecated from Ruby 2.6.
    object.add_builtin_method_by_str(globals, "<=>", cmp);
    object.add_builtin_method_by_str(globals, "eql?", eql);
    object.add_builtin_method_by_str(globals, "singleton_class", singleton_class);
    object.add_builtin_method_by_str(globals, "extend", extend);
    object.add_builtin_method_by_str(globals, "clone", dup);
    object.add_builtin_method_by_str(globals, "dup", dup);
    object.add_builtin_method_by_str(globals, "nil?", nil_);
    object.add_builtin_method_by_str(globals, "method", method);
    object.add_builtin_method_by_str(globals, "instance_variable_set", instance_variable_set);
    object.add_builtin_method_by_str(globals, "instance_variable_get", instance_variable_get);
    object.add_builtin_method_by_str(globals, "instance_variables", instance_variables);
    object.add_builtin_method_by_str(globals, "instance_of?", instance_of);
    object.add_builtin_method_by_str(globals, "freeze", freeze);
    object.add_builtin_method_by_str(globals, "send", send);
    object.add_builtin_method_by_str(globals, "__send__", send);
    object.add_builtin_method_by_str(globals, "to_enum", to_enum);
    object.add_builtin_method_by_str(globals, "enum_for", to_enum);
    object.add_builtin_method_by_str(globals, "methods", methods);
    object.add_builtin_method_by_str(globals, "singleton_methods", singleton_methods);
    object.add_builtin_method_by_str(globals, "respond_to?", respond_to);
    object.add_builtin_method_by_str(globals, "frozen?", frozen_);
}

fn initialize(_vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    Ok(self_val)
}

fn class(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    Ok(vm.globals.get_class(self_val).into())
}

fn object_id(_vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    let id = self_val.id();
    Ok(Value::integer(id as i64))
}

fn to_s(_: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;

    let s = match self_val.unpack() {
        RV::Uninitialized => "[Uninitialized]".to_string(),
        RV::Object(oref) => match oref.kind() {
            ObjKind::INVALID => unreachable!("Invalid rvalue. (maybe GC problem) {:?}", *oref),
            ObjKind::ORDINARY => oref.to_s(),
            ObjKind::REGEXP => format!("({})", oref.regexp().as_str()),
            _ => format!("{:?}", self_val),
        },
        _ => unreachable!(),
    };

    Ok(Value::string(s))
}

fn inspect(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    match self_val.as_rvalue() {
        Some(oref) => Ok(Value::string(oref.inspect()?)),
        None => Ok(Value::string(vm.val_inspect(self_val)?)),
    }
}

fn singleton_class(_: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    Ok(self_val.get_singleton_class()?.into())
}

/// Object#extend(*modules) -> self
/// https://docs.ruby-lang.org/ja/latest/method/Object/i/extend.html
fn extend(vm: &mut VM, self_val: Value, _: &Args2) -> VMResult {
    let mut singleton = self_val.get_singleton_class()?;
    for arg in vm.args().to_owned() {
        let module = arg.expect_module("arg")?;
        singleton.append_include(&mut vm.globals, module);
    }
    Ok(self_val)
}

fn dup(_: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    let val = self_val.shallow_dup();
    Ok(val)
}

/// eql?(other) -> bool
/// https://docs.ruby-lang.org/ja/latest/method/Object/i/eql=3f.html
fn eql(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(1)?;
    Ok(Value::bool(self_val.eql(&vm[0])))
}

fn nil_(_: &mut VM, _: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    Ok(Value::false_val())
}

fn method(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(1)?;
    let name = match vm[0].as_symbol() {
        Some(id) => id,
        None => return Err(VMError::wrong_type("1st arg", "Symbol", vm[0])),
    };
    let rec_class = vm.globals.get_class_for_method(self_val);
    let info = match rec_class.search_method(name) {
        Some(m) => m,
        None => {
            return Err(RubyError::name(format!(
                "undefined method `{:?}' for class `{}'",
                name,
                rec_class.name()
            )))
        }
    };
    let val = Value::method(name, self_val, info.fid(), info.owner());
    Ok(val)
}

fn instance_variable_set(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(2)?;
    let name = vm[0];
    let val = vm[1];
    let var_id = name.expect_symbol_or_string("1st arg")?;
    self_val.set_var(var_id, val);
    Ok(val)
}

fn instance_variable_get(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(1)?;
    let name = vm[0];
    let var_id = name.expect_symbol_or_string("1st arg")?;
    let self_obj = self_val.rvalue();
    let val = match self_obj.get_var(var_id) {
        Some(val) => val,
        None => Value::nil(),
    };
    Ok(val)
}

fn instance_variables(_: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    let receiver = self_val.rvalue();
    let res = match receiver.var_table() {
        Some(table) => table
            .keys()
            .filter(|x| x.starts_with("@"))
            .map(|x| Value::symbol(*x))
            .collect(),
        None => vec![],
    };
    Ok(Value::array_from(res))
}

fn instance_of(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(1)?;
    Ok(Value::bool(
        vm[0].id() == vm.globals.get_class(self_val).id(),
    ))
}

fn freeze(_: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    Ok(self_val)
}

fn equal(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(1)?;
    Ok(Value::bool(self_val.id() == vm[0].id()))
}

fn send(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_min(1)?;
    let receiver = self_val;
    let method_id = match vm[0].as_symbol() {
        Some(symbol) => symbol,
        None => return Err(RubyError::argument("Must be a symbol.")),
    };
    let fid = receiver.get_method_or_nomethod(&mut vm.globals, method_id)?;

    let (mut src, mut len) = vm.args_range();
    src += 1;
    len -= 1;
    let new_arg = Args2::new_with_block(len, args.block.clone());
    vm.eval_method_range(fid, self_val, src, len, &new_arg)
}

fn to_enum(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    if args.block.is_some() {
        return Err(RubyError::argument("Curently, block is not allowed."));
    };
    let (method, new_args) = if args.len() == 0 {
        let method = IdentId::EACH;
        let new_args = Args::new0();
        (method, new_args)
    } else {
        if !vm[0].is_packed_symbol() {
            return Err(RubyError::argument("2nd arg must be Symbol."));
        };
        let method = vm[0].as_packed_symbol();
        let mut new_args = Args::new(args.len() - 1);
        for i in 0..args.len() - 1 {
            new_args[i] = vm[i + 1];
        }
        (method, new_args)
    };
    vm.create_enumerator(method, self_val, new_args)
}

fn methods(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_range(0, 1)?;
    let regular = args.len() == 0 || vm[0].to_bool();
    // TODO: Only include public and protected.
    if regular {
        let class = vm.globals.get_class_for_method(self_val);
        module::instance_methods(vm, class.into(), args)
    } else {
        singleton_methods(vm, self_val, args)
    }
}

fn singleton_methods(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_range(0, 1)?;
    let all = args.len() == 0 || vm[0].to_bool();
    // TODO: Only include public and protected.
    let mut v = FxIndexSet::default();
    let root = match self_val.get_singleton_class() {
        Err(_) => Some(vm.globals.get_class_for_method(self_val)),
        Ok(class) => {
            for k in class.method_names() {
                v.insert(*k);
            }
            class.upper()
        }
    };
    if all {
        if let Some(mut module) = root {
            loop {
                if !module.is_singleton() && !module.is_included() {
                    break;
                }
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
        }
    }
    Ok(Value::array_from(
        v.into_iter().map(|id| Value::symbol(id)).collect(),
    ))
}

fn respond_to(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_range(1, 2)?;
    if args.len() == 2 {
        eprintln!("Warining: 2nd arg will not used. respont_to?()")
    };
    let method = vm[0].expect_string_or_symbol("1st arg")?;
    let b = vm
        .globals
        .find_method_from_receiver(self_val, method)
        .is_some();
    Ok(Value::bool(b))
}

fn match_(_: &mut VM, _: Value, args: &Args2) -> VMResult {
    args.check_args_num(1)?;
    Ok(Value::nil())
}

fn cmp(vm: &mut VM, self_val: Value, args: &Args2) -> VMResult {
    args.check_args_num(1)?;
    let res = if equal(vm, self_val, args)?.to_bool() {
        Value::integer(0)
    } else {
        Value::nil()
    };
    Ok(res)
}

fn frozen_(_: &mut VM, _: Value, args: &Args2) -> VMResult {
    args.check_args_num(0)?;
    Ok(Value::false_val())
}

#[cfg(test)]
mod test {
    use crate::tests::*;

    #[test]
    fn object_ops() {
        let program = r#"
        a = Object.new
        b = Object.new
        assert 0, a <=> a # => 0
        assert nil, a <=> b # => nil
        assert false, a == b
        assert false, a === b
        assert false, a.equal? b
        assert false, a.eql? b
        assert true, a == a
        assert true, a === a
        assert true, a.equal? a
        assert true, a.eql? a
        "#;
        assert_script(program);
    }

    #[test]
    fn to_s() {
        let program = r#"
        assert("", nil.to_s)
        assert("true", true.to_s)
        assert("false", false.to_s)
        assert("foo", :foo.to_s)
        assert("75", 75.to_s)
        assert("7.5", (7.5).to_s)
        assert("Ruby", "Ruby".to_s)
        assert("[]", [].to_s)
        assert("[7]", [7].to_s)
        assert("[:foo]", [:foo].to_s)
        assert("{}", {}.to_s)
        assert('{:foo=>"bar"}', {foo:"bar"}.to_s)
        assert 0, Object.new.to_s =~ /#<Object:0x.{16}>/
        assert 0, Object.new.inspect =~ /#<Object:0x.{16}>/
        o = Object.new
        def o.a=(x)
          @a = x
        end
        o.a = 100
        assert 0, o.to_s =~ /#<Object:0x.{16}>/
        assert 0, o.inspect =~ /#<Object:0x.{16} @a=100>/
        "#;
        assert_script(program);
    }

    #[test]
    fn dup() {
        let program = r#"
        obj = Object.new
        obj.instance_variable_set(:@foo, 155)
        obj2 = obj.dup
        obj2.instance_variable_set(:@foo, 555)
        assert(155, obj.instance_variable_get(:@foo))
        assert(555, obj2.instance_variable_get(:@foo))
        assert(false, obj.eql?(obj2))
        "#;
        assert_script(program);
    }

    #[test]
    fn nil() {
        let program = r#"
        assert(true, nil.nil?)
        assert(false, 4.nil?)
        assert(false, "nil".nil?)
        "#;
        assert_script(program);
    }

    #[test]
    fn to_i() {
        let program = r#"
        assert(3, 3.to_i)
        assert(4, 4.7.to_i)
        assert(-4, -4.7.to_i)
        assert(0, nil.to_i)
        assert_error { true.to_i }
        "#;
        assert_script(program);
    }

    #[test]
    fn instance_variables() {
        let program = r#"
        obj = Object.new
        obj.instance_variable_set("@foo", "foo")
        obj.instance_variable_set(:@bar, 777)
        assert(777, obj.instance_variable_get("@bar"))
        assert(nil, obj.instance_variable_get("@boo"))
        assert("foo", obj.instance_variable_get(:@foo))
        assert_error { obj.instance_variable_get(7) }
        assert_error { obj.instance_variable_set(:@foo) }
        assert_error { obj.instance_variable_set(9, 10) }

        def ary_cmp(a,b)
            return false if a - b != []
            return false if b - a != []
            true
        end

        assert(true, ary_cmp([:@foo, :@bar], obj.instance_variables))
        "#;
        assert_script(program);
    }

    #[test]
    fn instance_of() {
        let program = r#"
        class C < Object
        end
        class S < C
        end
        
        obj = S.new
        assert true, obj.instance_of?(S)
        assert false, obj.instance_of?(C)
        "#;
        assert_script(program);
    }

    #[test]
    fn object_send() {
        let program = r#"
        class Foo
            def foo(); "foo" end
            def bar(); "bar" end
            def baz(); "baz" end
        end

        # 任意のキーとメソッド(の名前)の関係をハッシュに保持しておく
        # レシーバの情報がここにはないことに注意
        methods = {1 => :foo, 2 => :bar, 3 => :baz}

        # キーを使って関連するメソッドを呼び出す
        # レシーバは任意(Foo クラスのインスタンスである必要もない)
        assert "foo", Foo.new.send(methods[1])
        assert "bar", Foo.new.send(methods[2])
        assert "baz", Foo.new.send(methods[3])
        "#;
        assert_script(program);
    }

    #[test]
    fn object_yield() {
        let program = r#"
        # ブロック付きメソッドの定義、
        # その働きは与えられたブロック(手続き)に引数1, 2を渡して実行すること
        def foo
            yield(1,2)
        end

        # fooに「2引数手続き、その働きは引数を配列に括ってpで印字する」というものを渡して実行させる
        assert [1, 2], foo {|a,b| [a, b]}  # => [1, 2] (要するに p [1, 2] を実行した)
        # 今度は「2引数手続き、その働きは足し算をしてpで印字する」というものを渡して実行させる
        assert 3, foo {|a, b| p a + b}  # => 3 (要するに p 1 + 2 を実行した)

        # 今度のブロック付きメソッドの働きは、
        # 与えられたブロックに引数10を渡して起動し、続けざまに引数20を渡して起動し、
        # さらに引数30を渡して起動すること
        def bar
            a = []
            a << yield(10)
            a << yield(20)
            a << yield(30)
        end

        # barに「1引数手続き、その働きは引数に3を足してpで印字する」というものを渡して実行させる
        assert [13, 23, 33], bar {|v| v + 3 }
        # => 13
        #    23
        #    33 (同じブロックが3つのyieldで3回起動された。
        #        具体的には 10 + 3; 20 + 3; 30 + 3 を実行した)

        "#;
        assert_script(program);
    }

    #[test]
    fn object_yield2() {
        let program = r#"
        class Array
            def iich
                len = self.size
                for i in 0...len
                    yield(self[i])
                end
            end
        end

        sum = 0
        [1,2,3,4,5].iich{|x| puts x, sum; sum = sum + x }
        assert(15 ,sum)
        "#;
        assert_script(program);
    }

    #[test]
    fn object_super() {
        let program = r#"
        class A
            def foo(a,b,c)
                [a,b,c]
            end 
            def boo(*a)
                a
            end
            def bee(a:1,b:2,c:3)
                [a,b,c]
            end
        end            
        
        class B < A
            def foo(a,b,c=300,d:400)
                super(a,b,c,d:d)
            end
            def boo(a,b,c)
                super
            end
            def bee(a,b,c)
                super()
            end
        end
        
        assert [100,200,300], B.new.foo(100,200,300)
        assert [100,200,300], B.new.boo(100,200,300)
        assert [1,2,3], B.new.bee(100,200,300)

        "#;
        assert_script(program);
    }

    #[test]
    fn object_methods() {
        let program = r#"
        class A
            def foo
            end
        end
        class B < A
            def bar
            end
        end
        a = A.new
        b = B.new
        c = B.new
        def c.baz
        end
        o = Object.new
        assert [:foo], a.methods - o.methods
        assert [:foo], a.methods(true) - o.methods
        assert [], a.methods(false)
        assert [:bar, :foo], b.methods - o.methods
        assert [:bar, :foo], b.methods(true) - o.methods
        assert [], b.methods(false)
        assert [:baz, :bar, :foo], c.methods - o.methods
        assert [:baz, :bar, :foo], c.methods(true) - o.methods
        assert [:baz], c.methods(false)
        "#;
        assert_script(program);
    }

    #[test]
    fn object_singleton_methods() {
        let program = r#"
        module Mod
            def mod
            end
        end
        class A
            def foo
            end
        end
        class B < A
            def bar
            end
            def self.method_on_class
            end
        end
        a = A.new
        b = B.new
        c = B.new
        def c.one
        end
        class << c
            include Mod
            def two
            end
        end
        assert [], a.singleton_methods
        assert [], a.singleton_methods(true)
        assert [], a.singleton_methods(false)
        assert [], b.singleton_methods
        assert [], b.singleton_methods(true)
        assert [], b.singleton_methods(false)
        assert [:one, :two, :mod], c.singleton_methods
        assert [:one, :two, :mod], c.singleton_methods(true)
        assert [:one, :two], c.singleton_methods(false)
        assert [:method_on_class], B.singleton_methods
        assert [:method_on_class], B.singleton_methods(true)
        assert [:method_on_class], B.singleton_methods(false)
        "#;
        assert_script(program);
    }

    #[test]
    fn object_respond_to() {
        let program = r#"
        class A
            def foo
            end
        end
        class B < A
            def bar
            end
        end
        a = A.new
        b = B.new
        assert(true, a.respond_to?(:foo))
        assert(false, a.respond_to? "bar")
        assert(true, b.respond_to? "foo")
        assert(true, b.respond_to?(:bar))
        "#;
        assert_script(program);
    }

    #[test]
    fn object_method() {
        let program = r#"
        assert 365, -365.method(:abs).call
        assert "RUBY", "Ruby".method(:upcase).call

        begin
          "Ruby".method(100)
        rescue => ex
          assert TypeError, ex.class
        end

        begin
          100.method(:xxx)
        rescue => ex
          assert NameError, ex.class
        end

        "#;
        assert_script(program);
    }

    #[test]
    fn object_extend() {
        let program = r#"
        module Foo
          def a; 'ok Foo'; end
        end
        
        module Bar
          def b; 'ok Bar'; end
        end
        
        obj = Object.new
        obj.extend Foo, Bar
        assert "ok Foo", obj.a
        assert "ok Bar", obj.b
        
        class Klass
          include Foo
          extend Bar
        end
        
        assert "ok Foo", Klass.new.a
        assert "ok Bar", Klass.b
        "#;
        assert_script(program);
    }
}
