use chumsky::prelude::*;

use crate::syntax::PExtra;
use crate::syntax::ast::{Arg, BinaryOp, ConstraintOp, Expr, Program, Stmt, UnaryOp};
use crate::syntax::lexer::Token;

#[derive(Clone)]
enum Suffix {
    Member(String),
    Call(Vec<Arg>),
}

fn binary(lhs: Expr, op: BinaryOp, rhs: Expr) -> Expr {
    Expr::Binary {
        lhs: Box::new(lhs),
        op,
        rhs: Box::new(rhs),
    }
}

// stmts = [ sep ] [ stmt { sep stmt } ] [ sep ]
fn stmts_parser<'src>(
    expr: impl Parser<'src, &'src [Token], Expr, PExtra<'src>> + Clone,
) -> impl Parser<'src, &'src [Token], Vec<Stmt>, PExtra<'src>> + Clone {
    let con_op = select! {
        Token::Eq    => ConstraintOp::Eq,
        Token::Ne    => ConstraintOp::Ne,
        Token::Ge    => ConstraintOp::Ge,
        Token::Le    => ConstraintOp::Le,
        Token::Gt    => ConstraintOp::Gt,
        Token::Lt    => ConstraintOp::Lt,
        Token::Colon => ConstraintOp::Colon,
    };

    // stmt = expr [ con_op expr ]
    let stmt = expr
        .clone()
        .then(con_op.then(expr).or_not())
        .map(|(lhs, constraint)| match constraint {
            Some((op, rhs)) => Stmt::Constraint { lhs, op, rhs },
            None => Stmt::Query(lhs),
        });

    // sep = SEP { SEP }
    let sep = just(Token::Sep).repeated().at_least(1).ignored();

    sep.clone()
        .or_not()
        .ignore_then(stmt.separated_by(sep).allow_trailing().collect::<Vec<_>>())
}

pub fn parser<'src>() -> impl Parser<'src, &'src [Token], Program, PExtra<'src>> + Clone {
    let expr = recursive::<_, Expr, PExtra<'src>, _, _>(|expr| {
        let ident = select! {
            Token::Ident(name) => name,
        };

        let number = select! {
            Token::Num(value) => Expr::Num(value),
        };

        // arg = [ ident EQUALS ] expr
        let arg = ident
            .then_ignore(just(Token::Eq))
            .or_not()
            .then(expr.clone())
            .map(|(key, val)| Arg { key, val });

        // LPAR [ args ] RPAR
        let call = arg
            .separated_by(just(Token::Comma))
            .collect::<Vec<_>>()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .map(Suffix::Call);

        // DOT ident
        let member = just(Token::Dot).ignore_then(ident).map(Suffix::Member);

        // model = [ ident ] LBRACE stmts RBRACE
        let model = ident
            .or_not()
            .then(stmts_parser(expr.clone()).delimited_by(just(Token::LBrace), just(Token::RBrace)))
            .map(|(name, body)| Expr::Model { name, body });

        let grouped = expr
            .clone()
            .delimited_by(just(Token::LParen), just(Token::RParen));

        let primary = choice((model, number, ident.map(Expr::Ident), grouped));

        // postfix = primary { DOT ident | LPAR [ args ] RPAR }
        let postfix = primary
            .foldl(
                choice((member, call)).repeated(),
                |object, suffix| match suffix {
                    Suffix::Member(field) => Expr::Member {
                        obj: Box::new(object),
                        field,
                    },
                    Suffix::Call(args) => Expr::Call {
                        callee: Box::new(object),
                        args,
                    },
                },
            )
            .boxed();

        // unary = (PLUS | MINUS) unary | postfix [ SUP unary ]
        let unary = recursive::<_, Expr, PExtra<'src>, _, _>(|unary| {
            let unary_op = select! {
                Token::Plus  => UnaryOp::Pos,
                Token::Minus => UnaryOp::Neg,
            };

            let prefix = unary_op.then(unary.clone()).map(|(op, expr)| Expr::Unary {
                op,
                expr: Box::new(expr),
            });

            let power = postfix
                .clone()
                .then(just(Token::Sup).ignore_then(unary).or_not())
                .map(|(lhs, rhs)| match rhs {
                    Some(rhs) => binary(lhs, BinaryOp::Pow, rhs),
                    None => lhs,
                });

            choice((prefix, power))
        });

        let mul_op = select! {
            Token::Star  => BinaryOp::Mul,
            Token::Slash => BinaryOp::Div,
        };

        // prod = unary { (STAR | SLASH) unary }
        let prod = unary
            .clone()
            .foldl(mul_op.then(unary).repeated(), |lhs, (op, rhs)| {
                binary(lhs, op, rhs)
            })
            .boxed();

        // expr = prod { (PLUS | MINUS) prod }
        let add_op = select! {
            Token::Plus  => BinaryOp::Add,
            Token::Minus => BinaryOp::Sub,
        };

        prod.clone()
            .foldl(add_op.then(prod).repeated(), |lhs, (op, rhs)| {
                binary(lhs, op, rhs)
            })
            .boxed()
    });

    // prog = stmts
    stmts_parser(expr)
        .map(|stmts| Program { stmts })
        .then_ignore(end())
}
