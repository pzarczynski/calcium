use std::fmt;
use std::ops;

use crate::Token;

pub type Program<'src> = Stmts<'src>;

#[derive(Debug, Clone, PartialEq)]
pub struct Stmts<'src>(pub Vec<Stmt<'src>>);

impl<'src> ops::Deref for Stmts<'src> {
    type Target = Vec<Stmt<'src>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'src> ops::DerefMut for Stmts<'src> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'src> From<Vec<Stmt<'src>>> for Stmts<'src> {
    fn from(v: Vec<Stmt<'src>>) -> Self {
        Stmts(v)
    }
}

impl fmt::Display for Stmts<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for stmt in self.iter() {
            write!(f, "{stmt}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt<'src> {
    Expr(Expr<'src>),
    Assign { lhs: Expr<'src>, rhs: Expr<'src> },
    Return(Expr<'src>),
}

impl fmt::Display for Stmt<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Stmt::Expr(expr) => write!(f, "{expr}; "),
            Stmt::Assign { lhs, rhs } => write!(f, "{lhs} = {rhs}; "),
            Stmt::Return(expr) => write!(f, "return {expr}; "),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr<'src> {
    Int(i64),
    Ident(&'src str),
    Prefix {
        op: PrefixOp,
        expr: Box<Expr<'src>>,
    },
    Infix {
        op: InfixOp,
        l: Box<Expr<'src>>,
        r: Box<Expr<'src>>,
    },
    Call {
        fun: Box<Expr<'src>>,
        arg: Box<Expr<'src>>,
    },
    Fn {
        arg: &'src str,
        body: Box<[Stmt<'src>]>,
    },
}

impl fmt::Display for Expr<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.format(f, 0)
    }
}

impl<'src> Expr<'src> {
    pub fn prec(&self) -> Prec {
        match self {
            Expr::Int(_) | Expr::Ident(_) => Prec::MAX,
            Expr::Fn { .. } => Prec::MIN,
            Expr::Call { .. } => Prec::MAX,
            Expr::Prefix { op, .. } => op.prec(),
            Expr::Infix { op, .. } => op.prec(),
        }
    }

    pub fn infix(lhs: Expr<'src>, op: InfixOp, rhs: Expr<'src>) -> Self {
        Expr::Infix {
            op,
            l: Box::new(lhs),
            r: Box::new(rhs),
        }
    }

    pub fn prefix(op: PrefixOp, expr: Expr<'src>) -> Self {
        Expr::Prefix {
            op,
            expr: Box::new(expr),
        }
    }

    pub fn call(fun: Expr<'src>, arg: Expr<'src>) -> Self {
        Expr::Call {
            fun: Box::new(fun),
            arg: Box::new(arg),
        }
    }

    pub fn call_many(fun: Expr<'src>, args: impl IntoIterator<Item = Expr<'src>>) -> Self {
        args.into_iter().fold(fun, Self::call)
    }

    pub fn function(
        args: impl IntoIterator<Item = &'src str, IntoIter: DoubleEndedIterator>,
        body: impl Into<Box<[Stmt<'src>]>>,
    ) -> Option<Self> {
        let mut iter = args.into_iter().rev();
        let last_arg = iter.next()?;
        let innermost = Expr::Fn {
            arg: last_arg,
            body: body.into(),
        };

        Some(iter.fold(innermost, |acc, arg| Expr::Fn {
            arg,
            body: Box::new([Stmt::Return(acc)]),
        }))
    }

    fn format(&self, f: &mut fmt::Formatter<'_>, min_prec: Prec) -> fmt::Result {
        let needs_parens = self.prec() < min_prec;
        if needs_parens {
            f.write_str("(")?;
        }

        match self {
            Self::Int(value) => f.write_fmt(format_args!("{value}"))?,
            Self::Ident(value) => f.write_str(value)?,

            Self::Prefix { op, expr } => {
                let bp = if matches!(expr.as_ref(), Self::Prefix { .. }) {
                    Prec::MAX
                } else {
                    op.prec()
                };

                write!(f, "{op}")?;
                expr.format(f, bp)?;
            }

            Self::Infix { op, l, r } => {
                let (l_bp, r_bp) = match op.assoc() {
                    Assoc::Left(p) => (p, p + 1),
                    Assoc::Right(p) => (p + 1, p),
                };

                l.format(f, l_bp)?;
                write!(f, " {op} ")?;
                r.format(f, r_bp)?;
            }

            Self::Fn { arg, body } => {
                write!(f, "fn({arg}) {{ ")?;
                for stmt in body.iter() {
                    write!(f, "{stmt}")?;
                }
                f.write_str("}")?;
            }

            Self::Call { fun, arg } => {
                fun.format(f, self.prec())?;
                write!(f, "({arg})")?;
            }
        }

        if needs_parens {
            f.write_str(")")?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixOp {
    Pos,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfixOp {
    Add,
    Sub,
    Mul,
    Div,
}

pub enum PostfixOrInfix {
    Call,
    Infix(InfixOp),
}

pub type Prec = u16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assoc {
    Left(Prec),
    Right(Prec),
}

impl Assoc {
    pub fn prec(&self) -> Prec {
        match self {
            Assoc::Left(p) | Assoc::Right(p) => *p,
        }
    }

    pub fn lhs(&self) -> Prec {
        match self {
            Assoc::Left(p) => *p,
            Assoc::Right(p) => *p + 1,
        }
    }

    pub fn rhs(&self) -> Prec {
        match self {
            Assoc::Left(p) => *p + 1,
            Assoc::Right(p) => *p,
        }
    }
}

impl InfixOp {
    pub fn prec(self) -> Prec {
        match self {
            InfixOp::Add | InfixOp::Sub => 20,
            InfixOp::Mul | InfixOp::Div => 30,
        }
    }

    pub fn assoc(self) -> Assoc {
        match self {
            InfixOp::Add | InfixOp::Sub | InfixOp::Mul | InfixOp::Div => Assoc::Left(self.prec()),
        }
    }
}

impl TryFrom<&Token<'_>> for InfixOp {
    type Error = ();

    fn try_from(value: &Token) -> Result<Self, Self::Error> {
        match value {
            Token::Plus => Ok(InfixOp::Add),
            Token::Minus => Ok(InfixOp::Sub),
            Token::Star => Ok(InfixOp::Mul),
            Token::Slash => Ok(InfixOp::Div),
            _ => Err(()),
        }
    }
}

impl fmt::Display for InfixOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sym = match self {
            InfixOp::Add => "+",
            InfixOp::Sub => "-",
            InfixOp::Mul => "*",
            InfixOp::Div => "/",
        };
        write!(f, "{sym}")
    }
}

impl PostfixOrInfix {
    pub fn prec(&self) -> Prec {
        match self {
            PostfixOrInfix::Call => Prec::MAX,
            PostfixOrInfix::Infix(op) => op.prec(),
        }
    }

    pub fn assoc(&self) -> Assoc {
        match self {
            PostfixOrInfix::Call => Assoc::Left(Prec::MAX),
            PostfixOrInfix::Infix(op) => op.assoc(),
        }
    }
}

impl PrefixOp {
    pub fn prec(self) -> u16 {
        match self {
            PrefixOp::Pos | PrefixOp::Neg => 40,
        }
    }

    pub fn assoc(self) -> Assoc {
        match self {
            PrefixOp::Pos | PrefixOp::Neg => Assoc::Left(self.prec()),
        }
    }
}

impl TryFrom<&Token<'_>> for PrefixOp {
    type Error = ();

    fn try_from(value: &Token) -> Result<Self, Self::Error> {
        match value {
            Token::Plus => Ok(PrefixOp::Pos),
            Token::Minus => Ok(PrefixOp::Neg),
            _ => Err(()),
        }
    }
}

impl fmt::Display for PrefixOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sym = match self {
            PrefixOp::Pos => "+",
            PrefixOp::Neg => "-",
        };
        write!(f, "{sym}")
    }
}
