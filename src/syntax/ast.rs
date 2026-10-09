#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Query(Expr), // a standalone expression to evaluate
    Constraint {
        lhs: Expr,
        op: ConstraintOp,
        rhs: Expr,
    }, // a relation between to expressions
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintOp {
    Eq,
    Ne,
    Ge,
    Le,
    Gt,
    Lt,
    Colon,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(String),
    Ident(String),
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },
    Binary {
        op: BinaryOp,
        lhs: Box<Expr>,
        rhs: Box<Expr>,
    },
    Member {
        obj: Box<Expr>,
        field: String,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Arg>,
    },
    Model {
        name: Option<String>,
        body: Vec<Stmt>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Pos,
    Neg,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    pub key: Option<String>,
    pub val: Expr,
}

impl Arg {
    pub fn positional(val: Expr) -> Self {
        Self { key: None, val }
    }

    pub fn keyword(key: impl Into<String>, val: Expr) -> Self {
        Self {
            key: Some(key.into()),
            val,
        }
    }
}
