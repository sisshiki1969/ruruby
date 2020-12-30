#![feature(test)]
extern crate ruruby;
use ruruby::test::*;

#[test]
fn objects() {
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
        assert(Exception, StandardError.superclass)
        assert(StandardError, RuntimeError.superclass)
        assert(RuntimeError, FrozenError.superclass)
        assert(RuntimeError, ArgumentError.superclass)

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

        assert(NilClass, nil.class)
        assert(TrueClass, true.class)
        assert(FalseClass, false.class)
        assert(Integer, 5.class)
        assert(Float, (5.0).class)
        assert(Complex, (1 + 3i).class)

        assert([Integer, Numeric, Object, Kernel, BasicObject], Integer.ancestors)
        assert([Kernel], Object.included_modules)
        assert([Kernel], Class.included_modules)

        assert(Regexp, Regexp.new("a").class)
    "#;
    assert_script(program);
}
