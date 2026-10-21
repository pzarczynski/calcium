use calcium::display::DisplayWithInterner;
use calcium::syntax::arbitrary::*;
use calcium::{Lexed, Parser};
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_pipeline_never_panics_on_raw_strings(input in "\\PC*") {
        Lexed::new("", &input);
    }

    #[test]
    fn test_lexer_span_invariants(input in r"[.\w]*") {
        let mut cursor = 0;
        for (_, span) in Lexed::new("", &input).tokens {
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
    fn test_ast_round_trip(ast in any::<Program>()) {
        let interner = get_interner();
        let printed = format!("{}", ast.with_interner(&interner));

        let lexed = Lexed::with_interner("", printed.as_str(), interner.clone());

        if !lexed.errors.is_empty() {
            panic!("Lexer failed:\nsource: {printed}\nerrors: {:?}", lexed.errors);
        }

        let parsed = Parser::parse(lexed.tokens.into_iter()).unwrap_or_else(|errs| {
            panic!("Parser failed:\nsource: {printed}\nerrors: {errs:?}")
        });

        prop_assert_eq!(ast, parsed);
    }
}
