mod lexer;
use crate::lexer::Lexer;

fn main() {
    let program = "a = 0;\n if a == 1_000 then 5 else 10";
    println!("{}", program);
    let mut lexer = Lexer::new(program);
    match lexer.tokenize() {
        Err(err) => println!("{:?}", err),
        Ok(tokens) => {
            for token in tokens {
                println!("{:?}", token);
            }
        }
    };
}
