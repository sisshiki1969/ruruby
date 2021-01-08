#![feature(test)]
extern crate ruruby;
use ruruby::test::*;

#[test]
fn superclass() {
    let program = r#"
        assert(nil, BasicObject.superclass)
        assert(BasicObject, Object.superclass)
        assert(Object, Module.superclass)
        assert(Module, Class.superclass)
        assert(Object, Numeric.superclass)
        assert(Numeric, Integer.superclass)
        assert(Numeric, Float.superclass)
        assert(Numeric, Complex.superclass)
        assert(Object, NilClass.superclass)
        assert(Object, TrueClass.superclass)
        assert(Object, FalseClass.superclass)
        assert(Object, Array.superclass)
        assert(Object, Symbol.superclass)
        assert(Object, Regexp.superclass)
        assert(Object, String.superclass)
        assert(Object, Hash.superclass)
        assert(Object, Range.superclass)
        assert(Object, Proc.superclass)
        assert(Object, Method.superclass)
        assert(Object, Fiber.superclass)
        assert(Object, Enumerator.superclass)
        assert(Object, Exception.superclass)
        "#;
    assert_script(program);
}

#[test]
fn class1() {
    let program = r#"
        assert(Class, Object.class)
        assert(Class, BasicObject.class)
        assert(Class, Module.class)
        assert(Class, Class.class)
        assert(Class, TrueClass.class)
        assert(Class, FalseClass.class)
        assert(Class, NilClass.class)
        assert(Class, Integer.class)
        assert(Class, Regexp.class)
        assert(Class, String.class)
        assert(Class, Range.class)
        assert(Class, Proc.class)
        assert(Class, Method.class)
        "#;
    assert_script(program);
}

#[test]
fn class2() {
    let program = r#"
        assert(NilClass, nil.class)
        assert(TrueClass, true.class)
        assert(FalseClass, false.class)
        assert(Integer, 5.class)
        assert(Float, (5.0).class)
        assert(Complex, (1 + 3i).class)

        assert([Integer, Numeric, Comparable, Object, Kernel, BasicObject], Integer.ancestors)
        assert([Float, Numeric, Comparable, Object, Kernel, BasicObject], Float.ancestors)
        assert([Comparable, Kernel], Integer.included_modules)
        assert([Kernel], Object.included_modules)
        assert([Kernel], Class.included_modules)

        assert(Regexp, Regexp.new("a").class)
    "#;
    assert_script(program);
}

#[test]
fn singleton() {
    let program = r#"
        assert(Class, Class.singleton_class.class)
        assert(Class.singleton_class, Class.singleton_class)
        assert(Module.singleton_class, Class.singleton_class.superclass)
        assert(Object.singleton_class, Class.singleton_class.superclass.superclass)
        assert(BasicObject.singleton_class, Class.singleton_class.superclass.superclass.superclass)
        "#;
    assert_script(program);
}

#[test]
fn class_name() {
    let program = r##"
        class A; end
        assert "A", A.to_s
        puts A.singleton_class.to_s
        #assert 0, A.singleton_class.to_s =~ /#<Class:A>/
        c = Class.new
        puts c.to_s
        assert 0, c.to_s =~ /#<Class:0x.{16}>/
        #assert 0, c.singleton_class.to_s =~ /#<Class:#<Class:0x.{16}>>/
        C = c
        assert "C", c.to_s
        #assert "#<Class:C>", c.singleton_class.to_s
        "##;
    assert_script(program);
}
