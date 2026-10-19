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

    // grammar rules

    fn stmt(&mut self) -> PResult<'src, Stmt<'src>> {
        if self.at_eoi() {
            return Err(self.err_here(vec![Expected::Stmt]));
        }

        if self.eat(Token::Fn) {
            return self.function();
        }

        let stmt = if self.eat(Token::Return) {
            Stmt::Return(self.expr()?)
        } else {
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
        let mut args = self.delimited(Token::Comma, Token::RParen, Self::ident)?;
        self.expect(Token::LBrace)?;
        let body: Box<[Stmt<'src>]> = self.until(Token::RBrace, Self::stmt)?.into();

        let last_arg = args
            .pop()
            .ok_or_else(|| self.err_here(vec![Expected::Ident]))?;

        let inner_fn = Expr::Fn {
            arg: last_arg,
            body,
        };

        let curried_fn = args.into_iter().rfold(inner_fn, |acc_expr, arg| Expr::Fn {
            arg,
            body: Box::new([Stmt::Return(acc_expr)]),
        });

        Ok(Stmt::Assign {
            lhs: Expr::Ident(name),
            rhs: curried_fn,
        })
    }

    fn expr(&mut self) -> PResult<'src, Expr<'src>> {
        self.pratt(0)
    }

    fn pratt(&mut self, min_prec: Prec) -> PResult<'src, Expr<'src>> {
        let mut lhs = self.prefix()?;

        while let Some(op) = self.postfix_or_infix()
            && op.prec() >= min_prec
        {
            self.toks.next(); // eat op

            lhs = match op {
                PostfixOrInfix::Call => {
                    let args = self.delimited(Token::Comma, Token::RParen, Self::expr)?;
                    Expr::call_many(lhs, args)
                }
                PostfixOrInfix::Infix(op) => {
                    let rhs = self.pratt(op.assoc().rhs())?;
                    Expr::infix(lhs, op, rhs)
                }
            };
        }

        Ok(lhs)
    }

    fn postfix_or_infix(&mut self) -> Option<PostfixOrInfix> {
        let (tok, _) = self.toks.peek()?;
        match tok {
            Token::LParen => Some(PostfixOrInfix::Call),
            tok => InfixOp::try_from(tok).ok().map(PostfixOrInfix::Infix),
        }
    }

    fn prefix(&mut self) -> PResult<'src, Expr<'src>> {
        let (tok, span) = self
            .eat_if(|t| t != Token::Sep)
            .ok_or_else(|| self.err_here(vec![Expected::Expr]))?;

        match tok {
            Token::Int(n) => Ok(Expr::Int(n)),
            Token::Ident(name) => Ok(Expr::Ident(name)),
            Token::LParen => {
                let inner = self.pratt(0)?;
                self.expect(Token::RParen)?;
                Ok(inner)
            }
            Token::Fn => {
                self.expect(Token::LParen)?;
                let args = self.delimited(Token::Comma, Token::RParen, Self::ident)?;
                self.expect(Token::LBrace)?;
                let body = self.until(Token::RBrace, Self::stmt)?;
                Expr::function(args, body).ok_or_else(|| self.err_here(vec![Expected::Ident]))
            }
            other => match PrefixOp::try_from(&other) {
                Ok(op) => Ok(Expr::prefix(op, self.pratt(op.assoc().rhs())?)),
                Err(_) => Err(PErr {
                    span: Some(span),
                    expected: vec![Expected::Expr],
                    got: Some(other),
                }),
            },
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

    fn at(&mut self, p: Token<'src>) -> bool {
        self.toks.peek().is_some_and(|(t, _)| *t == p)
    }

    fn at_eoi(&mut self) -> bool {
        self.toks.peek().is_none()
    }

    fn eat(&mut self, p: Token<'src>) -> bool {
        self.toks.next_if(|(t, _)| *t == p).is_some()
    }

    fn eat_if(&mut self, p: impl FnOnce(Token<'src>) -> bool) -> Option<Spanned<Token<'src>>> {
        self.toks.next_if(|(t, _)| p(*t))
    }

    fn eat_while(&mut self, p: Token<'src>) {
        while self.eat(p) {}
    }

    fn expect(&mut self, p: Token<'src>) -> PResult<'src, ()> {
        self.eat(p)
            .then_some(())
            .ok_or_else(|| self.err_here(vec![p.into()]))
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
        while self
            .eat_if(|t| !matches!(t, Token::Sep | Token::Fn | Token::Return | Token::RBrace))
            .is_some()
        {}
    }
}
