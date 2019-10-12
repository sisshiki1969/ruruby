pub mod eval;
pub mod lexer;
pub mod parser;
pub mod value;
use crate::eval::eval_node;
use crate::lexer::Lexer;
use crate::parser::Parser;

fn main() {
    let program = "if 5*4==16 +4  
     7; 
     end";
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
                Ok(node) => {
                    println!("{:?}", eval_node(&node));
                    parser.source_info.show_loc(&node.loc);
                }
                Err(err) => {
                    println!("{:?}", err);
                }
            }
        }
    };
}
