use logos::Span;

use super::ast::*;
use super::lexer::{Spanned, Token};
use std::iter::Peekable;

#[derive(Debug, Clone, PartialEq)]
pub struct PErr<'src> {
    pub span: Span,
    pub msg: String,
    pub expected: Option<&'static str>,
    pub got: Option<&'src str>,
}

pub trait Parsable<'src>: Sized {
    fn parse<I>(s: &mut Peekable<I>) -> Result<Self, Vec<PErr<'src>>>
    where
        I: Iterator<Item = Spanned<Token<'src>>>;
}

impl<'src> Parsable<'src> for Program {
    fn parse<I>(s: &mut Peekable<I>) -> Result<Self, Vec<PErr<'src>>>
    where
        I: Iterator<Item = Spanned<Token<'src>>>,
    {
        let mut exprs = Vec::new();
        let mut errors = Vec::new();

        while let Some((tok, _)) = s.peek() {
            match tok {
                Token::Sep => {
                    s.next();
                }
                _ => match Expr::parse(s) {
                    Ok(expr) => exprs.push(expr),
                    Err(mut errs) => errors.append(&mut errs),
                },
            }
        }

        if errors.is_empty() {
            Ok(Program { exprs })
        } else {
            Err(errors)
        }
    }
}

impl<'src> Parsable<'src> for Expr {
    fn parse<I>(s: &mut Peekable<I>) -> Result<Self, Vec<PErr<'src>>>
    where
        I: Iterator<Item = Spanned<Token<'src>>>,
    {
        let mut errors = Vec::new();

        match s.peek() {
            Some((Token::Ident(_), _)) | Some((Token::Num(_), _)) | Some((Token::LParen, _)) => {
                match Expr::pratt(s, 0) {
                    Ok(expr) => Ok(expr),
                    Err(mut errs) => {
                        errors.append(&mut errs);
                        Err(errors)
                    }
                }
            }
            Some((tok, span)) => {
                errors.push(PErr {
                    span: span.clone(),
                    msg: format!("Unexpected token: {tok:?}"),
                    expected: Some("expression"),
                    got: None,
                });
                s.next();
                Err(errors)
            }
            None => {
                errors.push(PErr {
                    span: Span::from(0..0),
                    msg: "Unexpected end of input".to_string(),
                    expected: Some("expression"),
                    got: None,
                });
                s.next();
                Err(errors)
            }
        }
    }
}

fn as_prefix_op(tok: &Token) -> Option<PrefixOp> {
    match tok {
        Token::Plus => Some(PrefixOp::Pos),
        Token::Minus => Some(PrefixOp::Neg),
        _ => None,
    }
}

fn as_infix_op(tok: &Token) -> Option<InfixOp> {
    match tok {
        Token::Plus => Some(InfixOp::Add),
        Token::Minus => Some(InfixOp::Sub),
        Token::Star => Some(InfixOp::Mul),
        Token::Slash => Some(InfixOp::Div),
        Token::Sup => Some(InfixOp::Pow),
        Token::Eq => Some(InfixOp::Assign),
        _ => None,
    }
}

impl<'src> Expr {
    fn parse_call_args<I>(s: &mut Peekable<I>) -> Result<Vec<Self>, Vec<PErr<'src>>>
    where
        I: Iterator<Item = Spanned<Token<'src>>>,
    {
        let mut args = Vec::new();

        if let Some((Token::RParen, _)) = s.peek() {
            s.next();
            return Ok(args);
        }

        loop {
            args.push(Expr::pratt(s, 0)?);

            match s.next() {
                Some((Token::Comma, _)) => {
                    if let Some((Token::RParen, _)) = s.peek() {
                        s.next();
                        break;
                    }
                }
                Some((Token::RParen, _)) => break,
                Some((other, span)) => {
                    return Err(vec![PErr {
                        span,
                        msg: format!("Expected ',' or ')', found {other:?}"),
                        expected: Some("',' or ')'"),
                        got: None,
                    }]);
                }
                None => {
                    return Err(vec![PErr {
                        span: 0..0,
                        msg: "Unclosed function arguments; reached EOF".to_string(),
                        expected: Some("')'"),
                        got: None,
                    }]);
                }
            }
        }

        Ok(args)
    }

    pub fn pratt<I>(s: &mut Peekable<I>, min_bp: u16) -> Result<Self, Vec<PErr<'src>>>
    where
        I: Iterator<Item = Spanned<Token<'src>>>,
    {
        let (tok, span) = match s.next() {
            Some(pair) => pair,
            None => {
                return Err(vec![PErr {
                    span: 0..0,
                    msg: "Unexpected end of input".to_string(),
                    expected: Some("expression"),
                    got: None,
                }]);
            }
        };

        let mut lhs = match tok {
            Token::Num(n) => Expr::Num(n.to_string()),
            Token::Ident(name) => Expr::Ident(name.to_string()),
            Token::LParen => {
                let inner = Expr::pratt(s, 0)?;
                match s.next() {
                    Some((Token::RParen, _)) => inner,
                    Some((other, err_span)) => {
                        return Err(vec![PErr {
                            span: err_span,
                            msg: format!("Expected ')', found {other:?}"),
                            expected: Some("')'"),
                            got: None,
                        }]);
                    }
                    None => {
                        return Err(vec![PErr {
                            span,
                            msg: "Unclosed '('; reached end of input".to_string(),
                            expected: Some("')'"),
                            got: None,
                        }]);
                    }
                }
            }

            tok if as_prefix_op(&tok).is_some() => {
                let op = as_prefix_op(&tok).unwrap();
                let (_, r_bp) = op.bp();
                let rhs = Expr::pratt(s, r_bp)?;
                Expr::prefix(op, rhs)
            }

            _ => {
                return Err(vec![PErr {
                    span,
                    msg: format!("Expected expression, found {tok:?}"),
                    expected: Some("expression"),
                    got: None,
                }]);
            }
        };
        while let Some((peeked_tok, _)) = s.peek() {
            if *peeked_tok == Token::LParen {
                if CALL_BP < min_bp {
                    break;
                }

                s.next();
                let args = Self::parse_call_args(s)?;
                lhs = Expr::Call {
                    fun: Box::new(lhs),
                    args,
                };
                continue;
            }

            if let Some(op) = as_infix_op(peeked_tok) {
                let (l_bp, r_bp) = op.bp();
                if l_bp < min_bp {
                    break;
                }

                s.next();
                let rhs = Expr::pratt(s, r_bp)?;
                lhs = Expr::infix(lhs, op, rhs);
                continue;
            }

            break;
        }

        Ok(lhs)
    }
}
