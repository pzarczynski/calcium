mod runtime;
mod syntax;

use syntax::{lex, parse};

fn main() -> Result<(), Vec<syntax::Error>> {
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

    let tokens = lex(script)?;
    let program = parse(&tokens);

    Ok(())
}
