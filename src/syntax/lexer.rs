use crate::syntax::{Span, span::FileId};
use logos::Logos;
use crate::{Ctx, Symbol};
use itertools::{Either, Itertools};

pub type Spanned<T> = (T, Span);

#[derive(Debug, Clone, PartialEq, Default)]
pub enum LexError {
    #[default]
    InvalidToken,
    IntOverflow,
}

fn int(lex: &mut logos::Lexer<Token>) -> Result<i64, LexError> {
    lex.slice().parse().map_err(|_| LexError::IntOverflow)
}

fn ident(lex: &mut logos::Lexer<Token>) -> Result<Symbol, LexError> {
    Ok(lex.extras.intern(lex.slice()))
}

#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[logos(error = LexError, extras = Ctx, skip r"[ \r\n\t\f]+")]
pub enum Token {
    #[regex(r";")]
    Sep,
    #[token("=")]
    Eq,
    #[token("fn")]
    Fn,
    #[token("return")]
    Return,
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token(",")]
    Comma,
    #[regex(r"[0-9]+", int)]
    Int(i64),
    #[regex("[A-Za-z_][a-zA-Z0-9_]*", ident)]
    Ident(Symbol),
}

pub struct Lexed {
    pub tokens: Vec<Spanned<Token>>,
    pub errors: Vec<Spanned<LexError>>,
}

impl Lexed {
    pub fn new(fileid: FileId, src: &str) -> Self {
        Self::with_ctx(fileid, src, Ctx::default())
    }

    pub fn with_ctx(fileid: FileId, src: &str, ctx: Ctx) -> Self {
        let (toks, errs): (Vec<_>, Vec<_>) = Token::lexer_with_extras(src, ctx)
            .spanned()
            .partition_map(|(res, range)| {
                let span = Span::from_range(fileid, range);
                match res {
                    Ok(tok) => Either::Left((tok, span)),
                    Err(e) => Either::Right((e, span)),
                }
            }
        );
        Self { tokens: toks, errors: errs }
    }
}
