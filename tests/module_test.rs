#![feature(test)]
extern crate ruruby;
extern crate test;
use ruruby::test::eval_script;
use ruruby::vm::value::Value;

#[test]
fn attr_accessor() {
    let program = "
    class Foo
        attr_accessor :car, :cdr
    end
    bar = Foo.new
    assert nil, bar.car
    assert nil, bar.cdr
    bar.car = 1000
    bar.cdr = :something
    assert 1000, bar.car
    assert :something, bar.cdr
    ";
    let expected = Value::Nil;
    eval_script(program, expected);
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
        return false if a - b != []
        return false if b - a != []
        true
    end
    assert(true, ary_cmp(A.constants, [:Bar, :Foo]))
    assert(true, ary_cmp(A.instance_methods - Class.instance_methods, [:fn, :fo]))
    "#;
    let expected = Value::Nil;
    eval_script(program, expected);
}
