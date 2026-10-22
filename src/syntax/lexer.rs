use std::cell::RefCell;

use crate::syntax::{Span, span::FileId};
use lasso::{Key as _, Rodeo, Spur};
use logos::Logos;

thread_local! {
    static INTERNER: RefCell<Rodeo> = RefCell::new(Rodeo::default());
}

pub type Spanned<T> = (T, Span);

#[derive(Debug, Clone, PartialEq, Default)]
pub enum LexError {
    #[default]
    UnexpectedToken,
    IntOverflow,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol(Spur);

impl Symbol {
    pub fn intern(s: &str) -> Self {
        INTERNER.with(|r| Self(r.borrow_mut().get_or_intern(s)))
    }

    pub fn as_str(&self) -> String {
        INTERNER.with(|r| r.borrow().resolve(&self.0).to_owned())
    }
}

impl TryFrom<usize> for Symbol {
    type Error = ();

    fn try_from(u: usize) -> Result<Self, Self::Error> {
        Spur::try_from_usize(u).map(Symbol).ok_or(())
    }
}

#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[logos(error = LexError, skip r"[ \r\n\t\f]+")]
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
    #[regex(
        r"[0-9]+", 
        |lex| lex.slice().parse().map_err(|_| LexError::IntOverflow)
    )]
    Int(i64),
    #[regex(
        "[A-Za-z_][a-zA-Z0-9_]*", 
        |lex| Symbol::intern(lex.slice())
    )]
    Ident(Symbol),
}

pub struct Lexed {
    pub tokens: Vec<(Token, Span)>,
    pub errors: Vec<(LexError, Span)>,
}

impl Lexed {
    pub fn new(fileid: FileId, src: &str) -> Self {
        let (mut tokens, mut errors) = (Vec::new(), Vec::new());
        let mut lexer = Token::lexer(src);
        while let Some(res) = lexer.next() {
            let span = Span::from_range(fileid, lexer.span());
            match res {
                Ok(tok) => tokens.push((tok, span)),
                Err(e) => errors.push((e, span)),
            }
        }
        Self { tokens, errors }
    }
}
