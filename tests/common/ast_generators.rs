use proptest::collection::vec;
use proptest::prelude::*;

use calcium::ast::*;

const KEYWORDS: &[&str] = &["fn", "return"];

fn leak(s: String) -> &'static str {
    &*s.leak()
}

pub fn arb_ident() -> impl Strategy<Value = &'static str> {
    "[a-zA-Z_][a-zA-Z0-9_]{0,6}"
        .prop_filter("must not be a keyword", |s| !KEYWORDS.contains(&s.as_str()))
        .prop_map(leak)
}

pub fn arb_int() -> impl Strategy<Value = i64> {
    "[0-9]{1,6}"
        .prop_map(leak)
        .prop_map(|s| s.parse::<i64>().unwrap())
}

pub fn arb_prefix_op() -> impl Strategy<Value = PrefixOp> {
    prop_oneof![Just(PrefixOp::Neg), Just(PrefixOp::Pos),]
}

pub fn arb_infix_op() -> impl Strategy<Value = InfixOp> {
    prop_oneof![
        Just(InfixOp::Add),
        Just(InfixOp::Sub),
        Just(InfixOp::Mul),
        Just(InfixOp::Div),
    ]
}

pub fn arb_expr(depth: u32, desired_size: u32) -> BoxedStrategy<Expr<'static>> {
    let leaf = prop_oneof![
        arb_int().prop_map(Expr::Int),
        arb_ident().prop_map(Expr::Ident),
    ];

    leaf.prop_recursive(depth, desired_size, 3, |inner| {
        prop_oneof![
            // prefix expr
            (arb_prefix_op(), inner.clone()).prop_map(|(op, expr)| Expr::prefix(op, expr)),
            // infix expr
            (inner.clone(), arb_infix_op(), inner.clone())
                .prop_map(|(lhs, op, rhs)| Expr::infix(lhs, op, rhs)),
            // curried call exprs
            (inner, vec(arb_expr(1, 3), 1..4)).prop_map(|(fun, args)| Expr::call_many(fun, args)),
        ]
    })
    .boxed()
}

/// Strategy for generating single statements.
pub fn arb_stmt(depth: u32, desired_size: u32) -> impl Strategy<Value = Stmt<'static>> {
    prop_oneof![
        // return stmt
        arb_expr(depth, desired_size).prop_map(Stmt::Return),
        // expr stmt
        arb_expr(depth, desired_size).prop_map(Stmt::Expr),
        // assign stmt
        (arb_expr(depth, desired_size), arb_expr(depth, desired_size))
            .prop_map(|(lhs, rhs)| Stmt::Assign { lhs, rhs }),
        // function stmt
        (
            arb_ident(),
            vec(arb_ident(), 1..4),
            vec(arb_stmt_leaf(), 0..4),
        )
            .prop_map(|(name, args, body)| Stmt::Assign {
                lhs: Expr::Ident(name),
                rhs: Expr::function(args, body).unwrap(),
            }),
    ]
}

fn arb_stmt_leaf() -> impl Strategy<Value = Stmt<'static>> {
    prop_oneof![
        arb_expr(1, 4).prop_map(Stmt::Return),
        arb_expr(1, 4).prop_map(Stmt::Expr),
        (arb_expr(1, 2), arb_expr(1, 2)).prop_map(|(lhs, rhs)| Stmt::Assign { lhs, rhs }),
    ]
}

pub fn arb_program() -> impl Strategy<Value = Program<'static>> {
    vec(arb_stmt(3, 10), 0..10).prop_map(Program::from)
}
