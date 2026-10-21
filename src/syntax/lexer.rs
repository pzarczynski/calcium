use crate::syntax::Span;
use lasso::{Key as _, Rodeo, Spur};
use logos::{Lexer, Logos};

pub type Spanned<'src, T> = (T, Span<'src>);

#[derive(Debug, Clone, PartialEq, Default)]
pub enum LexError {
    #[default]
    UnexpectedToken,
    IntOverflow,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Symbol(Spur);

impl TryFrom<usize> for Symbol {
    type Error = ();

    fn try_from(u: usize) -> Result<Self, Self::Error> {
        Spur::try_from_usize(u).map(Symbol).ok_or(())
    }
}

impl core::ops::Deref for Symbol {
    type Target = Spur;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type Interner = Rodeo;

// #[derive(Default)]
pub struct Extras {
    pub interner: Interner,
}

fn ident(lex: &mut Lexer<Token>) -> Symbol {
    Symbol(lex.extras.interner.get_or_intern(lex.slice()))
}

fn int(lex: &mut Lexer<Token>) -> Result<i64, LexError> {
    lex.slice().parse().map_err(|_| LexError::IntOverflow)
}

#[derive(Logos, Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[logos(extras = Extras, error = LexError, skip r"[ \r\n\t\f]+")]
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

pub struct Lexed<'src> {
    pub tokens: Vec<(Token, Span<'src>)>,
    pub errors: Vec<(LexError, Span<'src>)>,
    pub interner: Interner,
}

impl<'src> Lexed<'src> {
    pub fn new(source: &'src str, src: &str) -> Self {
        let interner = Interner::default();
        Self::with_interner(source, src, interner)
    }

    pub fn with_interner(source: &'src str, src: &str, interner: Interner) -> Self {
        let mut lexer = Token::lexer_with_extras(src, Extras { interner });
        let (tokens, errors) = Self::lex(source, &mut lexer);
        Self {
            tokens,
            errors,
            interner: lexer.extras.interner,
        }
    }

    fn lex(
        source: &'src str,
        lexer: &mut Lexer<Token>,
    ) -> (Vec<Spanned<'src, Token>>, Vec<Spanned<'src, LexError>>) {
        let (mut tokens, mut errors) = (Vec::new(), Vec::new());
        while let Some(res) = lexer.next() {
            let span = Span::from_range(source, lexer.span());
            match res {
                Ok(tok) => tokens.push((tok, span)),
                Err(e) => errors.push((e, span)),
            }
        }
        (tokens, errors)
    }
}
