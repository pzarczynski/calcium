// main.rs
mod runtime;
mod syntax;

use chumsky::prelude::*;
use logos::Logos;
use runtime::env::Scope;
use runtime::eval::exec_stmt;
use runtime::value::Value;
use std::sync::Arc;
use syntax::parser;

use crate::syntax::ast::Program;
use crate::syntax::lexer::Token;

fn parse(src: &str) -> Result<Program, Vec<Rich<'_, Token>>> {
    let tokens = Token::lexer(src)
        .collect::<Result<Vec<_>, _>>()
        .expect("lexing failed");
    parser()
        .then_ignore(end())
        .parse(tokens.as_slice())
        .into_result()
        .map_err(|errs| errs.into_iter().map(|err| err.into_owned()).collect())
}

pub fn run(src: &str, scope: &Arc<Scope>) -> Result<Vec<Value>, String> {
    let program = parse(src).map_err(|errs| format!("Parse error: {errs:?}"))?;

    let mut results = Vec::new();
    for stmt in &program.stmts {
        match exec_stmt(stmt, scope) {
            Ok(Some(val)) => results.push(val),
            Ok(None) => {}
            Err(e) => return Err(e.to_string()),
        }
    }

    Ok(results)
}

fn main() {
    let global_scope = Arc::new(Scope::default());

    let script = r#"
        x(y) = y - 1
        x(3)

        C = 5
        B = C + 1
        A = B * 2
        A(C = 10)

        f = u + v
        f(u = 7)

        Circle = {
            r = 5
            area = 3.14 * r^2
        }
        Circle.area
    "#;

    match run(script, &global_scope) {
        Ok(outputs) => {
            for (i, val) in outputs.iter().enumerate() {
                println!("[Output {i}]: {val}");
            }
        }
        Err(err) => eprintln!("Execution Error: {err}"),
    }
}
