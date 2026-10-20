use std::env;
use std::fs;
use std::process::ExitCode;

use ariadne::{Color, Label, Report, ReportKind, Source};
use calcium::display::DisplayWithInterner;
use calcium::syntax::parser::{Expected, PErr};
use calcium::{Interner, Lexed, Parser, Span};

fn format_expected(expected: &[Expected], interner: &Interner) -> String {
    match expected {
        [] => "something else".to_string(),
        [single] => format!("{}", single.with_interner(interner)),
        [init @ .., last] => {
            let joined = init
                .iter()
                .map(|e| format!("{}", e.with_interner(interner)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{joined} or {}", last.with_interner(interner))
        }
    }
}

fn report_errors(filename: &str, src: &str, interner: &Interner, errors: Vec<PErr>) {
    for err in errors {
        let span = err
            .span
            .unwrap_or_else(|| Span::from_eoi(filename, src.len()));

        let got_str = match &err.got {
            Some(tok) => format!("`{}`", tok.with_interner(interner)),
            None => "EOI".to_string(),
        };

        let expected_str = format_expected(&err.expected, interner);
        let msg = format!("unexpected {got_str}, expected {expected_str}");

        Report::build(ReportKind::Error, span)
            .with_message(&msg)
            .with_label(
                Label::new(span)
                    .with_message(format!("expected {expected_str} here"))
                    .with_color(Color::Red),
            )
            .finish()
            .eprint((filename, Source::from(src)))
            .unwrap();
    }
}

fn main() -> ExitCode {
    let Some(path) = env::args().nth(1) else {
        eprintln!("Usage: cargo run --bin dump_ast <file>");
        return ExitCode::FAILURE;
    };

    let src = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Failed to read `{path}`: {err}");
            return ExitCode::FAILURE;
        }
    };

    let lexed = Lexed::new(path.as_str(), &src);
    let interner = &lexed.interner;

    match Parser::parse(lexed.tokens.into_iter()) {
        Ok(program) => {
            println!("=== Debug AST ===");
            println!("{program:#?}");

            println!("\n=== Pretty Printed ===");
            println!("{}", program.with_interner(interner));
            ExitCode::SUCCESS
        }
        Err(errors) => {
            report_errors(path.as_str(), &src, interner, errors);
            ExitCode::FAILURE
        }
    }
}
