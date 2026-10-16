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

pub struct Parser<I: Iterator> {
    toks: Peekable<I>,
}

impl<'src, I> Parser<I>
where
    I: Iterator<Item = Spanned<Token<'src>>>,
{
    pub fn parse(src: I) -> Result<Program, Vec<PErr<'src>>> {
        let mut parser = Self {
            toks: src.peekable(),
        };
        let mut stmts = Vec::new();
        let mut errors = Vec::new();

        parser.eat_while_eq(&Token::Sep);

        while parser.toks.peek().is_some() {
            match parser.parse_stmt() {
                Ok(stmt) => stmts.push(stmt),
                Err(err) => {
                    errors.push(err);
                    parser.sync();
                }
            }
            parser.eat_while_eq(&Token::Sep);
        }

        errors
            .is_empty()
            .then(|| Program::from(stmts))
            .ok_or(errors)
    }

    // combinator helpers

    fn err_here(&mut self, expected: impl Into<Vec<Expected<'src>>>) -> PErr<'src> {
        match self.toks.peek().cloned() {
            Some((got, span)) => PErr {
                span: Some(span),
                expected: expected.into(),
                got: Some(got),
            },
            None => PErr {
                span: None,
                expected: expected.into(),
                got: None,
            },
        }
    }

    fn eat_if(&mut self, pred: impl FnOnce(&Token<'src>) -> bool) -> Option<Spanned<Token<'src>>> {
        self.toks.next_if(|(t, _)| pred(t))
    }

    fn eat_if_eq(&mut self, expected: &Token<'src>) -> Option<Span> {
        self.eat_if(|t| t == expected).map(|(_, span)| span)
    }

    fn eat_while_eq(&mut self, expected: &Token<'src>) -> () {
        std::iter::from_fn(|| self.eat_if_eq(expected)).for_each(drop)
    }

    fn expect(&mut self, expected: Token<'src>) -> PResult<'src, Span> {
        match self.eat_if_eq(&expected) {
            Some(span) => Ok(span),
            None => Err(self.err_here(expected)),
        }
    }

    fn expect_ident(&mut self) -> PResult<'src, String> {
        match self.eat_if(|t| matches!(t, Token::Ident(_))) {
            Some((Token::Ident(name), _)) => Ok(name.to_string()),
            _ => Err(self.err_here(Expected::Ident)),
        }
    }

    /// parses a sequence up to `close` (consuming `close`).
    fn parse_until<T>(
        &mut self,
        close: Token<'src>,
        mut f: impl FnMut(&mut Self) -> PResult<'src, T>,
    ) -> PResult<'src, Vec<T>> {
        let mut items = Vec::new();
        while self.eat_if_eq(&close).is_none() {
            if self.toks.peek().is_none() {
                return Err(self.err_here(close));
            }
            items.push(f(self)?);
        }
        Ok(items)
    }

    /// parses a delimiter-separated sequence up to `close` (supporting trailing delimiters).
    fn parse_delimited<T>(
        &mut self,
        sep: Token<'src>,
        close: Token<'src>,
        mut f: impl FnMut(&mut Self) -> PResult<'src, T>,
    ) -> PResult<'src, Vec<T>> {
        let mut items = Vec::new();
        while self.eat_if_eq(&close).is_none() {
            items.push(f(self)?);
            if self.eat_if_eq(&sep).is_none() {
                if self.eat_if_eq(&close).is_none() {
                    return Err(self.err_here(vec![Expected::Token(sep), Expected::Token(close)]));
                }
                break;
            }
        }
        Ok(items)
    }

    fn sync(&mut self) {
        while let Some((tok, _)) = self.toks.peek() {
            if matches!(tok, Token::Sep | Token::Fn | Token::Return | Token::RBrack) {
                break;
            }
            self.toks.next();
        }
    }

    // grammar rules

    fn parse_stmt(&mut self) -> PResult<'src, Stmt> {
        if self.eat_if_eq(&Token::Fn).is_some() {
            return self.parse_fn();
        }
        if self.eat_if_eq(&Token::Return).is_some() {
            return Ok(Stmt::Return(self.parse_expr()?));
        }
        if self.toks.peek().is_none() {
            return Err(self.err_here(vec![Expected::Stmt]));
        }

        let lhs = self.parse_expr()?;
        if self.eat_if_eq(&Token::Eq).is_some() {
            let rhs = self.parse_expr()?;
            Ok(Stmt::Assign { lhs, rhs })
        } else {
            Ok(Stmt::Expr(lhs))
        }
    }

    fn parse_fn(&mut self) -> PResult<'src, Stmt> {
        let name = self.expect_ident()?;
        self.expect(Token::LParen)?;
        let args = self.parse_delimited(Token::Comma, Token::RParen, Self::expect_ident)?;
        self.expect(Token::LBrack)?;
        let body = self.parse_until(Token::RBrack, Self::parse_stmt)?;

        Ok(Stmt::FnDef {
            name,
            args,
            body: Box::new(Stmts::from(body)),
        })
    }

    fn parse_expr(&mut self) -> PResult<'src, Expr> {
        self.pratt(0)
    }

    fn pratt(&mut self, min_bp: u16) -> PResult<'src, Expr> {
        let (tok, span) = self
            .toks
            .next()
            .ok_or_else(|| self.err_here(vec![Expected::Expr]))?;

        let mut lhs = match tok {
            Token::Num(n) => Expr::Num(n.to_string()),
            Token::Ident(name) => Expr::Ident(name.to_string()),
            Token::LParen => {
                let inner = self.pratt(0)?;
                self.expect(Token::RParen)?;
                inner
            }
            _ if let Ok(op) = PrefixOp::try_from(&tok) => {
                let (_, r_bp) = op.bp();
                let rhs = self.pratt(r_bp)?;
                Expr::prefix(op, rhs)
            }
            other => {
                return Err(PErr {
                    span: Some(span),
                    expected: vec![Expected::Expr],
                    got: Some(other),
                });
            }
        };

        while let Some((peeked_tok, _)) = self.toks.peek() {
            if *peeked_tok == Token::LParen {
                self.toks.next();
                let args = self.parse_delimited(Token::Comma, Token::RParen, Self::parse_expr)?;
                lhs = Expr::Call {
                    fun: Box::new(lhs),
                    args,
                };
                continue;
            }

            if let Ok(op) = InfixOp::try_from(peeked_tok) {
                let (l_bp, r_bp) = op.bp();
                if l_bp < min_bp {
                    break;
                }
                self.toks.next();
                let rhs = self.pratt(r_bp)?;
                lhs = Expr::infix(lhs, op, rhs);
                continue;
            }

            break;
        }

        Ok(lhs)
    }
}
