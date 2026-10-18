mod common;

use calcium::{Lexable, Parser, Token};
use common::ast_generators::*;
use proptest::prelude::*;

proptest! {
    // format(parse(format(ast))) == format(ast)
    #[test]
    fn test_pretty_printing_is_idempotent(ast in arb_program()) {
        let printed = ast.to_string();

        let lexed = Token::lex(printed.as_str()).collect::<Vec<_>>();
        let lexed_joined = lexed.clone().iter().map(|(t, _)| format!("{t}")).collect::<Vec<_>>().join(", ");
        let lexed_valid = lexed
            .iter().cloned().map(|(t, s)| match t {
                Token::Invalid => panic!("Lexer failed:\nsource: {printed}\nlexed: {lexed_joined}"),
                _ => (t, s)
            });

        let parsed = Parser::parse(lexed_valid).unwrap_or_else(|errs| {
            panic!("Parser failed:\nsource: {printed}\nerrors: {errs:?}")
        });

        let printed_twice = parsed.to_string();

        prop_assert_eq!(printed, printed_twice);
    }
}
