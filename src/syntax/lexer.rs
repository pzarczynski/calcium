use logos::Logos;

#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\f]+")]
pub enum Token {
    #[regex(r";|\r\n|\n")]
    Sep,

    #[token("=")]
    Eq,

    #[token("!=")]
    Ne,

    #[token(">=")]
    Ge,

    #[token("<=")]
    Le,

    #[token(">")]
    Gt,

    #[token("<")]
    Lt,

    #[token(":")]
    Colon,

    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("*")]
    Star,

    #[token("/")]
    Slash,

    #[token("^")]
    Sup,

    #[token(".")]
    Dot,

    #[token(",")]
    Comma,

    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token("{")]
    LBrace,

    #[token("}")]
    RBrace,

    #[regex(r"[0-9]+(\.[0-9]+)?", |lex| lex.slice().to_owned())]
    Num(String),

    #[regex("[A-Za-z_][a-zA-Z0-9_]*", |lex| lex.slice().to_owned())]
    Ident(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use logos::Logos;

    #[test]
    fn lexes_integer_and_float() {
        let tokens = Token::lexer("12 12.5")
            .collect::<Result<Vec<_>, _>>()
            .expect("lexing should succeed");

        assert_eq!(
            tokens,
            vec![Token::Num("12".to_owned()), Token::Num("12.5".to_owned())]
        );
    }

    #[test]
    fn rejects_unknown_character() {
        let result = Token::lexer("@").collect::<Result<Vec<_>, _>>();

        assert!(result.is_err());
    }
}
