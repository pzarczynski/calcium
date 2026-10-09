mod ast;
mod lexer;
mod parser;

fn main() -> Result<(), lexer::LexError> {
    let mut lexer = lexer::Lexer::new("(12 + 34) * 5");
    let tokens = lexer.tokenize()?;

    println!("{tokens:?}");

    Ok(())
}
