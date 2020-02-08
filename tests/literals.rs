#![feature(test)]
extern crate ruruby;
extern crate test;
use ruruby::test::eval_script;
use ruruby::vm::value::Value;

#[test]
fn bool_lit1() {
    let program = "(3==3)==true";
    let expected = Value::Bool(true);
    eval_script(program, expected);
}

#[test]
fn bool_lit2() {
    let program = "(3==9)==false";
    let expected = Value::Bool(true);
    eval_script(program, expected);
}

#[test]
fn nil_lit1() {
    let program = "nil";
    let expected = Value::Nil;
    eval_script(program, expected);
}

#[test]
fn string_lit1() {
    let program = r#""open "  "windows""#;
    let expected = Value::String("open windows".to_string());
    eval_script(program, expected);
}

#[test]
fn string_lit2() {
    let program = r#""open "
    "windows""#;
    let expected = Value::String("windows".to_string());
    eval_script(program, expected);
}

#[test]
fn interpolated_string_lit1() {
    let program = r###"
    x = 20
    f = "fibonacci";
    "#{f} #{def fibo(x); if x<2 then x else fibo(x-1)+fibo(x-2); end; end;""} fibo(#{x}) = #{fibo(x)}"
    "###;
    let expected = Value::String("fibonacci  fibo(20) = 6765".to_string());
    eval_script(program, expected);
}

#[test]
fn float_lit1() {
    let program = "
        assert(123000000.0, 12.3e7)
        assert(0.000031, 3.1e-5)
    ";
    let expected = Value::Nil;
    eval_script(program, expected);
}

#[test]
fn array_lit1() {
    let program = "
        assert([1,2,3], [1,2,3])
    ";
    let expected = Value::Nil;
    eval_script(program, expected);
}

#[test]
fn percent_notation() {
    let program = r#"
    assert(%w(We are the champions), ["We", "are", "the", "champions"])
"#;
    let expected = Value::Nil;
    eval_script(program, expected);
}
