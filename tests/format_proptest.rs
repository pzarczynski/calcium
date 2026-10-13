use proptest::prelude::*;

use calcium::ast::{Expr, InfixOp, PrefixOp, Program};
use calcium::{Lexable, Parsable, Token};

// AST generators

fn arb_prefix_op() -> impl Strategy<Value = PrefixOp> {
    prop_oneof![Just(PrefixOp::Pos), Just(PrefixOp::Neg)]
}

fn arb_infix_op() -> impl Strategy<Value = InfixOp> {
    prop_oneof![
        Just(InfixOp::Add),
        Just(InfixOp::Sub),
        Just(InfixOp::Mul),
        Just(InfixOp::Div),
        Just(InfixOp::Pow),
        Just(InfixOp::Assign),
    ]
}

fn arb_expr() -> impl Strategy<Value = Expr> {
    let leaf = prop_oneof![
        "[0-9]{1,6}(\\.[0-9]{1,4})?".prop_map(Expr::Num),
        "[a-zA-Z_][a-zA-Z0-9_]{0,6}".prop_map(Expr::Ident),
    ];

    leaf.prop_recursive(4, 48, 2, |inner| {
        prop_oneof![
            (arb_prefix_op(), inner.clone()).prop_map(|(op, expr)| Expr::Prefix {
                op,
                expr: Box::new(expr),
            }),
            (arb_infix_op(), inner.clone(), inner.clone()).prop_map(|(op, l, r)| {
                Expr::Infix {
                    op,
                    l: Box::new(l),
                    r: Box::new(r),
                }
            }),
            (inner.clone(), prop::collection::vec(inner, 0..=3)).prop_map(|(fun, args)| {
                Expr::Call {
                    fun: Box::new(fun),
                    args,
                }
            }),
        ]
    })
}

fn arb_program() -> impl Strategy<Value = Program> {
    prop::collection::vec(arb_expr(), 0..=5).prop_map(|exprs| Program { exprs })
}

// proptests

proptest! {
    // format(parse(format(ast))) == format(ast)
    #[test]
    fn test_pretty_printing_is_idempotent(ast in arb_program()) {
        let printed = ast.to_string();

        let lexed = Token::lex(printed.as_str()).collect::<Vec<_>>();
        let lexed_joined = lexed.clone().iter().map(|t| format!("{t:?}")).collect::<Vec<_>>().join(", ");
        let lexed_valid = lexed
            .iter().map(|t| t.to_owned().unwrap_or_else( |_| panic!("Lex failed on pretty-printed tokens:\nsource: {printed}\nlexed: {lexed_joined}")));

        let parsed = Program::parse(&mut lexed_valid.peekable()).unwrap_or_else(|errs| {
            panic!("Parser failed on pretty-printed tokens:\nSource: {printed}\nErrors: {errs:?}")
        });

        let printed_twice = parsed.to_string();

        prop_assert_eq!(printed, printed_twice);
    }
}
