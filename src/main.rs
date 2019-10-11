mod lexer;
use crate::lexer::Lexer;

fn main() {
    let program = r"
        a = 0;
        if a == 1_000 then
            5 # this is a comment
        else
            10 # also a comment";
    println!("{}", program);
    let lexer = Lexer::new(program);
    match lexer.tokenize() {
        Err(err) => println!("{:?}", err),
        Ok(result) => {
            for token in &result.tokens {
                println!("{}", token);
            }
            result.show_loc(&result.tokens[11].loc());
        }
    };
}
