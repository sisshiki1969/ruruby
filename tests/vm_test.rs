#![feature(test)]
#![allow(unused_imports, dead_code)]
extern crate ruruby;
extern crate test;
use ruruby::lexer::Lexer;
use ruruby::parser::{LvarCollector, Parser};
use ruruby::test::{assert_script, eval_script};
use ruruby::vm::value::RValue;
use ruruby::vm::*;
use test::Bencher;

#[test]
fn expr1() {
    let program = "4*(4+7*3)-95";
    let expected = RValue::FixNum(5);
    eval_script(program, expected);
}

#[test]
fn expr2() {
    let program = "2.0 + 4.0";
    let expected = RValue::FloatNum(6.0);
    eval_script(program, expected);
}

#[test]
fn expr3() {
    let program = "5.0 / 2";
    let expected = RValue::FloatNum(2.5);
    eval_script(program, expected);
}

#[test]
fn expr4() {
    let program = "15<<30";
    let expected = RValue::FixNum(16106127360);
    eval_script(program, expected);
}

#[test]
fn expr5() {
    let program = "23456>>3";
    let expected = RValue::FixNum(2932);
    eval_script(program, expected);
}

#[test]
fn expr6() {
    let program = "24+17 >> 3 == 5";
    let expected = RValue::Bool(true);
    eval_script(program, expected);
}
#[test]
fn expr7() {
    let program = "864 == 3+24<<5";
    let expected = RValue::Bool(true);
    eval_script(program, expected);
}

#[test]
fn expr8() {
    let program = "
        assert(320, 12745&854)
        assert(100799, 2486|98331)
        assert(1033, 8227^9258)
        assert(201, -275&475)
        assert(-1301, 487555|-25879)
        ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn expr9() {
    let program = "
        a=19
        a==17?23*45:14+7
        ";
    let expected = RValue::FixNum(21);
    eval_script(program, expected);
}

#[test]
fn expr10() {
    let program = r#"
        assert(3984, 12736%4376)
        assert(-392, 12736%-4376)
        assert(-3984, -12736%-4376)
        assert(392, -12736%4376)
        assert(26.603399999999937, 654.6234%34.89)
        assert(-8.286600000000064, 654.6234%-34.89)
        assert(8.286600000000064, -654.6234%34.89)
        assert(-26.603399999999937, -654.6234%-34.89)

        assert(-101, ~100)
        assert(44, ~-45)

        assert(true, !nil)
        assert(true, !false)
        assert(false, !true)
        assert(false, !0)
        assert(false, !"windows")
        "#;
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn op1() {
    let program = "4==5";
    let expected = RValue::Bool(false);
    eval_script(program, expected);
}

#[test]
fn op2() {
    let program = "4!=5";
    let expected = RValue::Bool(true);
    eval_script(program, expected);
}

#[test]
fn op3() {
    let program = "
        assert(true, nil==nil)
        assert(true, 4.0==4)
        assert(true, 4==4.0)
        assert(true, 12345678==12345678)
        assert(true, 1234.5678==1234.5678)
        ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn op4() {
    let program = "
        assert(false, nil!=nil)
        assert(false, 4.0!=4)
        assert(false, 4!=4.0)
        assert(false, 12345678!=12345678)
        assert(false, 1234.5678!=1234.5678)
        ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn op9() {
    let program = "
        assert(4, 4 || 5)
        assert(4, 4 || nil)
        assert(4, nil || 4)
        assert(nil, nil || nil)
        ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn op10() {
    let program = "4==4 && 4!=5 && 3<4 && 5>4 && 4<=4 && 4>=4";
    let expected = RValue::Bool(true);
    eval_script(program, expected);
}

#[test]
fn int1() {
    let i1 = 0x3fff_ffff_ffff_ffffu64 as i64;
    let i2 = 0x4000_0000_0000_0005u64 as i64;
    let program = format!("{}+6=={}", i1, i2);
    let expected = RValue::Bool(true);
    eval_script(program, expected);
}

#[test]
fn int2() {
    let i1 = 0x3fff_ffff_ffff_ffffu64 as i64;
    let i2 = 0x4000_0000_0000_0005u64 as i64;
    let program = format!("{}-6=={}", i2, i1);
    let expected = RValue::Bool(true);
    eval_script(program, expected);
}

#[test]
fn int3() {
    let i1 = 0xbfff_ffff_ffff_ffffu64 as i64;
    let i2 = 0xc000_0000_0000_0005u64 as i64;
    let program = format!("{}+6=={}", i1, i2);
    let expected = RValue::Bool(true);
    eval_script(program, expected);
}

#[test]
fn int4() {
    let i1 = 0xbfff_ffff_ffff_ffffu64 as i64;
    let i2 = 0xc000_0000_0000_0005u64 as i64;
    let program = format!("{}-6=={}", i2, i1);
    let expected = RValue::Bool(true);
    eval_script(program, expected);
}

#[test]
fn int_index() {
    let program = "
        i = 0b0100_1101
        assert(0, i[-5])
        assert(1, i[0])
        assert(0, i[1])
        assert(1, i[2])
        assert(1, i[3])
        assert(0, i[4])
        assert(0, i[5])
        assert(1, i[6])
        assert(0, i[7])
        assert(0, i[700])
    ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn objects() {
    let program = r#"
        assert(nil, Object.superclass)
        assert(Object, Module.superclass)
        assert(Module, Class.superclass)
        assert(Object, Integer.superclass)
        assert(Object, Regexp.superclass)
        assert(Object, String.superclass)
        assert(Object, Range.superclass)
        assert(Object, Proc.superclass)
        assert(Object, Method.superclass)

        assert(Class, Module.class)
        assert(Class, Class.class)
        assert(Class, Integer.class)
        assert(Class, Regexp.class)
        assert(Class, String.class)
        assert(Class, Range.class)
        assert(Class, Proc.class)
        assert(Class, Method.class)

        assert(Regexp, Regexp.new("a").class)
    "#;
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn triple_equal() {
    let program = r#"
        assert(true, 1 === 1)
        assert(false, 1 === 2)
        assert(false, "a" === 2)
        assert(false, 2 === "a")
        assert(false, "ruby" === "rust")
        assert(true, "ruby" === "ruby")
        assert(true, Integer === 100)
        assert(false, Integer === "ruby")
        assert(true, String === "ruby")
        assert(false, String === 100)
    "#;
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn if1() {
    let program = "if 5*4==16 +4 then 4;2*3+1 end";
    let expected = RValue::FixNum(7);
    eval_script(program, expected);
}

#[test]
fn if2() {
    let program = "if 
        5*4 ==16 +
        4
        3*3
        -2 end";
    let expected = RValue::FixNum(-2);
    eval_script(program, expected);
}

#[test]
fn if3() {
    let program = "if 5*9==16 +4
        7 elsif 4==4+9 then 8 elsif 3==1+2 then 10
        else 12 end";
    let expected = RValue::FixNum(10);
    eval_script(program, expected);
}

#[test]
fn if4() {
    let program = "if
            1+
            2==
            3
            4
            5
            end";
    let expected = RValue::FixNum(5);
    eval_script(program, expected);
}

#[test]
fn if5() {
    let program = "a = 77 if 1+2 == 3";
    let expected = RValue::FixNum(77);
    eval_script(program, expected);
}

#[test]
fn if6() {
    let program = "a = 77 if 1+3 == 3";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn unless1() {
    let program = "a = 5; unless a > 3 then 10 else 50 end";
    let expected = RValue::FixNum(50);
    eval_script(program, expected);
}

#[test]
fn unless2() {
    let program = "a = 5; unless a < 3 then 10 else 50 end";
    let expected = RValue::FixNum(10);
    eval_script(program, expected);
}

#[test]
fn unless3() {
    let program = "a = 5; a = 7 unless a == 3; a";
    let expected = RValue::FixNum(7);
    eval_script(program, expected);
}

#[test]
fn unless4() {
    let program = "a = 5; a = 7 unless a == 5; a";
    let expected = RValue::FixNum(5);
    eval_script(program, expected);
}

#[test]
fn for1() {
    let program = "
            y = 0
            for x in 0..9
            y=y+x
            end
            y";
    let expected = RValue::FixNum(45);
    eval_script(program, expected);
}

#[test]
fn for2() {
    let program = "
            y = 0
            for x in 0...9
            y=y+x
            end
            y";
    let expected = RValue::FixNum(36);
    eval_script(program, expected);
}

#[test]
fn for3() {
    let program = "
        y = 0
        for x in 0..9
            if x == 5 then break end
            y=y+x
        end
        y";
    let expected = RValue::FixNum(10);
    eval_script(program, expected);
}

#[test]
fn for4() {
    let program = "
        y = 0
        for x in 0..9
            if x == 5 then next end
            y=y+x
        end
        y";
    let expected = RValue::FixNum(40);
    eval_script(program, expected);
}

#[test]
fn for5() {
    let program = "
        assert(for a in 0..2 do end, 0..2)
        assert(for a in 0..2 do if a == 1 then break end end, nil)
    ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn while1() {
    let program = "
        assert((a = 0; while a < 5 do puts a; a+=1 end; a), 5)
        assert((a = 0; while a < 5 do puts a; break if a == 3; a+=1 end; a), 3)
        assert((a = 0; while a < 5 do puts a; a+=1 end), nil)
        assert((a = 0; while a < 5 do puts a; break if a == 3; a+=1 end), nil)
    ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn while2() {
    let program = "
        assert((a = 0; a+=1 while a < 5; a), 5)
    ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn until1() {
    let program = "
        assert((a = 0; until a == 4 do puts a; a+=1 end; a), 4)
        assert((a = 0; until a == 4 do puts a; break if a == 3; a+=1 end; a), 3)
        assert((a = 0; until a == 4 do puts a; a+=1 end), nil)
        assert((a = 0; until a == 4 do puts a; break if a == 3; a+=1 end), nil)
    ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn until2() {
    let program = "
        assert((a = 0; a+=1 until a == 5; a), 5)
    ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn proc_next() {
    let program = "
        p = Proc.new { |x|
            next 100 if x == 7
            200
        }
        assert(200, p.call(1))
        assert(100, p.call(7))
    ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn proc_return() {
    let program = "
        def func(y)
            [1,2,3,4].each do |x|
                return 100 if x == y
            end
            0
        end

        assert(100, func(3))
        assert(0, func(7))
        ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn local_var1() {
    let program = "
            ruby = 7
            mruby = (ruby - 4) * 5
            mruby - ruby";
    let expected = RValue::FixNum(8);
    eval_script(program, expected);
}

#[test]
fn global_var() {
    let program = "
            class A
                $global = 1250
            end
            class B
                class C
                    assert(1250, $global)
                end
            end
            ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn mul_assign1() {
    let program = "
            a,b,c = 1,2,3
            assert(1,a)
            assert(2,b)
            assert(3,c)
            ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn mul_assign2() {
    let program = "
            d,e = 1,2,3,4
            assert(1,d)
            assert(2,e)
            ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn mul_assign3() {
    let program = "
            f,g,h = 1,2
            assert(1,f)
            assert(2,g)
            assert(nil,h)
            ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn mul_assign4() {
    let program = "
            f = 1,2,3
            assert([1,2,3],f)
            assert([1,2,3],(f=1,2,3))
            ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn mul_assign5() {
    let program = "
            d = (a,b,c = [1,2])
            assert([a,b,c],[1,2,nil])
            assert(d,[1,2])
            ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn mul_assign6() {
    let program = "
            d = (a,b,c = [1,2,3,4,5])
            assert([a,b,c],[1,2,3])
            assert(d,[1,2,3,4,5])
            ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn mul_assign7() {
    let program = "
        a = [1,2,3]
        b = 5,*a,5
        c,d,e,f,g,h = *b
        assert(b,[5,1,2,3,5])
        assert([c,d,e,f,g,h],[5,1,2,3,5,nil])
        a = *[1,2,3]
        assert(a,[1,2,3])
        ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn const1() {
    let program = "
            Ruby = 777
            Ruby = Ruby * 2
            Ruby / 111";
    let expected = RValue::FixNum(14);
    eval_script(program, expected);
}

#[test]
fn const2() {
    let program = "
            BOO = 100
            class Foo
                FOO = 222
                assert 100, BOO
                def foo
                    assert 333, ::Bar::BAR
                end
            end
            class Bar
                BAR = 333
                assert 100, BOO
                assert 222, ::Foo::FOO
            end
            Foo.new.foo
    ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn range1() {
    let program = "
    assert(Range.new(5,10), 5..10)
    assert(Range.new(5,10, false), 5..10)
    assert(Range.new(5,10, true), 5...10)";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn range2() {
    let program = "
    assert(Range.new(5,10).first, 5)
    assert(Range.new(5,10).first(4), [5,6,7,8])
    assert(Range.new(5,10).first(100), [5,6,7,8,9,10])
    assert(Range.new(5,10,true).first(4), [5,6,7,8])
    assert(Range.new(5,10,true).first(100), [5,6,7,8,9])
    assert(Range.new(5,10).last, 10)
    assert(Range.new(5,10).last(4), [7,8,9,10])
    assert(Range.new(5,10).last(100), [5,6,7,8,9,10])
    assert(Range.new(5,10,true).last(4), [6,7,8,9])
    assert(Range.new(5,10,true).last(100), [5,6,7,8,9])";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn regexp1() {
    let program = r#"
    assert("abc!!g", "abcdefg".gsub(/def/, "!!"))
    assert("2.5".gsub(".", ","), "2,5")
    "#;
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn method1() {
    let program = r#"
    class Foo
        def foo(arg)
            "instance #{arg}"
        end
        def self.boo(arg)
            "class #{arg}"
        end
    end
    m = Foo.new.method(:foo)
    assert("instance 77", m.call(77))
    m = Foo.method(:boo)
    assert("class 99", m.call(99))
    "#;
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn local_scope() {
    let program = "
        a = 1
        class Foo
            a = 2
            def bar
                a = 3
                a
            end
            def boo(x)
                x * 2
            end
            assert(2,a)
        end
        assert(1,a)
        assert(3,Foo.new.bar)
        assert(10,Foo.new.boo(5))
        assert(1,a)";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn class1() {
    let program = "
        class Vec
            assert(Vec, self)
            def len(x,y)
                def sq(x)
                    x*x
                end
                sq(x)+sq(y)
            end
        end

        Vec.new.len(3,4)";
    let expected = RValue::FixNum(25);
    eval_script(program, expected);
}

#[test]
fn class2() {
    let program = "
        class Vec
            @xxx=100
            def set_xxx(x); @xxx = x; end
            def len(x,y)
                def sq(x); x*x; end
                sq(x)+sq(y)
            end
            def get_xxx; @xxx; end
            def self.get_xxx; @xxx = @xxx + 1; @xxx; end
        end

        foo1 = Vec.new
        assert(25, foo1.len(3,4))
        foo1.set_xxx(777)
        assert(777, foo1.get_xxx)
        foo2 = Vec.new
        foo2.set_xxx(999)
        assert(777, foo1.get_xxx)
        assert(999, foo2.get_xxx)
        assert(nil, Vec.new.get_xxx)
        assert(101, Vec.get_xxx)
        assert(102, Vec.get_xxx)
        assert(103, Vec.get_xxx)";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn class3() {
    let program = "
        class Foo
        end
        class Bar < Foo
        end
        assert(Foo, Bar.superclass)
        assert(Object, Bar.superclass.superclass)
        assert(Class, Bar.class)
        assert(Bar, Bar.new.class)
        assert(Integer, -3456.class)";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn initialize() {
    let program = "
    class Vec
        def initialize(x,y)
            @x=x;@y=y
        end
        def add(v)
            Vec.new(@x + v.x, @y + v.y)
        end
        def x; @x; end
        def y; @y; end
    end

    v1 = Vec.new(3, 5.9)
    assert(3, v1.x)
    assert(5.9, v1.y)
    v2 = v1.add(Vec.new(4.7, 8))
    assert(7.7, v2.x)
    assert(13.9, v2.y)";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn define_binop() {
    let program = "
    class Vec
        def initialize(x,y)
            @x=x;@y=y
        end
        def +(v)
            Vec.new(@x + v.x, @y + v.y)
        end
        def x; @x; end
        def y; @y; end
    end

    v1 = Vec.new(2,4)
    v2 = Vec.new(3,5)
    v = v1 + v2;
    assert v.x, 5
    assert v.y, 9
    ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn lambda_literal() {
    let program = "
        f0 = ->{100}
        f1 = ->x{x*6}
        f2 = ->(x,y){x*y}
        assert 100, f0.call
        assert 300, f1.call(50)
        assert 35, f2.call(5,7)";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn closure1() {
    let program = "
        def inc
            a = 100
            ->{a = a + 1; a}
        end

        assert 101, inc.call
        assert 101, inc.call
        assert 101, inc.call

        p = inc()
        assert 101, p.call
        assert 102, p.call
        assert 103, p.call";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn closure2() {
    let program = "
        a = 5;
        f = ->{ ->{ ->{ a } } }
        assert 5, f.call.call.call
        a = 7;
        assert 7, f.call.call.call";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn closure3() {
    let program = r#"
    def func
        a = 77 
        1.times {
            1.times {
                return Proc.new{
                    a = a + 1
                }
            }
        }
    end

    f = func
    assert 78, f.call
    assert 79, f.call
    assert 80, f.call
    assert 78, func.call
    "#;
    assert_script(program);
}

#[test]
fn method_chain1() {
    let program = "
        class Foo
            attr_accessor :a
            def initialize
                @a = 0
            end
            def inc
                @a = @a + 1
                self
            end
        end

        ans1 = Foo.new
            .inc
            .inc
            .a
        assert 2, ans1
        ans2 = Foo.new
            .inc()
            .inc()
            .a
        assert 2, ans2";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn method_chain2() {
    let program = "
        class Array
            def map(&fun)
                a = []
                for i in 0...self.length
                    a.push fun.call(self[i])
                end
                a
            end
        end
        a = 3
        assert [4,7,12,19], [1,2,3,4].map do |x| x*x+a end
        a = 1
        assert [2,5,10,17], [1,2,3,4].map do |x| x*x+a end
        assert [4,4,4,4], [1,2,3,4].map do || 4 end
        assert [7,7,7,7], [1,2,3,4].map do | | 7 end
        ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn assign_op() {
    let program = "
        a = 10
        assert 15, a+=5
        assert 9, a-=6
        assert 3, a/=3
        assert 30, a*=10
        assert 120, a<<=2
        assert 7, a>>=4
        assert 2, a&=2
        assert 11, a|=9
        ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn singleton() {
    let program = "
    class Foo
        def init
            def self.single
                77
            end
        end
        def single
            99
        end
    end

    f = Foo.new
    assert(99, f.single)
    f.init
    assert(77, f.single)
    class Foo
        def single
            200
        end
    end
    assert(77, f.single)
    assert(200, Foo.new.single)
        ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}

#[test]
fn is_a() {
    let program = "
        module M
        end
        class C
        end
        class S < C
        end

        obj = S.new
        assert true, obj.is_a?(S)
        assert true, obj.is_a?(C)
        assert true, obj.is_a?(Object)
        assert false, obj.is_a?(Integer)
        assert false, obj.is_a?(Array)
        assert false, obj.is_a?(M)
        ";
    let expected = RValue::Nil;
    eval_script(program, expected);
}
