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
fn string_lit1() {
    let program = r##"assert("open "  "windows", "open windows")"##;
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
    ";
    assert_script(program);
}

#[test]
fn percent_notation() {
    let program = r#"
        assert(%w(We are the champions), ["We", "are", "the", "champions"])
    "#;
    assert_script(program);
}
