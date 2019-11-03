#![feature(test)]
#![allow(unused_imports, dead_code)]
extern crate ruruby;
extern crate test;
use ruruby::lexer::Lexer;
use ruruby::parser::{LvarCollector, Parser};
use ruruby::value::Value;
use ruruby::vm::VM;
use test::Bencher;

fn eval_script(script: impl Into<String>, expected: Value) {
    let mut parser = Parser::new();
    let result = parser.parse_program(script.into(), None).unwrap();
    let mut eval = VM::new(Some(result.ident_table), Some(result.lvar_collector));
    eval.init_builtin();
    match eval.run(&result.node) {
        Ok(res) => {
            if res != expected {
                panic!("Expected:{:?} Got:{:?}", expected, res);
            }
        }
        Err(err) => panic!("Got runtime error: {:?}", err),
    }
}

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
fn expr1() {
    let program = "4*(4+7*3)-95";
    let expected = Value::FixNum(5);
    eval_script(program, expected);
}

#[test]
fn expr2() {
    let program = "2.0 + 4.0";
    let expected = Value::FloatNum(6.0);
    eval_script(program, expected);
}

#[test]
fn expr3() {
    let program = "5.0 / 2";
    let expected = Value::FloatNum(2.5);
    eval_script(program, expected);
}

#[test]
fn expr4() {
    let program = "15<<30";
    let expected = Value::FixNum(16106127360);
    eval_script(program, expected);
}

#[test]
fn expr5() {
    let program = "23456>>3";
    let expected = Value::FixNum(2932);
    eval_script(program, expected);
}

#[test]
fn expr6() {
    let program = "24+17 >> 3 == 5";
    let expected = Value::Bool(true);
    eval_script(program, expected);
}
#[test]
fn expr7() {
    let program = "864 == 3+24<<5";
    let expected = Value::Bool(true);
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
    let expected = Value::Nil;
    eval_script(program, expected);
}

#[test]
fn expr9() {
    let program = "
        a=19
        a==17?23*45:14+7
        ";
    let expected = Value::FixNum(21);
    eval_script(program, expected);
}

#[test]
fn op1() {
    let program = "4==5";
    let expected = Value::Bool(false);
    eval_script(program, expected);
}

#[test]
fn op2() {
    let program = "4!=5";
    let expected = Value::Bool(true);
    eval_script(program, expected);
}

#[test]
fn op9() {
    let program = "4!=4 || 1==1 && 2==3";
    let expected = Value::Bool(false);
    eval_script(program, expected);
}

#[test]
fn op10() {
    let program = "4==4 && 4!=5 && 3<4 && 5>4 && 4<=4 && 4>=4";
    let expected = Value::Bool(true);
    eval_script(program, expected);
}

#[test]
fn if1() {
    let program = "if 5*4==16 +4 then 4;2*3+1 end";
    let expected = Value::FixNum(7);
    eval_script(program, expected);
}

#[test]
fn if2() {
    let program = "if 
        5*4 ==16 +
        4
        3*3
        -2 end";
    let expected = Value::FixNum(-2);
    eval_script(program, expected);
}

#[test]
fn if3() {
    let program = "if 5*9==16 +4
        7 elsif 4==4+9 then 8 elsif 3==1+2 then 10
        else 12 end";
    let expected = Value::FixNum(10);
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
    let expected = Value::FixNum(5);
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
    let expected = Value::FixNum(45);
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
    let expected = Value::FixNum(36);
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
    let expected = Value::FixNum(10);
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
    let expected = Value::FixNum(40);
    eval_script(program, expected);
}

#[test]
fn local_var1() {
    let program = "
            ruby = 7
            mruby = (ruby - 4) * 5
            mruby - ruby";
    let expected = Value::FixNum(8);
    eval_script(program, expected);
}

#[test]
fn const1() {
    let program = "
            Ruby = 777
            Ruby = Ruby * 2
            Ruby / 111";
    let expected = Value::FixNum(14);
    eval_script(program, expected);
}

#[test]
fn func1() {
    let program = "
            def func(a,b,c)
                a+b+c
            end
    
            func(1,2,3)";
    let expected = Value::FixNum(6);
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
    let expected = Value::FixNum(120);
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
    let expected = Value::FixNum(6765);
    b.iter(|| eval_script(program, expected.clone()));
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
    let expected = Value::Nil;
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
    let expected = Value::FixNum(25);
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
    let expected = Value::Nil;
    eval_script(program, expected);
}
