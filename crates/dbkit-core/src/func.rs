use crate::expr::{Expr, ExprNode, IntoExpr, NumericExprType, VectorBinaryOp};
use crate::PgVector;

pub trait StringUnaryExpr {
    type Output;
}

impl StringUnaryExpr for String {
    type Output = String;
}

impl StringUnaryExpr for Option<String> {
    type Output = Option<String>;
}

pub trait StringLengthExpr {
    type Output;
}

impl StringLengthExpr for String {
    type Output = i32;
}

impl StringLengthExpr for Option<String> {
    type Output = Option<i32>;
}

fn unary_string_fn<T>(name: &'static str, arg: impl IntoExpr<T>) -> Expr<<T as StringUnaryExpr>::Output>
where
    T: StringUnaryExpr,
{
    let expr = arg.into_expr();
    Expr::new(ExprNode::Func {
        name,
        args: vec![expr.node],
    })
}

fn string_length_fn<T>(name: &'static str, arg: impl IntoExpr<T>) -> Expr<<T as StringLengthExpr>::Output>
where
    T: StringLengthExpr,
{
    let expr = arg.into_expr();
    Expr::new(ExprNode::Func {
        name,
        args: vec![expr.node],
    })
}

pub fn upper<T>(arg: impl IntoExpr<T>) -> Expr<<T as StringUnaryExpr>::Output>
where
    T: StringUnaryExpr,
{
    unary_string_fn("UPPER", arg)
}

pub fn trim<T>(arg: impl IntoExpr<T>) -> Expr<<T as StringUnaryExpr>::Output>
where
    T: StringUnaryExpr,
{
    unary_string_fn("TRIM", arg)
}

pub fn char_length<T>(arg: impl IntoExpr<T>) -> Expr<<T as StringLengthExpr>::Output>
where
    T: StringLengthExpr,
{
    string_length_fn("CHAR_LENGTH", arg)
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

pub fn least<T>(a: impl IntoExpr<T>, b: impl IntoExpr<T>) -> Expr<T> {
    let left = a.into_expr();
    let right = b.into_expr();
    Expr::new(ExprNode::Func {
        name: "LEAST",
        args: vec![left.node, right.node],
    })
}

pub fn greatest<T>(a: impl IntoExpr<T>, b: impl IntoExpr<T>) -> Expr<T> {
    let left = a.into_expr();
    let right = b.into_expr();
    Expr::new(ExprNode::Func {
        name: "GREATEST",
        args: vec![left.node, right.node],
    })
}

pub fn power<B, E>(base: impl IntoExpr<B>, exponent: impl IntoExpr<E>) -> Expr<f64>
where
    B: NumericExprType,
    E: NumericExprType,
{
    let base = base.into_expr();
    let exponent = exponent.into_expr();
    Expr::new(ExprNode::Func {
        name: "POWER",
        args: vec![base.node, exponent.node],
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

/// Marker trait for values that can participate in vector distance/similarity expressions.
pub trait VectorExpr<const N: usize> {}

impl<const N: usize> VectorExpr<N> for PgVector<N> {}
impl<const N: usize> VectorExpr<N> for Option<PgVector<N>> {}

fn vector_binary_fn<const N: usize, L, R>(name: &'static str, left: impl IntoExpr<L>, right: impl IntoExpr<R>) -> Expr<f32>
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

fn vector_binary_operator<const N: usize, L, R>(op: VectorBinaryOp, left: impl IntoExpr<L>, right: impl IntoExpr<R>) -> Expr<f32>
where
    L: VectorExpr<N>,
    R: VectorExpr<N>,
{
    let left = left.into_expr();
    let right = right.into_expr();
    Expr::new(ExprNode::VectorBinary {
        left: Box::new(left.node),
        op,
        right: Box::new(right.node),
    })
}

/// Euclidean (L2) distance using pgvector's `<->` operator.
///
/// Lower is more similar.
///
/// ANN note:
/// - This form is operator-based and can use pgvector ivfflat/hnsw indexes for
///   `ORDER BY ... LIMIT` nearest-neighbor queries.
pub fn l2_distance<const N: usize, L, R>(left: impl IntoExpr<L>, right: impl IntoExpr<R>) -> Expr<f32>
where
    L: VectorExpr<N>,
    R: VectorExpr<N>,
{
    vector_binary_operator::<N, L, R>(VectorBinaryOp::L2Distance, left, right)
}

/// Cosine distance using pgvector's `<=>` operator.
///
/// Lower is more similar.
///
/// ANN note:
/// - This form is operator-based and can use pgvector ivfflat/hnsw indexes for
///   `ORDER BY ... LIMIT` nearest-neighbor queries.
pub fn cosine_distance<const N: usize, L, R>(left: impl IntoExpr<L>, right: impl IntoExpr<R>) -> Expr<f32>
where
    L: VectorExpr<N>,
    R: VectorExpr<N>,
{
    vector_binary_operator::<N, L, R>(VectorBinaryOp::CosineDistance, left, right)
}

/// True inner product as a function expression (`INNER_PRODUCT(a, b)`).
///
/// Higher is more similar (for normalized embeddings, identical vectors are `1.0`).
///
/// ANN warning:
/// - This is intentionally a function call to preserve true inner-product semantics,
///   but function expressions are generally not pgvector ANN index-compatible for
///   `ORDER BY ... LIMIT`.
/// - For ANN-indexed retrieval, use [`inner_product_distance`] with `ORDER BY ASC`.
pub fn inner_product<const N: usize, L, R>(left: impl IntoExpr<L>, right: impl IntoExpr<R>) -> Expr<f32>
where
    L: VectorExpr<N>,
    R: VectorExpr<N>,
{
    vector_binary_fn::<N, L, R>("INNER_PRODUCT", left, right)
}

/// L1 (Manhattan) distance using pgvector's `<+>` operator.
///
/// Lower is more similar.
///
/// ANN note:
/// - This form is operator-based and can use pgvector ivfflat/hnsw indexes for
///   `ORDER BY ... LIMIT` nearest-neighbor queries.
pub fn l1_distance<const N: usize, L, R>(left: impl IntoExpr<L>, right: impl IntoExpr<R>) -> Expr<f32>
where
    L: VectorExpr<N>,
    R: VectorExpr<N>,
{
    vector_binary_operator::<N, L, R>(VectorBinaryOp::L1Distance, left, right)
}

/// Negative inner-product distance using pgvector's `<#>` operator.
///
/// Lower is more similar, so nearest-neighbor queries should use `ORDER BY ASC`.
///
/// ANN note:
/// - This form is operator-based and can use pgvector ivfflat/hnsw indexes for
///   `ORDER BY ... LIMIT` nearest-neighbor queries.
/// - Thresholds are inverted relative to true inner product
///   (for example `inner_product > 0.9` corresponds to
///   `inner_product_distance < -0.9`).
pub fn inner_product_distance<const N: usize, L, R>(left: impl IntoExpr<L>, right: impl IntoExpr<R>) -> Expr<f32>
where
    L: VectorExpr<N>,
    R: VectorExpr<N>,
{
    vector_binary_operator::<N, L, R>(VectorBinaryOp::InnerProductDistance, left, right)
}
