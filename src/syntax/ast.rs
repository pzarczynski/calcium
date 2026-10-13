use std::fmt;
use std::ops;

use crate::Token;

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

impl fmt::Display for Stmts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, stmt) in self.iter().enumerate() {
            if i > 0 {
                write!(f, "; ")?;
            }
            write!(f, "{stmt}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expr(Expr),
    Assign {
        lhs: Expr,
        rhs: Expr,
    },
    FnDef {
        name: String,
        args: Vec<String>,
        body: Box<Stmts>,
    },
    Return(Expr),
}

impl fmt::Display for Stmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Stmt::Expr(expr) => write!(f, "{expr}"),
            Stmt::Assign { lhs, rhs } => write!(f, "{lhs} = {rhs}"),
            Stmt::FnDef { name, args, body } => {
                write!(f, "fn {name}(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{arg}")?;
                }
                write!(f, ") {{ {} }}", body)
            }
            Stmt::Return(expr) => write!(f, "return {expr}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(String),
    Ident(String),
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
        args: Vec<Expr>,
    },
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.format(f, 0)
    }
}

impl Expr {
    pub fn bp(&self) -> u16 {
        match self {
            Expr::Num(_) | Expr::Ident(_) => MAX_BP,
            Expr::Prefix { op, .. } => op.prec(),
            Expr::Infix { op, .. } => op.prec(),
            Expr::Call { .. } => MAX_BP,
        }
    }

    pub fn infix(lhs: Expr, op: InfixOp, rhs: Expr) -> Self {
        Expr::Infix {
            op,
            l: Box::new(lhs),
            r: Box::new(rhs),
        }
    }

    pub fn prefix(op: PrefixOp, expr: Expr) -> Self {
        Expr::Prefix {
            op,
            expr: Box::new(expr),
        }
    }

    fn format(&self, f: &mut fmt::Formatter<'_>, min_bp: u16) -> fmt::Result {
        let needs_parens = self.bp() < min_bp;
        if needs_parens {
            f.write_str("(")?;
        }

        match self {
            Self::Num(value) | Self::Ident(value) => f.write_str(value)?,

            Self::Prefix { op, expr } => {
                let bp = if matches!(expr.as_ref(), Self::Prefix { .. }) {
                    MAX_BP
                } else {
                    op.prec()
                };

                write!(f, "{op}")?;
                expr.format(f, bp)?;
            }

            Self::Infix { op, l, r } => {
                let (l_bp, r_bp) = op.bp();

                l.format(f, l_bp)?;
                write!(f, " {op} ")?;
                r.format(f, r_bp)?;
            }

            Self::Call { fun, args } => {
                fun.format(f, MAX_BP)?;
                f.write_str("(")?;

                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    arg.format(f, 0)?;
                }

                f.write_str(")")?;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assoc {
    Left,
    Right,
}

impl InfixOp {
    pub fn prec(self) -> u16 {
        match self {
            InfixOp::Add | InfixOp::Sub => 20,
            InfixOp::Mul | InfixOp::Div => 30,
        }
    }

    pub fn assoc(self) -> Assoc {
        match self {
            InfixOp::Add | InfixOp::Sub | InfixOp::Mul | InfixOp::Div => Assoc::Left,
        }
    }

    pub fn bp(self) -> (u16, u16) {
        let p = self.prec();
        match self.assoc() {
            Assoc::Left => (p, p + 1),
            Assoc::Right => (p + 1, p),
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

impl PrefixOp {
    pub fn prec(self) -> u16 {
        match self {
            PrefixOp::Pos | PrefixOp::Neg => 40,
        }
    }

    pub fn bp(self) -> (u16, u16) {
        let p = self.prec();
        (p, p + 1)
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

pub const MAX_BP: u16 = u16::MAX;
