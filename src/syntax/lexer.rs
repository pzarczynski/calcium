use std::fmt;

use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\f]+")]
pub enum Token {
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

    #[regex(r"[0-9]+(\.[0-9]+)?", |lex| lex.slice().to_owned())]
    Num(String),

    #[regex("[A-Za-z_][a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    Ident(String),
}

impl fmt::Display for Token {
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
