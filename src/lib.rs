pub mod syntax;

use std::sync::Arc;

pub use syntax::ast;
pub use syntax::display;
pub use syntax::{Lexed, Parser, Span, Symbol, Token};

use lasso::ThreadedRodeo;

pub type Interner = Arc<ThreadedRodeo>;

#[derive(Debug, Clone, Default)]
pub struct Ctx(Arc<CtxInner>);

#[derive(Debug, Default)]
struct CtxInner { interner: ThreadedRodeo }

impl Ctx {    
    pub fn intern(&self, s: &str) -> Symbol {
        Symbol(self.0.interner.get_or_intern(s))
    }

    pub fn resolve<'a>(&'a self, sym: &Symbol) -> &'a str {
        self.0.interner.resolve(&sym.0)
    }
}