use proptest::collection::{btree_set, vec};
use proptest::prelude::*;

pub use crate::ast::Program;
use crate::syntax::Symbol;
use crate::{Ctx, ast::*};

fn arb_sym(ctx: Ctx) -> impl Strategy<Value = Symbol>  {
    "[a-z][a-z0-9_]{0,5}"
        .prop_filter("keyword", |s| !matches!(s.as_str(), "fn" | "return"))
        .prop_map(move |s| ctx.intern(&s))
}

fn arb_int() -> impl Strategy<Value = i64> {
    0..=i64::MAX
}

#[derive(Clone, Debug)]
pub struct ExprParams {
    pub ctx: Ctx,
    pub depth: u32,
    pub size: u32,
    pub branch: u32,
}

impl ExprParams {
    pub fn default_with_ctx(ctx: Ctx) -> Self {
        Self { ctx, ..Self::default() }
    }
}

impl Default for ExprParams {
    fn default() -> Self {
        Self {
            ctx: Ctx::default(),
            depth: 3,
            size: 10,
            branch: 5,
        }
    }
}

#[derive(Clone, Debug)]
pub struct StmtParams {
    pub depth: u32,
    pub size: u32,
    pub branch: u32,
    pub ctx: Ctx,
}

impl Default for StmtParams {
    fn default() -> Self {
        Self {
            depth: 3,
            size: 10,
            branch: 5,
            ctx: Ctx::default(),
        }
    }
}

impl Arbitrary for Expr {
    type Parameters = ExprParams;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(p: ExprParams) -> Self::Strategy {
        let leaf = prop_oneof![
            arb_int().prop_map(Expr::Int),
            arb_sym(p.ctx).prop_map(Expr::Ident)
        ];
        leaf.prop_recursive(p.depth, p.size, p.branch, |inner| {
            prop_oneof![
                3 => (any::<PrefixOp>(), inner.clone()).prop_map(|(o, e)| Expr::prefix(o, e)),
                5 => (inner.clone(), any::<InfixOp>(), inner.clone())
                        .prop_map(|(l, o, r)| Expr::infix(l, o, r)),
                2 => (inner.clone(), vec(inner, 1..4))
                        .prop_map(|(f, a)| Expr::call_many(f, a)),
            ]
        })
        .boxed()
    }
}

impl Arbitrary for Stmt {
    type Parameters = StmtParams;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(p: StmtParams) -> Self::Strategy {
        let expr = || any_with::<Expr>(ExprParams::default_with_ctx(p.ctx.clone()));
        let leaf = prop_oneof![
            expr().prop_map(Stmt::Return),
            expr().prop_map(Stmt::Expr),
            (expr(), expr()).prop_map(|(lhs, rhs)| Stmt::Assign { lhs, rhs }),
        ];
        leaf.prop_recursive(p.depth, p.size, p.branch, move |inner| {
            (arb_sym(p.ctx.clone()), btree_set(arb_sym(p.ctx.clone()), 1..4), vec(inner, 0..4))
                .prop_filter_map("invalid function", move |(name, args, body)| {
                    let rhs = Expr::function(args.into_iter().collect::<Vec<_>>(), body)?;
                    Some(Stmt::Assign { lhs: Expr::Ident(name), rhs,})
                },
            )
        })
        .boxed()
    }
}

impl Arbitrary for Program {
    type Parameters = StmtParams;
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(p: StmtParams) -> Self::Strategy {
        vec(any_with::<Stmt>(p), 0..10)
            .prop_map(|stmts| Program { stmts })
            .boxed()
    }
}