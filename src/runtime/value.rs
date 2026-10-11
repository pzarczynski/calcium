use crate::runtime::env::Scope;
use crate::syntax::ast::{BinaryOp, Expr, UnaryOp};
use core::fmt;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Symbol,
    Discrete(Vec<),  // discrete set of solutions
    Instance(Arc<Scope>), // internal scope
    Residual(Expr),       // residual expr when unbound variables are remaining
    Empty,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Discrete(vals) => {
                if vals.is_empty() {
                    write!(f, "∅")
                } else if vals.len() == 1 {
                    write!(f, "{}", vals[0])
                } else {
                    let items: Vec<String> = vals.iter().map(|v| v.to_string()).collect();
                    write!(f, "{{{}}}", items.join(", "))
                }
            }
            Value::Interval { min, max } => write!(f, "[{min}, {max}]"),
            Value::Instance(scope) => {
                let bindings = scope.bindings.read().unwrap();
                let keys: Vec<_> = bindings.keys().cloned().collect();
                write!(f, "Model {{ {} }}", keys.join(", "))
            }
            Value::Residual(expr) => write!(f, "{expr}"),
            Value::Empty => write!(f, "∅"),
        }
    }
}

impl Value {
    pub const fn empty() -> Self {
        Value::Discrete(Vec::new())
    }

    pub fn number(n: f64) -> Self {
        Value::Discrete(vec![n])
    }

    pub fn apply_unary(&self, op: UnaryOp) -> Self {
        match (op, self) {
            (UnaryOp::Pos, val) => val.clone(),

            (UnaryOp::Neg, Value::Discrete(vals)) => {
                let mut negated: Vec<f64> = vals.iter().map(|&x| -x).collect();
                negated.sort_by(|a, b| a.total_cmp(b));
                Value::Discrete(negated)
            }

            (UnaryOp::Neg, Value::Interval { min, max }) => Value::Interval {
                min: -*max,
                max: -*min,
            },

            (op, Value::Residual(expr)) => Value::Residual(Expr::Unary {
                op,
                expr: Box::new(expr.clone()),
            }),

            _ => Self::empty(),
        }
    }

    pub fn apply_binary(&self, op: BinaryOp, r: &Self) -> Self {
        match (self, r) {
            (Value::Discrete(l_vals), Value::Discrete(r_vals)) => {
                Self::minkowski_discrete(l_vals, op, r_vals)
            }

            (Value::Interval { min: a, max: b }, Value::Interval { min: c, max: d }) => {
                Self::interval_op(*a, *b, op, *c, *d)
            }

            (Value::Discrete(vals), Value::Interval { min, max }) => {
                if vals.len() == 1 {
                    Self::interval_op(vals[0], vals[0], op, *min, *max)
                } else {
                    let mut intervals: Vec<_> = vals
                        .iter()
                        .filter_map(|&v| match Self::interval_op(v, v, op, *min, *max) {
                            Value::Interval { min, max } => Some((min, max)),
                            _ => None,
                        })
                        .collect();

                    if intervals.is_empty() {
                        Self::empty()
                    } else {
                        let min_bound = intervals
                            .iter()
                            .map(|(min, _)| *min)
                            .fold(f64::INFINITY, f64::min);
                        let max_bound = intervals
                            .iter()
                            .map(|(_, max)| *max)
                            .fold(f64::NEG_INFINITY, f64::max);
                        Value::Interval {
                            min: min_bound,
                            max: max_bound,
                        }
                    }
                }
            }

            (Value::Interval { min, max }, Value::Discrete(vals)) => {
                if vals.len() == 1 {
                    Self::interval_op(*min, *max, op, vals[0], vals[0])
                } else {
                    let intervals: Vec<_> = vals
                        .iter()
                        .filter_map(|&v| match Self::interval_op(*min, *max, op, v, v) {
                            Value::Interval { min, max } => Some((min, max)),
                            _ => None,
                        })
                        .collect();

                    if intervals.is_empty() {
                        Self::empty()
                    } else {
                        let min_bound = intervals
                            .iter()
                            .map(|(min, _)| *min)
                            .fold(f64::INFINITY, f64::min);
                        let max_bound = intervals
                            .iter()
                            .map(|(_, max)| *max)
                            .fold(f64::NEG_INFINITY, f64::max);
                        Value::Interval {
                            min: min_bound,
                            max: max_bound,
                        }
                    }
                }
            }

            (Value::Residual(l), Value::Residual(r)) => Value::Residual(Expr::Binary {
                op,
                l: Box::new(l.clone()),
                r: Box::new(r.clone()),
            }),

            _ => Self::empty(),
        }
    }

    fn minkowski_discrete(l: &[f64], op: BinaryOp, r: &[f64]) -> Self {
        let mut results = Vec::with_capacity(l.len() * r.len());

        for &a in l {
            for &b in r {
                let res = match op {
                    BinaryOp::Add => Some(a + b),
                    BinaryOp::Sub => Some(a - b),
                    BinaryOp::Mul => Some(a * b),
                    BinaryOp::Div => {
                        if b == 0.0 {
                            None
                        } else {
                            Some(a / b)
                        }
                    }
                    BinaryOp::Pow => {
                        let p = a.powf(b);
                        if p.is_nan() { None } else { Some(p) }
                    }
                };

                if let Some(val) = res {
                    results.push(val);
                }
            }
        }

        results.sort_by(|a, b| a.total_cmp(b));
        results.dedup_by(|a, b| a.total_cmp(b).is_eq());

        Value::Discrete(results)
    }

    fn interval_op(a: f64, b: f64, op: BinaryOp, c: f64, d: f64) -> Self {
        match op {
            BinaryOp::Add => Value::Interval {
                min: a + c,
                max: b + d,
            },
            BinaryOp::Sub => Value::Interval {
                min: a - d,
                max: b - c,
            },
            BinaryOp::Mul => {
                let products = [a * c, a * d, b * c, b * d];
                let min = products.into_iter().fold(f64::INFINITY, f64::min);
                let max = products.into_iter().fold(f64::NEG_INFINITY, f64::max);
                Value::Interval { min, max }
            }
            BinaryOp::Div => {
                if c <= 0.0 && d >= 0.0 {
                    Self::empty()
                } else {
                    let divs = [a / c, a / d, b / c, b / d];
                    let min = divs.into_iter().fold(f64::INFINITY, f64::min);
                    let max = divs.into_iter().fold(f64::NEG_INFINITY, f64::max);
                    Value::Interval { min, max }
                }
            }
            BinaryOp::Pow => {
                let powers = [a.powf(c), a.powf(d), b.powf(c), b.powf(d)];
                if powers.iter().any(|x| x.is_nan()) {
                    Self::empty()
                } else {
                    let min = powers.into_iter().fold(f64::INFINITY, f64::min);
                    let max = powers.into_iter().fold(f64::NEG_INFINITY, f64::max);
                    Value::Interval { min, max }
                }
            }
        }
    }

    pub fn to_expr(&self) -> Option<Expr> {
        match self {
            Value::Discrete(vals) if vals.len() == 1 => Some(Expr::Num(vals[0].to_string())),
            Value::Residual(e) => Some(e.clone()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_minkowski_product() {
        let x = Value::empty();
        let y = Value::empty();
        let res = x.apply_binary(BinaryOp::Mul, &y);

        assert_eq!(res, Value::empty());
    }

    #[test]
    fn singleton_behave_like_scalars() {
        let a = Value::number(3.0);
        let b = Value::number(4.0);
        let res = a.apply_binary(BinaryOp::Add, &b);

        assert_eq!(res, Value::Discrete(vec![7.0]));
    }

    #[test]
    fn multi_solution_minkowski_sum() {
        // x in {-1, 1}, y in {10, 20}
        // x + y -> {-1+10, -1+20, 1+10, 1+20} = {9, 11, 19, 21}
        let x = Value::Discrete(vec![-1.0, 1.0]);
        let y = Value::Discrete(vec![10.0, 20.0]);
        let res = x.apply_binary(BinaryOp::Add, &y);

        assert_eq!(res, Value::Discrete(vec![9.0, 11.0, 19.0, 21.0]));
    }

    #[test]
    fn prune_division_by_zero_branches() {
        // {-2, 4} / {0, 2} -> keeping {-1, 2}
        let nums = Value::Discrete(vec![-2.0, 4.0]);
        let denoms = Value::Discrete(vec![0.0, 2.0]);
        let res = nums.apply_binary(BinaryOp::Div, &denoms);

        assert_eq!(res, Value::Discrete(vec![-1.0, 2.0]));
    }

    #[test]
    fn interval_addition() {
        let i1 = Value::Interval { min: 1.0, max: 3.0 };
        let i2 = Value::Interval { min: 2.0, max: 5.0 };
        let res = i1.apply_binary(BinaryOp::Add, &i2);

        assert_eq!(res, Value::Interval { min: 3.0, max: 8.0 });
    }
}
