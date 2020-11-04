#![feature(test)]
extern crate ruruby;

#[cfg(test)]
mod time_tests {
    use ruruby::test::*;

    #[test]
    fn time() {
        let program = "
        p Time.now.inspect
        a = Time.now
        assert a, a - 100 + 100
        assert a, a - 77.0 + 77.0
        assert Float, (Time.now - a).class
        assert_error { Time.now + a }
    ";
        assert_script(program);
    }
}

#[cfg(test)]
mod struct_tests {
    use ruruby::test::*;

    #[test]
    fn struct_test() {
        let program = r#"
        Customer = Struct.new(:name, :address) do
            def greeting
                "Hello #{name}!"
            end
        end
        assert "Hello Dave!", Customer.new("Dave", "123 Main").greeting
        assert "Hello Gave!", Customer["Gave", "456 Sub"].greeting
        "#;
        assert_script(program);
    }

    #[test]
    fn struct_inspect() {
        let program = r###"
        S = Struct.new(:a,:b)
        s = S.new(100,200)
        assert 100, s.a
        assert 200, s.b
        assert "#<struct S @a=100 @b=200>", s.inspect
        "###;
        assert_script(program);
    }
}

#[cfg(test)]
mod string_tests {
    use ruruby::test::*;

    #[test]
    fn string_test() {
        let program = r#"
        assert(true, "a" < "b")
        assert(false, "b" < "b")
        assert(false, "c" < "b")
        assert(false, "a" > "b")
        assert(false, "b" > "b")
        assert(true, "c" > "b")
        assert(-1, "a" <=> "b")
        assert(0, "b" <=> "b")
        assert(1, "b" <=> "a")
        assert_error { "a" < 9 }
        assert_error { "a" > 9 }
        assert(7, "hello世界".size)
        assert(true, "".empty?)
        assert(false, "s".empty?)
        assert(false, 2.chr.empty?)
        "#;
        assert_script(program);
    }

    #[test]
    fn string_add() {
        let program = r#"
        assert "this is a pen", "this is " + "a pen"
        "#;
        assert_script(program);
    }

    #[test]
    fn string_mul() {
        let program = r#"
        assert "rubyrubyrubyruby", "ruby" * 4
        assert "", "ruby" * 0
        "#;
        assert_script(program);
    }

    #[test]
    fn string_concat() {
        let program = r#"
        a = "Ruby"
        assert "Ruby is easy", a << " is easy"
        assert "Ruby is easy", a
        a << 33
        assert "Ruby is easy!", a
        "#;
        assert_script(program);
    }

    #[test]
    fn string_index() {
        let program = r#"
        assert "rubyruby"[3], "y" 
        assert "rubyruby"[0..2], "rub" 
        assert "rubyruby"[0..-2], "rubyrub" 
        assert "rubyruby"[2..-7], ""
        "#;
        assert_script(program);
    }

    #[test]
    fn string_index2() {
        let program = r#"
        a = "qwertyuiop"
        a[9] = "P"
        a[3,6] = "/"
        assert("qwe/P", a) 
        "#;
        assert_script(program);
    }

    #[test]
    fn string_format() {
        let program = r#"
        assert "-12-", "-%d-" % 12
        assert "-  12-", "-%4d-" % 12
        assert "-0012-", "-%04d-" % 12
        assert "-c-", "-%x-" % 12
        assert "-   c-", "-%4x-" % 12
        assert "-000c-", "-%04x-" % 12
        assert "-C-", "-%X-" % 12
        assert "-   C-", "-%4X-" % 12
        assert "-000C-", "-%04X-" % 12
        assert "-1001-", "-%b-" % 9
        assert "-  1001-", "-%6b-" % 9
        assert "-001001-", "-%06b-" % 9
        assert "12.50000", "%08.5f" % 12.5
        assert "0012.500", "%08.3f" % 12.5
        assert "1.34", "%.2f" % 1.345
        "#;
        assert_script(program);
    }

    #[test]
    fn string_start_with() {
        let program = r#"
        assert true, "ruby".start_with?("r")
        assert false, "ruby".start_with?("R")
        assert true, "魁ruby".start_with?("魁")
        "#;
        assert_script(program);
    }

    #[test]
    fn string_end_with() {
        let program = r#"
        assert true, "ruby".end_with?("by")
        assert false, "ruby".end_with?("yy")
        assert true, "ruby魂".end_with?("魂")
        "#;
        assert_script(program);
    }

    #[test]
    fn string_to_sym() {
        let program = r#"
        assert :ruby, "ruby".to_sym
        assert :rust, "rust".to_sym
        "#;
        assert_script(program);
    }

    #[test]
    fn string_split() {
        let program = r#"
        assert ["this", "is", "a", "pen"], "this is a pen       ".split(" ")
        assert ["this", "is", "a pen"], "this is a pen".split(" ", 3)
        "#;
        assert_script(program);
    }

    #[test]
    fn string_bytes() {
        let program = r#"
        assert [97, 98, 99, 100], "abcd".bytes
        assert [228, 184, 150, 231, 149, 140], "世界".bytes
        res = []
        "str".each_byte do |byte|
        res << byte
        end
        assert [115, 116, 114], res
        "#;
        assert_script(program);
    }

    #[test]
    fn string_chars() {
        let program = r#"
        assert ["a", "b", "c", "d"], "abcd".chars
        assert ["世", "界"], "世界".chars
        res = []
        "str".each_char do |byte|
        res << byte
        end
        assert ["s", "t", "r"], res
        "#;
        assert_script(program);
    }

    #[test]
    fn string_sum() {
        let program = r#"
        assert 394, "abcd".sum
        a = ""
        [114, 117, 98].map{ |elem| a += elem.chr}
        assert 329, a.sum
        "#;
        assert_script(program);
    }

    #[test]
    fn string_sub() {
        let program = r#"
        assert "abc!!g", "abcdefg".sub(/def/, "!!")
        #assert "a<<b>>cabc", "abcabc".sub(/b/, "<<\1>>")
        #assert "X<<bb>>xbb", "xxbbxbb".sub(/x+(b+)/, "X<<\1>>")
        assert "aBCabc", "abcabc".sub(/bc/) {|s| s.upcase }
        assert "abcabc", "abcabc".sub(/bd/) {|s| s.upcase }
        "#;
        assert_script(program);
    }

    #[test]
    fn string_scan() {
        let program = r#"
        assert ["fo", "ob", "ar"], "foobar".scan(/../)
        assert ["o", "o"], "foobar".scan("o")
        assert ["bar", "baz", "bar", "baz"], "foobarbazfoobarbaz".scan(/ba./)
        assert [["f"], ["o"], ["o"], ["b"], ["a"], ["r"]], "foobar".scan(/(.)/)
        assert [["ba", "r", ""], ["ba", "z", ""], ["ba", "r", ""], ["ba", "z", ""]], "foobarbazfoobarbaz".scan(/(ba)(.)()/)
        "foobarbazfoobarbaz".scan(/ba./) {|x| puts x}
        "#;
        assert_script(program);
    }

    #[test]
    fn string_slice_() {
        let program = r#"
        a = ["私の名前は一色です"] * 20
        assert "は", a[0].slice!(4)
        assert "私の名前一色です", a[0]
        assert "色", a[0].slice!(-3)
        assert "私の名前一です", a[0]
        assert nil, a[0].slice!(-9)
        assert "私の名前一です", a[0]
        assert "名前は一色", a[1].slice!(2,5)
        assert "私のです", a[1]
        assert nil, a[2].slice!(10,5)
        assert "色です", a[3].slice!(-3,5)
        assert nil, a[3].slice!(-10,5)
        a = "a"
        assert "a", a.slice!(0,1)
        assert "", a

        a = "abc agc afc"
        assert "abc", a.slice!(/a.c/)
        assert " agc afc", a

        "#;
        assert_script(program);
    }

    #[test]
    fn string_upcase() {
        let program = r#"
        assert "RUBY IS GREAT.", "ruby is great.".upcase
        a = ""
        [114, 117, 98, 121, 32, 105, 115, 32, 103, 114, 101, 97, 116, 46].map{ |elem| a += elem.chr }
        assert "RUBY IS GREAT.", a.upcase
        "#;
        assert_script(program);
    }

    #[test]
    fn string_chomp() {
        let program = r#"
        assert "Ruby", "Ruby\n\n\n".chomp
        a = ""
        [82, 117, 98, 121, 10, 10, 10].map{ |elem| a += elem.chr }
        assert "Ruby", a.chomp
        "#;
        assert_script(program);
    }

    #[test]
    fn string_toi() {
        let program = r#"
        assert 1578, "1578".to_i
        a = ""
        [49, 53, 55, 56].map{ |elem| a += elem.chr }
        assert 1578, a.to_i
        assert 0, "k".to_i
        "#;
        assert_script(program);
    }

    #[test]
    fn string_center() {
        let program = r#"
        assert("foo", "foo".center(1))
        assert("foo", "foo".center(2))
        assert("foo", "foo".center(3))
        assert("  foo  ", "foo".center(7))
        assert("  foo   ", "foo".center(8))
        assert("   foo   ", "foo".center(9))
        assert("   foo    ", "foo".center(10))
        assert("***foo****", "foo".center(10, "*"))
        assert("121foo1212", "foo".center(10, "12"))
        "#;
        assert_script(program);
    }

    #[test]
    fn string_ljust() {
        let program = r#"
        s = "戦闘妖精"
        assert_error { s.ljust }
        assert_error { s.ljust 8, "" }
        assert("戦闘妖精       ", s.ljust 11)
        assert("戦闘妖精$$$$$$$", s.ljust 11,"$")
        assert("戦闘妖精1231231", s.ljust 11,"123")
        "#;
        assert_script(program);
    }

    #[test]
    fn string_rjust() {
        let program = r#"
        s = "戦闘妖精"
        assert_error { s.rjust }
        assert_error { s.rjust 8, "" }
        assert("       戦闘妖精", s.rjust 11)
        assert("$$$$$$$戦闘妖精", s.rjust 11,"$")
        assert("1231231戦闘妖精", s.rjust 11,"123")
        "#;
        assert_script(program);
    }

    #[test]
    fn string_succ() {
        let program = r#"
        assert "aa".succ, "ab"
        assert "88".succ.succ, "90"
        assert "99".succ, "100"
        assert "ZZ".succ, "AAA"
        assert "a9".succ, "b0"
        #assert "-9".succ, "-10"
        assert ".".succ, "/"
        assert "aa".succ, "ab"
        
        # 繰り上がり
        assert "99".succ, "100"
        assert "a9".succ, "b0"
        assert "Az".succ, "Ba"
        assert "zz".succ, "aaa"
        #assert "-9".succ, "-10"
        assert "9".succ, "10"
        assert "09".succ, "10"
        assert "０".succ, "１"
        assert "９".succ, "１０"
        
        # アルファベット・数字とそれ以外の混在
        #assert "1.9.9".succ, "2.0.0"
        
        # アルファベット・数字以外のみ
        assert ".".succ, "/"
        #assert "\0".succ, "\001"
        #assert "\377".succ, "\001\000"
        "#;
        assert_script(program);
    }

    #[test]
    fn string_count() {
        let program = r#"
        assert 1, 'abcdefg'.count('c')
        assert 4, '123456789'.count('2378')
        #assert 4, '123456789'.count('2-8', '^4-6')
        "#;
        assert_script(program);
    }

    #[test]
    fn string_rstrip() {
        let program = r#"
        assert "   abc", "   abc\n".rstrip
        assert "   abc", "   abc \t\n\x00".rstrip
        assert "   abc", "   abc".rstrip
        assert "   abc", "   abc\x00".rstrip
        "#;
        assert_script(program);
    }

    #[test]
    fn string_ord() {
        let program = r#"
        assert 97, 'abcdefg'.ord
        "#;
        assert_script(program);
    }
}

#[cfg(test)]
mod regexp_tests {
    use ruruby::test::*;

    #[test]
    fn regexp1() {
        let program = r#"
        assert "abc!!g", "abcdefg".gsub(/def/, "!!")
        assert "2.5".gsub(".", ","), "2,5"
        assert true, /(aa).*(bb)/ === "andaadefbbje"
        assert "aadefbb", $&
        assert "aa", $1
        assert "bb", $2
        assert 4, "The cat sat in the hat" =~ /[csh](..) [csh]\1 in/
        assert "x-xBBGZbbBBBVZc", "xbbgz-xbbbvzbbc".gsub(/(b+.z)(..)/) { $2 + $1.upcase }
    "#;
        assert_script(program);
    }

    #[test]
    fn regexp2() {
        let program = r#"
        assert 3, "aaazzz" =~ /\172+/
        "#;
        assert_script(program);
    }

    #[test]
    fn regexp_error() {
        assert_error(r#"/+/"#);
        assert_error(r#"Regexp.new("+")"#);
    }
}

#[cfg(test)]
mod module_tests {
    use ruruby::test::*;

    #[test]
    fn module_op() {
        let program = r#"
        assert(true, Integer === 3)
        assert(false, Integer === "a")
        assert(false, Integer === [])
        assert(false, Array === 3)
        assert(false, Array === "a")
        assert(true, Array === [])

        class A
        end
        class B < A
        end
        class C < B
        end
        c = C.new
        assert(true, C === c)
        assert(true, B === c)
        assert(true, A === c)
        assert(true, Object === c)
        assert(false, Integer === c)
        "#;
        assert_script(program);
    }

    #[test]
    fn module_visibility() {
        let program = r#"
        class A
            public
            private
            protected
        end
        "#;
        assert_script(program);
    }

    #[test]
    fn module_function() {
        let program = r#"
    class Foo
        module_function
        def bar
            123
        end
    end
    assert(123, Foo.bar)
    assert(123, Foo.new.bar)

    class Bar
        def foo
            456
        end
        def bar
            789
        end
        module_function :foo, "bar"
    end
    assert(456, Bar.new.foo)
    assert(789, Bar.new.bar)
    assert(456, Bar.foo)
    assert(789, Bar.bar)
    "#;
        assert_script(program);
    }

    #[test]
    fn constants() {
        let program = r#"
    class Foo
        Bar = 100
        Ker = 777
    end
    
    class Bar < Foo
        Doo = 555
    end
    
    def ary_cmp(a,b)
        return false if a - b != []
        return false if b - a != []
        true
    end

    assert(100, Foo.const_get(:Bar))
    assert(100, Bar.const_get(:Bar))
    assert_error { Bar.const_get([]) }
    assert(true, ary_cmp(Foo.constants, [:Bar, :Ker]))
    assert(true, ary_cmp(Bar.constants, [:Doo, :Bar, :Ker]))
    "#;
        assert_script(program);
    }

    #[test]
    fn class_variables() {
        let program = r##"
        class One
            @@var1 = 1
        end
        class Two < One
            @@var2 = 2
        end
        assert([:"@@var2"], Two.class_variables(false))
        "##;
        assert_script(program);
    }

    #[test]
    fn attr_accessor() {
        let program = r#"
    class Foo
        attr_accessor :car, :cdr
        attr_reader :bar
        attr_writer :boo
        assert_error { attr_accessor 100 }
        assert_error { attr_reader 100 }
        assert_error { attr_writer 100 }
        def set_bar(x)
            @bar = x
        end
        def get_boo
            @boo
        end
    end
    bar = Foo.new
    assert nil, bar.car
    assert nil, bar.cdr
    assert nil, bar.bar
    assert_error { bar.boo }
    bar.car = 1000
    bar.cdr = :something
    assert_error { bar.bar = 4.7 }
    bar.set_bar(9.55)
    bar.boo = "Ruby"
    assert 1000, bar.car
    assert :something, bar.cdr
    assert 9.55, bar.bar
    assert "Ruby", bar.get_boo
    "#;
        assert_script(program);
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
        puts a,b
        return false if a - b != []
        return false if b - a != []
        true
    end
    assert(true, ary_cmp(A.constants, [:Bar, :Foo]))
    assert(true, ary_cmp(A.instance_methods - Class.instance_methods, [:fn, :fo]))
    assert(true, ary_cmp(A.instance_methods(false), [:fn, :fo]))
    "#;
        assert_script(program);
    }

    #[test]
    fn ancestors() {
        let program = r#"
        assert([Class, Module, Object, Kernel, BasicObject], Class.ancestors)
        assert([Kernel], Object.included_modules)
        assert([Kernel], Class.included_modules)
        assert(true, Class.singleton_class.singleton_class?)
        "#;
        assert_script(program);
    }

    #[test]
    fn module_eval() {
        let program = r##"
        class C; D = 777; end;
        D = 111
        x = "bow"
        C.module_eval "def foo; \"#{x}\"; end"
        assert("bow", C.new.foo)
        assert(777, C.module_eval("D"))
        C.module_eval do
            x = "view"  # you can capture or manipulate local variables in outer scope of the block.
            def bar
                "mew"
            end
        end
        assert("mew", C.new.bar)
        assert("view", x)
        assert(111, C.module_eval { D })
        "##;
        assert_script(program);
    }

    #[test]
    fn alias_method() {
        let program = r##"
        class Foo
          def foo
            55
          end
          alias_method :bar1, :foo
          alias_method "bar2", :foo
          alias_method :bar3, "foo"
          alias_method "bar4", "foo"
          assert_error { alias_method 124, :foo }
          assert_error { alias_method :bar5, [] }
        end
        f = Foo.new
        assert(55, f.bar1)
        assert(55, f.bar2)
        assert(55, f.bar3)
        assert(55, f.bar4)
        "##;
        assert_script(program);
    }

    #[test]
    fn const_defined() {
        let program = r#"
        assert(true, Object.const_defined?(:Kernel))
        assert(false, Object.const_defined?(:Kernels))
        assert(true, Object.const_defined? "Array")
        assert(false, Object.const_defined? "Arrays")
        "#;
        assert_script(program);
    }
}

#[cfg(test)]
mod range_tests {
    use ruruby::test::*;

    #[test]
    fn range_test() {
        let program = r#"
            assert(3, (3..100).begin)
            assert(100, (3..100).end)
            assert("3..100", (3..100).to_s)
            assert("3..100", (3..100).inspect)
            assert([6, 8, 10], (3..5).map{|x| x * 2})
            assert(
                [2, 4, 6, 8],
                [[1, 2], [3, 4]].flat_map{|i| i.map{|j| j * 2}}
            )
            assert([2, 3, 4, 5], (2..5).to_a)
            assert([2, 3, 4], (2...5).to_a)
            assert(true, (5..7).all? {|v| v > 0 })
            assert(false, (-1..3).all? {|v| v > 0 })
            assert(true, (0...3).exclude_end?)
            assert(false, (0..3).exclude_end?)
        "#;
        assert_script(program);
    }

    #[test]
    fn range1() {
        let program = "
            assert(Range.new(5,10), 5..10)
            assert(Range.new(5,10, false), 5..10)
            assert(Range.new(5,10, true), 5...10)";
        assert_script(program);
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
        assert_script(program);
    }

    #[test]
    fn range_include() {
        let program = r#"
        assert(true, (3..7).include? 3)
        assert(true, (3..7).include? 7)
        assert(true, (3..7).include? 5)
        assert(true, (3..7).include? 5.7)
        assert(true, (3..7).include? 7.0)
        assert(false, (3..7).include? 0)
        assert(false, (3..7).include? 7.1)
        assert(false, (3..7).include? "6")

        assert(true, (3...7).include? 3)
        assert(false, (3...7).include? 7)
        assert(true, (3...7).include? 5.7)

        assert(true, (3.3..7.1).include? 3.3)
        assert(true, (3.3..7.1).include? 7.1)
        assert(true, (3.3..7.1).include? 4.5)
        assert(true, (3.3..7.1).include? 7)
        assert(false, (3.3..7.1).include? 3.2)
        assert(false, (3.3..7.1).include? 7.2)
        assert(false, (3.3..7.1).include? 3)
        assert(false, (3.3..7.1).include?(:a))

        assert(true, (3.3...7.1).include? 3.3)
        assert(false, (3.3...7.1).include? 7.1)
        assert(true, (3.3...7.1).include? 4.5)
        assert(false, (3.3...7.0).include? 7)
        "#;
        assert_script(program);
    }

    #[test]
    fn range_include2() {
        let program = r#"
        class Foo
            attr_accessor :x
            include Comparable
            def initialize(x)
                @x = x
            end
            def <=>(other)
                self.x<=>other.x
            end
        end

        assert true, (Foo.new(3)..Foo.new(6)).include? Foo.new(3)
        assert true, (Foo.new(3)..Foo.new(6)).include? Foo.new(6)
        assert false, (Foo.new(3)..Foo.new(6)).include? Foo.new(0)
        assert false, (Foo.new(3)..Foo.new(6)).include? Foo.new(7)
        "#;
        assert_script(program);
    }
}

#[cfg(test)]
mod proc_tests {
    use ruruby::test::*;

    #[test]
    fn proc() {
        let program = "
        foo = 42
        p = Proc.new { foo }
        p2 = proc { foo }
        l = lambda { foo }
        assert(42, p[])
        assert(42, p2[])
        assert(42, l[])
        ";
        assert_script(program);
    }
}

#[cfg(test)]
mod process_tests {
    use ruruby::test::*;

    #[test]
    fn process() {
        let program = r#"
        Process.pid
        Process.clock_gettime(0)
        Process::CLOCK_MONOTONIC
        "#;
        assert_script(program);
    }
}

#[cfg(test)]
mod object_tests {
    use ruruby::test::*;

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
}



#[cfg(test)]
mod method_tests {
    use ruruby::test::*;
    #[test]
    fn method() {
        let program = r#"
    class Foo
      def foo(); "foo"; end
      def bar(); "bar"; end
      def baz(); "baz"; end
    end

    obj = Foo.new

    # 任意のキーとメソッドの関係をハッシュに保持しておく
    methods = {1 => obj.method(:foo),
               2 => obj.method(:bar),
               3 => obj.method(:baz)}
    
    # キーを使って関連するメソッドを呼び出す
    assert "foo" ,methods[1].call       # => "foo"
    assert "bar" ,methods[2].call       # => "bar"
    assert "baz" ,methods[3].call       # => "baz"
        "#;
        assert_script(program);
    }
}
