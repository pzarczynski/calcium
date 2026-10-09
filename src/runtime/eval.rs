use std::collections::HashSet;
use std::sync::Arc;

use crate::runtime::env::{EvalError, Scope};
use crate::runtime::value::Value;
use crate::syntax::ast::{ConstraintOp, Expr, Stmt};

/// Public entry point for expression evaluation
pub fn eval_expr(expr: &Expr, scope: &Arc<Scope>) -> Result<Value, EvalError> {
    let mut visited = HashSet::new();
    eval_expr_internal(expr, scope, &mut visited)
}

fn eval_expr_internal(
    expr: &Expr,
    scope: &Arc<Scope>,
    visited: &mut HashSet<String>,
) -> Result<Value, EvalError> {
    match expr {
        // --- Numbers ---
        Expr::Num(raw) => raw
            .parse::<f64>()
            .map(Value::number)
            .map_err(|_| EvalError::InvalidNumber(raw.clone())),

        // --- Identifiers ---
        Expr::Ident(name) => {
            // 1. Check if bound to an explicit value (overrides/constants)
            if let Some(val) = scope.get_value(name) {
                return Ok(val);
            }

            // 2. Check if bound to an AST formula
            if let Some(formula) = scope.get_binding(name) {
                if visited.contains(name) {
                    return Err(EvalError::RecursionCycle(name.clone()));
                }

                visited.insert(name.clone());
                // Evaluate formula within *current* scope so child-frame overrides apply!
                let res = eval_expr_internal(&formula, scope, visited);
                visited.remove(name);
                return res;
            }

            // 3. Fallback: Unbound variable becomes a residual expression
            Ok(Value::Residual(Expr::Ident(name.clone()))) //[cite: 2]
        }

        // --- Unary Operations ---
        Expr::Unary { op, expr } => {
            //[cite: 2]
            let val = eval_expr_internal(expr, scope, visited)?;
            Ok(val.apply_unary(*op))
        }

        // --- Binary Operations ---
        Expr::Binary { op, lhs, rhs } => {
            //[cite: 2]
            let l_val = eval_expr_internal(lhs, scope, visited)?;
            let r_val = eval_expr_internal(rhs, scope, visited)?;

            // If either operand is residual, fold into a residual AST
            if (matches!(l_val, Value::Residual(_)) || matches!(r_val, Value::Residual(_)))
                && let (Some(l_expr), Some(r_expr)) = (l_val.to_expr(), r_val.to_expr())
            {
                return Ok(Value::Residual(Expr::Binary {
                    //[cite: 2]
                    op: *op,
                    lhs: Box::new(l_expr),
                    rhs: Box::new(r_expr),
                }));
            }

            Ok(l_val.apply_binary(*op, &r_val))
        }

        // --- Models / Encapsulated Scopes ---
        Expr::Model { body, .. } => {
            //[cite: 2]
            let model_scope = Scope::new_child(Arc::clone(scope));
            for stmt in body {
                exec_stmt(stmt, &model_scope)?;
            }
            Ok(Value::Instance(model_scope))
        }

        // --- Field Access (.field) ---
        Expr::Member { obj, field } => {
            //[cite: 2]
            let target_val = eval_expr_internal(obj, scope, visited)?;
            match target_val {
                Value::Instance(inst_scope) => {
                    eval_expr_internal(&Expr::Ident(field.clone()), &inst_scope, visited) //[cite: 2]
                }
                other => Err(EvalError::FieldAccessOnNonModel {
                    obj: other,
                    field: field.clone(),
                }),
            }
        }

        // --- Call / Contextual Substitution ---
        Expr::Call { callee, args } => {
            //[cite: 2]
            // Extract callee identifier if applicable (to read signatures)
            let callee_name = match callee.as_ref() {
                Expr::Ident(name) => Some(name.as_str()), //[cite: 2]
                _ => None,
            };

            let signature = callee_name.and_then(|name| scope.get_signature(name));

            // Create a child substitution frame
            let child_scope = Scope::new_child(Arc::clone(scope));

            for (i, arg) in args.iter().enumerate() {
                //[cite: 2]
                // Arguments are evaluated in the caller's scope
                let arg_val = eval_expr_internal(&arg.val, scope, visited)?; //[cite: 2]

                if let Some(param_name) = &arg.key {
                    //[cite: 2]
                    // Named keyword override: x(y = 3)
                    child_scope.insert_value(param_name.clone(), arg_val);
                } else if let Some(sig) = &signature {
                    // Positional substitution: x(3) mapped via signature
                    if i < sig.len() {
                        child_scope.insert_value(sig[i].clone(), arg_val);
                    } else {
                        return Err(EvalError::TooManyPositionalArgs {
                            name: callee_name.unwrap_or("anonymous").to_string(),
                            expected: sig.len(),
                            got: args.len(), //[cite: 2]
                        });
                    }
                } else {
                    return Err(EvalError::MissingSignature(
                        callee_name.unwrap_or("anonymous").to_string(),
                    ));
                }
            }

            // Evaluate the callee inside the child scope containing overrides
            eval_expr_internal(callee, &child_scope, visited)
        }
    }
}

/// Executes a statement against a scope
pub fn exec_stmt(stmt: &Stmt, scope: &Arc<Scope>) -> Result<Option<Value>, EvalError> {
    //[cite: 2]
    match stmt {
        Stmt::Query(expr) => {
            //[cite: 2]
            let val = eval_expr(expr, scope)?;
            Ok(Some(val))
        }

        Stmt::Constraint {
            lhs,
            op: ConstraintOp::Eq,
            rhs,
        } => {
            //[cite: 2]
            match lhs {
                // Signature definition: x(y) = y - 1
                Expr::Call { callee, args } => {
                    //[cite: 2]
                    if let Expr::Ident(name) = callee.as_ref() {
                        //[cite: 2]
                        let mut params = Vec::new();
                        for arg in args {
                            //[cite: 2]
                            if let Expr::Ident(p) = &arg.val {
                                //[cite: 2]
                                params.push(p.clone());
                            }
                        }
                        scope.insert_signature(name, params);
                        scope.insert_binding(name, rhs.clone());
                        return Ok(None);
                    }
                }

                // Simple variable assignment: x = y - 1
                Expr::Ident(name) => {
                    //[cite: 2]
                    scope.insert_binding(name, rhs.clone());
                    return Ok(None);
                }

                _ => {}
            }

            // Non-trivial equality relations (e.g. x^2 = 1) will feed into the constraint solver
            Ok(None)
        }

        Stmt::Constraint { .. } => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::ast::*; //[cite: 2]

    #[test]
    fn test_positional_and_keyword_substitution() {
        let scope = Arc::new(Scope::default());

        // x(y) = y - 1
        let def = Stmt::Constraint {
            //[cite: 2]
            lhs: Expr::Call {
                //[cite: 2]
                callee: Box::new(Expr::Ident("x".into())), //[cite: 2]
                args: vec![Arg::positional(Expr::Ident("y".into()))], //[cite: 2]
            },
            op: ConstraintOp::Eq, //[cite: 2]
            rhs: Expr::Binary {
                //[cite: 2]
                op: BinaryOp::Sub,                      //[cite: 2]
                lhs: Box::new(Expr::Ident("y".into())), //[cite: 2]
                rhs: Box::new(Expr::Num("1".into())),   //[cite: 2]
            },
        };
        exec_stmt(&def, &scope).unwrap();

        // Query: x(3)
        let pos_call = Expr::Call {
            //[cite: 2]
            callee: Box::new(Expr::Ident("x".into())), //[cite: 2]
            args: vec![Arg::positional(Expr::Num("3".into()))], //[cite: 2]
        };
        assert_eq!(eval_expr(&pos_call, &scope).unwrap(), Value::number(2.0));

        // Query: x(y = 5)
        let kw_call = Expr::Call {
            //[cite: 2]
            callee: Box::new(Expr::Ident("x".into())), //[cite: 2]
            args: vec![Arg::keyword("y", Expr::Num("5".into()))], //[cite: 2]
        };
        assert_eq!(eval_expr(&kw_call, &scope).unwrap(), Value::number(4.0));
    }

    #[test]
    fn test_deep_propagation() {
        let scope = Arc::new(Scope::default());

        // B = C + 1
        scope.insert_binding(
            "B",
            Expr::Binary {
                //[cite: 2]
                op: BinaryOp::Add,                      //[cite: 2]
                lhs: Box::new(Expr::Ident("C".into())), //[cite: 2]
                rhs: Box::new(Expr::Num("1".into())),   //[cite: 2]
            },
        );

        // A = B * 2
        scope.insert_binding(
            "A",
            Expr::Binary {
                //[cite: 2]
                op: BinaryOp::Mul,                      //[cite: 2]
                lhs: Box::new(Expr::Ident("B".into())), //[cite: 2]
                rhs: Box::new(Expr::Num("2".into())),   //[cite: 2]
            },
        );

        // Query: A(C = 5) -> B becomes 6 -> A becomes 12
        let call = Expr::Call {
            //[cite: 2]
            callee: Box::new(Expr::Ident("A".into())), //[cite: 2]
            args: vec![Arg::keyword("C", Expr::Num("5".into()))], //[cite: 2]
        };

        assert_eq!(eval_expr(&call, &scope).unwrap(), Value::number(12.0));
    }

    #[test]
    fn test_partial_evaluation_residual() {
        let scope = Arc::new(Scope::default());

        // f = x + y
        scope.insert_binding(
            "f",
            Expr::Binary {
                op: BinaryOp::Add,                      //[cite: 2]
                lhs: Box::new(Expr::Ident("x".into())), //[cite: 2]
                rhs: Box::new(Expr::Ident("y".into())), //[cite: 2]
            },
        );

        // Query: f(x = 2) -> y is still unbound -> returns Residual(2 + y)
        let call = Expr::Call {
            callee: Box::new(Expr::Ident("f".into())), //[cite: 2]
            args: vec![Arg::keyword("x", Expr::Num("2".into()))], //[cite: 2]
        };

        match eval_expr(&call, &scope).unwrap() {
            Value::Residual(Expr::Binary { op, lhs, rhs }) => {
                assert_eq!(op, BinaryOp::Add); //[cite: 2]
                assert_eq!(*lhs, Expr::Num("2".into())); //[cite: 2]
                assert_eq!(*rhs, Expr::Ident("y".into())); //[cite: 2]
            }
            other => panic!("Expected residual expression, got {other:?}"),
        }
    }
}
