use logos::Logos;
use std::{fmt, ops::Range};
use thiserror::Error;

pub type Span = Range<usize>;
pub type Spanned<T> = (T, Span);

#[derive(Debug, Clone, PartialEq, Error)]
pub enum LErr {
    #[error("invalid token '{token}' at {span}")]
    InvalidToken { token: String, span: Span },
}

pub trait Lexable<'src>: Sized {
    type Source: ?Sized + 'src;
    fn lex(src: &'src Self::Source) -> impl Iterator<Item = Result<Spanned<Self>, Span>> + 'src;
}

impl<'src, T> Lexable<'src> for T
where
    T: Logos<'src> + 'src,
    T::Extras: Default,
{
    type Source = T::Source;

    fn lex(src: &'src Self::Source) -> impl Iterator<Item = Result<Spanned<Self>, Span>> + 'src {
        T::lexer(src)
            .spanned()
            .map(move |(res, span)| res.map(|tok| (tok, span.clone())).map_err(|_| span))
    }
}

#[derive(Logos, Debug, Clone, PartialEq, Eq, Hash)]
#[logos(skip r"[ \t\f]+")]
pub enum Token<'src> {
    #[regex(r";")]
    Sep,

    #[token("=")]
    Eq,

    #[token(":")]
    Colon,

    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("*")]
    Star,

    #[token("/")]
    Slash,

    #[token("^")]
    Sup,

    #[token(",")]
    Comma,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[regex(r"[0-9]+(\.[0-9]+)?", |lex| lex.slice())]
    Num(&'src str),

    #[regex("[A-Za-z_][a-zA-Z0-9_]*", |lex| lex.slice())]
    Ident(&'src str),
}

impl fmt::Display for Token<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Sep => write!(f, ";"),
            Token::Eq => write!(f, "="),
            Token::Colon => write!(f, ":"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Sup => write!(f, "^"),
            Token::Comma => write!(f, ","),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::Num(n) => write!(f, "{n}"),
            Token::Ident(id) => write!(f, "{id}"),
        }
    }
}
