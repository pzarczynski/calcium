pub mod ast;
pub mod lexer;
mod parser;

pub use parser::parser;

use chumsky::prelude::*;
use lexer::Token;

type PError<'src> = Rich<'src, Token>;
type PExtra<'src> = extra::Err<PError<'src>>;
