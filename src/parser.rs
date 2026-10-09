use crate::ast;
use crate::lexer;
use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
#[logos(skip r"[ \t\n\f]+")]
enum Token {
    #[regex("[0-9]+")]
    Num,

    #[token("+")]
    Add,

    #[token("-")]
    Sub,

    #[token("*")]
    Mul,

    #[token("/")]
    Div,

    #[token("(")]
    LPar,

    #[token(")")]
    RPar,
}

pub struct Parser {
    tokens: Vec<lexer::Token>,
    cursor: usize,
}

impl Parser {
    pub fn new(tokens: Vec<lexer::Token>) -> Self {
        Self { tokens, cursor: 0 }
    }

    fn peek(&self) -> Option<&lexer::Token> {
        self.tokens.get(self.cursor)
    }

    fn consume(&mut self) -> Option<lexer::Token> {
        if self.cursor < self.tokens.len() {
            let tok = self.tokens[self.cursor].clone();
            self.cursor += 1;
            Some(tok)
        } else {
            None
        }
    }

    pub fn parse(&mut self) -> Result<ast::Expr, String> {
        let ast = self.parse_expr()?;
        if let Some(leftover) = self.peek() {
            return Err(format!("Unparsed token remaining: {:?}", leftover));
        }
        Ok(ast)
    }

    // Handles '+' and '-'
    fn parse_expr(&mut self) -> Result<ast::Expr, String> {
        let mut lhs = self.parse_term()?;

        while let Some(token) = self.peek() {
            let op = match token {
                lexer::Token::Plus => ast::BinaryOp::Add,
                lexer::Token::Minus => ast::BinaryOp::Sub,
                _ => break,
            };
            self.consume();
            let rhs = self.parse_term()?;
            lhs = ast::Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }

        Ok(lhs)
    }

    // Handles '*' and '/'
    fn parse_term(&mut self) -> Result<ast::Expr, String> {
        let mut lhs = self.parse_factor()?;

        while let Some(token) = self.peek() {
            let op = match token {
                Token::Star => ast::BinaryOp::Mul,
                Token::Slash => ast::BinaryOp::Div,
                _ => break,
            };
            self.consume();
            let rhs = self.parse_factor()?;
            lhs = Expr::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            };
        }

        Ok(lhs)
    }

    // Handles numbers and parenthesized sub-expressions
    fn parse_factor(&mut self) -> Result<Expr, String> {
        match self.consume() {
            Some(Token::Number(n)) => Ok(Expr::Number(n)),
            Some(Token::LParen) => {
                let inner = self.parse_expr()?;
                match self.consume() {
                    Some(Token::RParen) => Ok(inner),
                    other => Err(format!("Expected ')', got {:?}", other)),
                }
            }
            other => Err(format!("Expected factor, got {:?}", other)),
        }
    }
}
