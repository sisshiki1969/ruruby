#![feature(test)]
extern crate ruruby;
extern crate test;
use ruruby::tests::*;
use ruruby::*;
use test::bench::Bencher;

#[test]
fn func1() {
    let program = "
        def func(a,b,c)
            a+b+c
        end

        func(1,2,3)";
    let expected = Value::integer(6);
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
    let expected = Value::integer(120);
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
fn argument_delegete() {
    let program = r##"
        def f(a,b,...)
          assert 1, a
          assert 2, b
        end
        f(1,2,3,4)
        f(1,2,3)
        f(1,2)
        assert_error {f(1)}

        assert_error {
          eval "f(...)"
        }
        def g(...)
          h(...)
        end
        def h(*x)
          x
        end
        assert [1,2,3], g(1,2,3)
        def g(a,b,...)
            h(...)
        end
        assert [3,4], g(1,2,3,4)
    "##;
    assert_script(program);
}

#[test]
fn optional_param() {
    let program = "
        def fn(a = 0, b = 1, c = 2) [a,b,c] end
    
        assert([0,1,2], fn())
        assert([5,1,2], fn(5))
        assert([5,7,2], fn(5,7))
        assert([5,7,10], fn(5,7,10))

        def fx(a, b = 1, c = 2) [a,b,c] end

        assert([5,1,2], fx(5))
        assert([5,7,2], fx(5,7))
        assert([5,7,10], fx(5,7,10))
        ";
    assert_script(program);
}

#[test]
fn parameters() {
    let program = "
        def fn(a,b,c,d,e=100,f=77,*g,h,i,kw:100, &p)
            [a,b,c,d,e,f,g,h,i,kw,p&.call]
        end

        assert([1,2,3,4,5,6,[7,8],9,10,100,nil], fn(1,2,3,4,5,6,7,8,9,10))
        assert([1,2,3,4,100,77,[],5,6,100,nil], fn(1,2,3,4,5,6))
        assert([1,2,3,4,100,77,[],5,6,88,nil], fn(1,2,3,4,5,6,kw:88))
        assert([1,2,3,4,5,6,[7,8],9,10,55,nil], fn(1,2,3,4,5,6,7,8,9,10,kw:55))

        p = Proc.new{42}
        assert([1,2,3,4,5,6,[7,8],9,10,100,42], fn(1,2,3,4,5,6,7,8,9,10,&p))
        assert([1,2,3,4,100,77,[],5,6,100,42], fn(1,2,3,4,5,6,&p))
        assert([1,2,3,4,100,77,[],5,6,88,42], fn(1,2,3,4,5,6,kw:88,&p))
        assert([1,2,3,4,5,6,[7,8],9,10,55,42], fn(1,2,3,4,5,6,7,8,9,10,kw:55,&p))
        ";
    assert_script(program);
}

#[test]
fn keyword_arguments() {
    let program = "
        def f(a,b=0,*c,d)
            [a,b,c,d]
        end
        assert [0,1,[2],{x:0,y:1,z:2}], f(0,1,2,x:0,y:1,z:2)

        def f(a,b=0,c)
            [a,b,c]
        end
        assert [0,1,{x:0,y:1,z:2}], f(0,1,x:0,y:1,z:2)

        def f(a,b=0,*c)
            [a,b,c]
        end
        assert [0,1,[{x:0,y:1,z:2}]], f(0,1,x:0,y:1,z:2)

        def f(a,b=0,*c)
            [a,b,c]
        end
        assert [0,1,[2,{x:0,y:1,z:2}]], f(0,1,2,x:0,y:1,z:2)

        def f(a,b=3)
            [a,b]
        end
        assert [0,{x:0,y:1,z:2}], f(0,x:0,y:1,z:2)
        ";
    assert_script(program);
}

#[test]
fn rest_parameter() {
    let program = "
        def fn(a,b,*,c,d)
            [a,b,c,d]
        end
        assert([0,1,4,5], fn(0,1,2,3,4,5))
        ";
    assert_script(program);
}

#[test]
fn kwrest_parameters() {
    let program = "
        def fn(a, *b, **c)
            [a, b, c]
        end
        def gn(a, *b, kw2:50, **c)
            [a, b, kw2, c]
        end
    
        assert([1,[],{}], fn(1))
        assert([1,[2],{}], fn(1,2))
        assert([1,[2,3],{}], fn(1,2,3))
        assert([1,[2,3,4],{kw1:77,kw2:88}], fn(1,2,3,4,kw1:77,kw2:88))
        assert([1,[2,3,4],88,{kw1:77}], gn(1,2,3,4,kw1:77,kw2:88))
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
fn block_yield() {
    let program = r#"
        class A
          def self.fn
            yield
          end
        end
        one = 100
        two = 200
        block = Proc.new { one }
        assert_error { A.fn }
        assert 200, A.fn { two }
        assert self, A.fn { self }
        assert 100, A.fn(&block)
    "#;
    assert_script(program);
}

#[test]
fn block_capture() {
    let program = r#"
        def fn2(&block)
            block.call
        end
        def fn1(&block)
            fn2(&block)
        end
        x = 100
        y = 200
        block = Proc.new { x }
        assert_error { fn }
        assert 200, fn2(){ y }
        assert 100, fn2(&block)
        assert 200, fn1(){ y }
        assert 100, fn1(&block)
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
    // https://docs.ruby-lang.org/ja/latest/doc/spec=2fcall.html
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
fn double_splat_argument() {
    // https://docs.ruby-lang.org/ja/latest/doc/spec=2fcall.html#
    let program = r#"
        def foo(**param)
            param
        end

        assert ({}), foo(**{})
        assert ({a:2,b:3}), foo(a:2,b:3)
        assert ({a:2,b:3}), foo(**{a:2,b:3})
        assert ({a:2,b:3,c:1}), foo(**{a:2,b:3},c:1)
    "#;
    assert_script(program);
}

#[test]
fn intrinsic_conversion_to_hash() {
    let program = r#"
        def foo(*p)
            p
        end

        assert [1,2,3], foo(1,2,3)
        assert [1,2,{:abs=>"abs", :kvm=>"kvm"}], foo(1,2,:abs=>"abs",:kvm=>"kvm")
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

#[test]
fn paren() {
    let program = r#"
        assert_error { eval("p (7, 8)") }
        assert(7, p(7))
        assert(7, p (7))
        assert([7, 8], p 7, 8)
        assert([7, 8], p(7, 8))
    "#;
    assert_script(program);
}

#[test]
fn command() {
    let program = r#"
        def foo(*x)
            x
        end
        assert_error { eval("p (7, 8)") }
        assert([:a], foo :a)
        assert([1], foo +1)
        assert([-1], foo -1)
        assert([1,2], foo *[1,2])
        assert([["We","love","Ruby"]], foo %w!We love Ruby!)
    "#;
    assert_script(program);
}

#[test]
fn func_name_extention() {
    let program = r#"
        def while!
            88
        end
        def while?
            99
        end
        assert 88, while!
        assert 99, while?
        foo = 100
        def foo!
            77
        end
        def foo?
            66
        end
        assert(77, foo!)
        assert(66, foo?)
    "#;
    assert_script(program);
}

#[test]
fn singleton_method() {
    let program = r#"
        obj = Object.new
        def obj.def
            42
        end
        assert(42, obj.def)
    "#;
    assert_script(program);
}

#[test]
fn assign_like_method() {
    let program = r#"
        class Foo
            def foo=(val)
                @foo=val
            end
            def foo
                @foo
            end
        end
        f=Foo.new
        f.foo=77
        assert(77, f.foo)
    "#;
    assert_script(program);
}

#[test]
fn op_method() {
    let program = r#"
    class Foo
        attr_accessor :x
        def initialize(x)
            @x = x
        end
        def +(other)
            self.x + other.x
        end
        def -(other)
            self.x - other.x
        end
        def *(other)
            self.x * other.x
        end
        def ==(other)
            self.x == other.x
        end
        def !=(other)
            self.x != other.x
        end
        def <=(other)
            self.x <= other.x
        end
        def >=(other)
            self.x >= other.x
        end
        def <<(other)
            self.x << other.x
        end
        def >>(other)
            self.x >> other.x
        end
        def [](idx)
            self.x[idx]
        end
        def []=(idx, val)
            self.x[idx] = val
        end
    end
    assert 100, Foo.new(25) + Foo.new(75)
    assert 50, Foo.new(75) - Foo.new(25)
    assert 75, Foo.new(25) * Foo.new(3)
    assert true, Foo.new(25) == Foo.new(25)
    assert false, Foo.new(26) == Foo.new(25)
    assert false, Foo.new(25) != Foo.new(25)
    assert true, Foo.new(26) != Foo.new(25)
    assert true, Foo.new(25) >= Foo.new(25)
    assert false, Foo.new(24) >= Foo.new(25)
    assert true, Foo.new(25) <= Foo.new(25)
    assert false, Foo.new(26) <= Foo.new(25)
    assert 6, Foo.new(25) >> Foo.new(2)
    assert 100, Foo.new(25) << Foo.new(2)
    assert 3, Foo.new([1,2,3,4])[2]
    ary = Foo.new([5,4,3,2,1,0])
    assert 3, ary[2]
    ary[2] = 77
    assert 77, ary[2]
    "#;
    assert_script(program);
}

#[test]
fn method_missing() {
    let program = r##"
    class A
      attr_accessor :a
      def initialize(a)
        @a = a
      end
      def method_missing(method, *arg)
        "method_missing #{@a} #{method} #{arg}"
      end
    end
    
    a = A.new(100)
    b = A.new(200)
    assert "method_missing 100 amber [1, 2]", a.amber 1,2
    assert "method_missing 200 gold [3, 4]", b.gold 3,4
"##;
    assert_script(program);
}

#[test]
fn double_colon() {
    let program = r##"
    class C
      def self.fn
        100
      end
    end
    assert 100, C::fn
    "##;
    assert_script(program);
}
