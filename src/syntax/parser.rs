use crate::syntax::ast::*;
use crate::syntax::{Span, Spanned, Symbol, Token};
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
    #[inline]
    fn from(t: Token) -> Self {
        Self::Token(t)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PErr<'src> {
    pub span: Option<Span<'src>>,
    pub expected: Vec<Expected>,
    pub got: Option<Token>,
}

pub type PResult<'src, T> = Result<T, PErr<'src>>;

pub struct Parser<'src, I>
where
    I: Iterator<Item = Spanned<'src, Token>>,
{
    toks: Peekable<I>,
    errors: Vec<PErr<'src>>,
}

impl<'src, I> Parser<'src, I>
where
    I: Iterator<Item = Spanned<'src, Token>>,
{
    pub fn parse(src: I) -> Result<Program, Vec<PErr<'src>>> {
        let mut parser = Self {
            toks: src.peekable(),
            errors: Vec::new(),
        };

        let stmts = parser.parse_stmts_until(EOI);

        if parser.errors.is_empty() {
            Ok(Program::from(stmts))
        } else {
            Err(parser.errors)
        }
    }

    // =========================================================================
    //  Grammar rules
    // =========================================================================

    /// Parses a sequence of statements until `end_delim`,
    /// performing stmt-level error recovery along the way.
    fn parse_stmts_until(&mut self, end_delim: Option<Token>) -> Vec<Stmt> {
        let mut stmts = Vec::new();

        loop {
            self.eat_while(Token::Sep);

            if self.at(end_delim) || self.at(EOI) {
                break;
            }

            match self.stmt() {
                Ok(stmt) => stmts.push(stmt),
                Err(err) => {
                    self.errors.push(err);
                    self.sync(end_delim);
                }
            }
        }

        stmts
    }

    fn stmt(&mut self) -> PResult<'src, Stmt> {
        match self.peek_tok() {
            None => Err(self.err_here([Expected::Stmt])),
            Some(Token::Fn) => self.fn_decl(),
            Some(Token::Return) => {
                self.advance(); // consume `return`
                let val = self.expr()?;
                self.expect_stmt_end()?;
                Ok(Stmt::Return(val))
            }
            Some(_) => {
                let lhs = self.expr()?;
                self.finish_expr_or_assign_stmt(lhs)
            }
        }
    }

    fn finish_expr_or_assign_stmt(&mut self, lhs: Expr) -> PResult<'src, Stmt> {
        let stmt = if self.eat(Token::Eq) {
            let rhs = self.expr()?;
            Stmt::Assign { lhs, rhs }
        } else {
            Stmt::Expr(lhs)
        };
        self.expect_stmt_end()?;
        Ok(stmt)
    }

    /// Expects a statement terminator `;`. Allows omitting the trailing
    /// separator right before a closing brace `}`.
    fn expect_stmt_end(&mut self) -> PResult<'src, ()> {
        if self.eat(Token::Sep) || self.at(Some(Token::RBrace)) {
            Ok(())
        } else {
            Err(self.err_here([Expected::Token(Token::Sep)]))
        }
    }

    /// Parses `fn ident(args...) { body }`.
    fn fn_decl(&mut self) -> PResult<'src, Stmt> {
        self.advance(); // consume `fn`
        let name = self.ident()?;

        self.expect(Token::LParen)?;
        if self.at(Some(Token::RParen)) {
            return Err(self.err_here([Expected::Ident]));
        }
        let mut args = self.delimited(Token::Comma, Token::RParen, Self::ident)?;

        let body: Box<[Stmt]> = self.block()?.into();

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

    /// Parses `{ stmt* }` with inner separator handling and error recovery.
    fn block(&mut self) -> PResult<'src, Vec<Stmt>> {
        self.expect(Token::LBrace)?;
        let stmts = self.parse_stmts_until(Some(Token::RBrace));
        self.expect(Token::RBrace)?;
        Ok(stmts)
    }

    #[inline]
    fn expr(&mut self) -> PResult<'src, Expr> {
        self.pratt(0)
    }

    fn pratt(&mut self, min_prec: Prec) -> PResult<'src, Expr> {
        let mut lhs = self.prefix()?;

        while let Some(op) = self.peek_postfix_or_infix()
            && op.prec() >= min_prec
        {
            self.advance(); // consume operator or `(`

            lhs = match op {
                PostfixOrInfix::Call => {
                    let args = self.delimited(Token::Comma, Token::RParen, Self::expr)?;
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

    fn prefix(&mut self) -> PResult<'src, Expr> {
        let Some(tok) = self.peek_tok() else {
            return Err(self.err_here([Expected::Expr]));
        };

        match tok {
            Token::Int(n) => {
                self.advance();
                Ok(Expr::Int(n))
            }
            Token::Ident(sym) => {
                self.advance();
                Ok(Expr::Ident(sym))
            }
            Token::LParen => {
                self.advance();
                let inner = self.expr()?;
                self.expect(Token::RParen)?;
                Ok(inner)
            }
            other => {
                if let Ok(op) = PrefixOp::try_from(other) {
                    self.advance();
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

    #[inline]
    fn peek_tok(&mut self) -> Option<Token> {
        self.toks.peek().map(|&(tok, _)| tok)
    }

    #[inline]
    fn advance(&mut self) -> Option<Spanned<'src, Token>> {
        self.toks.next()
    }

    #[inline]
    fn at(&mut self, expected: Option<Token>) -> bool {
        self.peek_tok() == expected
    }

    #[inline]
    fn eat(&mut self, expected: Token) -> bool {
        self.toks.next_if(|&(t, _)| t == expected).is_some()
    }

    #[inline]
    fn eat_while(&mut self, expected: Token) {
        while self.eat(expected) {}
    }

    #[inline]
    fn expect(&mut self, expected: Token) -> PResult<'src, ()> {
        if self.eat(expected) {
            Ok(())
        } else {
            Err(self.err_here([expected.into()]))
        }
    }

    fn ident(&mut self) -> PResult<'src, Symbol> {
        match self.peek_tok() {
            Some(Token::Ident(name)) => {
                self.advance();
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
        mut f: impl FnMut(&mut Self) -> PResult<'src, T>,
    ) -> PResult<'src, Vec<T>> {
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
    fn err_here<const N: usize>(&mut self, expected: [Expected; N]) -> PErr<'src> {
        let (got, span) = self.toks.peek().cloned().unzip();
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
            self.advance();
        }

        while let Some(tok) = self.peek_tok()
            && Some(tok) != end_delim
            && !matches!(tok, Token::Fn | Token::Return | Token::RBrace)
            && !self.eat(Token::Sep)
        {
            self.advance();
        }
    }
}
