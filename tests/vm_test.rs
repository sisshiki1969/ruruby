#![feature(test)]
#![allow(unused_imports, dead_code)]
extern crate ruruby;
use ruruby::parse::Lexer;
use ruruby::test::*;
use ruruby::*;

#[test]
fn expr1() {
    let program = "4*(4+7*3)-95";
    let expected = Value::integer(5);
    eval_script(program, expected);
}

#[test]
fn expr2() {
    let program = "2.0 + 4.0";
    let expected = Value::float(6.0);
    eval_script(program, expected);
}

#[test]
fn expr3() {
    let program = "5.0 / 2";
    let expected = Value::float(2.5);
    eval_script(program, expected);
}

#[test]
fn expr4() {
    let program = "15<<30";
    let expected = Value::integer(16106127360);
    eval_script(program, expected);
}

#[test]
fn expr5() {
    let program = "23456>>3";
    let expected = Value::integer(2932);
    eval_script(program, expected);
}

#[test]
fn expr6() {
    let program = "24+17 >> 3 == 5";
    let expected = Value::bool(true);
    eval_script(program, expected);
}
#[test]
fn expr7() {
    let program = "864 == 3+24<<5";
    let expected = Value::bool(true);
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
        a = 854
        assert(320, 12745&a)
        a = 98331
        assert(100799, 2486|a)
        a = 9258
        assert(1033, 8227^a)
        a = 475
        assert(201, -275&a)
        a = -25879
        assert(-1301, 487555|a)
        ";
    assert_script(program);
}

#[test]
fn expr9() {
    let program = "
        a=19
        a==17?23*45:14+7
        ";
    let expected = Value::integer(21);
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
        assert(256, 4**4)

        assert(-101, ~100)
        assert(44, ~-45)

        assert(true, !nil)
        assert(true, !false)
        assert(false, !true)
        assert(false, !0)
        assert(false, !"windows")
        "#;
    assert_script(program);
}

#[test]
fn expr11() {
    let program = r#"
        assert(true, true || false && false)
        assert(false, (true or false and false))
        assert(false, true^5)
        assert(false, true^true)
        assert(true, true^false)
        assert(true, true^nil)
        assert(true, false^5)
        assert(true, false^true)
        assert(false, false^false)
        assert(false, false^nil)
    "#;
    assert_script(program);
}

#[test]
fn op1() {
    let program = "4==5";
    let expected = Value::bool(false);
    eval_script(program, expected);
}

#[test]
fn op2() {
    let program = "4!=5";
    let expected = Value::bool(true);
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
    assert_script(program);
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
    assert_script(program);
}

#[test]
fn op5() {
    let program = "
        a = 42
        assert(true, a == 42)
        assert(false, a == 43)
        assert(false, a != 42)
        assert(true, a != 43)

        assert(true, a <= 43)
        assert(true, a <= 42)
        assert(false, a <= 41)
        assert(true, a < 43)
        assert(false, a < 42)
        assert(false, a < 41)
        assert(false, a >= 43)
        assert(true, a >= 42)
        assert(true, a >= 41)
        assert(false, a > 43)
        assert(false, a > 42)
        assert(true, a > 41)
        ";
    assert_script(program);
}

#[test]
fn op6() {
    let program = "
        a = 42
        assert(true, a == 42.0)
        assert(false, a == 43.0)
        assert(false, a != 42.0)
        assert(true, a != 43.0)

        assert(true, a <= 43.0)
        assert(true, a <= 42.0)
        assert(false, a <= 41.0)
        assert(true, a < 43.0)
        assert(false, a < 42.0)
        assert(false, a < 41.0)
        assert(false, a >= 43.0)
        assert(true, a >= 42.0)
        assert(true, a >= 41.0)
        assert(false, a > 43.0)
        assert(false, a > 42.0)
        assert(true, a > 41.0)
        ";
    assert_script(program);
}

#[test]
fn op9() {
    let program = "
        assert(4, 4 || 5)
        assert(4, 4 || nil)
        assert(5, nil || 5)
        assert(false, nil || false)
        assert(5, 4 && 5)
        assert(nil, 4 && nil)
        assert(nil, nil && 5)
        assert(nil, nil && false)

        assert(4, (4 or 5))
        assert(4, (4 or nil))
        assert(5, (nil or 5))
        assert(false, (nil or false))
        assert(5, (4 and 5))
        assert(nil, (4 and nil))
        assert(nil, (nil and 5))
        assert(nil, (nil and false))
        ";
    assert_script(program);
}

#[test]
fn op10() {
    let program = "4==4 && 4!=5 && 3<4 && 5>4 && 4<=4 && 4>=4";
    let expected = Value::bool(true);
    eval_script(program, expected);
}

#[test]
fn op11() {
    let program = "
        assert(nil, a&&=4)
        a = 3
        assert(4, a&&=4)
        assert(4, b||=4)
        assert(4, b||=5)
        ";
    assert_script(program);
}

#[test]
fn op_negate() {
    let program = "
    a = 3.5
    assert(-3.5, -a)
    a = 3
    assert(-3, -a)
    assert(-5, -a=5)
    assert(5, a)
    ";
    assert_script(program);
}

#[test]
fn index_op() {
    let program = "
        assert_error{ :a[3] }
        assert_error{ Object[3] }
        assert_error{ :a[3] = 100 }
        assert_error{ Object[3] = 200 }
        ";
    assert_script(program);
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
    assert_script(program);
}

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
        assert(Object, Regexp.superclass)
        assert(Object, String.superclass)
        assert(Object, Range.superclass)
        assert(Object, Proc.superclass)
        assert(Object, Method.superclass)

        assert(Class, Object.class)
        assert(Class, BasicObject.class)
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
    assert_script(program);
}

#[test]
fn triple_equal() {
    let program = r#"
        assert true, 1 === 1
        assert false, 1 === 2
        assert false, "a" === 2
        assert false, 2 === "a"
        assert false, "ruby" === "rust"
        assert true, "ruby" === "ruby"
        assert false, Integer === Integer
        assert true, Integer === 100
        assert false, Integer === "ruby"
        assert true, String === "ruby"
        assert false, String === 100
        assert true, /\A[A-Z]*\z/ === "HELLO"
        assert false, /\A[a-z]*\z/ === "HELLO"
        assert 4, "aabcdxafv" =~ /dx.f/
        assert 3, "sdrgbgbgbff" =~ /(gb)*f/
    "#;
    assert_script(program);
}

#[test]
fn if1() {
    let program = "if 5*4==16 +4 then 4;2*3+1 end";
    let expected = Value::integer(7);
    eval_script(program, expected);
}

#[test]
fn if2() {
    let program = "if 
        5*4 ==16 +
        4
        3*3
        -2 end";
    let expected = Value::integer(-2);
    eval_script(program, expected);
}

#[test]
fn if3() {
    let program = "if 5*9==16 +4
        7 elsif 4==4+9 then 8 elsif 3==1+2 then 10
        else 12 end";
    let expected = Value::integer(10);
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
    let expected = Value::integer(5);
    eval_script(program, expected);
}

#[test]
fn if5() {
    let program = "a = 77 if 1+2 == 3";
    let expected = Value::integer(77);
    eval_script(program, expected);
}

#[test]
fn if6() {
    let program = "a = 77 if 1+3 == 3";
    let expected = Value::nil();
    eval_script(program, expected);
}

#[test]
fn if_cmp_ops() {
    let program = "
        a = 42
        # JMP_F_EQI ..
        assert(true, if a == 42 then true else false end)
        assert(true, if a == 43 then false else true end)
        assert(true, if a != 43 then true else false end)
        assert(true, if a != 42 then false else true end)

        assert(true, if a < 41 then false else true end)
        assert(true, if a < 42 then false else true end)
        assert(true, if a < 43 then true else false end)

        assert(true, if a <= 41 then false else true end)
        assert(true, if a <= 42 then true else false end)
        assert(true, if a <= 43 then true else false end)

        assert(true, if a > 43 then false else true end)
        assert(true, if a > 42 then false else true end)
        assert(true, if a > 41 then true else false end)

        assert(true, if a >= 43 then false else true end)
        assert(true, if a >= 42 then true else false end)
        assert(true, if a >= 41 then true else false end)

        # JMP_F_EQ ..
        assert(true, if a == 42.0 then true else false end)
        assert(true, if a == 43.0 then false else true end)
        assert(true, if a != 43.0 then true else false end)
        assert(true, if a != 42.0 then false else true end)

        assert(true, if a < 41.0 then false else true end)
        assert(true, if a < 42.0 then false else true end)
        assert(true, if a < 43.0 then true else false end)

        assert(true, if a <= 41.0 then false else true end)
        assert(true, if a <= 42.0 then true else false end)
        assert(true, if a <= 43.0 then true else false end)

        assert(true, if a > 43.0 then false else true end)
        assert(true, if a > 42.0 then false else true end)
        assert(true, if a > 41.0 then true else false end)

        assert(true, if a >= 43.0 then false else true end)
        assert(true, if a >= 42.0 then true else false end)
        assert(true, if a >= 41.0 then true else false end)
    ";
    assert_script(program);
}

#[test]
fn unless1() {
    let program = "a = 5; unless a > 3 then 10 else 50 end";
    let expected = Value::integer(50);
    eval_script(program, expected);
}

#[test]
fn unless2() {
    let program = "a = 5; unless a < 3 then 10 else 50 end";
    let expected = Value::integer(10);
    eval_script(program, expected);
}

#[test]
fn unless3() {
    let program = "a = 5; a = 7 unless a == 3; a";
    let expected = Value::integer(7);
    eval_script(program, expected);
}

#[test]
fn unless4() {
    let program = "a = 5; a = 7 unless a == 5; a";
    let expected = Value::integer(5);
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
    let expected = Value::integer(45);
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
    let expected = Value::integer(36);
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
    let expected = Value::integer(10);
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
    let expected = Value::integer(40);
    eval_script(program, expected);
}

#[test]
fn for5() {
    let program = "
        ans = for a in 0..2 do
            end
        assert(0..2, ans)
        ans = for a in 0..2 do
            if a == 1
                break 4
            end
        end
        assert(4, ans)
    ";
    assert_script(program);
}

#[test]
fn while1() {
    let program = "
        assert((a = 0; while a < 5 do puts a; a+=1 end; a), 5)
        assert((a = 0; while a < 5 do puts a; break if a == 3; a+=1 end; a), 3)
        assert((a = 0; while a < 5 do puts a; a+=1 end), nil)
        assert((a = 0; while a < 5 do puts a; break if a == 3; a+=1 end), nil)
    ";
    assert_script(program);
}

#[test]
fn while2() {
    let program = "
        assert((a = 0; a+=1 while a < 5; a), 5)
    ";
    assert_script(program);
}

#[test]
fn until1() {
    let program = "
        assert((a = 0; until a == 4 do puts a; a+=1 end; a), 4)
        assert((a = 0; until a == 4 do puts a; break if a == 3; a+=1 end; a), 3)
        assert((a = 0; until a == 4 do puts a; a+=1 end), nil)
        assert((a = 0; until a == 4 do puts a; break if a == 3; a+=1 end), nil)
    ";
    assert_script(program);
}

#[test]
fn until2() {
    let program = "
        assert((a = 0; a+=1 until a == 5; a), 5)
    ";
    assert_script(program);
}

#[test]
fn case_opt0() {
    let program = "
        i = 11
        case i
        when 0 then
            r = 0
        when 1 then
            r = 1
        when 5 then
            r = 5
        when 11 then
            r = 11
        else
            r = 13
        end
        assert 11, r
    ";
    assert_script(program);
}

#[test]
fn case_opt1() {
    let program = "
        i = :foo
        case i
        when :aoo then
            r = 0
        when :boo then
            r = 1
        when :foo then
            r = 5
        when :doo then
            r = 11
        else
            r = 13
        end
        assert 5, r
    ";
    assert_script(program);
}

#[test]
fn case_opt2() {
    let program = r#"
        i = "afoo"
        case i
        when "aoo" then
            r = 0
        when "boo" then
            r = 1
        when "foo" then
            r = 5
        when "doo" then
            r = 11
        else
            r = 13
        end
        assert 13, r
    "#;
    assert_script(program);
}

#[test]
fn case1() {
    let program = "
        i = 11
        j = 3
        case i
        when i - 11 then
            r = 0
        when 1 then
            r = 1
        when 5 then
            r = 5
        when j * j + 2 then
            r = 11
        end
        assert 11, r
    ";
    assert_script(program);
}

#[test]
fn case_less_case() {
    let program = "
        i = 11
        j = 3
        case
        when i == i - 11 then
            r = 0
        when 1 == i then
            r = 1
        when i == 5 then
            r = 5
        when j * j + 2 == i then
            r = 11
        end
        assert 11, r
    ";
    assert_script(program);
}

#[test]
fn block_break() {
    let program = "
        assert(100, loop { break 100 })
    ";
    assert_script(program);
}

#[test]
fn block_next() {
    let program = "
        a = []
        3.times{|x| if x == 1 then next end; a << x}
        assert([0,2], a)
    ";
    assert_script(program);
}

#[test]
fn block_return() {
    let program = "
        assert_error { return }
    ";
    assert_script(program);
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
    assert_script(program);
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
    assert_script(program);
}

#[test]
fn const_test() {
    let program = "
        class CONST
          def fn
            77
          end
        end
        assert 77, CONST.new.fn
        CONST = 100
        assert 100, CONST
        CONST = 200
        assert 200, CONST
            ";
    assert_script(program);
}

#[test]
fn local_var1() {
    let program = "
            ruby = 7
            mruby = (ruby - 4) * 5
            mruby - ruby";
    let expected = Value::integer(8);
    eval_script(program, expected);
}

#[test]
fn local_var2() {
    let program = "
        a = 100
        b = a += 3
        assert(103, b)";
    assert_script(program);
}

#[test]
fn instance_var1() {
    let program = "
        assert(nil, @some)
        @some = 100
        @some += 15
        assert(115, @some)
        assert(125, @some += 10)";
    assert_script(program);
}

#[test]
fn class_var1() {
    let program = "
    class A
        @@a = 100
        def get
            @@a
        end
        def set(val)
            @@a = val
        end
    end
    assert(100, A.new.get)
    A.new.set(77)
    assert(77, A.new.get)
    ";
    assert_script(program);
}

#[test]
fn class_var2() {
    let program = "
    class A
        @@a = 100
        def get
            @@a
        end
        def set(val)
            @@a = val
        end
    end
    class B < A
        @@a = 77
    end
    assert(77, A.new.get)
    assert(77, B.new.get)
    B.new.set(42)
    assert(42, A.new.get)
    assert(42, B.new.get)
    A.new.set(99)
    assert(99, A.new.get)
    assert(99, B.new.get)
    ";
    assert_script(program);
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
    assert_script(program);
}

#[test]
fn mul_assign1() {
    let program = r#"
            assert_error { eval("@foo + 4 = 100") }
            a,b,c = 1,2,3
            assert(1,a)
            assert(2,b)
            assert(3,c)
            "#;
    assert_script(program);
}

#[test]
fn mul_assign2() {
    let program = "
            d,e = 1,2,3,4
            assert(1,d)
            assert(2,e)
            ";
    assert_script(program);
}

#[test]
fn mul_assign3() {
    let program = "
            f,g,h = 1,2
            assert(1,f)
            assert(2,g)
            assert(nil,h)
            assert([5,6], (i,j = 5,6))
            ";
    assert_script(program);
}

#[test]
fn mul_assign4() {
    let program = "
            f = 1,2,3
            assert([1,2,3],f)
            assert([1,2,3],(f=1,2,3))
            ";
    assert_script(program);
}

#[test]
fn mul_assign5() {
    let program = "
            d = (a,b,c = [1,2])
            assert([a,b,c],[1,2,nil])
            assert(d,[1,2])
            ";
    assert_script(program);
}

#[test]
fn mul_assign6() {
    let program = "
            d = (a,b,c = [1,2,3,4,5])
            assert([a,b,c],[1,2,3])
            assert(d,[1,2,3,4,5])
            ";
    assert_script(program);
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
    assert_script(program);
}

#[test]
fn mul_assign8() {
    let program = "
        a,b = 1,2
        assert(1, a)
        assert(2, b)
        a = [1,2]
        a[0], a[1] = a[1], a[0]
        assert([2,1], a)
        ";
    assert_script(program);
}

#[test]
fn mul_assign9() {
    let program = "
        def f(val)
            @r << val
            val
        end
        @r = []; a = {}
        assert 1, a[f 0] = (f 1)
        assert [0, 1], @r
        @r = []; a = {}
        assert [1, 2], a[f 0] = (f 1), (f 2)
        assert [0, 1, 2], @r
        ";
    assert_script(program);
}

#[test]
fn assign1() {
    let program = "
        assert(13, 5+a=8)
        assert(8, a)
        assert(-1, 5-a=6)
        assert(6, a)
        assert(3, 5-C=2)
        assert(2, C)
        assert(1, 5-C+=2)
        assert(4, C)
        assert(3, 5-@c=2)
        assert(2, @c)
        assert(20, 4*a=5)
        assert(5, a)
        assert(4..7, 4..a=7)
        assert(7, 4+3*n=1)
        a,b = c = 1,2
        assert(1, a)
        assert(2, b)
        assert(1, c)
        a = []
        assert(19, 4+5*a[0]=3)
        assert(3, a[0])
        ";
    assert_script(program);
}

#[test]
fn const1() {
    let program = "
            Ruby = 777
            Ruby = Ruby * 2
            Ruby / 111";
    let expected = Value::integer(14);
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
    assert_script(program);
}

#[test]
fn const3() {
    let program = "
        a = Class.new
        a::B = Class.new
        a::B::C = Class.new
        a::B::C::D = 777
        assert(777, a::B::C::D)
    ";
    assert_script(program);
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
    assert_script(program);
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
    assert_script(program);
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
    let expected = Value::integer(25);
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
    assert_script(program);
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
    assert_script(program);
}

#[test]
fn class4() {
    let program = "
        class C
          class << self
            def foo
              77
            end
          end
        end
        assert 77, C.foo

        class D
        end
        class << D
          def boo
            99
          end
        end
        assert 99, D.boo
        ";
    assert_script(program);
}

#[test]
fn class5() {
    let program = r##"
        class A
        end
        class A::B
        end
        class A::B::C
          D = 100
        end
        assert 100, A::B::C::D
        assert "A::B::C", A::B::C.inspect
        "##;
    assert_script(program);
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
    assert_script(program);
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
    assert_script(program);
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
    assert_script(program);
}

#[test]
fn closure2() {
    let program = "
        a = 5;
        f = ->{ ->{ ->{ a } } }
        assert 5, f.call.call.call
        a = 7;
        assert 7, f.call.call.call";
    assert_script(program);
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
    assert_script(program);
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
    assert_script(program);
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
    assert_script(program);
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
    assert_script(program);
}

#[test]
fn singleton2() {
    let program = "
    class Foo
    end

    class Bar
        def Foo.f
            100
        end
    end

    assert(100, Foo.f)
        ";
    assert_script(program);
}

#[test]
fn singleton3() {
    let program = "
    class A < Array
        def foo
            100
        end
    end

    a = A.new
    assert(A, a.class)
    assert(Array, a.class.superclass)
    assert(100, a.foo)
    ";
    assert_script(program);
}

#[test]
fn singleton4() {
    let program = "
    assert(Class, [].singleton_class.class)
    assert(Array, [].singleton_class.superclass)
    ";
    assert_script(program);
}

#[test]
fn defined() {
    let program = r##"
    assert("expression", defined? 1)
    assert("expression", defined? 1.1)
    assert("expression", defined? "1.1")
    assert("expression", defined? [])
    assert("expression", defined? {})
    assert("method", defined? 1+1)
    assert("method", defined? -(1))
    assert(nil, defined? a)
    assert(nil, defined? @a)
    assert(nil, defined? $a)
    a = [1,2]
    @a = 100
    $a = 100
    assert("local-variable", defined? a)
    assert("instance-variable", defined? @a)
    assert("global-variable", defined? $a)
    assert("method", defined? a.each)
    assert("method", defined? a[2])
    "##;
    assert_script(program);
}
