use crate::runtime::value::Value;
use crate::syntax::ast::Expr;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum EvalError {
    #[error("Parse number error for '{0}'")]
    InvalidNumber(String),

    #[error("Field access '{field}' on non-model value: {obj:?}")]
    FieldAccessOnNonModel { obj: Value, field: String },

    #[error("Cannot apply positional arguments to '{0}': no declared signature")]
    MissingSignature(String),

    #[error("Too many positional arguments for '{name}': expected {expected}, got {got}")]
    TooManyPositionalArgs {
        name: String,
        expected: usize,
        got: usize,
    },

    #[error("Cyclic dependency detected while resolving symbol '{0}'")]
    RecursionCycle(String),
}

#[derive(Debug, Default)]
pub struct Scope {
    parent: Option<Arc<Scope>>,
    values: RwLock<HashMap<String, Value>>,
    pub bindings: RwLock<HashMap<String, Expr>>,
    signatures: RwLock<HashMap<String, Vec<String>>>,
}

impl Scope {
    pub fn new_child(parent: Arc<Scope>) -> Arc<Self> {
        Arc::new(Self {
            parent: Some(parent),
            values: RwLock::new(HashMap::new()),
            bindings: RwLock::new(HashMap::new()),
            signatures: RwLock::new(HashMap::new()),
        })
    }

    pub fn get_value(&self, name: &str) -> Option<Value> {
        if let Some(val) = self.values.read().unwrap().get(name) {
            Some(val.clone())
        } else if let Some(parent) = &self.parent {
            parent.get_value(name)
        } else {
            None
        }
    }

    pub fn get_binding(&self, name: &str) -> Option<Expr> {
        if let Some(expr) = self.bindings.read().unwrap().get(name) {
            Some(expr.clone())
        } else if let Some(parent) = &self.parent {
            parent.get_binding(name)
        } else {
            None
        }
    }

    pub fn get_signature(&self, name: &str) -> Option<Vec<String>> {
        if let Some(sig) = self.signatures.read().unwrap().get(name) {
            Some(sig.clone())
        } else if let Some(parent) = &self.parent {
            parent.get_signature(name)
        } else {
            None
        }
    }

    pub fn insert_value(&self, name: impl Into<String>, val: Value) {
        self.values.write().unwrap().insert(name.into(), val);
    }

    pub fn insert_binding(&self, name: impl Into<String>, expr: Expr) {
        self.bindings.write().unwrap().insert(name.into(), expr);
    }

    pub fn insert_signature(&self, name: impl Into<String>, sig: Vec<String>) {
        self.signatures.write().unwrap().insert(name.into(), sig);
    }
}

impl PartialEq for Scope {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
