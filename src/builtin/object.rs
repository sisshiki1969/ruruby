use crate::*;

pub fn init(globals: &mut Globals) {
    let mut object = globals.builtins.object;
    let object_class = object.as_mut_class();
    object_class.add_builtin_method_by_str("class", class);
    object_class.add_builtin_method_by_str("object_id", object_id);
    object_class.add_builtin_method_by_str("to_s", to_s);
    object_class.add_builtin_method_by_str("inspect", inspect);
    object_class.add_builtin_method_by_str("singleton_class", singleton_class);
    object_class.add_builtin_method_by_str("clone", dup);
    object_class.add_builtin_method_by_str("dup", dup);
    object_class.add_builtin_method_by_str("eql?", eql);
    object_class.add_builtin_method_by_str("nil?", nil);
    object_class.add_builtin_method_by_str("to_i", toi);
    object_class.add_builtin_method_by_str("method", method);
    object_class.add_builtin_method_by_str("instance_variable_set", instance_variable_set);
    object_class.add_builtin_method_by_str("instance_variable_get", instance_variable_get);
    object_class.add_builtin_method_by_str("instance_variables", instance_variables);
    object_class.add_builtin_method_by_str("instance_of?", instance_of);
    object_class.add_builtin_method_by_str("freeze", freeze);
    object_class.add_builtin_method_by_str("super", super_);
    object_class.add_builtin_method_by_str("equal?", equal);
    object_class.add_builtin_method_by_str("send", send);
    object_class.add_builtin_method_by_str("__send__", send);
    object_class.add_builtin_method_by_str("eval", eval);
    object_class.add_builtin_method_by_str("to_enum", to_enum);
    object_class.add_builtin_method_by_str("enum_for", to_enum);
    object_class.add_builtin_method_by_str("respond_to?", respond_to);
    object_class.add_builtin_method_by_str("instance_exec", instance_exec);
}

fn class(_vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let class = self_val.get_class();
    Ok(class)
}

fn object_id(_vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    let id = self_val.id();
    Ok(Value::integer(id as i64))
}

fn to_s(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;

    let s = match self_val.unpack() {
        RV::Uninitialized => "[Uninitialized]".to_string(),
        RV::Nil => "".to_string(),
        RV::Bool(b) => match b {
            true => "true".to_string(),
            false => "false".to_string(),
        },
        RV::Integer(i) => i.to_string(),
        RV::Float(f) => {
            if f.fract() == 0.0 {
                format!("{:.1}", f)
            } else {
                f.to_string()
            }
        }
        RV::Symbol(i) => format!("{:?}", i),
        RV::Object(oref) => match &oref.kind {
            ObjKind::Invalid => unreachable!("Invalid rvalue. (maybe GC problem) {:?}", *oref),
            ObjKind::Class(cinfo) => match cinfo.name() {
                Some(id) => format! {"{:?}", id},
                None => format! {"#<Class:0x{:x}>", oref.id()},
            },
            ObjKind::Ordinary => {
                let class_name = self_val.get_class().as_class().name_str();
                format!("#<{}:{:016x}>", class_name, self_val.id())
            }
            ObjKind::Regexp(rref) => format!("({})", rref.as_str()),
            _ => format!("{:?}", oref.kind),
        },
    };

    Ok(Value::string(s))
}

fn inspect(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    match self_val.as_rvalue() {
        Some(oref) => {
            let s = oref.inspect(vm)?;
            Ok(Value::string(s))
        }
        None => {
            let s = vm.val_inspect(self_val)?;
            Ok(Value::string(s))
        }
    }
}

fn singleton_class(vm: &mut VM, self_val: Value, _: &Args) -> VMResult {
    vm.get_singleton_class(self_val)
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

fn nil(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(0)?;
    Ok(Value::bool(self_val.is_nil()))
}

fn toi(_: &mut VM, self_val: Value, _: &Args) -> VMResult {
    //args.check_args_num( 1, 1)?;
    if self_val.is_nil() {
        Ok(Value::integer(0))
    } else {
        Err(RubyError::undefined_method(
            IdentId::get_id("to_i"),
            self_val,
        ))
    }
}

fn method(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let name = match args[0].as_symbol() {
        Some(id) => id,
        None => return Err(RubyError::typeerr("An argument must be a Symbol.")),
    };
    let method = vm.get_method_from_receiver(self_val, name)?;
    let val = Value::method(name, self_val, method);
    Ok(val)
}

fn instance_variable_set(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(2)?;
    let name = args[0];
    let val = args[1];
    let var_id = match name.as_symbol() {
        Some(symbol) => symbol,
        None => match name.as_string() {
            Some(s) => IdentId::get_id(s),
            None => return Err(RubyError::typeerr("1st arg must be Symbol or String.")),
        },
    };
    let self_obj = self_val.rvalue_mut();
    self_obj.set_var(var_id, val);
    Ok(val)
}

fn instance_variable_get(_: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_num(1)?;
    let name = args[0];
    let var_id = match name.as_symbol() {
        Some(symbol) => symbol,
        None => match name.as_string() {
            Some(s) => IdentId::get_id(s),
            None => return Err(RubyError::typeerr("1st arg must be Symbol or String.")),
        },
    };
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
    let context = vm.current_context();
    let iseq = context.iseq_ref.unwrap();
    if let ISeqKind::Method(m) = context.kind {
        let class = match iseq.class_defined {
            Some(list) => list.class,
            None => {
                return Err(RubyError::nomethod(format!(
                    "no superclass method `{:?}' for {:?}.",
                    m, self_val
                )));
            }
        };
        let method = match class.superclass() {
            Some(class) => match vm.globals.find_method(class, m) {
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
            vm.eval_send(method, context.self_value, &args)
        } else {
            vm.eval_send(method, context.self_value, &args)
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
    let res = vm.eval_send(method, self_val, &new_args)?;
    Ok(res)
}

fn eval(vm: &mut VM, _: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 4)?;
    let mut arg0 = args[0];
    let program = arg0.expect_string("1st arg")?;
    //#[cfg(debug_assertions)]
    //eprintln!("eval: {}", program);
    if args.len() > 1 {
        if !args[1].is_nil() {
            return Err(RubyError::argument("Currently, 2nd arg must be Nil."));
        }
    }
    let path = if args.len() > 2 {
        let mut arg2 = args[2];
        let name = arg2.expect_string("3rd arg")?;
        std::path::PathBuf::from(name)
    } else {
        std::path::PathBuf::from("(eval)")
    };

    let method = vm.parse_program_eval(path, program)?;
    let args = Args::new0();
    let outer = vm.current_context();
    let res = vm.eval_block(&Block::Block(method, outer), &args)?;
    Ok(res)
}

fn to_enum(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    if args.block.is_some() {
        return Err(RubyError::argument("Curently, block is not allowed."));
    };
    let outer = vm.current_context();
    let (method, new_args) = if args.len() == 0 {
        let method = IdentId::EACH;
        let mut new_args = Args::new0();
        new_args.block = Block::Block(*METHODREF_ENUM, outer);
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
        new_args.block = Block::Block(*METHODREF_ENUM, outer);
        (method, new_args)
    };
    let val = vm.create_enumerator(method, self_val, new_args)?;
    Ok(val)
}

fn respond_to(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    args.check_args_range(1, 1)?;
    let method = args[0].expect_string_or_symbol("1st arg")?;
    let b = vm
        .globals
        .find_method_from_receiver(self_val, method)
        .is_some();
    Ok(Value::bool(b))
}

fn instance_exec(vm: &mut VM, self_val: Value, args: &Args) -> VMResult {
    let block = args.expect_block()?;
    vm.eval_block_self(block, self_val, args)
}

#[cfg(test)]
mod test {
    use crate::test::*;

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
        assert("foo", obj.instance_variable_get(:@foo))

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
    fn object_eval() {
        let program = r#"
        a = 100
        eval("b = 100; assert(100, b);")
        assert(77, eval("a = 77"))
        assert(77, a)
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
