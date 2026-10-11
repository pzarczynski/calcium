use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub exprs: Vec<Expr>,
}

impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, expr) in self.exprs.iter().enumerate() {
            if i > 0 {
                write!(f, "; ")?;
            }
            write!(f, "{expr}")?;
        }
        Ok(())
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
    pub fn binding_power(&self) -> u16 {
        match self {
            Expr::Num(_) | Expr::Ident(_) => ATOM_BP,
            Expr::Call { .. } => CALL_BP,
            Expr::Prefix { op, .. } => op.prec(),
            Expr::Infix { op, .. } => op.prec(),
        }
    }

    fn format(&self, f: &mut fmt::Formatter<'_>, parent_bp: u16) -> fmt::Result {
        let needs_parens = self.binding_power() < parent_bp;
        if needs_parens {
            write!(f, "(")?;
        }

        match self {
            Expr::Num(n) => write!(f, "{n}")?,
            Expr::Ident(id) => write!(f, "{id}")?,

            Expr::Prefix { op, expr } => {
                let sym = match op {
                    PrefixOp::Pos => "+",
                    PrefixOp::Neg => "-",
                };
                write!(f, "{sym}")?;

                if matches!(&**expr, Expr::Prefix { op: inner, .. } if inner == op) {
                    write!(f, " ")?;
                }

                expr.format(f, op.prec())?;
            }

            Expr::Infix { op, l, r } => {
                let sym = match op {
                    InfixOp::Assign => "=",
                    InfixOp::Add => "+",
                    InfixOp::Sub => "-",
                    InfixOp::Mul => "*",
                    InfixOp::Div => "/",
                    InfixOp::Pow => "^",
                };

                let (l_bp, r_bp) = op.bp();
                l.format(f, l_bp)?;
                write!(f, " {sym} ")?;
                r.format(f, r_bp)?;
            }

            Expr::Call { fun, args } => {
                fun.format(f, CALL_BP)?;
                write!(f, "(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    arg.format(f, 0)?;
                }
                write!(f, ")")?;
            }
        }

        if needs_parens {
            write!(f, ")")?;
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
    Pow,
    Assign,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Assoc {
    Left,
    Right,
}

impl InfixOp {
    pub fn prec(self) -> u16 {
        match self {
            InfixOp::Assign => 10,
            InfixOp::Add | InfixOp::Sub => 20,
            InfixOp::Mul | InfixOp::Div => 30,
            InfixOp::Pow => 50,
        }
    }

    pub fn assoc(self) -> Assoc {
        match self {
            InfixOp::Assign | InfixOp::Pow => Assoc::Right,
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

pub const CALL_BP: u16 = 60;
pub const ATOM_BP: u16 = u16::MAX;
