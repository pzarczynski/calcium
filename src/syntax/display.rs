use crate::{Ctx, Symbol, Token, ast::*, syntax::parser::Expected};
use std::fmt;

pub struct WithCtx<'a, T: ?Sized> {
    pub node: &'a T,
    pub ctx: &'a Ctx,
}

pub trait AttachCtx {
    fn attach<'a>(&'a self, ctx: &'a Ctx) -> WithCtx<'a, Self> {
        WithCtx { node: self, ctx }
    }
}

impl<T: ?Sized> AttachCtx for T {}

impl fmt::Display for WithCtx<'_, Symbol> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.ctx.resolve(self.node))
    }
}

impl fmt::Display for WithCtx<'_, Token> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ctx = self.ctx;
        match self.node {
            Token::Sep => f.write_str(";"),
            Token::Eq => f.write_str("="),
            Token::Fn => f.write_str("fn"),
            Token::Return => f.write_str("return"),
            Token::LParen => f.write_str("("),
            Token::RParen => f.write_str(")"),
            Token::LBrace => f.write_str("{"),
            Token::RBrace => f.write_str("}"),
            Token::Plus => f.write_str("+"),
            Token::Minus => f.write_str("-"),
            Token::Star => f.write_str("*"),
            Token::Slash => f.write_str("/"),
            Token::Comma => f.write_str(","),
            Token::Int(n) => write!(f, "{n}"),
            Token::Ident(sym) => write!(f, "{}", sym.attach(ctx)),
        }
    }
}

impl fmt::Display for WithCtx<'_, [Stmt]> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut iter = self.node.iter();
        if let Some(first) = iter.next() {
            write!(f, "{}", first.attach(self.ctx))?;
            for stmt in iter {
                write!(f, " {}", stmt.attach(self.ctx))?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for WithCtx<'_, Program> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fmt_prog = self.node.stmts.as_slice().attach(self.ctx);
        write!(f, "{}", fmt_prog)
    }
}

impl fmt::Display for WithCtx<'_, Stmt> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ctx = self.ctx;
        match self.node {
            Stmt::Expr(expr) => write!(f, "{};", expr.attach(ctx)),
            Stmt::Return(expr) => write!(f, "return {};", expr.attach(ctx)),
            Stmt::Assign {
                lhs: Expr::Ident(name),
                rhs: Expr::Fn { arg, body },
            } => {
                write!(f, "fn {}", name.attach(ctx))?;
                fmt_fn_chain(arg, body, f, ctx)?;
                f.write_str(";")
            }
            Stmt::Assign { lhs, rhs } => {
                write!(f, "{} = {};", lhs.attach(ctx), rhs.attach(ctx))
            }
        }
    }
}

/// Shared helper to un-curry both named `fn f(a, b) { ... }` and anonymous `fn(a, b) { ... }`.
fn fmt_fn_chain(
    first_arg: &Symbol,
    first_body: &[Stmt],
    f: &mut fmt::Formatter<'_>,
    ctx: &Ctx,
) -> fmt::Result {
    write!(f, "({}", first_arg.attach(ctx))?;
    let mut body = first_body;
    while let [
        Stmt::Return(Expr::Fn {
            arg,
            body: next_body,
        }),
    ] = body
    {
        write!(f, ", {}", arg.attach(ctx))?;
        body = next_body;
    }
    write!(f, ") {{ {} }}", body.attach(ctx))
}

impl fmt::Display for WithCtx<'_, Expr> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.node.fmt_prec(f, self.ctx, 0)
    }
}

impl Expr {
    fn fmt_prec(&self, f: &mut fmt::Formatter<'_>, ctx: &Ctx, min_prec: Prec) -> fmt::Result {
        let needs_parens = self.prec() < min_prec;
        if needs_parens {
            f.write_str("(")?;
        }

        match self {
            Self::Int(val) => write!(f, "{val}")?,
            Self::Ident(sym) => write!(f, "{}", sym.attach(ctx))?,

            Self::Prefix { op, expr } => {
                write!(f, "{op}")?;
                // Passing `op.prec() + 1` naturally forces parens on nested prefixes
                // because `expr.prec() < op.prec() + 1` evaluates to true when `expr` is also a Prefix
                expr.fmt_prec(f, ctx, op.prec() + 1)?;
            }

            Self::Infix { op, lhs, rhs } => {
                let (l_bp, r_bp) = match op.assoc() {
                    Assoc::Left(p) => (p, p + 1),
                    Assoc::Right(p) => (p + 1, p),
                };

                lhs.fmt_prec(f, ctx, l_bp)?;
                write!(f, " {op} ")?;
                rhs.fmt_prec(f, ctx, r_bp)?;
            }

            Self::Fn { arg, body } => {
                f.write_str("fn")?;
                fmt_fn_chain(arg, body, f, ctx)?;
            }

            Self::Call { fun, arg } => {
                fun.fmt_prec(f, ctx, self.prec())?;
                f.write_str("(")?;
                arg.fmt_prec(f, ctx, 0)?;
                f.write_str(")")?;
            }
        }

        if needs_parens {
            f.write_str(")")?;
        }

        Ok(())
    }
}

impl fmt::Display for PrefixOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pos => f.write_str("+"),
            Self::Neg => f.write_str("-"),
        }
    }
}

impl fmt::Display for InfixOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Add => f.write_str("+"),
            Self::Sub => f.write_str("-"),
            Self::Mul => f.write_str("*"),
            Self::Div => f.write_str("/"),
        }
    }
}

impl fmt::Display for WithCtx<'_, Expected> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.node {
            Expected::Expr => f.write_str("expression"),
            Expected::Ident => f.write_str("identifier"),
            Expected::Token(tok) => write!(f, "`{}`", tok.attach(self.ctx)),
            Expected::Stmt => f.write_str("statement"),
        }
    }
}
