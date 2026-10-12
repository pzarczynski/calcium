use chumsky::{input::ValueInput, prelude::*};

use super::ast::*;
use super::lexer::Token;

use chumsky::pratt::{infix, left, postfix, prefix, right};

pub type PErr<'src> = Rich<'src, Token>;
type PExtra<'src> = extra::Err<PErr<'src>>;

pub type Span = chumsky::span::SimpleSpan;
pub type Spanned<T> = (T, Span);

fn prefix_assoc(op: PrefixOp) -> u16 {
    op.prec()
}

fn infix_assoc(op: InfixOp) -> chumsky::pratt::Associativity {
    let p = op.prec();
    match op.assoc() {
        Assoc::Left => chumsky::pratt::left(p),
        Assoc::Right => chumsky::pratt::right(p),
    }
}

fn prefix_expr(op: PrefixOp, expr: Expr) -> Expr {
    Expr::Prefix {
        op,
        expr: Box::new(expr),
    }
}

fn infix_expr(l: Expr, op: InfixOp, r: Expr) -> Expr {
    Expr::Infix {
        l: Box::new(l),
        op,
        r: Box::new(r),
    }
}

fn call_expr(fun: Expr, args: Vec<Expr>) -> Expr {
    Expr::Call {
        fun: Box::new(fun),
        args,
    }
}

pub fn parser<'src, I>() -> impl Parser<'src, I, Program, PExtra<'src>> + Clone
where
    I: ValueInput<'src, Token = Token, Span = Span>,
{
    let expr = recursive::<_, Expr, PExtra<'src>, _, _>(|expr| {
        let ident = select! {
            Token::Ident(name) => Expr::Ident(name),
        };

        let number = select! {
            Token::Num(v) => Expr::Num(v),
        };

        let grouped = expr
            .clone()
            .delimited_by(just(Token::LParen), just(Token::RParen));

        let primary = choice((number, ident, grouped));

        let call = expr
            .clone()
            .separated_by(just(Token::Comma))
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LParen), just(Token::RParen));

        let pow_op = select! {
            Token::Sup => InfixOp::Pow
        };

        let unary_op = select! {
            Token::Plus  => PrefixOp::Pos,
            Token::Minus => PrefixOp::Neg,
        };

        let mul_op = select! {
            Token::Star  => InfixOp::Mul,
            Token::Slash => InfixOp::Div,
        };

        let sum_op = select! {
            Token::Plus  => InfixOp::Add,
            Token::Minus => InfixOp::Sub,
        };

        let assign_op = select! {
            Token::Eq => InfixOp::Assign
        };

        primary
            .pratt((
                infix(infix_assoc(InfixOp::Assign), assign_op, |l, op, r, _| {
                    infix_expr(l, op, r)
                }),
                infix(infix_assoc(InfixOp::Add), sum_op, |l, op, r, _| {
                    infix_expr(l, op, r)
                }),
                infix(infix_assoc(InfixOp::Mul), mul_op, |l, op, r, _| {
                    infix_expr(l, op, r)
                }),
                prefix(prefix_assoc(PrefixOp::Pos), unary_op, |op, expr, _| {
                    prefix_expr(op, expr)
                }),
                infix(infix_assoc(InfixOp::Pow), pow_op, |l, op, r, _| {
                    infix_expr(l, op, r)
                }),
                postfix(CALL_BP, call, |fun, args, _| call_expr(fun, args)),
            ))
            .boxed()
    });

    let sep = just(Token::Sep).repeated().at_least(1).ignored();

    sep.clone()
        .or_not()
        .ignore_then(expr.separated_by(sep).allow_trailing().collect::<Vec<_>>())
        .map(|exprs| Program { exprs })
        .then_ignore(end())
}
