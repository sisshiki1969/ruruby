pub mod lexer;
pub mod parser;
pub mod value;
use crate::lexer::Lexer;
use crate::parser::Parser;

fn main() {
    let program = "(7+ 4) *
    5 -49 ; 6*7;";
    println!("{}", program);
    let lexer = Lexer::new(program);
    match lexer.tokenize() {
        Err(err) => println!("{:?}", err),
        Ok(result) => {
            for token in &result.tokens {
                println!("{}", token);
            }
            let mut parser = Parser::new(result);
            match parser.parse_comp_stmt() {
                Ok(_) => {}
                Err(err) => {
                    println!("{:?}", err);
                }
            }
        }
    };
}
