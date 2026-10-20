use crate::Token;
use crate::syntax::Symbol;
use std::ops;

pub type Program = Stmts;

#[derive(Debug, Clone, PartialEq)]
pub struct Stmts(pub Vec<Stmt>);

impl ops::Deref for Stmts {
    type Target = Vec<Stmt>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for Stmts {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<Stmt>> for Stmts {
    fn from(v: Vec<Stmt>) -> Self {
        Stmts(v)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    Assign { lhs: Expr, rhs: Expr },
    Return(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Ident(Symbol),
    Prefix {
        op: PrefixOp,
        expr: Box<Expr>,
    },
    Infix {
        op: InfixOp,
        l: Box<Expr>,
        r: Box<Expr>,
    },
    Call {
        fun: Box<Expr>,
        arg: Box<Expr>,
    },
    Fn {
        arg: Symbol,
        body: Box<[Stmt]>,
    },
}

impl Expr {
    pub fn prec(&self) -> Prec {
        match self {
            Expr::Int(_) | Expr::Ident(_) => Prec::MAX,
            Expr::Fn { .. } => Prec::MIN,
            Expr::Call { .. } => Prec::MAX,
            Expr::Prefix { op, .. } => op.prec(),
            Expr::Infix { op, .. } => op.prec(),
        }
    }

    pub fn prefix(op: PrefixOp, expr: Expr) -> Self {
        Expr::Prefix {
            op,
            expr: Box::new(expr),
        }
    }

    pub fn infix(lhs: Expr, op: InfixOp, rhs: Expr) -> Self {
        Expr::Infix {
            op,
            l: Box::new(lhs),
            r: Box::new(rhs),
        }
    }

    pub fn call(fun: Expr, arg: Expr) -> Self {
        Expr::Call {
            fun: Box::new(fun),
            arg: Box::new(arg),
        }
    }

    pub fn call_many(fun: Expr, args: impl IntoIterator<Item = Expr>) -> Self {
        args.into_iter().fold(fun, Self::call)
    }

    pub fn function(
        args: impl IntoIterator<Item = Symbol, IntoIter: DoubleEndedIterator>,
        body: impl Into<Box<[Stmt]>>,
    ) -> Option<Self> {
        let mut iter = args.into_iter().rev();
        let innermost = Expr::Fn {
            arg: iter.next()?,
            body: body.into(),
        };

        Some(iter.fold(innermost, |acc, arg| Expr::Fn {
            arg,
            body: Box::new([Stmt::Return(acc)]),
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "arbitrary", derive(proptest_derive::Arbitrary))]
pub enum PrefixOp {
    Pos,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "arbitrary", derive(proptest_derive::Arbitrary))]
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

impl TryFrom<Token> for PrefixOp {
    type Error = ();

    fn try_from(value: Token) -> Result<Self, Self::Error> {
        match value {
            Token::Plus => Ok(PrefixOp::Pos),
            Token::Minus => Ok(PrefixOp::Neg),
            _ => Err(()),
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

impl TryFrom<Token> for InfixOp {
    type Error = ();

    fn try_from(value: Token) -> Result<Self, Self::Error> {
        match value {
            Token::Plus => Ok(InfixOp::Add),
            Token::Minus => Ok(InfixOp::Sub),
            Token::Star => Ok(InfixOp::Mul),
            Token::Slash => Ok(InfixOp::Div),
            _ => Err(()),
        }
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
