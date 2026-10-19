mod common;

use calcium::{Lex, Parser, Token};
use common::ast_generators::*;
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_pipeline_never_panics_on_raw_strings(input in "\\PC*") {
        Token::lex(&input).for_each(drop);
    }

    #[test]
    fn test_lexer_span_invariants(input in r"[.\w]*") {
        let mut cursor = 0;
        for (_, span) in Token::lex(&input) {
            let (start, end) = (span.start, span.end);

            prop_assert!(start >= cursor);
            prop_assert!(start <= end);
            prop_assert!(end <= input.len());

            prop_assert!(input.is_char_boundary(start));
            prop_assert!(input.is_char_boundary(end));

            cursor = end;
    }
    }

    // parse(format(ast)) == ast
    #[test]
    fn test_ast_round_trip(ast in arb_program()) {
        let printed = ast.to_string();

        let lexed = Token::lex(printed.as_str()).collect::<Vec<_>>();
        let lexed_joined = lexed.clone().iter().map(|t| format!("{t:?}")).collect::<Vec<_>>().join(", ");
        let lexed_valid = lexed
            .iter().cloned().map(|(t, s)| match t {
                Token::Invalid => panic!("Lexer failed:\nsource: {printed}\nlexed: {lexed_joined}"),
                _ => (t, s)
            });

        let parsed = Parser::parse(lexed_valid).unwrap_or_else(|errs| {
            panic!("Parser failed:\nsource: {printed}\nerrors: {errs:?}")
        });

        prop_assert_eq!(ast, parsed);
    }
}
