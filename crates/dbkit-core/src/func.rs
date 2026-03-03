use crate::expr::{Expr, ExprNode, IntoExpr};
use crate::PgVector;

pub fn upper(arg: impl IntoExpr<String>) -> Expr<String> {
    let expr = arg.into_expr();
    Expr::new(ExprNode::Func {
        name: "UPPER",
        args: vec![expr.node],
    })
}

pub fn count<T>(arg: impl IntoExpr<T>) -> Expr<i64> {
    let expr = arg.into_expr();
    Expr::new(ExprNode::Func {
        name: "COUNT",
        args: vec![expr.node],
    })
}

pub fn sum<T>(arg: impl IntoExpr<T>) -> Expr<T> {
    let expr = arg.into_expr();
    Expr::new(ExprNode::Func {
        name: "SUM",
        args: vec![expr.node],
    })
}

pub fn coalesce<T>(a: impl IntoExpr<T>, b: impl IntoExpr<T>) -> Expr<T> {
    let left = a.into_expr();
    let right = b.into_expr();
    Expr::new(ExprNode::Func {
        name: "COALESCE",
        args: vec![left.node, right.node],
    })
}

pub fn date_trunc<T>(part: impl IntoExpr<String>, value: impl IntoExpr<T>) -> Expr<T> {
    let part = part.into_expr();
    let value = value.into_expr();
    Expr::new(ExprNode::Func {
        name: "DATE_TRUNC",
        args: vec![part.node, value.node],
    })
}

pub trait VectorExpr<const N: usize> {}

impl<const N: usize> VectorExpr<N> for PgVector<N> {}
impl<const N: usize> VectorExpr<N> for Option<PgVector<N>> {}

fn vector_binary_fn<const N: usize, L, R>(
    name: &'static str,
    left: impl IntoExpr<L>,
    right: impl IntoExpr<R>,
) -> Expr<f32>
where
    L: VectorExpr<N>,
    R: VectorExpr<N>,
{
    let left = left.into_expr();
    let right = right.into_expr();
    Expr::new(ExprNode::Func {
        name,
        args: vec![left.node, right.node],
    })
}

pub fn l2_distance<const N: usize, L, R>(
    left: impl IntoExpr<L>,
    right: impl IntoExpr<R>,
) -> Expr<f32>
where
    L: VectorExpr<N>,
    R: VectorExpr<N>,
{
    vector_binary_fn::<N, L, R>("L2_DISTANCE", left, right)
}

pub fn cosine_distance<const N: usize, L, R>(
    left: impl IntoExpr<L>,
    right: impl IntoExpr<R>,
) -> Expr<f32>
where
    L: VectorExpr<N>,
    R: VectorExpr<N>,
{
    vector_binary_fn::<N, L, R>("COSINE_DISTANCE", left, right)
}

pub fn inner_product<const N: usize, L, R>(
    left: impl IntoExpr<L>,
    right: impl IntoExpr<R>,
) -> Expr<f32>
where
    L: VectorExpr<N>,
    R: VectorExpr<N>,
{
    vector_binary_fn::<N, L, R>("INNER_PRODUCT", left, right)
}

pub fn l1_distance<const N: usize, L, R>(
    left: impl IntoExpr<L>,
    right: impl IntoExpr<R>,
) -> Expr<f32>
where
    L: VectorExpr<N>,
    R: VectorExpr<N>,
{
    vector_binary_fn::<N, L, R>("L1_DISTANCE", left, right)
}
