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

            fibo(20)";
    let expected = Value::fixnum(6765);
    b.iter(|| eval_script(program, expected.clone()));
}

#[test]
fn func4() {
    let program = "
        def fact(a)
            puts(a)
            return 1 if a == 1
            return a * fact(a-1)
        end
    
        fact(5)";
    let expected = Value::fixnum(120);
    eval_script(program, expected);
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
