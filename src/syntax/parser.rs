use super::ast::*;
use super::lexer::{Span, Spanned, Token};
use std::fmt;
use std::iter::Peekable;

#[derive(Debug, Clone, PartialEq)]
enum Expected<'src> {
    Token(Token<'src>),
    Expr,
    Stmt,
    Stmts,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PErr<'src> {
    pub span: Option<Span>,
    pub expected: Vec<Expected<'src>>,
    pub got: Option<Token<'src>>,
}

impl fmt::Display for PErr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let expected_str = self
            .expected
            .iter()
            .map(|e| match e {
                Expected::Token(tok) => format!("token '{}'", tok),
                Expected::Expr => "expression".to_string(),
                Expected::Stmt => "statement".to_string(),
                Expected::Stmts => "statements".to_string(),
            })
            .collect::<Vec<_>>()
            .join(", ");

        let got_str = match &self.got {
            Some(tok) => format!("token '{}'", tok),
            None => "end of input".to_string(),
        };

        match &self.span {
            Some(span) => write!(
                f,
                "Parse error at {}..{}: expected {}, got {}",
                span.start, span.end, expected_str, got_str
            ),
            None => write!(f, "Parse error: expected {}, got {}", expected_str, got_str),
        }
    }
}

trait Eater {
    fn eat(&mut self) -> Option<()>;
}

impl<I, T> Eater for Peekable<I>
where
    I: Iterator<Item = T>,
{
    fn eat(&mut self) -> Option<()> {
        self.next().map(|_| ())
    }
}

pub struct Parser;

impl Parser {
    pub fn parse<'src>(
        src: impl Iterator<Item = Spanned<Token<'src>>>,
    ) -> Result<Program, Vec<PErr<'src>>> {
        let mut toks = src.peekable();
        let mut stmts = Vec::new();
        let mut errors = Vec::new();

        while let Some((tok, span)) = toks.next()
            && tok != Token::Sep
        {
            match Parser::parse_stmt(&mut toks) {
                Ok(stmt) => stmts.push(stmt),
                Err(err) => errors.push(err),
            }
        }

        errors
            .is_empty()
            .then(|| Program::from(stmts))
            .ok_or(errors)
    }

    fn parse_stmt<'src>(
        toks: &mut Peekable<impl Iterator<Item = Spanned<Token<'src>>>>,
    ) -> Result<Stmt, PErr<'src>> {
        let mut errors = Vec::new();

        match toks.peek() {
            Some((Token::Fn, _)) => Parser::parse_fn(toks),
            Some((Token::Return, _)) => {
                toks.next();
                let expr = Parser::parse_expr(toks)?;
                Ok(Stmt::Return(expr))
            }
            Some((Token::Ident(_), _)) => {
                let lhs = Parser::parse_expr(toks)?;
                match toks.peek() {
                    Some((Token::Eq, _)) => {
                        toks.next();
                        let rhs = Parser::parse_expr(toks)?;
                        Ok(Stmt::Assign { lhs, rhs })
                    }
                    _ => Ok(Stmt::Expr(lhs)),
                }
            }
            Some((tok, span)) => Err(PErr {
                span: Some(span.clone()),
                expected: vec![Expected::Token(*tok), Expected::Expr],
                got: Some(*tok),
            }),
            None => Err(PErr {
                span: None,
                expected: vec![Expected::Stmt],
                got: None,
            }),
        }
    }

    fn parse_fn<'src>(
        toks: &mut Peekable<impl Iterator<Item = Spanned<Token<'src>>>>,
    ) -> Result<Stmt, PErr<'src>> {
        let name = match toks.next() {
            Some((Token::Ident(name), _)) => Ok(name.to_string()),
            Some((tok, span)) => Err(PErr {
                span: Some(span),
                expected: vec![Expected::Token(tok)],
                got: Some(tok),
            }),
            None => Err(PErr {
                span: None,
                expected: vec![Expected::Token(Token::Ident("<function name>"))],
                got: None,
            }),
        }?;

        Ok(Stmt::FnDef {
            name,
            args: Parser::parse_fn_args(toks)?,
            body: Box::new(Parser::parse_fn_body(toks)?),
        })
    }

    fn parse_fn_args<'src>(
        toks: &mut Peekable<impl Iterator<Item = Spanned<Token<'src>>>>,
    ) -> Result<Vec<String>, PErr<'src>> {
        let mut args = Vec::new();

        match toks.next() {
            Some((Token::LParen, _)) => Ok(()),
            Some((tok, span)) => Err(PErr {
                span: Some(span),
                expected: vec![Expected::Token(Token::LParen)],
                got: Some(tok),
            }),
            None => Err(PErr {
                span: None,
                expected: vec![Expected::Token(Token::LParen)],
                got: None,
            }),
        }?;

        loop {
            match toks.next() {
                Some((Token::Ident(arg), _)) => Ok(args.push(arg.to_string())),
                Some((Token::RParen, _)) => break,
                Some((tok, span)) => Err(PErr {
                    span: Some(span),
                    expected: vec![
                        Expected::Token(Token::Ident("<argument>")),
                        Expected::Token(Token::RParen),
                    ],
                    got: Some(tok),
                }),
                None => Err(PErr {
                    span: None,
                    expected: vec![
                        Expected::Token(Token::Ident("<argument>")),
                        Expected::Token(Token::RParen),
                    ],
                    got: None,
                }),
            }?;

            match toks.peek() {
                Some((Token::Comma, _)) => Ok(toks.eat()),
                Some((Token::RParen, _)) => Ok(()),
                Some((tok, span)) => Err(PErr {
                    span: Some(span.clone()),
                    expected: vec![
                        Expected::Token(Token::Comma),
                        Expected::Token(Token::RParen),
                    ],
                    got: None,
                }),
                None => Err(PErr {
                    span: None,
                    expected: vec![
                        Expected::Token(Token::Comma),
                        Expected::Token(Token::RParen),
                    ],
                    got: None,
                }),
            }?
        }

        Ok(args)
    }

    fn parse_fn_body<'src>(
        toks: &mut Peekable<impl Iterator<Item = Spanned<Token<'src>>>>,
    ) -> Result<Stmts, PErr<'src>> {
        let mut stmts = Vec::new();

        match toks.next() {
            Some((Token::LBrack, _)) => Ok(()),
            Some((tok, span)) => Err(PErr {
                span: Some(span),
                expected: vec![Expected::Token(Token::LBrack)],
                got: Some(tok),
            }),
            None => Err(PErr {
                span: None,
                expected: vec![Expected::Token(Token::LBrack)],
                got: None,
            }),
        }?;

        stmts.extend(
            toks.take_while(|(tok, _)| *tok != Token::RBrack)
                .map(|(tok, _)| Parser::parse_stmt(toks)?),
        );

        Ok(Stmts::from(stmts))
    }

    fn parse_expr<'src, I>(
        toks: &mut Peekable<impl Iterator<Item = Spanned<Token<'src>>>>,
    ) -> Result<Expr, PErr<'src>> {
        match toks.peek() {
            Some((Token::Ident(_), _)) | Some((Token::Num(_), _)) | Some((Token::LParen, _)) => {
                Expr::pratt(toks, 0)?
            }
            Some((tok, span)) => {
                errors.push(PErr {
                    span: span.clone(),
                    expected: "expression",
                    got: Some(tok.to_string()),
                });
                toks.next();
                Err(errors)
            }
            None => {
                errors.push(PErr {
                    span: Span::from(0..0),
                    expected: "expression",
                    got: None,
                });
                toks.next();
                Err(errors)
            }
        }
    }
}

impl<'src> Expr {
    fn parse_call_args<I>(toks: &mut Peekable<I>) -> Result<Vec<Self>, Vec<PErr>>
    where
        I: Iterator<Item = Spanned<Token<'src>>>,
    {
        let mut args = Vec::new();

        if let Some((Token::RParen, _)) = toks.peek() {
            toks.next();
            return Ok(args);
        }

        loop {
            args.push(Expr::pratt(toks, 0)?);

            match toks.next() {
                Some((Token::Comma, _)) => {
                    if let Some((Token::RParen, _)) = toks.peek() {
                        toks.next();
                        break;
                    }
                }
                Some((Token::RParen, _)) => break,
                Some((other, span)) => {
                    return Err(vec![PErr {
                        span,
                        expected: "',' or ')'",
                        got: Some(other.to_string()),
                    }]);
                }
                None => {
                    return Err(vec![PErr {
                        span: 0..0,
                        expected: "')'",
                        got: None,
                    }]);
                }
            }
        }

        Ok(args)
    }

    pub fn pratt<I>(toks: &mut Peekable<I>, min_bp: u16) -> Result<Self, Vec<PErr>>
    where
        I: Iterator<Item = Spanned<Token<'src>>>,
    {
        let (tok, span) = match toks.next() {
            Some(pair) => pair,
            None => {
                return Err(vec![PErr {
                    span: 0..0,
                    expected: "expression",
                    got: None,
                }]);
            }
        };

        let mut lhs = match tok {
            Token::Num(n) => Expr::Num(n.to_string()),
            Token::Ident(name) => Expr::Ident(name.to_string()),
            Token::LParen => {
                let inner = Expr::pratt(toks, 0)?;
                match toks.next() {
                    Some((Token::RParen, _)) => inner,
                    Some((other, err_span)) => {
                        return Err(vec![PErr {
                            span: err_span,
                            expected: "')'",
                            got: None,
                        }]);
                    }
                    None => {
                        return Err(vec![PErr {
                            span,
                            expected: "')'",
                            got: None,
                        }]);
                    }
                }
            }

            tok if as_prefix_op(&tok).is_some() => {
                let op = as_prefix_op(&tok).unwrap();
                let (_, r_bp) = op.bp();
                let rhs = Expr::pratt(toks, r_bp)?;
                Expr::prefix(op, rhs)
            }

            _ => {
                return Err(vec![PErr {
                    span,
                    expected: "expression",
                    got: None,
                }]);
            }
        };
        while let Some((peeked_tok, _)) = toks.peek() {
            if *peeked_tok == Token::LParen {
                toks.next();
                let args = Self::parse_call_args(toks)?;
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

                toks.next();
                let rhs = Expr::pratt(toks, r_bp)?;
                lhs = Expr::infix(lhs, op, rhs);
                continue;
            }

            break;
        }

        Ok(lhs)
    }
}
