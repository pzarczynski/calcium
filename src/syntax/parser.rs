use crate::syntax::ast::*;
use crate::syntax::lexer::{Span, Spanned, Token};
use std::iter::Peekable;

#[derive(Debug, Clone, PartialEq)]
pub enum Expected<'src> {
    Token(Token<'src>),
    Ident,
    Expr,
    Stmt,
}

impl<'src> From<Token<'src>> for Expected<'src> {
    fn from(t: Token<'src>) -> Self {
        Self::Token(t)
    }
}

trait TokenMatcher<'src>: Copy {
    fn matches(self, tok: Token<'src>) -> bool;
}

impl TokenMatcher<'_> for Token<'_> {
    fn matches(self, tok: Token) -> bool {
        self == tok
    }
}

impl<'src, F> TokenMatcher<'src> for F
where
    F: FnOnce(Token<'src>) -> bool + Copy,
{
    fn matches(self, tok: Token<'src>) -> bool {
        self(tok)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PErr<'src> {
    pub span: Option<Span>,
    pub expected: Vec<Expected<'src>>,
    pub got: Option<Token<'src>>,
}

pub type PResult<'src, T> = Result<T, PErr<'src>>;

pub struct Parser<'src, I>
where
    I: Iterator<Item = Spanned<Token<'src>>>,
{
    toks: Peekable<I>,
}

impl<'src, I> Parser<'src, I>
where
    I: Iterator<Item = Spanned<Token<'src>>>,
{
    pub fn parse(src: I) -> Result<Program<'src>, Vec<PErr<'src>>> {
        let mut parser = Parser {
            toks: src.peekable(),
        };

        let mut stmts = Vec::new();
        let mut errors = Vec::new();

        loop {
            parser.eat_while(Token::Sep);
            if parser.at_eoi() {
                break;
            }
            match parser.stmt() {
                Ok(stmt) => stmts.push(stmt),
                Err(err) => {
                    errors.push(err);
                    parser.sync();
                }
            }
        }

        if errors.is_empty() {
            Ok(Program::from(stmts))
        } else {
            Err(errors)
        }
    }

    // combinator helpers

    fn err_here(&mut self, expected: impl Into<Vec<Expected<'src>>>) -> PErr<'src> {
        let (got, span) = self.toks.peek().cloned().unzip();
        PErr {
            span,
            expected: expected.into(),
            got,
        }
    }

    fn at_eoi(&mut self) -> bool {
        self.toks.peek().is_none()
    }

    fn eat(&mut self, p: impl TokenMatcher<'src>) -> bool {
        self.toks.next_if(|(t, _)| p.matches(*t)).is_some()
    }

    fn eat_while(&mut self, p: impl TokenMatcher<'src>) {
        while self.eat(p) {}
    }

    fn expect(&mut self, expected: Token<'src>) -> PResult<'src, ()> {
        self.eat(expected)
            .then_some(())
            .ok_or_else(|| self.err_here(vec![expected.into()]))
    }

    fn ident(&mut self) -> PResult<'src, &'src str> {
        match self.toks.peek() {
            Some(&(Token::Ident(name), _)) => {
                self.toks.next();
                Ok(name)
            }
            _ => Err(self.err_here(vec![Expected::Ident])),
        }
    }

    /// parses a sequence up to `close` (consuming `close`).
    fn until<T>(
        &mut self,
        close: Token<'src>,
        mut f: impl FnMut(&mut Self) -> PResult<'src, T>,
    ) -> PResult<'src, Vec<T>> {
        let mut items = Vec::new();
        while !self.eat(close) {
            if self.at_eoi() {
                return Err(self.err_here(vec![close.into()]));
            }
            items.push(f(self)?);
        }
        Ok(items)
    }

    /// parses a delimiter-separated sequence up to `close`.
    fn delimited<T>(
        &mut self,
        sep: Token<'src>,
        close: Token<'src>,
        mut f: impl FnMut(&mut Self) -> PResult<'src, T>,
    ) -> PResult<'src, Vec<T>> {
        let mut items = Vec::new();
        while !self.eat(close) {
            items.push(f(self)?);
            if !self.eat(sep) {
                if !self.eat(close) {
                    return Err(self.err_here(vec![sep.into(), close.into()]));
                }
                break;
            }
        }
        Ok(items)
    }

    fn sync(&mut self) {
        self.eat_while(|t| !matches!(t, Token::Sep | Token::Fn | Token::Return | Token::RBrack))
    }

    // grammar rules

    fn stmt(&mut self) -> PResult<'src, Stmt<'src>> {
        if self.eat(Token::Fn) {
            return self.function();
        }

        let stmt = if self.eat(Token::Return) {
            Stmt::Return(self.expr()?)
        } else {
            if self.at_eoi() {
                return Err(self.err_here(vec![Expected::Stmt]));
            }
            let lhs = self.expr()?;
            if self.eat(Token::Eq) {
                let rhs = self.expr()?;
                Stmt::Assign { lhs, rhs }
            } else {
                Stmt::Expr(lhs)
            }
        };

        self.expect(Token::Sep)?;
        Ok(stmt)
    }

    fn function(&mut self) -> PResult<'src, Stmt<'src>> {
        let name = self.ident()?;
        self.expect(Token::LParen)?;
        let args = self.delimited(Token::Comma, Token::RParen, Self::ident)?;
        self.expect(Token::LBrack)?;
        let body = self.until(Token::RBrack, Self::stmt)?.into();
        Ok(Stmt::FnDef { name, args, body })
    }

    fn expr(&mut self) -> PResult<'src, Expr<'src>> {
        self.pratt(0)
    }

    fn pratt(&mut self, min_bp: u16) -> PResult<'src, Expr<'src>> {
        let mut lhs = self.prefix()?;

        while let Some(op) = self.peek_infix() {
            if op.bp() < min_bp {
                break;
            }
            self.toks.next(); // consume the operator token

            lhs = match op {
                PostfixOrInfix::Call => {
                    let args = self.delimited(Token::Comma, Token::RParen, Self::expr)?;
                    Expr::Call {
                        fun: Box::new(lhs),
                        args,
                    }
                }
                PostfixOrInfix::Infix(op) => {
                    let rhs = self.pratt(op.bp().1)?;
                    Expr::infix(lhs, op, rhs)
                }
            };
        }

        Ok(lhs)
    }

    fn peek_infix(&mut self) -> Option<PostfixOrInfix> {
        let (tok, _) = self.toks.peek()?;
        match tok {
            Token::LParen => Some(PostfixOrInfix::Call),
            tok => InfixOp::try_from(tok).ok().map(PostfixOrInfix::Infix),
        }
    }

    fn prefix(&mut self) -> PResult<'src, Expr<'src>> {
        let (tok, span) = self
            .toks
            .next_if(|(t, _)| *t != Token::Sep)
            .ok_or_else(|| self.err_here(vec![Expected::Expr]))?;

        match tok {
            Token::Num(n) => Ok(Expr::Num(n)),
            Token::Ident(name) => Ok(Expr::Ident(name)),
            Token::LParen => {
                let inner = self.pratt(0)?;
                self.expect(Token::RParen)?;
                Ok(inner)
            }
            other => match PrefixOp::try_from(&other) {
                Ok(op) => Ok(Expr::prefix(op, self.pratt(op.bp().1)?)),
                Err(_) => Err(PErr {
                    span: Some(span),
                    expected: vec![Expected::Expr],
                    got: Some(other),
                }),
            },
        }
    }
}
