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

/// Strategies for generating numeric string representations.
pub fn arb_num() -> impl Strategy<Value = &'static str> {
    "[0-9]{1,6}".prop_map(leak)
}

/// Strategies for prefix operators.
pub fn arb_prefix_op() -> impl Strategy<Value = PrefixOp> {
    prop_oneof![Just(PrefixOp::Neg), Just(PrefixOp::Pos),]
}

/// Strategies for infix operators.
pub fn arb_infix_op() -> impl Strategy<Value = InfixOp> {
    prop_oneof![
        Just(InfixOp::Add),
        Just(InfixOp::Sub),
        Just(InfixOp::Mul),
        Just(InfixOp::Div),
    ]
}

/// Recursive strategy for expressions.
///
/// `depth`: Maximum tree recursion depth (e.g. 4)
/// `desired_size`: Approximate maximum nodes in the tree (e.g. 16)
/// `expected_branch_size`: Expected items in collection branches (e.g. 2 for call args)
pub fn arb_expr(depth: u32, desired_size: u32) -> BoxedStrategy<Expr<'static>> {
    let leaf = prop_oneof![
        arb_num().prop_map(Expr::Num),
        arb_ident().prop_map(Expr::Ident),
    ];

    leaf.prop_recursive(depth, desired_size, 3, |inner| {
        prop_oneof![
            // 1. Prefix expressions: -x, !y
            (arb_prefix_op(), inner.clone()).prop_map(|(op, expr)| Expr::prefix(op, expr)),
            // 2. Infix expressions: a + b
            (inner.clone(), arb_infix_op(), inner.clone())
                .prop_map(|(lhs, op, rhs)| Expr::infix(lhs, op, rhs)),
            // 3. Postfix / Call expressions: fun(arg1, arg2)
            (inner, proptest::collection::vec(arb_expr(1, 3), 0..4)).prop_map(|(fun, args)| {
                Expr::Call {
                    fun: Box::new(fun),
                    args,
                }
            }),
        ]
    })
    .boxed() // <-- Add .boxed() here
}

/// Strategy for generating single statements.
pub fn arb_stmt(depth: u32, size: u32) -> impl Strategy<Value = Stmt<'static>> {
    prop_oneof![
        arb_expr(depth, size).prop_map(Stmt::Return),
        arb_expr(depth, size).prop_map(Stmt::Expr),
        (arb_expr(depth, size), arb_expr(depth, size))
            .prop_map(|(lhs, rhs)| Stmt::Assign { lhs, rhs }),
        (
            arb_ident(),
            vec(arb_ident(), 0..4),
            vec(arb_stmt_leaf(), 0..4), // Keep inner bodies small to prevent combinatorial blow-up
        )
            .prop_map(|(name, args, body)| Stmt::FnDef {
                name,
                args,
                body: body.into(),
            }),
    ]
}

/// Shallow leaf statement helper to prevent deep recursion inside nested function defs.
fn arb_stmt_leaf() -> impl Strategy<Value = Stmt<'static>> {
    prop_oneof![
        arb_expr(1, 4).prop_map(Stmt::Return),
        arb_expr(1, 4).prop_map(Stmt::Expr),
        (arb_expr(1, 2), arb_expr(1, 2)).prop_map(|(lhs, rhs)| Stmt::Assign { lhs, rhs }),
    ]
}

/// Strategy for generating a full Program.
pub fn arb_program() -> impl Strategy<Value = Program<'static>> {
    vec(arb_stmt(3, 10), 0..10).prop_map(Program::from)
}
