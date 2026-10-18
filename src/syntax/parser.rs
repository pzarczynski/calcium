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

impl<'src> From<Token<'src>> for Vec<Expected<'src>> {
    fn from(t: Token<'src>) -> Self {
        vec![t.into()]
    }
}

impl<'src> From<Expected<'src>> for Vec<Expected<'src>> {
    fn from(e: Expected<'src>) -> Self {
        vec![e]
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
    pub fn parse(src: I) -> Result<Program, Vec<PErr<'src>>> {
        let mut parser = Parser {
            toks: src.peekable(),
        };
        let mut stmts = Vec::new();
        let mut errors = Vec::new();

        while {
            parser.maybe_all_eq(&Token::Sep);
            parser.toks.peek().is_some()
        } {
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

    fn maybe(&mut self, pred: impl FnOnce(&Token<'src>) -> bool) -> Option<Spanned<Token<'src>>> {
        self.toks.next_if(|(t, _)| pred(t))
    }

    fn maybe_all(&mut self, pred: impl Fn(&Token<'src>) -> bool) {
        while self.maybe(&pred).is_some() {}
    }

    fn maybe_eq(&mut self, expected: &Token<'src>) -> Option<Span> {
        self.maybe(|t| t == expected).map(|(_, span)| span)
    }

    fn maybe_all_eq(&mut self, expected: &Token<'src>) {
        self.maybe_all(|t| t == expected);
    }

    fn only(&mut self, expected: Token<'src>) -> PResult<'src, Span> {
        self.maybe_eq(&expected)
            .ok_or_else(|| self.err_here(expected))
    }

    fn only_ident(&mut self) -> PResult<'src, String> {
        self.toks
            .next_if_map(|(tok, span)| match tok {
                Token::Ident(name) => Ok(name.to_string()),
                other => Err((other, span)),
            })
            .ok_or_else(|| self.err_here(Expected::Ident))
    }

    /// parses a sequence up to `close` (consuming `close`).
    fn until<T>(
        &mut self,
        close: Token<'src>,
        mut f: impl FnMut(&mut Self) -> PResult<'src, T>,
    ) -> PResult<'src, Vec<T>> {
        let mut items = Vec::new();
        while self.maybe_eq(&close).is_none() {
            if self.toks.peek().is_none() {
                return Err(self.err_here(close));
            }
            items.push(f(self)?);
        }

        Ok(items)
    }

    /// parses a delimiter-separated sequence up to `close` (supporting trailing delimiters).
    fn delimited<T>(
        &mut self,
        sep: Token<'src>,
        close: Token<'src>,
        mut f: impl FnMut(&mut Self) -> PResult<'src, T>,
    ) -> PResult<'src, Vec<T>> {
        let mut items = Vec::new();
        while self.maybe_eq(&close).is_none() {
            items.push(f(self)?);
            if self.maybe_eq(&sep).is_none() {
                if self.maybe_eq(&close).is_none() {
                    return Err(self.err_here(vec![sep.into(), close.into()]));
                }
                break;
            }
        }
        Ok(items)
    }

    fn sync(&mut self) {
        self.maybe_all(|t| !matches!(t, Token::Sep | Token::Fn | Token::Return | Token::RBrack))
    }

    // grammar rules

    fn stmt(&mut self) -> PResult<'src, Stmt> {
        if self.maybe_eq(&Token::Fn).is_some() {
            return self.function();
        }

        let stmt = if self.maybe_eq(&Token::Return).is_some() {
            Stmt::Return(self.expr()?)
        } else {
            if self.toks.peek().is_none() {
                return Err(self.err_here(Expected::Stmt));
            }
            let lhs = self.expr()?;
            if self.maybe_eq(&Token::Eq).is_some() {
                let rhs = self.expr()?;
                Stmt::Assign { lhs, rhs }
            } else {
                Stmt::Expr(lhs)
            }
        };

        self.only(Token::Sep)?;
        Ok(stmt)
    }

    fn function(&mut self) -> PResult<'src, Stmt> {
        let name = self.only_ident()?;
        self.only(Token::LParen)?;
        let args = self.delimited(Token::Comma, Token::RParen, Self::only_ident)?;
        self.only(Token::LBrack)?;
        let body = self.until(Token::RBrack, Self::stmt)?.into();
        Ok(Stmt::FnDef { name, args, body })
    }

    fn expr(&mut self) -> PResult<'src, Expr> {
        self.pratt(0)
    }

    fn pratt(&mut self, min_bp: u16) -> PResult<'src, Expr> {
        let mut lhs = self.prefix()?;

        while let Some((op, l_bp)) = self.peek_infix() {
            if l_bp < min_bp {
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

    fn peek_infix(&mut self) -> Option<(PostfixOrInfix, u16)> {
        let (tok, _) = self.toks.peek()?;
        match tok {
            Token::LParen => Some((PostfixOrInfix::Call, 1000)),
            tok => InfixOp::try_from(tok).ok().map(|op| {
                let l_bp = op.bp().0;
                (PostfixOrInfix::Infix(op), l_bp)
            }),
        }
    }

    fn prefix(&mut self) -> PResult<'src, Expr> {
        let (tok, span) = self
            .toks
            .next()
            .ok_or_else(|| self.err_here(Expected::Expr))?;
        match tok {
            Token::Num(n) => Ok(Expr::Num(n.to_string())),
            Token::Ident(name) => Ok(Expr::Ident(name.to_string())),
            Token::LParen => {
                let inner = self.pratt(0)?;
                self.only(Token::RParen)?;
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
