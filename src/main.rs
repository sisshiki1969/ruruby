pub mod eval;
pub mod lexer;
pub mod node;
pub mod parser;
pub mod value;
use crate::eval::Evaluator;
use crate::lexer::Lexer;
use crate::parser::Parser;

fn main() {
    let program = "
    def fact(a)
        puts(a)
        if a == 1
            1
        else
            a * fact(a-1)
        end
    end
    
    puts(fact(5))";
    println!("{}", program);
    let lexer = Lexer::new(program);
    match lexer.tokenize() {
        Err(err) => println!("{:?}", err),
        Ok(result) => {
            for token in &result.tokens {
                println!("{}", token);
            }
            let mut parser = Parser::new(result);
            match parser.parse_program() {
                Ok(node) => {
                    println!("node: {}", node);
                    let mut eval = Evaluator::new(parser.source_info, parser.ident_table);
                    println!("result: {:?}", eval.eval_node(&node));
                    //eval.source_info.show_loc(&node.loc());
                }
                Err(err) => {
                    println!("{:?}", err);
                }
            }
        }
    };
}
