use crate::syntax::ast::*;
use std::fmt;

impl fmt::Display for Stmts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt_stmts(self.iter(), f)
    }
}

fn fmt_stmts<'a, I>(stmts: I, f: &mut fmt::Formatter<'_>) -> fmt::Result
where
    I: IntoIterator<Item = &'a Stmt>,
{
    let mut iter = stmts.into_iter();
    if let Some(first) = iter.next() {
        write!(f, "{first}")?;
        for stmt in iter {
            write!(f, " {stmt}")?;
        }
    }
    Ok(())
}

impl fmt::Display for Stmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Stmt::Expr(expr) => write!(f, "{expr};"),
            Stmt::Return(expr) => write!(f, "return {expr};"),
            Stmt::Assign {
                lhs: Expr::Ident(name),
                rhs: rhs @ Expr::Fn { .. },
            } => fmt_fn_decl(name, rhs, f),
            Stmt::Assign { lhs, rhs } => write!(f, "{lhs} = {rhs};"),
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.fmt_prec(f, 0)
    }
}

/// Un-curries nested `Expr::Fn` chains back into `fn name(arg1, arg2) { body };`
fn fmt_fn_decl(name: &str, mut curr: &Expr, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "fn {name}(")?;

    let mut first = true;
    let body = loop {
        match curr {
            Expr::Fn { arg, body } => {
                if !first {
                    f.write_str(", ")?;
                }
                first = false;
                f.write_str(arg)?;

                // Peel off auto-generated currying layers: `[Stmt::Return(Expr::Fn { .. })]`
                if let [Stmt::Return(next @ Expr::Fn { .. })] = body.as_ref() {
                    curr = next;
                } else {
                    break body;
                }
            }
            _ => unreachable!("fmt_fn_decl called on non-Fn expression"),
        }
    };

    f.write_str(") { ")?;
    fmt_stmts(body.iter(), f)?;
    f.write_str(" }")
}

impl Expr {
    fn fmt_prec(&self, f: &mut fmt::Formatter<'_>, min_prec: Prec) -> fmt::Result {
        // Nested prefix expressions like `-(-x)` or `-(+x)` must be parenthesized
        // to avoid lexing ambiguity (e.g. `--` token) and preserve clarity.
        let needs_parens = self.prec() < min_prec;
        if needs_parens {
            f.write_str("(")?;
        }

        match self {
            Self::Int(val) => write!(f, "{val}")?,
            Self::Ident(name) => f.write_str(name)?,

            Self::Prefix { op, expr } => {
                write!(f, "{op}")?;
                if matches!(expr.as_ref(), Self::Prefix { .. }) {
                    write!(f, "({expr})")?;
                } else {
                    expr.fmt_prec(f, op.prec())?;
                }
            }

            Self::Infix { op, l, r } => {
                let (l_bp, r_bp) = match op.assoc() {
                    Assoc::Left(p) => (p, p + 1),
                    Assoc::Right(p) => (p + 1, p),
                };

                l.fmt_prec(f, l_bp)?;
                write!(f, " {op} ")?;
                r.fmt_prec(f, r_bp)?;
            }

            // Fallback in case an `Expr::Fn` is ever formatted directly in isolation
            Self::Fn { arg, body } => {
                write!(f, "fn({arg}) {{ ")?;
                fmt_stmts(body.iter(), f)?;
                f.write_str(" }")?;
            }

            Self::Call { fun, arg } => {
                fun.fmt_prec(f, self.prec())?;
                write!(f, "({arg})")?;
            }
        }

        if needs_parens {
            f.write_str(")")?;
        }

        Ok(())
    }
}

impl fmt::Display for InfixOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Add => "+",
            Self::Sub => "-",
            Self::Mul => "*",
            Self::Div => "/",
        })
    }
}

impl fmt::Display for PrefixOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Pos => "+",
            Self::Neg => "-",
        })
    }
}
