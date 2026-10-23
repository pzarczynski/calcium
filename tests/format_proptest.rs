use proptest::prelude::*;
use veles::display::DisplayWithInterner;
use veles::syntax::arbitrary::*;
use veles::{Lexed, Parser};

proptest! {
    // format(parse(format(ast))) == format(ast)
    #[test]
    fn test_pretty_printing_is_idempotent(ast in any::<Program>()) {
        let printed = format!("{}", ast.with_interner(&interner));

        let lexed = Lexed::with_interner("", printed.as_str(), interner.clone());

        if !lexed.errors.is_empty() {
            panic!("Lexer failed:\nsource: {printed}\nerrors: {:?}", lexed.errors);
        }

        let parsed = Parser::parse(lexed.tokens.into_iter()).unwrap_or_else(|errs| {
            panic!("Parser failed:\nsource: {printed}\nerrors: {errs:?}")
        });

        let printed_twice = format!("{}", parsed.with_interner(&interner));
        prop_assert_eq!(printed, printed_twice);
    }
}
