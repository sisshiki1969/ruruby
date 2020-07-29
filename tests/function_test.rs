#![feature(test)]
extern crate ruruby;
extern crate test;
use ruruby::test::*;
use ruruby::*;
use test::Bencher;

#[test]
fn func1() {
    let program = "
        def func(a,b,c)
            a+b+c
        end

        func(1,2,3)";
    let expected = Value::fixnum(6);
    eval_script(program, expected);
}

#[test]
fn func2() {
    let program = "
        def fact(a)
            puts(a)
            if a == 1
                1
            else
                a * fact(a-1)
            end
        end

        fact(5)";
    let expected = Value::fixnum(120);
    eval_script(program, expected);
}

#[bench]
fn func3(b: &mut Bencher) {
    let program = "
        def fibo(x)
            if x <= 2
                1
            else
                fibo(x-1) + fibo(x-2)
            end
        end

        assert(55, fibo(10))";
    b.iter(|| assert_script(program));
}

#[bench]
fn func4(b: &mut Bencher) {
    let program = "
        def fact(a)
            return 1 if a == 1
            return a * fact(a-1)
        end

        assert(120, fact(5))";
    b.iter(|| assert_script(program));
}

#[test]
fn optional_param() {
    let program = "
        def fn(a = 0, b = 1, c = 2)
            [a,b,c]
        end
    
        assert([0,1,2], fn())
        assert([5,1,2], fn(5))
        assert([5,7,2], fn(5,7))
        assert([5,7,10], fn(5,7,10))

        def fx(a, b = 1, c = 2)
            [a,b,c]
        end

        assert([5,1,2], fx(5))
        assert([5,7,2], fx(5,7))
        assert([5,7,10], fx(5,7,10))
        ";
    assert_script(program);
}

#[test]
fn parameters() {
    let program = "
        def fn(a,b,c,d,e=100,f=77,*g,h,i,kw:100,&j)
            [a,b,c,d,e,f,g,h,i,kw]
        end
    
        assert([1,2,3,4,5,6,[7,8],9,10,100], fn(1,2,3,4,5,6,7,8,9,10))
        assert([1,2,3,4,100,77,[],5,6,100], fn(1,2,3,4,5,6))
        assert([1,2,3,4,100,77,[],5,6,88], fn(1,2,3,4,5,6,kw:88))
        assert([1,2,3,4,5,6,[7,8],9,10,55], fn(1,2,3,4,5,6,7,8,9,10,kw:55))
        ";
    assert_script(program);
}

#[test]
fn return1() {
    let program = "
        def fn
            return 1,2,3
        end
        assert(fn, [1,2,3])
        ";
    assert_script(program);
}

#[test]
fn argument_number() {
    let program = r#"
    def fn1(a,b,c); end
    def fn2; end
    def fn3(a, *b); end
    def fn4(a, b=nil, c=1); end
    fn1(1,2,3)
    fn2
    fn3(1)
    fn3(1,2,3)
    fn4(1)
    fn4(1,2)
    fn4(1,2,3)
    assert_error { fn1 }
    assert_error { fn1(1,2,3,4) }
    assert_error { fn2(1) }
    assert_error { fn3 }
    assert_error { fn4 }
    assert_error { fn4(1,2,3,4) }
    "#;
    assert_script(program);
}

#[test]
fn block_argument() {
    let program = r#"
        block = Proc.new {|x| x.upcase }
        assert ["THESE", "ARE", "PENCILS"], ["These", "are", "pencils"].map(&block)
    "#;
    assert_script(program);
}

#[test]
fn splat_argument() {
    // https://docs.ruby-lang.org/ja/latest/doc/spec=2fcall.html#block_arg
    let program = r#"
        def foo(*param)
            param
        end

        assert [1, 2, 3, 4], foo(1, *[2, 3, 4])
        assert [1], foo(1, *[])
        assert [1, 2, 3, 4, 5], foo(1, *[2, 3, 4], 5)
        assert [1, 2, 3, 4, 5, 6], foo(1, *[2, 3, 4], 5, *[6])
    "#;
    assert_script(program);
}

#[test]
fn safe_navigation() {
    // https://docs.ruby-lang.org/ja/latest/doc/spec=2fcall.html#block_arg
    let program = r#"
        a = nil
        class C
            def foo
                4
            end
        end
        assert(nil, a&.foo)
        assert(4, C.new&.foo)
    "#;
    assert_script(program);
}
