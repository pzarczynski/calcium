use logos::Logos as _;

pub mod ast;
mod lexer;
mod parser;

use lexer::Token;
use parser::{Span, Spanned, parser};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("unexpected character")]
    Lex { span: Span },

    #[error("{msg}")]
    Parse {
        span: Span,
        msg: String,
        expected: Vec<String>,
        found: Option<String>,
    },
}

impl Error {
    pub fn span(&self) -> Span {
        match self {
            Self::Lex { span } | Self::Parse { span, .. } => *span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Lexed {
    pub tokens: Vec<Spanned<Token>>,
    pub eoi: Span,
}

pub fn lex(src: &str) -> Result<Lexed, Vec<Error>> {
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    for (res, range) in Token::lexer(src).spanned() {
        let span = Span::from(range);
        match res {
            Ok(tok) => tokens.push((tok, span)),
            Err(()) => errors.push(Error::Lex { span }),
        }
    }

    if errors.is_empty() {
        Ok(Lexed {
            tokens,
            eoi: Span::from(src.len()..src.len()),
        })
    } else {
        Err(errors)
    }
}

pub fn parse(lexed: &Lexed) -> Result<ast::Program, Vec<Error>> {
    let input = lexed.tokens.as_slice().map(lexed.eoi, |(t, s)| (t, s));
    let (prog, errs) = parser().parse(input).into_output_errors();

    match prog {
        Some(p) if errs.is_empty() => Ok(p),
        _ => Err(errs
            .into_iter()
            .map(|e| Error::Parse {
                span: *e.span(),
                msg: e.reason().to_string(),
                expected: e.expected().map(|p| p.to_string()).collect(),
                found: e.found().map(|t| format!("{t:?}")),
            })
            .collect()),
    }
}
