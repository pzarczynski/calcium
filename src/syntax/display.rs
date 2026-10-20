use crate::{Interner, Symbol, Token, ast::*, syntax::parser::Expected};
use std::fmt;

pub struct WithInterner<'a, T: ?Sized> {
    pub node: &'a T,
    pub interner: &'a Interner,
}

pub trait DisplayWithInterner {
    fn with_interner<'a>(&'a self, interner: &'a Interner) -> WithInterner<'a, Self> {
        WithInterner {
            node: self,
            interner,
        }
    }
}

impl<T: ?Sized> DisplayWithInterner for T {}

impl fmt::Display for WithInterner<'_, Symbol> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.interner.resolve(self.node))
    }
}

impl fmt::Display for WithInterner<'_, Token> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let i = self.interner;
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
            Token::Ident(sym) => write!(f, "{}", sym.with_interner(i)),
        }
    }
}

impl fmt::Display for WithInterner<'_, [Stmt]> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut iter = self.node.iter();
        if let Some(first) = iter.next() {
            write!(f, "{}", first.with_interner(self.interner))?;
            for stmt in iter {
                write!(f, " {}", stmt.with_interner(self.interner))?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for WithInterner<'_, Stmts> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.node.0.as_slice().with_interner(self.interner))
    }
}

impl fmt::Display for WithInterner<'_, Stmt> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let i = self.interner;
        match self.node {
            Stmt::Expr(expr) => write!(f, "{};", expr.with_interner(i)),
            Stmt::Return(expr) => write!(f, "return {};", expr.with_interner(i)),
            Stmt::Assign {
                lhs: Expr::Ident(name),
                rhs: Expr::Fn { arg, body },
            } => {
                write!(f, "fn {}", name.with_interner(i))?;
                fmt_fn_chain(arg, body, f, i)?;
                f.write_str(";")
            }
            Stmt::Assign { lhs, rhs } => {
                write!(f, "{} = {};", lhs.with_interner(i), rhs.with_interner(i))
            }
        }
    }
}

/// Shared helper to un-curry both named `fn f(a, b) { ... }` and anonymous `fn(a, b) { ... }`.
fn fmt_fn_chain(
    first_arg: &Symbol,
    first_body: &[Stmt],
    f: &mut fmt::Formatter<'_>,
    i: &Interner,
) -> fmt::Result {
    write!(f, "({}", first_arg.with_interner(i))?;
    let mut body = first_body;
    while let [
        Stmt::Return(Expr::Fn {
            arg,
            body: next_body,
        }),
    ] = body
    {
        write!(f, ", {}", arg.with_interner(i))?;
        body = next_body;
    }
    write!(f, ") {{ {} }}", body.with_interner(i))
}

impl fmt::Display for WithInterner<'_, Expr> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.node.fmt_prec(f, self.interner, 0)
    }
}

impl Expr {
    fn fmt_prec(&self, f: &mut fmt::Formatter<'_>, i: &Interner, min_prec: Prec) -> fmt::Result {
        let needs_parens = self.prec() < min_prec;
        if needs_parens {
            f.write_str("(")?;
        }

        match self {
            Self::Int(val) => write!(f, "{val}")?,
            Self::Ident(sym) => write!(f, "{}", sym.with_interner(i))?,

            Self::Prefix { op, expr } => {
                write!(f, "{op}")?;
                // Passing `op.prec() + 1` naturally forces parens on nested prefixes
                // because `expr.prec() < op.prec() + 1` evaluates to true when `expr` is also a Prefix
                expr.fmt_prec(f, i, op.prec() + 1)?;
            }

            Self::Infix { op, l, r } => {
                let (l_bp, r_bp) = match op.assoc() {
                    Assoc::Left(p) => (p, p + 1),
                    Assoc::Right(p) => (p + 1, p),
                };

                l.fmt_prec(f, i, l_bp)?;
                write!(f, " {op} ")?;
                r.fmt_prec(f, i, r_bp)?;
            }

            Self::Fn { arg, body } => {
                f.write_str("fn")?;
                fmt_fn_chain(arg, body, f, i)?;
            }

            Self::Call { fun, arg } => {
                fun.fmt_prec(f, i, self.prec())?;
                f.write_str("(")?;
                arg.fmt_prec(f, i, 0)?;
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

impl fmt::Display for WithInterner<'_, Expected> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.node {
            Expected::Expr => f.write_str("expression"),
            Expected::Ident => f.write_str("identifier"),
            Expected::Token(tok) => write!(f, "`{}`", tok.with_interner(self.interner)),
            Expected::Stmt => f.write_str("statement"),
        }
    }
}
