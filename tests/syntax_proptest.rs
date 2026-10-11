use proptest::prelude::*;

use calcium::syntax::ast::{Expr, InfixOp, PrefixOp, Program};
use calcium::syntax::{lex, parse};

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

fn arb_num_str() -> impl Strategy<Value = String> {
    prop_oneof![
        "[0-9]{1,9}".prop_map(|s| s),
        ("[0-9]{1,6}", "[0-9]{1,6}").prop_map(|(a, b)| format!("{a}.{b}")),
    ]
}

fn arb_ident_str() -> impl Strategy<Value = String> {
    "[a-zA-Z_][a-zA-Z0-9_]{0,8}"
}

fn arb_expr() -> impl Strategy<Value = Expr> {
    let leaf = prop_oneof![
        arb_num_str().prop_map(Expr::Num),
        arb_ident_str().prop_map(Expr::Ident),
    ];

    leaf.prop_recursive(3, 32, 2, |inner| {
        prop_oneof![
            (arb_prefix_op(), inner.clone()).prop_map(|(op, expr)| Expr::Prefix {
                op,
                expr: Box::new(expr)
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
    #[test]
    fn test_pipeline_never_panics_on_raw_strings(input in "\\PC*") {
        if let Ok(lexed) = lex(&input) {
            let _ = parse(&lexed);
        }
    }

    #[test]
    fn test_lexer_span_invariants(input in ".*") {
        if let Ok(lexed) = lex(&input) {
            let mut cursor = 0;
            for (_tok, span) in &lexed.tokens {
                let (start, end) = (span.start, span.end);

                prop_assert!(start >= cursor);
                prop_assert!(start <= end);
                prop_assert!(end <= input.len());

                prop_assert!(input.is_char_boundary(start));
                prop_assert!(input.is_char_boundary(end));

                cursor = end;
            }
        }
    }

    // parse(format(ast)) == ast
    #[test]
    fn test_ast_round_trip(orig_ast in arb_program()) {
        let src = orig_ast.to_string();

        let lexed = lex(&src).unwrap_or_else(|errs| {
            panic!("lexer failed on original AST:\nsource: {src}\nerrors: {errs:?}")
        });

        let parsed_ast = parse(&lexed).unwrap_or_else(|errs| {
            panic!("parser failed on valid tokens:\nsource: {src}\nerrors: {errs:?}")
        });
        println!("original AST: {orig_ast}");
        println!("parsed AST: {parsed_ast}");

        prop_assert_eq!(orig_ast, parsed_ast);
    }
}
