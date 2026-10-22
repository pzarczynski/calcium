use crate::syntax::ast::*;
use crate::syntax::{Span, Spanned, Symbol, Token};
use itertools::Itertools as _;
use std::iter::Peekable;

const EOI: Option<Token> = None;

#[derive(Debug, Clone, PartialEq)]
pub enum Expected {
    Token(Token),
    Ident,
    Expr,
    Stmt,
}

impl From<Token> for Expected {
    fn from(t: Token) -> Self {
        Self::Token(t)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PErr {
    pub span: Option<Span>,
    pub expected: Vec<Expected>,
    pub got: Option<Token>,
}

pub type PResult<T> = Result<T, PErr>;

trait Parse: Iterator {
    fn parse(&mut self) -> Result<Program, Vec<PErr>>;
    fn parse_stmts_until(
        &mut self,
        end_delim: Option<Token>,
    ) -> impl Iterator<Item = PResult<Stmt>>;
    fn parse_stmt(&mut self) -> PResult<Stmt>;
    fn expect_stmt_end(&mut self) -> PResult<()>;
    fn parse_fn_decl(&mut self) -> PResult<Stmt>;
    fn parse_block(&mut self) -> PResult<Vec<Stmt>>;
    fn parse_expr(&mut self) -> PResult<Expr>;
    fn pratt(&mut self, min_prec: Prec) -> PResult<Expr>;
    fn peek_postfix_or_infix(&mut self) -> Option<PostfixOrInfix>;
    fn parse_prefix(&mut self) -> PResult<Expr>;

    fn repeat<T>(
        &mut self,
        f: impl FnMut(&mut Self) -> PResult<Option<T>> + '_,
    ) -> impl Iterator<Item = PResult<T>> + '_;

    fn peek_tok(&mut self) -> Option<Token>;
    fn at(&mut self, expected: Option<Token>) -> bool;
    fn eat(&mut self, expected: Token) -> bool;
    fn eat_while(&mut self, expected: Token) -> Self;
    fn expect(&mut self, expected: Token) -> PResult<Self>;
    fn ident(&mut self) -> PResult<Symbol>;
    fn delimited<T>(
        &mut self,
        sep: Token,
        close: Token,
        f: impl FnMut(&mut Self) -> PResult<T>,
    ) -> PResult<Vec<T>>;

    fn err_here<const N: usize>(&mut self, expected: [Expected; N]) -> PErr;
    fn sync(&mut self, end_delim: Option<Token>);
}

impl<I> Parse for I
where
    I: Iterator<Item = Spanned<Token>>,
{
    fn repeat<T>(
        &mut self,
        f: impl FnMut(&mut Self) -> PResult<Option<T>> + '_,
    ) -> impl Iterator<Item = PResult<T>> + '_ {
        std::iter::from_fn(move || f(self).transpose())
    }
}

impl<I> Parse for Peekable<I>
where
    I: Iterator<Item = Spanned<Token>>,
{
    fn parse(&mut self) -> Result<Program, Vec<PErr>> {
        let (stmts, errs) = self.parse_stmts_until(EOI).partition_result();

        errs.is_empty()
            .then(|| Program { stmts })
            .ok_or_else(|| errs)
    }

    fn parse_stmts_until(
        &mut self,
        end_delim: Option<Token>,
    ) -> impl Iterator<Item = PResult<Stmt>> {
        self.take_while(|&(t, _)| Some(t) != end_delim)
            .repeat(|this| this.parse_stmt())
    }

    fn parse_stmt(&mut self) -> PResult<Stmt> {
        match self.peek_tok() {
            None => Err(self.err_here([Expected::Stmt])),
            Some(Token::Fn) => self.parse_fn_decl(),
            Some(Token::Return) => {
                self.next(); // consume `return`
                let val = self.parse_expr()?;
                self.expect_stmt_end()?;
                Ok(Stmt::Return(val))
            }
            Some(_) => {
                let lhs = self.parse_expr()?;
                let stmt = if self.eat(Token::Eq) {
                    let rhs = self.parse_expr()?;
                    Stmt::Assign { lhs, rhs }
                } else {
                    Stmt::Expr(lhs)
                };
                self.expect_stmt_end()?;
                Ok(stmt)
            }
        }
    }

    /// Expects a statement terminator `;`. Allows omitting the trailing
    /// separator right before a closing brace `}`.
    fn expect_stmt_end(&mut self) -> PResult<()> {
        if self.eat(Token::Sep) || self.at(Some(Token::RBrace)) {
            Ok(())
        } else {
            Err(self.err_here([Expected::Token(Token::Sep)]))
        }
    }

    fn parse_fn_decl(&mut self) -> PResult<Stmt> {
        self.next(); // consume `fn`
        let name = self.ident()?;

        self.expect(Token::LParen)?;
        if self.at(Some(Token::RParen)) {
            return Err(self.err_here([Expected::Ident]));
        }
        let mut args = self.delimited(Token::Comma, Token::RParen, Self::ident)?;

        let body: Box<[Stmt]> = self.parse_block()?.into();

        // Guaranteed `Some` because we rejected empty `()` above
        let last_arg = args.pop().expect("non-empty args");
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

    fn parse_block(&mut self) -> PResult<Vec<Stmt>> {
        let (stmts, errs) = self
            .expect(Token::LBrace)?
            .parse_stmts_until(Some(Token::RBrace))
            .partition_result();

        errs.is_empty()
            .then(|| Program { stmts })
            .ok_or_else(|| errs)
    }

    fn parse_expr(&mut self) -> PResult<Expr> {
        self.pratt(0)
    }

    fn pratt(&mut self, min_prec: Prec) -> PResult<Expr> {
        let mut lhs = self.parse_prefix()?;

        while let Some(op) = self.peek_postfix_or_infix()
            && op.prec() >= min_prec
        {
            self.next(); // consume operator or `(`

            lhs = match op {
                PostfixOrInfix::Call => {
                    let args = self.delimited(Token::Comma, Token::RParen, Self::parse_expr)?;
                    Expr::call_many(lhs, args)
                }
                PostfixOrInfix::Infix(infix_op) => {
                    let rhs = self.pratt(infix_op.assoc().rhs())?;
                    Expr::infix(lhs, infix_op, rhs)
                }
            };
        }

        Ok(lhs)
    }

    fn peek_postfix_or_infix(&mut self) -> Option<PostfixOrInfix> {
        let tok = self.peek_tok()?;
        match tok {
            Token::LParen => Some(PostfixOrInfix::Call),
            _ => InfixOp::try_from(tok).ok().map(PostfixOrInfix::Infix),
        }
    }

    fn parse_prefix(&mut self) -> PResult<Expr> {
        let Some(tok) = self.peek_tok() else {
            return Err(self.err_here([Expected::Expr]));
        };

        match tok {
            Token::Int(n) => {
                self.next();
                Ok(Expr::Int(n))
            }
            Token::Ident(sym) => {
                self.next();
                Ok(Expr::Ident(sym))
            }
            Token::LParen => {
                self.next();
                let inner = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(inner)
            }
            other => {
                if let Ok(op) = PrefixOp::try_from(other) {
                    self.next();
                    let rhs = self.pratt(op.assoc().rhs())?;
                    Ok(Expr::prefix(op, rhs))
                } else {
                    Err(self.err_here([Expected::Expr]))
                }
            }
        }
    }

    // =========================================================================
    //  Combinators
    // =========================================================================

    fn peek_tok(&mut self) -> Option<Token> {
        self.peek().map(|&(tok, _)| tok)
    }

    fn at(&mut self, expected: Option<Token>) -> bool {
        self.peek_tok() == expected
    }

    fn eat(&mut self, expected: Token) -> bool {
        self.next_if(|&(t, _)| t == expected).is_some()
    }

    fn eat_while(&mut self, expected: Token) {
        while self.eat(expected) {}
    }

    fn expect(&mut self, expected: Token) -> PResult<&mut Self> {
        if self.eat(expected) {
            Ok(self)
        } else {
            Err(self.err_here([expected.into()]))
        }
    }

    fn ident(&mut self) -> PResult<Symbol> {
        match self.peek_tok() {
            Some(Token::Ident(name)) => {
                self.next();
                Ok(name)
            }
            _ => Err(self.err_here([Expected::Ident])),
        }
    }

    /// Parses a `sep`-separated sequence up to `close` (consuming `close`).
    /// Supports both empty lists `()` and trailing separators `(a, b,)`.
    fn delimited<T>(
        &mut self,
        sep: Token,
        close: Token,
        mut f: impl FnMut(&mut Self) -> PResult<T>,
    ) -> PResult<Vec<T>> {
        let mut items = Vec::new();

        while !self.eat(close) {
            if self.at(EOI) {
                return Err(self.err_here([sep.into(), close.into()]));
            }

            items.push(f(self)?);

            if !self.eat(sep) {
                if self.eat(close) {
                    break;
                }
                return Err(self.err_here([sep.into(), close.into()]));
            }
        }

        Ok(items)
    }

    /// Constructs a `PErr` at the current lookahead position without consuming the token.
    fn err_here<const N: usize>(&mut self, expected: [Expected; N]) -> PErr {
        let (got, span) = self.peek().cloned().unzip();
        PErr {
            span,
            expected: expected.into(),
            got,
        }
    }

    /// Advances until a statement boundary or `end_delim`.
    /// Guarantees forward progress even if halted on an unexpected token.
    fn sync(&mut self, end_delim: Option<Token>) {
        // If we are stuck on a synchronization token (like a stray `}` at top-level)
        // that is NOT our enclosing block's closing delimiter, consume it so we make progress.
        if let Some(tok) = self.peek_tok()
            && Some(tok) != end_delim
            && matches!(tok, Token::Fn | Token::Return | Token::RBrace)
        {
            self.next();
        }

        while let Some(tok) = self.peek_tok()
            && Some(tok) != end_delim
            && !matches!(tok, Token::Fn | Token::Return | Token::RBrace)
            && !self.eat(Token::Sep)
        {
            self.next();
        }
    }
}
