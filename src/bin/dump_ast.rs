use std::env;
use std::fs;
use std::process::ExitCode;

use ariadne::{Color, Label, Report, ReportKind, Source};
use calcium::syntax::parser::{Expected, PErr};
use calcium::{Lex, Parser, Token};

fn format_expected(expected: &[Expected]) -> String {
    match expected {
        [] => "something else".to_string(),
        [single] => format!("{single:?}"),
        [init @ .., last] => {
            let joined = init
                .iter()
                .map(|e| format!("{e:?}"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{joined} or {last:?}")
        }
    }
}

fn report_errors(filename: &str, src: &str, errors: Vec<PErr>) {
    let eoi_span = src.len()..src.len();

    for err in errors {
        let span = err.span.unwrap_or_else(|| eoi_span.clone().into());

        let got_str = match &err.got {
            Some(tok) => format!("`{tok:?}`"), // lub `{tok}` jeśli masz Display
            None => "end of input".to_string(),
        };

        let expected_str = format_expected(&err.expected);
        let msg = format!("unexpected {got_str}, expected {expected_str}");

        Report::build(ReportKind::Error, span.clone())
            .with_message(&msg)
            .with_label(
                Label::new(span.clone())
                    .with_message(format!("expected {expected_str} here"))
                    .with_color(Color::Red),
            )
            .finish()
            .eprint(Source::from(src))
            .unwrap();
    }
}

fn main() -> ExitCode {
    let Some(path) = env::args().nth(1) else {
        eprintln!("Usage: cargo run -- <file.ca>");
        return ExitCode::FAILURE;
    };

    let src = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Failed to read `{path}`: {err}");
            return ExitCode::FAILURE;
        }
    };

    let tokens = Token::lex(&src);

    match Parser::parse(tokens) {
        Ok(program) => {
            println!("=== Debug AST ===");
            println!("{program:#?}");

            println!("\n=== Pretty Printed ===");
            println!("{program}");
            ExitCode::SUCCESS
        }
        Err(errors) => {
            report_errors(&path, &src, errors);
            ExitCode::FAILURE
        }
    }
}
