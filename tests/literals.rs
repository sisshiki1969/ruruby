#![feature(test)]
extern crate ruruby;
extern crate test;
use ruruby::test::*;
use ruruby::*;

#[test]
fn bool_lit1() {
    let program = "(3==3)==true";
    let expected = Value::bool(true);
    eval_script(program, expected);
}

#[test]
fn bool_lit2() {
    let program = "(3==9)==false";
    let expected = Value::bool(true);
    eval_script(program, expected);
}

#[test]
fn nil_lit1() {
    let program = "nil";
    let expected = Value::nil();
    eval_script(program, expected);
}

#[test]
fn int1() {
    let i1 = 0x3fff_ffff_ffff_ffffu64 as i64;
    let i2 = 0x4000_0000_0000_0005u64 as i64;
    let program = format!("{}+6=={}", i1, i2);
    let expected = Value::bool(true);
    eval_script(program, expected);
}

#[test]
fn int2() {
    let i1 = 0x3fff_ffff_ffff_ffffu64 as i64;
    let i2 = 0x4000_0000_0000_0005u64 as i64;
    let program = format!("{}-6=={}", i2, i1);
    let expected = Value::bool(true);
    eval_script(program, expected);
}

#[test]
fn int3() {
    let i1 = 0xbfff_ffff_ffff_ffffu64 as i64;
    let i2 = 0xc000_0000_0000_0005u64 as i64;
    let program = format!("{}+6=={}", i1, i2);
    let expected = Value::bool(true);
    eval_script(program, expected);
}

#[test]
fn int4() {
    let i1 = 0xbfff_ffff_ffff_ffffu64 as i64;
    let i2 = 0xc000_0000_0000_0005u64 as i64;
    let program = format!("{}-6=={}", i2, i1);
    let expected = Value::bool(true);
    eval_script(program, expected);
}

#[test]
fn imaginary() {
    let program = r##"
    assert(3+5.44i, Complex(3, 5.44))
    assert(3-5.44i, Complex(3, -5.44))
    assert(5+44i, Complex(5, 44))
    assert(5-44i, Complex(5, -44))
    "##;
    assert_script(program);
}

#[test]
fn string_lit1() {
    let program = r##"assert("open "  "windows", "open windows")"##;
    assert_script(program);
}

#[test]
fn string_lit2() {
    let program = r##"assert("\"ruby\\t is\\n 'great'\"", '"ruby\t is\n \'great\'"')"##;
    assert_script(program);
}

#[test]
fn string_lit3() {
    let program = r##"
        assert("鬼", ?鬼)
        assert("あ", ?\u3042)
        assert("剪", ?\u526a)
        assert("剪", ?\u526A)
        #assert_error { ?\uffff }
    "##;
    assert_script(program);
}

#[test]
fn interpolated_string_lit1() {
    let program = r###"
        x = 20
        f = "fibonacci";
        res = "#{f} #{def fibo(x); if x<2 then x else fibo(x-1)+fibo(x-2); end; end;""} fibo(#{x}) = #{fibo(x)}"
        assert("fibonacci  fibo(20) = 6765", res)
    "###;
    assert_script(program);
}

#[test]
fn float_lit1() {
    let program = "
        assert(123000000.0, 12.3e7)
        assert(0.000031, 3.1e-5)
    ";
    assert_script(program);
}

#[test]
fn array_lit1() {
    let program = "
        assert([1,2,3], [1,2,3])
        a = 100
        @b = 200
        $c = 300
        assert([100, 200, 300],[a, @b, $c])
    ";
    assert_script(program);
}

#[test]
fn hash_lit1() {
    let program = "
        assert([{a:1, b:2, c:3}], [{:a=>1, :b=>2, :c=>3}])
        a = 100
        @b = 200
        $c = 300
        assert([{e:100, f:200, g:300}], [{e:a, :f=>@b, g:$c}])
    ";
    assert_script(program);
}

#[test]
fn regexp_literal() {
    let program = r#"
        j = "Ruby"
        assert 1, "aaRubyvv" =~ /a#{j}v/
        assert :"CRuby(MRI)", :"C#{j}(MRI)"
        "#;
    assert_script(program);
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
    assert_script(program);
}

#[test]
fn percent_notation() {
    let program = r#"
        assert(%w(We are the champions), ["We", "are", "the", "champions"])
        assert(%w{We are the champions}, ["We", "are", "the", "champions"])
        assert(%w<We are the champions>, ["We", "are", "the", "champions"])
        assert(%i(We are the champions), [:"We", :"are", :"the", :"champions"])
        assert(%q{evidence}, "evidence")
        assert(%q[evidence], "evidence")
        assert(%q<evidence>, "evidence")
        assert(%q\evidence\, "evidence")
        assert(%q{]>{evidence}}, "]>{evidence}")
        assert(%Q{evidence}, "evidence")
        assert(%{evidence}, "evidence")
        assert(%Q[and#{123} #{:e34}ff #{$a=130} #$a], "and123 e34ff 130 130")
        assert(%{evi{d)[#{"e""n"}}ce}, "evi{d)[en}ce")
    "#;
    assert_script(program);
}
