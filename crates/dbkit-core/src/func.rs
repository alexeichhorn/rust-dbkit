use crate::compile::CompiledSql;
use crate::expr::{AggregateExpr, Expr, ExprNode, IntoExpr, NumericExprType, TrimDirection, VectorBinaryOp};
use crate::query::Select;
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

pub trait StringBinaryExpr<Rhs, Result> {
    type Output;
}

impl<Result> StringBinaryExpr<String, Result> for String {
    type Output = Result;
}

impl<Result> StringBinaryExpr<Option<String>, Result> for String {
    type Output = Option<Result>;
}

impl<Result> StringBinaryExpr<String, Result> for Option<String> {
    type Output = Option<Result>;
}

impl<Result> StringBinaryExpr<Option<String>, Result> for Option<String> {
    type Output = Option<Result>;
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

fn string_fn<T>(name: &'static str, arg: impl IntoExpr<T>, extra_args: Vec<ExprNode>) -> Expr<<T as StringUnaryExpr>::Output>
where
    T: StringUnaryExpr,
{
    let mut args = vec![arg.into_expr().node];
    args.extend(extra_args);
    Expr::new(ExprNode::Func { name, args })
}

fn binary_string_fn<L, R, O>(name: &'static str, left: impl IntoExpr<L>, right: impl IntoExpr<R>) -> Expr<O> {
    let left = left.into_expr();
    let right = right.into_expr();
    Expr::new(ExprNode::Func {
        name,
        args: vec![left.node, right.node],
    })
}

fn directed_trim_fn<T>(
    arg: impl IntoExpr<T>,
    direction: TrimDirection,
    characters: Option<Expr<String>>,
) -> Expr<<T as StringUnaryExpr>::Output>
where
    T: StringUnaryExpr,
{
    Expr::new(ExprNode::Trim {
        direction,
        expr: Box::new(arg.into_expr().node),
        characters: characters.map(|characters| Box::new(characters.node)),
    })
}

pub fn upper<T>(arg: impl IntoExpr<T>) -> Expr<<T as StringUnaryExpr>::Output>
where
    T: StringUnaryExpr,
{
    unary_string_fn("UPPER", arg)
}

pub fn lower<T>(arg: impl IntoExpr<T>) -> Expr<<T as StringUnaryExpr>::Output>
where
    T: StringUnaryExpr,
{
    unary_string_fn("LOWER", arg)
}

pub fn trim<T>(arg: impl IntoExpr<T>) -> Expr<<T as StringUnaryExpr>::Output>
where
    T: StringUnaryExpr,
{
    unary_string_fn("TRIM", arg)
}

pub fn trim_chars<T>(arg: impl IntoExpr<T>, characters: impl IntoExpr<String>) -> Expr<<T as StringUnaryExpr>::Output>
where
    T: StringUnaryExpr,
{
    directed_trim_fn(arg, TrimDirection::Both, Some(characters.into_expr()))
}

pub fn trim_start<T>(arg: impl IntoExpr<T>) -> Expr<<T as StringUnaryExpr>::Output>
where
    T: StringUnaryExpr,
{
    directed_trim_fn(arg, TrimDirection::Leading, None)
}

pub fn trim_start_chars<T>(arg: impl IntoExpr<T>, characters: impl IntoExpr<String>) -> Expr<<T as StringUnaryExpr>::Output>
where
    T: StringUnaryExpr,
{
    directed_trim_fn(arg, TrimDirection::Leading, Some(characters.into_expr()))
}

pub fn trim_end<T>(arg: impl IntoExpr<T>) -> Expr<<T as StringUnaryExpr>::Output>
where
    T: StringUnaryExpr,
{
    directed_trim_fn(arg, TrimDirection::Trailing, None)
}

pub fn trim_end_chars<T>(arg: impl IntoExpr<T>, characters: impl IntoExpr<String>) -> Expr<<T as StringUnaryExpr>::Output>
where
    T: StringUnaryExpr,
{
    directed_trim_fn(arg, TrimDirection::Trailing, Some(characters.into_expr()))
}

pub fn char_length<T>(arg: impl IntoExpr<T>) -> Expr<<T as StringLengthExpr>::Output>
where
    T: StringLengthExpr,
{
    string_length_fn("CHAR_LENGTH", arg)
}

/// Returns the first `count` characters, or all but the last `|count|` when negative.
/// Maps to PostgreSQL `LEFT`.
pub fn left<T>(arg: impl IntoExpr<T>, count: impl IntoExpr<i32>) -> Expr<<T as StringUnaryExpr>::Output>
where
    T: StringUnaryExpr,
{
    string_fn("LEFT", arg, vec![count.into_expr().node])
}

/// Returns the last `count` characters, or all but the first `|count|` when negative.
/// Maps to PostgreSQL `RIGHT`.
pub fn right<T>(arg: impl IntoExpr<T>, count: impl IntoExpr<i32>) -> Expr<<T as StringUnaryExpr>::Output>
where
    T: StringUnaryExpr,
{
    string_fn("RIGHT", arg, vec![count.into_expr().node])
}

/// Returns up to `count` characters from the 1-based `start`.
/// From `"abcdef"`, `(2, 3)` yields `"bcd"` and `(0, 3)` yields `"ab"`; negative counts are rejected.
/// Maps to PostgreSQL `SUBSTRING`.
pub fn substring<T>(arg: impl IntoExpr<T>, start: impl IntoExpr<i32>, count: impl IntoExpr<i32>) -> Expr<<T as StringUnaryExpr>::Output>
where
    T: StringUnaryExpr,
{
    string_fn("SUBSTRING", arg, vec![start.into_expr().node, count.into_expr().node])
}

/// Repeats the text `count` times.
/// Repeating `"ab"` three times yields `"ababab"`; non-positive counts yield an empty string.
/// Maps to PostgreSQL `REPEAT`.
pub fn repeat<T>(arg: impl IntoExpr<T>, count: impl IntoExpr<i32>) -> Expr<<T as StringUnaryExpr>::Output>
where
    T: StringUnaryExpr,
{
    string_fn("REPEAT", arg, vec![count.into_expr().node])
}

/// Pads on the left to `length` by cycling `fill`, truncating the source on the right if needed.
/// Padding `"ab"` to 5 with `"xy"` yields `"xyxab"`; empty fill adds nothing and non-positive length yields `""`.
/// Maps to PostgreSQL `LPAD`.
pub fn pad_start<T>(arg: impl IntoExpr<T>, length: impl IntoExpr<i32>, fill: impl IntoExpr<String>) -> Expr<<T as StringUnaryExpr>::Output>
where
    T: StringUnaryExpr,
{
    string_fn("LPAD", arg, vec![length.into_expr().node, fill.into_expr().node])
}

/// Pads on the right to `length` by cycling `fill`, truncating the source on the right if needed.
/// Padding `"ab"` to 5 with `"xy"` yields `"abxyx"`; empty fill adds nothing and non-positive length yields `""`.
/// Maps to PostgreSQL `RPAD`.
pub fn pad_end<T>(arg: impl IntoExpr<T>, length: impl IntoExpr<i32>, fill: impl IntoExpr<String>) -> Expr<<T as StringUnaryExpr>::Output>
where
    T: StringUnaryExpr,
{
    string_fn("RPAD", arg, vec![length.into_expr().node, fill.into_expr().node])
}

/// Returns the encoded byte length of a text expression, preserving input nullability.
/// Maps to PostgreSQL `OCTET_LENGTH`.
pub fn byte_length<T>(arg: impl IntoExpr<T>) -> Expr<<T as StringLengthExpr>::Output>
where
    T: StringLengthExpr,
{
    string_length_fn("OCTET_LENGTH", arg)
}

/// Returns eight times the encoded byte length, preserving input nullability.
/// Maps to PostgreSQL `BIT_LENGTH`.
pub fn bit_length<T>(arg: impl IntoExpr<T>) -> Expr<<T as StringLengthExpr>::Output>
where
    T: StringLengthExpr,
{
    string_length_fn("BIT_LENGTH", arg)
}

/// Returns the 1-based position of `substring` in `expression`, or zero when absent.
/// Returns NULL if either argument is NULL; `position("banana", "ana")` evaluates to `2`.
/// Maps to PostgreSQL `STRPOS`.
pub fn position<L, R>(expression: impl IntoExpr<L>, substring: impl IntoExpr<R>) -> Expr<<L as StringBinaryExpr<R, i32>>::Output>
where
    L: StringBinaryExpr<R, i32>,
{
    binary_string_fn("STRPOS", expression, substring)
}

/// Tests whether `expression` begins with the exact, case-sensitive `prefix`.
/// Returns NULL if either argument is NULL; `starts_with("PostgreSQL", "Post")` evaluates to `true`.
/// Maps to PostgreSQL `STARTS_WITH`.
pub fn starts_with<L, R>(expression: impl IntoExpr<L>, prefix: impl IntoExpr<R>) -> Expr<<L as StringBinaryExpr<R, bool>>::Output>
where
    L: StringBinaryExpr<R, bool>,
{
    binary_string_fn("STARTS_WITH", expression, prefix)
}

pub fn count<T>(arg: impl IntoExpr<T>) -> AggregateExpr<i64> {
    let expr = arg.into_expr();
    Expr::new(ExprNode::Func {
        name: "COUNT",
        args: vec![expr.node],
    })
}

pub fn sum<T>(arg: impl IntoExpr<T>) -> AggregateExpr<T> {
    let expr = arg.into_expr();
    Expr::new(ExprNode::Func {
        name: "SUM",
        args: vec![expr.node],
    })
}

pub trait NullableAggregateOutput {
    type Output;
}

macro_rules! impl_nullable_aggregate_output {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl NullableAggregateOutput for $ty {
                type Output = Option<$ty>;
            }

            impl NullableAggregateOutput for Option<$ty> {
                type Output = Option<$ty>;
            }
        )+
    };
}

impl_nullable_aggregate_output!(
    String,
    i16,
    i32,
    i64,
    f32,
    f64,
    uuid::Uuid,
    chrono::NaiveDateTime,
    chrono::DateTime<chrono::Utc>,
    chrono::NaiveDate,
    chrono::NaiveTime,
    crate::PgInterval,
);

pub fn min<T>(arg: impl IntoExpr<T>) -> AggregateExpr<<T as NullableAggregateOutput>::Output>
where
    T: NullableAggregateOutput,
{
    let expr = arg.into_expr();
    Expr::new(ExprNode::Func {
        name: "MIN",
        args: vec![expr.node],
    })
}

pub fn max<T>(arg: impl IntoExpr<T>) -> AggregateExpr<<T as NullableAggregateOutput>::Output>
where
    T: NullableAggregateOutput,
{
    let expr = arg.into_expr();
    Expr::new(ExprNode::Func {
        name: "MAX",
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

fn exists_expr(subquery: CompiledSql) -> Expr<bool> {
    Expr::new(ExprNode::Exists { subquery })
}

pub fn exists<Out, Loads, Lock, DistinctState, GroupState>(subquery: Select<Out, Loads, Lock, DistinctState, GroupState>) -> Expr<bool> {
    exists_expr(subquery.compile_for_exists())
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
