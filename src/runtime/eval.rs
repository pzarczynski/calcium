use std::collections::HashSet;
use std::sync::Arc;

use crate::runtime::env::Scope;
use crate::runtime::value::Value;
use crate::syntax::ast::{ConstraintOp, Expr, Stmt};
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
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

pub fn eval_expr(expr: &Expr, scope: &Arc<Scope>) -> Result<Value, EvalError> {
    _eval_expr(expr, scope, &mut HashSet::new())
}

fn _eval_expr(
    expr: &Expr,
    scope: &Arc<Scope>,
    env: &mut HashSet<String>,
) -> Result<Value, EvalError> {
    match expr {
        Expr::Num(raw) => raw
            .parse::<f64>()
            .map(Value::number)
            .map_err(|_| EvalError::InvalidNumber(raw.clone())),

        Expr::Ident(name) => {
            if let Some(val) = scope.get_value(name) {
                return Ok(val);
            }

            if let Some(formula) = scope.get_binding(name) {
                if env.contains(name) {
                    return Err(EvalError::RecursionCycle(name.clone()));
                }

                env.insert(name.clone());
                let res = _eval_expr(&formula, scope, env);
                env.remove(name);
                return res;
            }

            Ok(Value::Residual(Expr::Ident(name.clone())))
        }

        Expr::Unary { op, expr } => Ok(_eval_expr(expr, scope, env)?.apply_unary(*op)),

        Expr::Binary { op, l, r } => {
            let l_val = _eval_expr(l, scope, env)?;
            let r_val = _eval_expr(r, scope, env)?;

            if (matches!(l_val, Value::Residual(_)) || matches!(r_val, Value::Residual(_)))
                && let (Some(l_expr), Some(r_expr)) = (l_val.to_expr(), r_val.to_expr())
            {
                return Ok(Value::Residual(Expr::Binary {
                    op: *op,
                    l: Box::new(l_expr),
                    r: Box::new(r_expr),
                }));
            }

            Ok(l_val.apply_binary(*op, &r_val))
        }

        Expr::Model { body, .. } => {
            let model_scope = Scope::new_child(Arc::clone(scope));
            for stmt in body {
                exec_stmt(stmt, &model_scope)?;
            }
            Ok(Value::Instance(model_scope))
        }

        Expr::Member { obj, field } => {
            let tgt_val = _eval_expr(obj, scope, env)?;
            match tgt_val {
                Value::Instance(inst_scope) => {
                    _eval_expr(&Expr::Ident(field.clone()), &inst_scope, env)
                }
                other => Err(EvalError::FieldAccessOnNonModel {
                    obj: other,
                    field: field.clone(),
                }),
            }
        }

        Expr::Call { callee: fun, args } => {
            let fun_name = match fun.as_ref() {
                Expr::Ident(name) => Some(name.as_str()),
                _ => None,
            };

            let sig = fun_name.and_then(|name| scope.get_signature(name));
            let child_scope = Scope::new_child(Arc::clone(scope));

            for (i, arg) in args.iter().enumerate() {
                let arg_val = _eval_expr(&arg.val, scope, env)?;

                if let Some(param_name) = &arg.key {
                    child_scope.insert_value(param_name.clone(), arg_val);
                } else if let Some(sig) = &sig {
                    if i < sig.len() {
                        child_scope.insert_value(sig[i].clone(), arg_val);
                    } else {
                        return Err(EvalError::TooManyPositionalArgs {
                            name: fun_name.unwrap_or("<fn>").to_string(),
                            expected: sig.len(),
                            got: args.len(),
                        });
                    }
                } else {
                    return Err(EvalError::MissingSignature(
                        fun_name.unwrap_or("<fn>").to_string(),
                    ));
                }
            }

            _eval_expr(fun, &child_scope, env)
        }
    }
}

pub fn exec_stmt(stmt: &Stmt, scope: &Arc<Scope>) -> Result<Option<Value>, EvalError> {
    match stmt {
        Stmt::Query(expr) => Ok(Some(eval_expr(expr, scope)?)),

        Stmt::Constraint {
            l,
            op: ConstraintOp::Eq,
            r,
        } => {
            match l {
                Expr::Call { callee: fun, args } => {
                    if let Expr::Ident(name) = fun.as_ref() {
                        let mut params = Vec::new();
                        for arg in args {
                            if let Expr::Ident(p) = &arg.val {
                                params.push(p.clone());
                            }
                        }
                        scope.insert_signature(name, params);
                        scope.insert_binding(name, r.clone());
                        return Ok(None);
                    }
                }

                Expr::Ident(name) => {
                    scope.insert_binding(name, r.clone());
                    return Ok(None);
                }

                _ => {}
            }

            Ok(None)
        }

        Stmt::Constraint { .. } => Ok(None),
    }
}
