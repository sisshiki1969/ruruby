use crate::*;

pub fn init() {
    let mut object = BuiltinClass::object();
    object.append_include_without_increment_version(BuiltinClass::kernel());
    BuiltinClass::set_toplevel_constant("Object", object);

    object.add_builtin_method_by_str("initialize", initialize);
    object.add_builtin_method_by_str("class", class);
    object.add_builtin_method_by_str("object_id", object_id);
    object.add_builtin_method_by_str("to_s", to_s);
    object.add_builtin_method_by_str("inspect", inspect);
    object.add_builtin_method_by_str("equal?", equal);
    object.add_builtin_method_by_str("==", equal);
    object.add_builtin_method_by_str("===", equal);
    object.add_builtin_method_by_str("=~", match_); // This method is deprecated from Ruby 2.6.
    object.add_builtin_method_by_str("<=>", cmp);
    object.add_builtin_method_by_str("eql?", eql);
    object.add_builtin_method_by_str("singleton_class", singleton_class);
    object.add_builtin_method_by_str("clone", dup);
    object.add_builtin_method_by_str("dup", dup);
    object.add_builtin_method_by_str("nil?", nil_);
    object.add_builtin_method_by_str("method", method);
    object.add_builtin_method_by_str("instance_variable_set", instance_variable_set);
    object.add_builtin_method_by_str("instance_variable_get", instance_variable_get);
    object.add_builtin_method_by_str("instance_variables", instance_variables);
    object.add_builtin_method_by_str("instance_of?", instance_of);
    object.add_builtin_method_by_str("freeze", freeze);
    object.add_builtin_method_by_str("super", super_);
    object.add_builtin_method_by_str("send", send);
    object.add_builtin_method_by_str("__send__", send);
    object.add_builtin_method_by_str("to_enum", to_enum);
    object.add_builtin_method_by_str("enum_for", to_enum);
    object.add_builtin_method_by_str("methods", methods);
    object.add_builtin_method_by_str("singleton_methods", singleton_methods);
    object.add_builtin_method_by_str("respond_to?", respond_to);
    object.add_builtin_method_by_str("instance_exec", instance_exec);
}

fn initialize(_vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    Ok(self_val)
}

fn class(_vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    Ok(self_val.get_class().into())
}

fn object_id(_vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let id = self_val.id();
    Ok(Value::integer(id as i64))
}

fn to_s(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;

    let s = match self_val.unpack() {
        RV::Uninitialized => "[Uninitialized]".to_string(),
        RV::Object(oref) => match &oref.kind {
            ObjKind::Invalid => unreachable!("Invalid rvalue. (maybe GC problem) {:?}", *oref),
            ObjKind::Ordinary => oref.to_s(),
            ObjKind::Regexp(rref) => format!("({})", rref.as_str()),
            _ => format!("{:?}", oref.kind),
        },
        _ => unreachable!(),
    };

    Ok(Value::string(s))
}

fn inspect(_: &mut VM, self_val: Value, _: &Args) -> VMResult {
    match self_val.as_rvalue() {
        Some(oref) => Ok(Value::string(oref.inspect()?)),
        None => unreachable!(),
    }
}

fn singleton_class(_: &mut VM, self_val: Value, _: &Args) -> VMResult {
    self_val.get_singleton_class().map(|c| c.into())
}

fn dup(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let val = self_val.dup();
    Ok(val)
}

fn eql(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    Ok(Value::bool(self_val == args[0]))
}

fn nil_(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(Value::false_val())
}

fn method(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let name = match args[0].as_symbol() {
        Some(id) => id,
        None => return Err(RubyError::wrong_type("1st arg", "Symbol", args[0])),
    };
    let method = vm.get_method_from_receiver(self_val, name)?;
    let val = Value::method(name, self_val, method);
    Ok(val)
}

fn instance_variable_set(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(2)?;
    let name = args[0];
    let val = args[1];
    let var_id = name.expect_symbol_or_string("1st arg")?;
    self_val.set_var(var_id, val);
    Ok(val)
}

fn instance_variable_get(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let name = args[0];
    let var_id = name.expect_symbol_or_string("1st arg")?;
    let self_obj = self_val.rvalue();
    let val = match self_obj.get_var(var_id) {
        Some(val) => val,
        None => Value::nil(),
    };
    Ok(val)
}

fn instance_variables(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    let receiver = self_val.rvalue();
    let res = match receiver.var_table() {
        Some(table) => table
            .keys()
            .filter(|x| IdentId::starts_with(**x, "@"))
            .map(|x| Value::symbol(*x))
            .collect(),
        None => vec![],
    };
    Ok(Value::array_from(res))
}

fn instance_of(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    Ok(Value::bool(args[0].id() == self_val.get_class().id()))
}

fn freeze(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(self_val)
}

fn super_(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    //args.check_args_num( 0)?;
    let context = vm.context();
    let iseq = context.iseq_ref.unwrap();
    if let ISeqKind::Method(Some(m)) = context.kind {
        let class = iseq.class_defined.last().unwrap();
        let method = match class.superclass() {
            Some(class) => match MethodRepo::find_method(class, m) {
                Some(m) => m,
                None => {
                    return Err(RubyError::nomethod(format!(
                        "no superclass method `{:?}' for {:?}",
                        m, self_val
                    )));
                }
            },
            None => {
                return Err(RubyError::nomethod(format!(
                    "no superclass method `{:?}' for {:?}.",
                    m, self_val
                )));
            }
        };
        if args.len() == 0 {
            let param_num = iseq.params.param_ident.len();
            let mut args = Args::new0();
            for i in 0..param_num {
                args.push(context[i]);
            }
            vm.eval_method(method, context.self_value, &args)
        } else {
            vm.eval_method(method, context.self_value, &args)
        }
    } else {
        return Err(RubyError::nomethod("super called outside of method"));
    }
}

fn equal(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    Ok(Value::bool(self_val.id() == args[0].id()))
}

fn send(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_min(1)?;
    let receiver = self_val;
    let method_id = match args[0].as_symbol() {
        Some(symbol) => symbol,
        None => return Err(RubyError::argument("Must be a symbol.")),
    };
    let method = vm.get_method_from_receiver(receiver, method_id)?;

    let mut new_args = Args::new(args.len() - 1);
    for i in 0..args.len() - 1 {
        new_args[i] = args[i + 1];
    }
    new_args.block = args.block.clone();
    let res = vm.eval_method(method, self_val, &new_args)?;
    Ok(res)
}

fn to_enum(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    if args.block.is_some() {
        return Err(RubyError::argument("Curently, block is not allowed."));
    };
    let (method, new_args) = if args.len() == 0 {
        let method = IdentId::EACH;
        let new_args = Args::new0();
        (method, new_args)
    } else {
        if !args[0].is_packed_symbol() {
            return Err(RubyError::argument("2nd arg must be Symbol."));
        };
        let method = args[0].as_packed_symbol();
        let mut new_args = Args::new(args.len() - 1);
        for i in 0..args.len() - 1 {
            new_args[i] = args[i + 1];
        }
        (method, new_args)
    };
    vm.create_enumerator(method, self_val, new_args)
}

fn methods(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let regular = args.len() == 0 || args[0].to_bool();
    // TODO: Only include public and protected.
    if regular {
        let class = self_val.get_class_for_method();
        module::instance_methods(vm, class.into(), args)
    } else {
        singleton_methods(vm, self_val, args)
    }
}

fn singleton_methods(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(0, 1)?;
    let all = args.len() == 0 || args[0].to_bool();
    // TODO: Only include public and protected.
    let mut v = FxIndexSet::default();
    let root = match self_val.get_singleton_class() {
        Err(_) => Some(self_val.get_class_for_method()),
        Ok(class) => {
            for k in class.method_table().keys() {
                v.insert(Value::symbol(*k));
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
                for k in module.method_table().keys() {
                    v.insert(Value::symbol(*k));
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
    Ok(Value::array_from(v.iter().cloned().collect()))
}

fn respond_to(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 2)?;
    if args.len() == 2 {
        eprintln!("Warining: 2nd arg will not used. respont_to?()")
    };
    let method = args[0].expect_string_or_symbol("1st arg")?;
    let b = MethodRepo::find_method_from_receiver(self_val, method).is_some();
    Ok(Value::bool(b))
}

fn instance_exec(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let block = args.expect_block()?;
    let class = self_val.get_class_for_method();
    vm.class_push(class);
    let res = vm.eval_block_self(block, self_val, args);
    vm.class_pop();
    res
}

fn match_(_: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    Ok(Value::nil())
}

fn cmp(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let res = if equal(vm, self_val, args)?.to_bool() {
        Value::integer(0)
    } else {
        Value::nil()
    };
    Ok(res)
}

#[cfg(test)]
mod test {
    use crate::test::*;

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
            def foo(a,b,c,d:0)
                assert [100,200,300,500], [a,b,c,d]
            end 
            def boo(a,b,c)
                assert [100,200,300], [a,b,c]
            end           
         end
        
        class B < A
            def foo(a,b,c=300,d:400)
                super(a,b,c,d:d)
            end
            def boo(a,b,c)
                super
            end
        end
        
        B.new.foo(100,200,d:500)
        B.new.boo(100,200,300)

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
        assert [:foo, :instance_exec, :methods, :__send__, :freeze, :instance_variable_get, :nil?, :singleton_class, :equal?, :===, :inspect, :initialize, :singleton_methods, :to_enum, :super, :instance_variables, :method, :clone, :=~, :class, :to_s, :==, :respond_to?, :enum_for, :send, :instance_of?, :instance_variable_set, :dup, :eql?, :object_id, :<=>, :Integer, :sleep, :loop, :__FILE__, :is_a?, :require_relative, :assert, :puts, :eval, :Array, :lambda, :abort, :rand, :__dir__, :block_given?, :require, :print, :"`", :Complex, :proc, :exit, :raise, :kind_of?, :load, :assert_error, :p, :at_exit, :"/alias_method", :method_missing, :__id__], a.methods
        assert [:foo, :instance_exec, :methods, :__send__, :freeze, :instance_variable_get, :nil?, :singleton_class, :equal?, :===, :inspect, :initialize, :singleton_methods, :to_enum, :super, :instance_variables, :method, :clone, :=~, :class, :to_s, :==, :respond_to?, :enum_for, :send, :instance_of?, :instance_variable_set, :dup, :eql?, :object_id, :<=>, :Integer, :sleep, :loop, :__FILE__, :is_a?, :require_relative, :assert, :puts, :eval, :Array, :lambda, :abort, :rand, :__dir__, :block_given?, :require, :print, :"`", :Complex, :proc, :exit, :raise, :kind_of?, :load, :assert_error, :p, :at_exit, :"/alias_method", :method_missing, :__id__], a.methods(true)
        assert [], a.methods(false)
        assert [:bar, :foo, :instance_exec, :methods, :__send__, :freeze, :instance_variable_get, :nil?, :singleton_class, :equal?, :===, :inspect, :initialize, :singleton_methods, :to_enum, :super, :instance_variables, :method, :clone, :=~, :class, :to_s, :==, :respond_to?, :enum_for, :send, :instance_of?, :instance_variable_set, :dup, :eql?, :object_id, :<=>, :Integer, :sleep, :loop, :__FILE__, :is_a?, :require_relative, :assert, :puts, :eval, :Array, :lambda, :abort, :rand, :__dir__, :block_given?, :require, :print, :"`", :Complex, :proc, :exit, :raise, :kind_of?, :load, :assert_error, :p, :at_exit, :"/alias_method", :method_missing, :__id__], b.methods
        assert [:bar, :foo, :instance_exec, :methods, :__send__, :freeze, :instance_variable_get, :nil?, :singleton_class, :equal?, :===, :inspect, :initialize, :singleton_methods, :to_enum, :super, :instance_variables, :method, :clone, :=~, :class, :to_s, :==, :respond_to?, :enum_for, :send, :instance_of?, :instance_variable_set, :dup, :eql?, :object_id, :<=>, :Integer, :sleep, :loop, :__FILE__, :is_a?, :require_relative, :assert, :puts, :eval, :Array, :lambda, :abort, :rand, :__dir__, :block_given?, :require, :print, :"`", :Complex, :proc, :exit, :raise, :kind_of?, :load, :assert_error, :p, :at_exit, :"/alias_method", :method_missing, :__id__], b.methods(true)
        assert [], b.methods(false)
        assert [:baz, :bar, :foo, :instance_exec, :methods, :__send__, :freeze, :instance_variable_get, :nil?, :singleton_class, :equal?, :===, :inspect, :initialize, :singleton_methods, :to_enum, :super, :instance_variables, :method, :clone, :=~, :class, :to_s, :==, :respond_to?, :enum_for, :send, :instance_of?, :instance_variable_set, :dup, :eql?, :object_id, :<=>, :Integer, :sleep, :loop, :__FILE__, :is_a?, :require_relative, :assert, :puts, :eval, :Array, :lambda, :abort, :rand, :__dir__, :block_given?, :require, :print, :"`", :Complex, :proc, :exit, :raise, :kind_of?, :load, :assert_error, :p, :at_exit, :"/alias_method", :method_missing, :__id__], c.methods
        assert [:baz, :bar, :foo, :instance_exec, :methods, :__send__, :freeze, :instance_variable_get, :nil?, :singleton_class, :equal?, :===, :inspect, :initialize, :singleton_methods, :to_enum, :super, :instance_variables, :method, :clone, :=~, :class, :to_s, :==, :respond_to?, :enum_for, :send, :instance_of?, :instance_variable_set, :dup, :eql?, :object_id, :<=>, :Integer, :sleep, :loop, :__FILE__, :is_a?, :require_relative, :assert, :puts, :eval, :Array, :lambda, :abort, :rand, :__dir__, :block_given?, :require, :print, :"`", :Complex, :proc, :exit, :raise, :kind_of?, :load, :assert_error, :p, :at_exit, :"/alias_method", :method_missing, :__id__], c.methods(true)
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
    fn object_etc() {
        let program = r#"
        #assert 365, -365.method(:abs).call
        assert "RUBY", "Ruby".method(:upcase).call
        assert_error { "Ruby".method(100) }
        "#;
        assert_script(program);
    }
}
