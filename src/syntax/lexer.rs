use logos::Logos;
use std::{fmt, ops::Range};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl<T: Into<usize>> From<Range<T>> for Span {
    fn from(r: Range<T>) -> Self {
        Span {
            start: r.start.into(),
            end: r.end.into(),
        }
    }
}

impl ariadne::Span for Span {
    type SourceId = ();

    fn source(&self) -> &Self::SourceId {
        &()
    }

    fn start(&self) -> usize {
        self.start
    }

    fn end(&self) -> usize {
        self.end
    }
}

pub type Spanned<T> = (T, Span);

pub trait Lex<'src>: Logos<'src> + Sized + 'src
where
    Self::Extras: Default,
{
    fn lex(src: &'src Self::Source) -> impl Iterator<Item = Spanned<Self>>;
}

impl<'src> Lex<'src> for Token {
    fn lex(src: &'src Self::Source) -> impl Iterator<Item = Spanned<Self>> {
        Self::lexer(src).spanned().map(|(tok, r)| {
            let span: Span = r.into();
            match tok {
                Ok(tok) => (tok, span),
                Err(_) => (Self::Invalid, span),
            }
        })
    }
}

#[derive(Logos, Debug, Clone, PartialEq, Eq, Hash)]
#[logos(skip r"[ \r\n\t\f]+")]
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

    #[regex(r"[0-9]+", |lex| lex.slice().parse::<i64>().unwrap())]
    Int(i64),

    #[regex("[A-Za-z_][a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    Ident(String),

    Invalid,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Sep => write!(f, ";"),
            Token::Eq => write!(f, "="),
            Token::Fn => write!(f, "fn"),
            Token::Return => write!(f, "return"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Star => write!(f, "*"),
            Token::Slash => write!(f, "/"),
            Token::Comma => write!(f, ","),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::LBrace => write!(f, "{{"),
            Token::RBrace => write!(f, "}}"),
            Token::Int(n) => write!(f, "{n}"),
            Token::Ident(id) => write!(f, "{id}"),
            Token::Invalid => write!(f, "<invalid>"),
        }
    }
}
