use std::marker::PhantomData;
use std::ops::{Add, BitAnd, BitOr, BitXor, Div, Mul, Not, Shl, Shr, Sub};

use crate::compile::CompiledSql;
use crate::func::{StringBinaryExpr, StringUnaryExpr};
use crate::schema::{Column, ColumnRef};
use crate::types::{PgInterval, PgVector};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    String(String),
    Array(Vec<String>),
    Bytes(Vec<u8>),
    Json(serde_json::Value),
    Uuid(uuid::Uuid),
    DateTime(chrono::NaiveDateTime),
    DateTimeUtc(chrono::DateTime<chrono::Utc>),
    Date(chrono::NaiveDate),
    Time(chrono::NaiveTime),
    Interval(PgInterval),
    Vector(Vec<f32>),
    Enum { type_name: &'static str, value: String },
}

pub trait ColumnValue<T> {
    fn into_value(self) -> Option<Value>;
}

impl<T> ColumnValue<T> for T
where
    T: Into<Value>,
{
    fn into_value(self) -> Option<Value> {
        Some(self.into())
    }
}

impl<T> ColumnValue<Option<T>> for Option<T>
where
    T: Into<Value>,
{
    fn into_value(self) -> Option<Value> {
        Some(self.map_or(Value::Null, Into::into))
    }
}

impl<T> ColumnValue<Option<T>> for &Option<T>
where
    T: Clone + Into<Value>,
{
    fn into_value(self) -> Option<Value> {
        Some(self.as_ref().map_or(Value::Null, |value| value.clone().into()))
    }
}

impl ColumnValue<String> for &str {
    fn into_value(self) -> Option<Value> {
        Some(Value::String(self.to_string()))
    }
}

impl<T> ColumnValue<T> for &T
where
    T: Clone + Into<Value>,
{
    fn into_value(self) -> Option<Value> {
        Some(self.clone().into())
    }
}

impl<T> ColumnValue<Option<T>> for T
where
    T: Into<Value>,
{
    fn into_value(self) -> Option<Value> {
        Some(self.into())
    }
}

impl<T> ColumnValue<Option<T>> for &T
where
    T: Clone + Into<Value>,
{
    fn into_value(self) -> Option<Value> {
        Some(self.clone().into())
    }
}

impl ColumnValue<Option<String>> for &str {
    fn into_value(self) -> Option<Value> {
        Some(Value::String(self.to_string()))
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i16> for Value {
    fn from(value: i16) -> Self {
        Self::I16(value)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::I32(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::I64(value)
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Self::F32(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::F64(value)
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<Vec<String>> for Value {
    fn from(value: Vec<String>) -> Self {
        Self::Array(value)
    }
}

impl From<Vec<u8>> for Value {
    fn from(value: Vec<u8>) -> Self {
        Self::Bytes(value)
    }
}

impl From<serde_json::Value> for Value {
    fn from(value: serde_json::Value) -> Self {
        Self::Json(value)
    }
}

impl From<uuid::Uuid> for Value {
    fn from(value: uuid::Uuid) -> Self {
        Self::Uuid(value)
    }
}

impl From<chrono::NaiveDateTime> for Value {
    fn from(value: chrono::NaiveDateTime) -> Self {
        Self::DateTime(value)
    }
}

impl From<chrono::DateTime<chrono::Utc>> for Value {
    fn from(value: chrono::DateTime<chrono::Utc>) -> Self {
        Self::DateTimeUtc(value)
    }
}

impl From<chrono::NaiveDate> for Value {
    fn from(value: chrono::NaiveDate) -> Self {
        Self::Date(value)
    }
}

impl From<chrono::NaiveTime> for Value {
    fn from(value: chrono::NaiveTime) -> Self {
        Self::Time(value)
    }
}

impl From<PgInterval> for Value {
    fn from(value: PgInterval) -> Self {
        Self::Interval(value)
    }
}

impl<const N: usize> From<PgVector<N>> for Value {
    fn from(value: PgVector<N>) -> Self {
        Self::Vector(value.to_vec())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Eq,
    Ne,
    IsDistinctFrom,
    IsNotDistinctFrom,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy)]
#[doc(hidden)]
pub enum CastType {
    Boolean,
    SmallInt,
    Integer,
    BigInt,
    Real,
    DoublePrecision,
    Text,
    Uuid,
    Timestamp,
    TimestampTz,
    Date,
    Time,
    Interval,
}

#[derive(Debug, Clone, Copy)]
pub enum BoolOp {
    And,
    Or,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Not,
    BitNot,
}

#[derive(Debug, Clone, Copy)]
pub enum VectorBinaryOp {
    L2Distance,
    CosineDistance,
    InnerProductDistance,
    L1Distance,
}

#[derive(Debug, Clone, Copy)]
pub enum IntervalField {
    Days,
    Hours,
    Minutes,
    Seconds,
}

#[derive(Debug, Clone, Copy)]
pub enum TrimDirection {
    Both,
    Leading,
    Trailing,
}

#[derive(Debug, Clone)]
pub enum ExprNode {
    Column(ColumnRef),
    Value(Value),
    Row {
        values: Vec<ExprNode>,
    },
    Func {
        name: &'static str,
        args: Vec<ExprNode>,
    },
    Normalize {
        expr: Box<ExprNode>,
        form: crate::func::NormalizationForm,
    },
    Trim {
        direction: TrimDirection,
        expr: Box<ExprNode>,
        characters: Option<Box<ExprNode>>,
    },
    AggregateFilter {
        aggregate: Box<ExprNode>,
        predicate: Box<ExprNode>,
    },
    VectorBinary {
        left: Box<ExprNode>,
        op: VectorBinaryOp,
        right: Box<ExprNode>,
    },
    MakeInterval {
        field: IntervalField,
        value: Box<ExprNode>,
    },
    Cast {
        expr: Box<ExprNode>,
        target: CastType,
    },
    Binary {
        left: Box<ExprNode>,
        op: BinaryOp,
        right: Box<ExprNode>,
    },
    Bool {
        left: Box<ExprNode>,
        op: BoolOp,
        right: Box<ExprNode>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<ExprNode>,
    },
    In {
        expr: Box<ExprNode>,
        values: Vec<Value>,
    },
    RowIn {
        expr: Box<ExprNode>,
        rows: Vec<Vec<Value>>,
    },
    IsNull {
        expr: Box<ExprNode>,
        negated: bool,
    },
    Like {
        expr: Box<ExprNode>,
        pattern: Value,
        case_insensitive: bool,
    },
    Exists {
        subquery: CompiledSql,
    },
}

#[derive(Debug, Clone)]
#[doc(hidden)]
pub struct ScalarExpression;

#[derive(Debug, Clone)]
#[doc(hidden)]
pub struct AggregateExpression;

#[derive(Debug, Clone)]
pub struct Expr<T, Kind = ScalarExpression> {
    pub node: ExprNode,
    _marker: PhantomData<(T, Kind)>,
}

pub type AggregateExpr<T> = Expr<T, AggregateExpression>;

#[derive(Debug, Clone)]
pub struct RowExpr<T> {
    node: ExprNode,
    _marker: PhantomData<T>,
}

impl<T, Kind> Expr<T, Kind> {
    pub fn new(node: ExprNode) -> Self {
        Self {
            node,
            _marker: PhantomData,
        }
    }
}

impl<T> AggregateExpr<T> {
    /// Applies a PostgreSQL aggregate `FILTER (WHERE ...)` clause.
    pub fn filter<B>(self, predicate: Expr<B>) -> Expr<T>
    where
        B: BooleanExprType,
    {
        Expr::new(ExprNode::AggregateFilter {
            aggregate: Box::new(self.node),
            predicate: Box::new(predicate.node),
        })
    }
}

impl<T, Kind> Expr<Option<T>, Kind> {
    /// Replaces NULL with a required fallback using SQL `COALESCE`.
    pub fn unwrap_or(self, fallback: impl IntoExpr<T>) -> Expr<T> {
        crate::func::coalesce(self, fallback)
    }

    /// Replaces NULL with Rust's `T::default()`, evaluated when building the query.
    pub fn unwrap_or_default(self) -> Expr<T>
    where
        T: Default + IntoExpr<T>,
    {
        self.unwrap_or(T::default())
    }
}

impl<M, T> Column<M, Option<T>> {
    /// Replaces NULL with a required fallback using SQL `COALESCE`.
    pub fn unwrap_or(self, fallback: impl IntoExpr<T>) -> Expr<T> {
        self.into_expr().unwrap_or(fallback)
    }

    /// Replaces NULL with Rust's `T::default()`, evaluated when building the query.
    pub fn unwrap_or_default(self) -> Expr<T>
    where
        T: Default + IntoExpr<T>,
    {
        self.into_expr().unwrap_or_default()
    }
}

impl<T, Kind> Expr<T, Kind>
where
    T: StringUnaryExpr,
{
    /// Removes leading and trailing spaces using SQL `TRIM`, preserving nullability.
    pub fn trim(self) -> Expr<T::Output> {
        crate::func::trim(self)
    }

    /// Converts text to lowercase using SQL `LOWER`, preserving nullability.
    pub fn lower(self) -> Expr<T::Output> {
        crate::func::lower(self)
    }

    /// Tests a literal, case-sensitive prefix. NULL in either operand produces NULL.
    pub fn starts_with<R>(self, prefix: impl IntoExpr<R>) -> Expr<<T as StringBinaryExpr<R, bool>>::Output>
    where
        T: StringBinaryExpr<R, bool>,
    {
        crate::func::starts_with(self, prefix)
    }
}

impl<M, T> Column<M, T>
where
    T: StringUnaryExpr,
{
    /// Removes leading and trailing spaces using SQL `TRIM`, preserving nullability.
    pub fn trim(self) -> Expr<T::Output> {
        self.into_expr().trim()
    }

    /// Converts text to lowercase using SQL `LOWER`, preserving nullability.
    pub fn lower(self) -> Expr<T::Output> {
        self.into_expr().lower()
    }

    /// Tests a literal, case-sensitive prefix. NULL in either operand produces NULL.
    pub fn starts_with<R>(self, prefix: impl IntoExpr<R>) -> Expr<<T as StringBinaryExpr<R, bool>>::Output>
    where
        T: StringBinaryExpr<R, bool>,
    {
        self.into_expr().starts_with(prefix)
    }
}

impl<T> RowExpr<T> {
    pub fn new(node: ExprNode) -> Self {
        Self {
            node,
            _marker: PhantomData,
        }
    }
}

pub trait IntoExpr<T> {
    fn into_expr(self) -> Expr<T>;
}

pub trait ExprOperand {
    type Value;

    fn into_operand_expr(self) -> Expr<Self::Value>;
}

#[doc(hidden)]
pub trait BitwiseOperand {
    type Value;

    fn into_bitwise_expr(self) -> Expr<Self::Value>;
}

#[doc(hidden)]
pub trait SqlInteger: Into<Value> + Into<i64> {}

impl<T> SqlInteger for T where T: Into<Value> + Into<i64> {}

#[doc(hidden)]
pub struct ValueComparisonMarker;

#[doc(hidden)]
pub struct ExprComparisonMarker;

#[doc(hidden)]
pub struct OptionalValueComparisonMarker;

#[doc(hidden)]
pub struct NullableExprComparisonMarker;

pub trait ComparisonValue<T, Marker = ValueComparisonMarker> {
    type Output;

    fn into_comparison_expr(self) -> Expr<T>;
}

#[doc(hidden)]
pub trait CompatibleColumn<Rhs> {
    type Output;
}

#[doc(hidden)]
pub trait ValueComparisonOutput {
    type Output;
}

#[doc(hidden)]
pub trait RowValueComparisonOutput {
    type Output: BooleanExprType;
}

impl<T> ValueComparisonOutput for T
where
    T: Into<Value>,
{
    type Output = bool;
}

impl<T> ValueComparisonOutput for Option<T>
where
    T: Into<Value>,
{
    type Output = Option<bool>;
}

impl<T> CompatibleColumn<T> for T
where
    T: Into<Value>,
{
    type Output = bool;
}

impl<T> CompatibleColumn<Option<T>> for T
where
    T: Into<Value>,
{
    type Output = Option<bool>;
}

impl<T> CompatibleColumn<T> for Option<T>
where
    T: Into<Value>,
{
    type Output = Option<bool>;
}

impl<T> CompatibleColumn<Option<T>> for Option<T>
where
    T: Into<Value>,
{
    type Output = Option<bool>;
}

#[doc(hidden)]
pub trait BooleanExprType {}

impl BooleanExprType for bool {}
impl BooleanExprType for Option<bool> {}

#[doc(hidden)]
pub trait BooleanOutput<Rhs>: BooleanExprType {
    type Output: BooleanExprType;
}

impl BooleanOutput<bool> for bool {
    type Output = bool;
}

impl BooleanOutput<Option<bool>> for bool {
    type Output = Option<bool>;
}

impl BooleanOutput<bool> for Option<bool> {
    type Output = Option<bool>;
}

impl BooleanOutput<Option<bool>> for Option<bool> {
    type Output = Option<bool>;
}

macro_rules! impl_row_value_comparison_output {
    ($first:ident, $($rest:ident),+) => {
        impl<$first, $($rest),+> RowValueComparisonOutput for ($first, $($rest,)+)
        where
            $first: ValueComparisonOutput,
            ($($rest,)+): RowValueComparisonOutput,
            <$first as ValueComparisonOutput>::Output:
                BooleanOutput<<($($rest,)+) as RowValueComparisonOutput>::Output>,
        {
            type Output = <<$first as ValueComparisonOutput>::Output as BooleanOutput<
                <($($rest,)+) as RowValueComparisonOutput>::Output,
            >>::Output;
        }

        impl_row_value_comparison_output!($($rest),+);
    };
    ($last:ident) => {
        impl<$last> RowValueComparisonOutput for ($last,)
        where
            $last: ValueComparisonOutput,
            <$last as ValueComparisonOutput>::Output: BooleanExprType,
        {
            type Output = <$last as ValueComparisonOutput>::Output;
        }
    };
}

impl_row_value_comparison_output!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16);

pub(crate) fn into_predicate<T>(expr: Expr<T>) -> Expr<bool>
where
    T: BooleanExprType,
{
    Expr::new(expr.node)
}

pub trait SqlAdd<Rhs> {
    type Output;
}

pub trait SqlSub<Rhs> {
    type Output;
}

pub trait SqlMul<Rhs> {
    type Output;
}

pub trait SqlDiv<Rhs> {
    type Output;
}

#[doc(hidden)]
pub trait SqlCast<Target> {}

#[doc(hidden)]
pub trait SqlCastTarget {
    const CAST_TYPE: CastType;
}

#[doc(hidden)]
pub trait SqlBitwise<Rhs> {
    type Output;
}

#[doc(hidden)]
pub trait SqlShift<Rhs> {
    type Output;
}

pub trait NumericExprType {}

mod row_columns_private {
    pub trait Sealed {}
}

pub trait RowColumns: row_columns_private::Sealed {
    type ValueTuple;

    fn into_row_expr(self) -> RowExpr<Self::ValueTuple>;
}

pub fn row<R>(columns: R) -> RowExpr<R::ValueTuple>
where
    R: RowColumns,
{
    columns.into_row_expr()
}

impl<T, Kind> IntoExpr<T> for Expr<T, Kind> {
    fn into_expr(self) -> Expr<T> {
        Expr::new(self.node)
    }
}

impl<T, Kind> ExprOperand for Expr<T, Kind> {
    type Value = T;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl<T, Kind> BitwiseOperand for Expr<T, Kind> {
    type Value = T;

    fn into_bitwise_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl<T, Kind> ComparisonValue<T, ExprComparisonMarker> for Expr<T, Kind>
where
    T: ValueComparisonOutput,
{
    type Output = <T as ValueComparisonOutput>::Output;

    fn into_comparison_expr(self) -> Expr<T> {
        self.into_expr()
    }
}

impl<T, Kind> ComparisonValue<Option<T>, ExprComparisonMarker> for Expr<T, Kind> {
    type Output = Option<bool>;

    fn into_comparison_expr(self) -> Expr<Option<T>> {
        Expr::new(self.node)
    }
}

impl<T, Kind> ComparisonValue<T, NullableExprComparisonMarker> for Expr<Option<T>, Kind> {
    type Output = Option<bool>;

    fn into_comparison_expr(self) -> Expr<T> {
        Expr::new(self.node)
    }
}

impl<M, T> IntoExpr<T> for Column<M, T> {
    fn into_expr(self) -> Expr<T> {
        Expr::new(ExprNode::Column(self.as_ref()))
    }
}

impl<M, T> ExprOperand for Column<M, T> {
    type Value = T;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl<M, T> BitwiseOperand for Column<M, T> {
    type Value = T;

    fn into_bitwise_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl<T> BitwiseOperand for T
where
    T: SqlInteger,
{
    type Value = T;

    fn into_bitwise_expr(self) -> Expr<Self::Value> {
        Expr::new(ExprNode::Value(self.into()))
    }
}

impl<M, T> ComparisonValue<T, ExprComparisonMarker> for Column<M, T>
where
    T: ValueComparisonOutput,
{
    type Output = <T as ValueComparisonOutput>::Output;

    fn into_comparison_expr(self) -> Expr<T> {
        self.into_expr()
    }
}

impl<M, T> ComparisonValue<Option<T>, ExprComparisonMarker> for Column<M, T> {
    type Output = Option<bool>;

    fn into_comparison_expr(self) -> Expr<Option<T>> {
        Expr::new(ExprNode::Column(self.as_ref()))
    }
}

impl<M, T> ComparisonValue<T, NullableExprComparisonMarker> for Column<M, Option<T>> {
    type Output = Option<bool>;

    fn into_comparison_expr(self) -> Expr<T> {
        Expr::new(ExprNode::Column(self.as_ref()))
    }
}

macro_rules! impl_row_tuple_support {
    ($(($($model:ident:$col_ty:ident:$col_ident:ident:$value_ty:ident:$value_ident:ident),+)),+ $(,)?) => {
        $(
            impl<$($model, $col_ty),+> RowColumns for ($(Column<$model, $col_ty>,)+) {
                type ValueTuple = ($($col_ty,)+);

                fn into_row_expr(self) -> RowExpr<Self::ValueTuple> {
                    let ($($col_ident,)+) = self;
                    RowExpr::new(ExprNode::Row {
                        values: vec![$(ExprNode::Column($col_ident.as_ref())),+],
                    })
                }
            }

            impl<$($model, $col_ty),+> row_columns_private::Sealed for ($(Column<$model, $col_ty>,)+) {}

            impl<$($col_ty),+> RowExpr<($($col_ty,)+)>
            where
                ($($col_ty,)+): RowValueComparisonOutput,
            {
                pub fn in_<I, $($value_ty),+>(self, values: I) -> Expr<<($($col_ty,)+) as RowValueComparisonOutput>::Output>
                where
                    I: IntoIterator<Item = ($($value_ty,)+)>,
                    $($value_ty: ColumnValue<$col_ty>,)+
                {
                    let rows = values
                        .into_iter()
                        .map(|($($value_ident,)+)| vec![$($value_ident.into_value().unwrap_or(Value::Null)),+])
                        .collect();

                    Expr::new(ExprNode::RowIn {
                        expr: Box::new(self.node),
                        rows,
                    })
                }
            }
        )+
    };
}

impl_row_tuple_support!(
    (M1:T1:c1:V1:v1, M2:T2:c2:V2:v2),
    (M1:T1:c1:V1:v1, M2:T2:c2:V2:v2, M3:T3:c3:V3:v3),
    (M1:T1:c1:V1:v1, M2:T2:c2:V2:v2, M3:T3:c3:V3:v3, M4:T4:c4:V4:v4),
    (M1:T1:c1:V1:v1, M2:T2:c2:V2:v2, M3:T3:c3:V3:v3, M4:T4:c4:V4:v4, M5:T5:c5:V5:v5),
    (M1:T1:c1:V1:v1, M2:T2:c2:V2:v2, M3:T3:c3:V3:v3, M4:T4:c4:V4:v4, M5:T5:c5:V5:v5, M6:T6:c6:V6:v6),
    (M1:T1:c1:V1:v1, M2:T2:c2:V2:v2, M3:T3:c3:V3:v3, M4:T4:c4:V4:v4, M5:T5:c5:V5:v5, M6:T6:c6:V6:v6, M7:T7:c7:V7:v7),
    (M1:T1:c1:V1:v1, M2:T2:c2:V2:v2, M3:T3:c3:V3:v3, M4:T4:c4:V4:v4, M5:T5:c5:V5:v5, M6:T6:c6:V6:v6, M7:T7:c7:V7:v7, M8:T8:c8:V8:v8),
    (M1:T1:c1:V1:v1, M2:T2:c2:V2:v2, M3:T3:c3:V3:v3, M4:T4:c4:V4:v4, M5:T5:c5:V5:v5, M6:T6:c6:V6:v6, M7:T7:c7:V7:v7, M8:T8:c8:V8:v8, M9:T9:c9:V9:v9),
    (M1:T1:c1:V1:v1, M2:T2:c2:V2:v2, M3:T3:c3:V3:v3, M4:T4:c4:V4:v4, M5:T5:c5:V5:v5, M6:T6:c6:V6:v6, M7:T7:c7:V7:v7, M8:T8:c8:V8:v8, M9:T9:c9:V9:v9, M10:T10:c10:V10:v10),
    (M1:T1:c1:V1:v1, M2:T2:c2:V2:v2, M3:T3:c3:V3:v3, M4:T4:c4:V4:v4, M5:T5:c5:V5:v5, M6:T6:c6:V6:v6, M7:T7:c7:V7:v7, M8:T8:c8:V8:v8, M9:T9:c9:V9:v9, M10:T10:c10:V10:v10, M11:T11:c11:V11:v11),
    (M1:T1:c1:V1:v1, M2:T2:c2:V2:v2, M3:T3:c3:V3:v3, M4:T4:c4:V4:v4, M5:T5:c5:V5:v5, M6:T6:c6:V6:v6, M7:T7:c7:V7:v7, M8:T8:c8:V8:v8, M9:T9:c9:V9:v9, M10:T10:c10:V10:v10, M11:T11:c11:V11:v11, M12:T12:c12:V12:v12),
    (M1:T1:c1:V1:v1, M2:T2:c2:V2:v2, M3:T3:c3:V3:v3, M4:T4:c4:V4:v4, M5:T5:c5:V5:v5, M6:T6:c6:V6:v6, M7:T7:c7:V7:v7, M8:T8:c8:V8:v8, M9:T9:c9:V9:v9, M10:T10:c10:V10:v10, M11:T11:c11:V11:v11, M12:T12:c12:V12:v12, M13:T13:c13:V13:v13),
    (M1:T1:c1:V1:v1, M2:T2:c2:V2:v2, M3:T3:c3:V3:v3, M4:T4:c4:V4:v4, M5:T5:c5:V5:v5, M6:T6:c6:V6:v6, M7:T7:c7:V7:v7, M8:T8:c8:V8:v8, M9:T9:c9:V9:v9, M10:T10:c10:V10:v10, M11:T11:c11:V11:v11, M12:T12:c12:V12:v12, M13:T13:c13:V13:v13, M14:T14:c14:V14:v14),
    (M1:T1:c1:V1:v1, M2:T2:c2:V2:v2, M3:T3:c3:V3:v3, M4:T4:c4:V4:v4, M5:T5:c5:V5:v5, M6:T6:c6:V6:v6, M7:T7:c7:V7:v7, M8:T8:c8:V8:v8, M9:T9:c9:V9:v9, M10:T10:c10:V10:v10, M11:T11:c11:V11:v11, M12:T12:c12:V12:v12, M13:T13:c13:V13:v13, M14:T14:c14:V14:v14, M15:T15:c15:V15:v15),
    (M1:T1:c1:V1:v1, M2:T2:c2:V2:v2, M3:T3:c3:V3:v3, M4:T4:c4:V4:v4, M5:T5:c5:V5:v5, M6:T6:c6:V6:v6, M7:T7:c7:V7:v7, M8:T8:c8:V8:v8, M9:T9:c9:V9:v9, M10:T10:c10:V10:v10, M11:T11:c11:V11:v11, M12:T12:c12:V12:v12, M13:T13:c13:V13:v13, M14:T14:c14:V14:v14, M15:T15:c15:V15:v15, M16:T16:c16:V16:v16)
);

impl<T, V> ComparisonValue<T, ValueComparisonMarker> for V
where
    T: ValueComparisonOutput,
    V: Into<Value>,
{
    type Output = <T as ValueComparisonOutput>::Output;

    fn into_comparison_expr(self) -> Expr<T> {
        Expr::new(ExprNode::Value(self.into()))
    }
}

impl<T> ComparisonValue<Option<T>, OptionalValueComparisonMarker> for Option<T>
where
    T: Into<Value>,
{
    type Output = Option<bool>;

    fn into_comparison_expr(self) -> Expr<Option<T>> {
        Expr::new(ExprNode::Value(self.map_or(Value::Null, Into::into)))
    }
}

impl<T> ComparisonValue<Option<T>, OptionalValueComparisonMarker> for &Option<T>
where
    T: Clone + Into<Value>,
{
    type Output = Option<bool>;

    fn into_comparison_expr(self) -> Expr<Option<T>> {
        Expr::new(ExprNode::Value(self.as_ref().map_or(Value::Null, |value| value.clone().into())))
    }
}

impl IntoExpr<String> for String {
    fn into_expr(self) -> Expr<String> {
        Expr::new(ExprNode::Value(Value::String(self)))
    }
}

impl ExprOperand for String {
    type Value = String;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl IntoExpr<String> for &str {
    fn into_expr(self) -> Expr<String> {
        Expr::new(ExprNode::Value(Value::String(self.to_string())))
    }
}

impl ExprOperand for &str {
    type Value = String;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl IntoExpr<bool> for bool {
    fn into_expr(self) -> Expr<bool> {
        Expr::new(ExprNode::Value(Value::Bool(self)))
    }
}

impl ExprOperand for bool {
    type Value = bool;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl IntoExpr<i16> for i16 {
    fn into_expr(self) -> Expr<i16> {
        Expr::new(ExprNode::Value(Value::I16(self)))
    }
}

impl ExprOperand for i16 {
    type Value = i16;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl NumericExprType for i16 {}

impl IntoExpr<i32> for i32 {
    fn into_expr(self) -> Expr<i32> {
        Expr::new(ExprNode::Value(Value::I32(self)))
    }
}

impl ExprOperand for i32 {
    type Value = i32;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl NumericExprType for i32 {}

impl IntoExpr<i64> for i64 {
    fn into_expr(self) -> Expr<i64> {
        Expr::new(ExprNode::Value(Value::I64(self)))
    }
}

impl ExprOperand for i64 {
    type Value = i64;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl NumericExprType for i64 {}

impl IntoExpr<f32> for f32 {
    fn into_expr(self) -> Expr<f32> {
        Expr::new(ExprNode::Value(Value::F32(self)))
    }
}

impl ExprOperand for f32 {
    type Value = f32;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl NumericExprType for f32 {}

impl IntoExpr<f64> for f64 {
    fn into_expr(self) -> Expr<f64> {
        Expr::new(ExprNode::Value(Value::F64(self)))
    }
}

impl ExprOperand for f64 {
    type Value = f64;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl NumericExprType for f64 {}

impl IntoExpr<uuid::Uuid> for uuid::Uuid {
    fn into_expr(self) -> Expr<uuid::Uuid> {
        Expr::new(ExprNode::Value(Value::Uuid(self)))
    }
}

impl ExprOperand for uuid::Uuid {
    type Value = uuid::Uuid;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl IntoExpr<chrono::NaiveDateTime> for chrono::NaiveDateTime {
    fn into_expr(self) -> Expr<chrono::NaiveDateTime> {
        Expr::new(ExprNode::Value(Value::DateTime(self)))
    }
}

impl ExprOperand for chrono::NaiveDateTime {
    type Value = chrono::NaiveDateTime;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl IntoExpr<chrono::DateTime<chrono::Utc>> for chrono::DateTime<chrono::Utc> {
    fn into_expr(self) -> Expr<chrono::DateTime<chrono::Utc>> {
        Expr::new(ExprNode::Value(Value::DateTimeUtc(self)))
    }
}

impl ExprOperand for chrono::DateTime<chrono::Utc> {
    type Value = chrono::DateTime<chrono::Utc>;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl IntoExpr<chrono::NaiveDate> for chrono::NaiveDate {
    fn into_expr(self) -> Expr<chrono::NaiveDate> {
        Expr::new(ExprNode::Value(Value::Date(self)))
    }
}

impl ExprOperand for chrono::NaiveDate {
    type Value = chrono::NaiveDate;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl IntoExpr<chrono::NaiveTime> for chrono::NaiveTime {
    fn into_expr(self) -> Expr<chrono::NaiveTime> {
        Expr::new(ExprNode::Value(Value::Time(self)))
    }
}

impl ExprOperand for chrono::NaiveTime {
    type Value = chrono::NaiveTime;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl IntoExpr<PgInterval> for PgInterval {
    fn into_expr(self) -> Expr<PgInterval> {
        Expr::new(ExprNode::Value(Value::Interval(self)))
    }
}

impl ExprOperand for PgInterval {
    type Value = PgInterval;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl IntoExpr<Vec<String>> for Vec<String> {
    fn into_expr(self) -> Expr<Vec<String>> {
        Expr::new(ExprNode::Value(Value::Array(self)))
    }
}

impl ExprOperand for Vec<String> {
    type Value = Vec<String>;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl IntoExpr<Vec<u8>> for Vec<u8> {
    fn into_expr(self) -> Expr<Vec<u8>> {
        Expr::new(ExprNode::Value(Value::Bytes(self)))
    }
}

impl ExprOperand for Vec<u8> {
    type Value = Vec<u8>;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl IntoExpr<serde_json::Value> for serde_json::Value {
    fn into_expr(self) -> Expr<serde_json::Value> {
        Expr::new(ExprNode::Value(Value::Json(self)))
    }
}

impl ExprOperand for serde_json::Value {
    type Value = serde_json::Value;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

impl<const N: usize> IntoExpr<PgVector<N>> for PgVector<N> {
    fn into_expr(self) -> Expr<PgVector<N>> {
        Expr::new(ExprNode::Value(Value::from(self)))
    }
}

impl<const N: usize> ExprOperand for PgVector<N> {
    type Value = PgVector<N>;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self.into_expr()
    }
}

macro_rules! impl_nullable_binary_output {
    ($trait:ident, $lhs:ty, $rhs:ty, $output:ty) => {
        impl $trait<$rhs> for Option<$lhs> {
            type Output = Option<$output>;
        }

        impl $trait<Option<$rhs>> for $lhs {
            type Output = Option<$output>;
        }

        impl $trait<Option<$rhs>> for Option<$lhs> {
            type Output = Option<$output>;
        }
    };
}

macro_rules! impl_numeric_arithmetic {
    ($(($ty:ty, $output:ty)),* $(,)?) => {
        $(
            impl SqlAdd<$ty> for $ty {
                type Output = $output;
            }

            impl SqlSub<$ty> for $ty {
                type Output = $output;
            }

            impl SqlMul<$ty> for $ty {
                type Output = $output;
            }

            impl_nullable_binary_output!(SqlAdd, $ty, $ty, $output);
            impl_nullable_binary_output!(SqlSub, $ty, $ty, $output);
            impl_nullable_binary_output!(SqlMul, $ty, $ty, $output);
        )*
    };
}

impl_numeric_arithmetic!((i16, i32), (i32, i32), (i64, i64), (f32, f32), (f64, f64));

macro_rules! impl_numeric_division {
    ($($lhs:ty, $rhs:ty => $output:ty);* $(;)?) => {
        $(
            impl SqlDiv<$rhs> for $lhs {
                type Output = $output;
            }

            impl_nullable_binary_output!(SqlDiv, $lhs, $rhs, $output);
        )*
    };
}

impl_numeric_division! {
    i16, i16 => i16;
    i16, i32 => i32;
    i16, i64 => i64;
    i16, f32 => f64;
    i16, f64 => f64;
    i32, i16 => i32;
    i32, i32 => i32;
    i32, i64 => i64;
    i32, f32 => f64;
    i32, f64 => f64;
    i64, i16 => i64;
    i64, i32 => i64;
    i64, i64 => i64;
    i64, f32 => f64;
    i64, f64 => f64;
    f32, i16 => f64;
    f32, i32 => f64;
    f32, i64 => f64;
    f32, f32 => f32;
    f32, f64 => f64;
    f64, i16 => f64;
    f64, i32 => f64;
    f64, i64 => f64;
    f64, f32 => f64;
    f64, f64 => f64;
}

macro_rules! impl_cast_target {
    ($target:ty, $cast_type:ident) => {
        impl SqlCastTarget for $target {
            const CAST_TYPE: CastType = CastType::$cast_type;
        }
    };
}

impl_cast_target!(bool, Boolean);
impl_cast_target!(i16, SmallInt);
impl_cast_target!(i32, Integer);
impl_cast_target!(i64, BigInt);
impl_cast_target!(f32, Real);
impl_cast_target!(f64, DoublePrecision);
impl_cast_target!(String, Text);
impl_cast_target!(uuid::Uuid, Uuid);
impl_cast_target!(chrono::NaiveDateTime, Timestamp);
impl_cast_target!(chrono::DateTime<chrono::Utc>, TimestampTz);
impl_cast_target!(chrono::NaiveDate, Date);
impl_cast_target!(chrono::NaiveTime, Time);
impl_cast_target!(PgInterval, Interval);

macro_rules! impl_sql_casts {
    ($source:ty => $($target:ty),+ $(,)?) => {
        $(
            impl SqlCast<$target> for $source {}
        )+
    };
}

impl_sql_casts!(i16 => i16, i32, i64, f32, f64, String);
impl_sql_casts!(i32 => bool, i16, i32, i64, f32, f64, String);
impl_sql_casts!(i64 => i16, i32, i64, f32, f64, String);
impl_sql_casts!(f32 => i16, i32, i64, f32, f64, String);
impl_sql_casts!(f64 => i16, i32, i64, f32, f64, String);
impl_sql_casts!(bool => i32, String);
impl_sql_casts!(
    String => bool,
    i16,
    i32,
    i64,
    f32,
    f64,
    String,
    uuid::Uuid,
    chrono::NaiveDateTime,
    chrono::DateTime<chrono::Utc>,
    chrono::NaiveDate,
    chrono::NaiveTime,
    PgInterval
);
impl_sql_casts!(uuid::Uuid => String);
impl_sql_casts!(chrono::NaiveDate => String, chrono::NaiveDateTime, chrono::DateTime<chrono::Utc>);
impl_sql_casts!(chrono::NaiveTime => String, PgInterval);
impl_sql_casts!(
    chrono::NaiveDateTime => String,
    chrono::NaiveDate,
    chrono::NaiveTime,
    chrono::DateTime<chrono::Utc>
);
impl_sql_casts!(
    chrono::DateTime<chrono::Utc> => String,
    chrono::NaiveDate,
    chrono::NaiveTime,
    chrono::NaiveDateTime
);
impl_sql_casts!(PgInterval => String, chrono::NaiveTime);

fn cast_expr<Output, Kind>(node: ExprNode, target: CastType) -> Expr<Output, Kind> {
    Expr::new(ExprNode::Cast {
        expr: Box::new(node),
        target,
    })
}

macro_rules! impl_cast_method {
    ($($source:ty),+ $(,)?) => {
        $(
            impl<Kind> Expr<$source, Kind> {
                pub fn cast<Target>(self) -> Expr<Target>
                where
                    $source: SqlCast<Target>,
                    Target: SqlCastTarget,
                {
                    cast_expr(self.node, Target::CAST_TYPE)
                }
            }

            impl<Kind> Expr<Option<$source>, Kind> {
                pub fn cast<Target>(self) -> Expr<Option<Target>>
                where
                    $source: SqlCast<Target>,
                    Target: SqlCastTarget,
                {
                    cast_expr(self.node, Target::CAST_TYPE)
                }
            }

            impl<M> Column<M, $source> {
                pub fn cast<Target>(self) -> Expr<Target>
                where
                    $source: SqlCast<Target>,
                    Target: SqlCastTarget,
                {
                    cast_expr(ExprNode::Column(self.as_ref()), Target::CAST_TYPE)
                }
            }

            impl<M> Column<M, Option<$source>> {
                pub fn cast<Target>(self) -> Expr<Option<Target>>
                where
                    $source: SqlCast<Target>,
                    Target: SqlCastTarget,
                {
                    cast_expr(ExprNode::Column(self.as_ref()), Target::CAST_TYPE)
                }
            }
        )+
    };
}

impl_cast_method!(
    bool,
    i16,
    i32,
    i64,
    f32,
    f64,
    String,
    uuid::Uuid,
    chrono::NaiveDateTime,
    chrono::DateTime<chrono::Utc>,
    chrono::NaiveDate,
    chrono::NaiveTime,
    PgInterval,
);

impl<T> SqlBitwise<T> for T
where
    T: SqlInteger,
{
    type Output = T;
}

impl<T> SqlBitwise<T> for Option<T>
where
    T: SqlInteger,
{
    type Output = Option<T>;
}

impl<T> SqlBitwise<Option<T>> for T
where
    T: SqlInteger,
{
    type Output = Option<T>;
}

impl<T> SqlBitwise<Option<T>> for Option<T>
where
    T: SqlInteger,
{
    type Output = Option<T>;
}

macro_rules! impl_mixed_bitwise_output {
    ($(($lhs:ty, $rhs:ty, $output:ty)),* $(,)?) => {
        $(
            impl SqlBitwise<$rhs> for $lhs {
                type Output = $output;
            }

            impl_nullable_binary_output!(SqlBitwise, $lhs, $rhs, $output);
        )*
    };
}

impl_mixed_bitwise_output!(
    (i16, i32, i32),
    (i16, i64, i64),
    (i32, i16, i32),
    (i32, i64, i64),
    (i64, i16, i64),
    (i64, i32, i64),
);

macro_rules! impl_shift_output {
    ($rhs:ty) => {
        impl<T> SqlShift<$rhs> for T
        where
            T: SqlInteger,
        {
            type Output = T;
        }

        impl<T> SqlShift<$rhs> for Option<T>
        where
            T: SqlInteger,
        {
            type Output = Option<T>;
        }

        impl<T> SqlShift<Option<$rhs>> for T
        where
            T: SqlInteger,
        {
            type Output = Option<T>;
        }

        impl<T> SqlShift<Option<$rhs>> for Option<T>
        where
            T: SqlInteger,
        {
            type Output = Option<T>;
        }
    };
}

impl_shift_output!(i16);
impl_shift_output!(i32);

impl SqlAdd<PgInterval> for chrono::NaiveDateTime {
    type Output = chrono::NaiveDateTime;
}

impl SqlSub<PgInterval> for chrono::NaiveDateTime {
    type Output = chrono::NaiveDateTime;
}

impl SqlAdd<PgInterval> for chrono::DateTime<chrono::Utc> {
    type Output = chrono::DateTime<chrono::Utc>;
}

impl SqlSub<PgInterval> for chrono::DateTime<chrono::Utc> {
    type Output = chrono::DateTime<chrono::Utc>;
}

impl_nullable_binary_output!(SqlAdd, chrono::NaiveDateTime, PgInterval, chrono::NaiveDateTime);
impl_nullable_binary_output!(SqlSub, chrono::NaiveDateTime, PgInterval, chrono::NaiveDateTime);
impl_nullable_binary_output!(SqlAdd, chrono::DateTime<chrono::Utc>, PgInterval, chrono::DateTime<chrono::Utc>);
impl_nullable_binary_output!(SqlSub, chrono::DateTime<chrono::Utc>, PgInterval, chrono::DateTime<chrono::Utc>);

impl<Rhs, Kind> Add<Expr<Rhs, Kind>> for chrono::NaiveDateTime
where
    chrono::NaiveDateTime: SqlAdd<Rhs>,
{
    type Output = Expr<<chrono::NaiveDateTime as SqlAdd<Rhs>>::Output>;

    fn add(self, rhs: Expr<Rhs, Kind>) -> Self::Output {
        binary_expr(self.into_expr().node, BinaryOp::Add, rhs.node)
    }
}

impl<Rhs, Kind> Sub<Expr<Rhs, Kind>> for chrono::NaiveDateTime
where
    chrono::NaiveDateTime: SqlSub<Rhs>,
{
    type Output = Expr<<chrono::NaiveDateTime as SqlSub<Rhs>>::Output>;

    fn sub(self, rhs: Expr<Rhs, Kind>) -> Self::Output {
        binary_expr(self.into_expr().node, BinaryOp::Sub, rhs.node)
    }
}

impl<Rhs, Kind> Add<Expr<Rhs, Kind>> for chrono::DateTime<chrono::Utc>
where
    chrono::DateTime<chrono::Utc>: SqlAdd<Rhs>,
{
    type Output = Expr<<chrono::DateTime<chrono::Utc> as SqlAdd<Rhs>>::Output>;

    fn add(self, rhs: Expr<Rhs, Kind>) -> Self::Output {
        binary_expr(self.into_expr().node, BinaryOp::Add, rhs.node)
    }
}

impl<Rhs, Kind> Sub<Expr<Rhs, Kind>> for chrono::DateTime<chrono::Utc>
where
    chrono::DateTime<chrono::Utc>: SqlSub<Rhs>,
{
    type Output = Expr<<chrono::DateTime<chrono::Utc> as SqlSub<Rhs>>::Output>;

    fn sub(self, rhs: Expr<Rhs, Kind>) -> Self::Output {
        binary_expr(self.into_expr().node, BinaryOp::Sub, rhs.node)
    }
}

fn binary_expr<Out>(left: ExprNode, op: BinaryOp, right: ExprNode) -> Expr<Out> {
    Expr::new(ExprNode::Binary {
        left: Box::new(left),
        op,
        right: Box::new(right),
    })
}

impl<Lhs, RhsExpr, Kind> Add<RhsExpr> for Expr<Lhs, Kind>
where
    RhsExpr: ExprOperand,
    Lhs: SqlAdd<RhsExpr::Value>,
{
    type Output = Expr<<Lhs as SqlAdd<RhsExpr::Value>>::Output>;

    fn add(self, rhs: RhsExpr) -> Self::Output {
        binary_expr(self.node, BinaryOp::Add, rhs.into_operand_expr().node)
    }
}

impl<Lhs, RhsExpr, Kind> Sub<RhsExpr> for Expr<Lhs, Kind>
where
    RhsExpr: ExprOperand,
    Lhs: SqlSub<RhsExpr::Value>,
{
    type Output = Expr<<Lhs as SqlSub<RhsExpr::Value>>::Output>;

    fn sub(self, rhs: RhsExpr) -> Self::Output {
        binary_expr(self.node, BinaryOp::Sub, rhs.into_operand_expr().node)
    }
}

impl<Lhs, RhsExpr, Kind> Mul<RhsExpr> for Expr<Lhs, Kind>
where
    RhsExpr: ExprOperand,
    Lhs: SqlMul<RhsExpr::Value>,
{
    type Output = Expr<<Lhs as SqlMul<RhsExpr::Value>>::Output>;

    fn mul(self, rhs: RhsExpr) -> Self::Output {
        binary_expr(self.node, BinaryOp::Mul, rhs.into_operand_expr().node)
    }
}

impl<Lhs, RhsExpr, Kind> Div<RhsExpr> for Expr<Lhs, Kind>
where
    RhsExpr: ExprOperand,
    Lhs: SqlDiv<RhsExpr::Value>,
{
    type Output = Expr<<Lhs as SqlDiv<RhsExpr::Value>>::Output>;

    fn div(self, rhs: RhsExpr) -> Self::Output {
        binary_expr(self.node, BinaryOp::Div, rhs.into_operand_expr().node)
    }
}

impl<M, Lhs, RhsExpr> Add<RhsExpr> for Column<M, Lhs>
where
    RhsExpr: ExprOperand,
    Lhs: SqlAdd<RhsExpr::Value>,
{
    type Output = Expr<<Lhs as SqlAdd<RhsExpr::Value>>::Output>;

    fn add(self, rhs: RhsExpr) -> Self::Output {
        binary_expr(ExprNode::Column(self.as_ref()), BinaryOp::Add, rhs.into_operand_expr().node)
    }
}

impl<M, Lhs, RhsExpr> Sub<RhsExpr> for Column<M, Lhs>
where
    RhsExpr: ExprOperand,
    Lhs: SqlSub<RhsExpr::Value>,
{
    type Output = Expr<<Lhs as SqlSub<RhsExpr::Value>>::Output>;

    fn sub(self, rhs: RhsExpr) -> Self::Output {
        binary_expr(ExprNode::Column(self.as_ref()), BinaryOp::Sub, rhs.into_operand_expr().node)
    }
}

impl<M, Lhs, RhsExpr> Mul<RhsExpr> for Column<M, Lhs>
where
    RhsExpr: ExprOperand,
    Lhs: SqlMul<RhsExpr::Value>,
{
    type Output = Expr<<Lhs as SqlMul<RhsExpr::Value>>::Output>;

    fn mul(self, rhs: RhsExpr) -> Self::Output {
        binary_expr(ExprNode::Column(self.as_ref()), BinaryOp::Mul, rhs.into_operand_expr().node)
    }
}

impl<M, Lhs, RhsExpr> Div<RhsExpr> for Column<M, Lhs>
where
    RhsExpr: ExprOperand,
    Lhs: SqlDiv<RhsExpr::Value>,
{
    type Output = Expr<<Lhs as SqlDiv<RhsExpr::Value>>::Output>;

    fn div(self, rhs: RhsExpr) -> Self::Output {
        binary_expr(ExprNode::Column(self.as_ref()), BinaryOp::Div, rhs.into_operand_expr().node)
    }
}

macro_rules! impl_literal_numeric_op {
    ($trait:ident, $method:ident, $sql_trait:ident, $op:expr, $($lhs:ty),+ $(,)?) => {
        $(
            impl<M, Rhs> $trait<Column<M, Rhs>> for $lhs
            where
                $lhs: $sql_trait<Rhs>,
            {
                type Output = Expr<<$lhs as $sql_trait<Rhs>>::Output>;

                fn $method(self, rhs: Column<M, Rhs>) -> Self::Output {
                    binary_expr(self.into_expr().node, $op, ExprNode::Column(rhs.as_ref()))
                }
            }

            impl<Rhs, Kind> $trait<Expr<Rhs, Kind>> for $lhs
            where
                $lhs: $sql_trait<Rhs>,
            {
                type Output = Expr<<$lhs as $sql_trait<Rhs>>::Output>;

                fn $method(self, rhs: Expr<Rhs, Kind>) -> Self::Output {
                    binary_expr(self.into_expr().node, $op, rhs.node)
                }
            }
        )+
    };
}

impl_literal_numeric_op!(Add, add, SqlAdd, BinaryOp::Add, i16, i32, i64, f32, f64);
impl_literal_numeric_op!(Sub, sub, SqlSub, BinaryOp::Sub, i16, i32, i64, f32, f64);
impl_literal_numeric_op!(Mul, mul, SqlMul, BinaryOp::Mul, i16, i32, i64, f32, f64);
impl_literal_numeric_op!(Div, div, SqlDiv, BinaryOp::Div, i16, i32, i64, f32, f64);

macro_rules! impl_typed_binary_operator {
    ($trait:ident, $method:ident, $sql_trait:ident, $op:expr) => {
        impl<Lhs, Rhs, Kind> $trait<Rhs> for Expr<Lhs, Kind>
        where
            Rhs: BitwiseOperand,
            Lhs: $sql_trait<Rhs::Value>,
        {
            type Output = Expr<<Lhs as $sql_trait<Rhs::Value>>::Output>;

            fn $method(self, rhs: Rhs) -> Self::Output {
                binary_expr(self.node, $op, rhs.into_bitwise_expr().node)
            }
        }

        impl<M, Lhs, Rhs> $trait<Rhs> for Column<M, Lhs>
        where
            Rhs: BitwiseOperand,
            Lhs: $sql_trait<Rhs::Value>,
        {
            type Output = Expr<<Lhs as $sql_trait<Rhs::Value>>::Output>;

            fn $method(self, rhs: Rhs) -> Self::Output {
                binary_expr(ExprNode::Column(self.as_ref()), $op, rhs.into_bitwise_expr().node)
            }
        }
    };
}

impl_typed_binary_operator!(BitAnd, bitand, SqlBitwise, BinaryOp::BitAnd);
impl_typed_binary_operator!(BitOr, bitor, SqlBitwise, BinaryOp::BitOr);
impl_typed_binary_operator!(BitXor, bitxor, SqlBitwise, BinaryOp::BitXor);
impl_typed_binary_operator!(Shl, shl, SqlShift, BinaryOp::Shl);
impl_typed_binary_operator!(Shr, shr, SqlShift, BinaryOp::Shr);

macro_rules! impl_literal_bitwise_operator {
    ($trait:ident, $method:ident, $sql_trait:ident, $op:expr, $($lhs:ty),+ $(,)?) => {
        $(
            impl<M, Rhs> $trait<Column<M, Rhs>> for $lhs
            where
                $lhs: $sql_trait<Rhs>,
            {
                type Output = Expr<<$lhs as $sql_trait<Rhs>>::Output>;

                fn $method(self, rhs: Column<M, Rhs>) -> Self::Output {
                    binary_expr(self.into_expr().node, $op, ExprNode::Column(rhs.as_ref()))
                }
            }

            impl<Rhs, Kind> $trait<Expr<Rhs, Kind>> for $lhs
            where
                $lhs: $sql_trait<Rhs>,
            {
                type Output = Expr<<$lhs as $sql_trait<Rhs>>::Output>;

                fn $method(self, rhs: Expr<Rhs, Kind>) -> Self::Output {
                    binary_expr(self.into_expr().node, $op, rhs.node)
                }
            }
        )+
    };
}

macro_rules! impl_literal_bitwise_operators {
    ($($lhs:ty),+ $(,)?) => {
        impl_literal_bitwise_operator!(BitAnd, bitand, SqlBitwise, BinaryOp::BitAnd, $($lhs),+);
        impl_literal_bitwise_operator!(BitOr, bitor, SqlBitwise, BinaryOp::BitOr, $($lhs),+);
        impl_literal_bitwise_operator!(BitXor, bitxor, SqlBitwise, BinaryOp::BitXor, $($lhs),+);
        impl_literal_bitwise_operator!(Shl, shl, SqlShift, BinaryOp::Shl, $($lhs),+);
        impl_literal_bitwise_operator!(Shr, shr, SqlShift, BinaryOp::Shr, $($lhs),+);
    };
}

impl_literal_bitwise_operators!(i16, i32, i64);

impl<T, Kind> Not for Expr<T, Kind>
where
    T: SqlBitwise<T>,
{
    type Output = Expr<<T as SqlBitwise<T>>::Output>;

    fn not(self) -> Self::Output {
        Expr::new(ExprNode::Unary {
            op: UnaryOp::BitNot,
            expr: Box::new(self.node),
        })
    }
}

impl<M, T> Not for Column<M, T>
where
    T: SqlBitwise<T>,
{
    type Output = Expr<<T as SqlBitwise<T>>::Output>;

    fn not(self) -> Self::Output {
        Expr::new(ExprNode::Unary {
            op: UnaryOp::BitNot,
            expr: Box::new(ExprNode::Column(self.as_ref())),
        })
    }
}

impl<T, Kind> Expr<T, Kind>
where
    T: 'static,
{
    pub fn eq<V>(self, value: V) -> Expr<<T as ValueComparisonOutput>::Output>
    where
        T: ValueComparisonOutput,
        V: ColumnValue<T>,
    {
        match value.into_value() {
            Some(Value::Null) => Expr::new(ExprNode::IsNull {
                expr: Box::new(self.node),
                negated: false,
            }),
            Some(value) => Expr::new(ExprNode::Binary {
                left: Box::new(self.node),
                op: BinaryOp::Eq,
                right: Box::new(ExprNode::Value(value)),
            }),
            None => Expr::new(ExprNode::IsNull {
                expr: Box::new(self.node),
                negated: false,
            }),
        }
    }

    pub fn eq_col<M2, U>(self, other: Column<M2, U>) -> Expr<<T as CompatibleColumn<U>>::Output>
    where
        T: CompatibleColumn<U>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Eq,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn ne<V>(self, value: V) -> Expr<<T as ValueComparisonOutput>::Output>
    where
        T: ValueComparisonOutput,
        V: ColumnValue<T>,
    {
        match value.into_value() {
            Some(Value::Null) => Expr::new(ExprNode::IsNull {
                expr: Box::new(self.node),
                negated: true,
            }),
            Some(value) => Expr::new(ExprNode::Binary {
                left: Box::new(self.node),
                op: BinaryOp::Ne,
                right: Box::new(ExprNode::Value(value)),
            }),
            None => Expr::new(ExprNode::IsNull {
                expr: Box::new(self.node),
                negated: true,
            }),
        }
    }

    pub fn ne_col<M2, U>(self, other: Column<M2, U>) -> Expr<<T as CompatibleColumn<U>>::Output>
    where
        T: CompatibleColumn<U>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Ne,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn is_distinct_from_col<M2, U>(self, other: Column<M2, U>) -> Expr<bool>
    where
        T: CompatibleColumn<U>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::IsDistinctFrom,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn is_not_distinct_from_col<M2, U>(self, other: Column<M2, U>) -> Expr<bool>
    where
        T: CompatibleColumn<U>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::IsNotDistinctFrom,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn lt<V>(self, value: V) -> Expr<<T as ValueComparisonOutput>::Output>
    where
        T: ValueComparisonOutput,
        V: ColumnValue<T>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Lt,
            right: Box::new(ExprNode::Value(value.into_value().unwrap_or(Value::Null))),
        })
    }

    pub fn lt_col<M2, U>(self, other: Column<M2, U>) -> Expr<<T as CompatibleColumn<U>>::Output>
    where
        T: CompatibleColumn<U>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Lt,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn le<V>(self, value: V) -> Expr<<T as ValueComparisonOutput>::Output>
    where
        T: ValueComparisonOutput,
        V: ColumnValue<T>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Le,
            right: Box::new(ExprNode::Value(value.into_value().unwrap_or(Value::Null))),
        })
    }

    pub fn le_col<M2, U>(self, other: Column<M2, U>) -> Expr<<T as CompatibleColumn<U>>::Output>
    where
        T: CompatibleColumn<U>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Le,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn gt<V>(self, value: V) -> Expr<<T as ValueComparisonOutput>::Output>
    where
        T: ValueComparisonOutput,
        V: ColumnValue<T>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Gt,
            right: Box::new(ExprNode::Value(value.into_value().unwrap_or(Value::Null))),
        })
    }

    pub fn gt_col<M2, U>(self, other: Column<M2, U>) -> Expr<<T as CompatibleColumn<U>>::Output>
    where
        T: CompatibleColumn<U>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Gt,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn ge<V>(self, value: V) -> Expr<<T as ValueComparisonOutput>::Output>
    where
        T: ValueComparisonOutput,
        V: ColumnValue<T>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Ge,
            right: Box::new(ExprNode::Value(value.into_value().unwrap_or(Value::Null))),
        })
    }

    pub fn ge_col<M2, U>(self, other: Column<M2, U>) -> Expr<<T as CompatibleColumn<U>>::Output>
    where
        T: CompatibleColumn<U>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Ge,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn between<L, U>(self, low: L, high: U) -> Expr<<T as ValueComparisonOutput>::Output>
    where
        T: ValueComparisonOutput,
        L: ColumnValue<T>,
        U: ColumnValue<T>,
    {
        let low_value = low.into_value().unwrap_or(Value::Null);
        let high_value = high.into_value().unwrap_or(Value::Null);
        let node = self.node;
        let left = ExprNode::Binary {
            left: Box::new(node.clone()),
            op: BinaryOp::Ge,
            right: Box::new(ExprNode::Value(low_value)),
        };
        let right = ExprNode::Binary {
            left: Box::new(node),
            op: BinaryOp::Le,
            right: Box::new(ExprNode::Value(high_value)),
        };
        Expr::new(ExprNode::Bool {
            left: Box::new(left),
            op: BoolOp::And,
            right: Box::new(right),
        })
    }

    pub fn like<V>(self, pattern: V) -> Expr<<T as ValueComparisonOutput>::Output>
    where
        T: ValueComparisonOutput,
        V: ColumnValue<T>,
    {
        Expr::new(ExprNode::Like {
            expr: Box::new(self.node),
            pattern: pattern.into_value().unwrap_or(Value::Null),
            case_insensitive: false,
        })
    }

    pub fn ilike<V>(self, pattern: V) -> Expr<<T as ValueComparisonOutput>::Output>
    where
        T: ValueComparisonOutput,
        V: ColumnValue<T>,
    {
        Expr::new(ExprNode::Like {
            expr: Box::new(self.node),
            pattern: pattern.into_value().unwrap_or(Value::Null),
            case_insensitive: true,
        })
    }

    pub fn is_null(self) -> Expr<bool> {
        Expr::new(ExprNode::IsNull {
            expr: Box::new(self.node),
            negated: false,
        })
    }

    pub fn is_not_null(self) -> Expr<bool> {
        Expr::new(ExprNode::IsNull {
            expr: Box::new(self.node),
            negated: true,
        })
    }

    pub fn in_<I, V>(self, values: I) -> Expr<<T as ValueComparisonOutput>::Output>
    where
        T: ValueComparisonOutput,
        I: IntoIterator<Item = V>,
        V: ColumnValue<T>,
    {
        let mut binds = Vec::new();
        for value in values {
            if let Some(value) = value.into_value() {
                binds.push(value);
            }
        }
        Expr::new(ExprNode::In {
            expr: Box::new(self.node),
            values: binds,
        })
    }
}

impl<T, Kind> Expr<T, Kind>
where
    T: BooleanExprType,
{
    pub fn and<U>(self, other: Expr<U>) -> Expr<<T as BooleanOutput<U>>::Output>
    where
        T: BooleanOutput<U>,
        U: BooleanExprType,
    {
        Expr::new(ExprNode::Bool {
            left: Box::new(self.node),
            op: BoolOp::And,
            right: Box::new(other.node),
        })
    }

    pub fn or<U>(self, other: Expr<U>) -> Expr<<T as BooleanOutput<U>>::Output>
    where
        T: BooleanOutput<U>,
        U: BooleanExprType,
    {
        Expr::new(ExprNode::Bool {
            left: Box::new(self.node),
            op: BoolOp::Or,
            right: Box::new(other.node),
        })
    }

    pub fn not(self) -> Expr<T> {
        Expr::new(ExprNode::Unary {
            op: UnaryOp::Not,
            expr: Box::new(self.node),
        })
    }
}

impl<M, T> Column<M, T>
where
    T: 'static,
{
    pub fn eq<V>(self, value: V) -> Expr<<T as ValueComparisonOutput>::Output>
    where
        T: ValueComparisonOutput,
        V: ColumnValue<T>,
    {
        match value.into_value() {
            Some(Value::Null) => Expr::new(ExprNode::IsNull {
                expr: Box::new(ExprNode::Column(self.as_ref())),
                negated: false,
            }),
            Some(value) => Expr::new(ExprNode::Binary {
                left: Box::new(ExprNode::Column(self.as_ref())),
                op: BinaryOp::Eq,
                right: Box::new(ExprNode::Value(value)),
            }),
            None => Expr::new(ExprNode::IsNull {
                expr: Box::new(ExprNode::Column(self.as_ref())),
                negated: false,
            }),
        }
    }

    pub fn eq_col<M2, U>(self, other: Column<M2, U>) -> Expr<<T as CompatibleColumn<U>>::Output>
    where
        T: CompatibleColumn<U>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Eq,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn ne_col<M2, U>(self, other: Column<M2, U>) -> Expr<<T as CompatibleColumn<U>>::Output>
    where
        T: CompatibleColumn<U>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Ne,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn is_distinct_from_col<M2, U>(self, other: Column<M2, U>) -> Expr<bool>
    where
        T: CompatibleColumn<U>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::IsDistinctFrom,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn is_not_distinct_from_col<M2, U>(self, other: Column<M2, U>) -> Expr<bool>
    where
        T: CompatibleColumn<U>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::IsNotDistinctFrom,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn ne<V>(self, value: V) -> Expr<<T as ValueComparisonOutput>::Output>
    where
        T: ValueComparisonOutput,
        V: ColumnValue<T>,
    {
        match value.into_value() {
            Some(Value::Null) => Expr::new(ExprNode::IsNull {
                expr: Box::new(ExprNode::Column(self.as_ref())),
                negated: true,
            }),
            Some(value) => Expr::new(ExprNode::Binary {
                left: Box::new(ExprNode::Column(self.as_ref())),
                op: BinaryOp::Ne,
                right: Box::new(ExprNode::Value(value)),
            }),
            None => Expr::new(ExprNode::IsNull {
                expr: Box::new(ExprNode::Column(self.as_ref())),
                negated: true,
            }),
        }
    }

    pub fn lt<V, Marker>(self, value: V) -> Expr<V::Output>
    where
        V: ComparisonValue<T, Marker>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Lt,
            right: Box::new(value.into_comparison_expr().node),
        })
    }

    pub fn lt_col<M2, U>(self, other: Column<M2, U>) -> Expr<<T as CompatibleColumn<U>>::Output>
    where
        T: CompatibleColumn<U>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Lt,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn le<V, Marker>(self, value: V) -> Expr<V::Output>
    where
        V: ComparisonValue<T, Marker>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Le,
            right: Box::new(value.into_comparison_expr().node),
        })
    }

    pub fn le_col<M2, U>(self, other: Column<M2, U>) -> Expr<<T as CompatibleColumn<U>>::Output>
    where
        T: CompatibleColumn<U>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Le,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn gt<V, Marker>(self, value: V) -> Expr<V::Output>
    where
        V: ComparisonValue<T, Marker>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Gt,
            right: Box::new(value.into_comparison_expr().node),
        })
    }

    pub fn gt_col<M2, U>(self, other: Column<M2, U>) -> Expr<<T as CompatibleColumn<U>>::Output>
    where
        T: CompatibleColumn<U>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Gt,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn ge<V, Marker>(self, value: V) -> Expr<V::Output>
    where
        V: ComparisonValue<T, Marker>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Ge,
            right: Box::new(value.into_comparison_expr().node),
        })
    }

    pub fn ge_col<M2, U>(self, other: Column<M2, U>) -> Expr<<T as CompatibleColumn<U>>::Output>
    where
        T: CompatibleColumn<U>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Ge,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn between<L, U>(self, low: L, high: U) -> Expr<<T as ValueComparisonOutput>::Output>
    where
        T: ValueComparisonOutput,
        L: ColumnValue<T>,
        U: ColumnValue<T>,
    {
        let left = ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Ge,
            right: Box::new(ExprNode::Value(low.into_value().unwrap_or(Value::Null))),
        };
        let right = ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Le,
            right: Box::new(ExprNode::Value(high.into_value().unwrap_or(Value::Null))),
        };
        Expr::new(ExprNode::Bool {
            left: Box::new(left),
            op: BoolOp::And,
            right: Box::new(right),
        })
    }

    pub fn like<V>(self, pattern: V) -> Expr<<T as ValueComparisonOutput>::Output>
    where
        T: ValueComparisonOutput,
        V: ColumnValue<T>,
    {
        Expr::new(ExprNode::Like {
            expr: Box::new(ExprNode::Column(self.as_ref())),
            pattern: pattern.into_value().unwrap_or(Value::Null),
            case_insensitive: false,
        })
    }

    pub fn ilike<V>(self, pattern: V) -> Expr<<T as ValueComparisonOutput>::Output>
    where
        T: ValueComparisonOutput,
        V: ColumnValue<T>,
    {
        Expr::new(ExprNode::Like {
            expr: Box::new(ExprNode::Column(self.as_ref())),
            pattern: pattern.into_value().unwrap_or(Value::Null),
            case_insensitive: true,
        })
    }

    pub fn in_<I, V>(self, values: I) -> Expr<<T as ValueComparisonOutput>::Output>
    where
        T: ValueComparisonOutput,
        I: IntoIterator<Item = V>,
        V: ColumnValue<T>,
    {
        Expr::new(ExprNode::In {
            expr: Box::new(ExprNode::Column(self.as_ref())),
            values: values.into_iter().map(|value| value.into_value().unwrap_or(Value::Null)).collect(),
        })
    }

    pub fn is_null(self) -> Expr<bool> {
        Expr::new(ExprNode::IsNull {
            expr: Box::new(ExprNode::Column(self.as_ref())),
            negated: false,
        })
    }

    pub fn is_not_null(self) -> Expr<bool> {
        Expr::new(ExprNode::IsNull {
            expr: Box::new(ExprNode::Column(self.as_ref())),
            negated: true,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ConditionKind {
    Any,
    All,
}

#[derive(Debug, Clone)]
pub struct Condition<T = bool> {
    kind: ConditionKind,
    exprs: Vec<ExprNode>,
    _marker: PhantomData<T>,
}

impl Condition<bool> {
    pub fn any() -> Self {
        Self {
            kind: ConditionKind::Any,
            exprs: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn all() -> Self {
        Self {
            kind: ConditionKind::All,
            exprs: Vec::new(),
            _marker: PhantomData,
        }
    }
}

impl<T> Condition<T>
where
    T: BooleanExprType,
{
    pub fn add<U>(mut self, expr: Expr<U>) -> Condition<<T as BooleanOutput<U>>::Output>
    where
        T: BooleanOutput<U>,
        U: BooleanExprType,
    {
        self.exprs.push(expr.node);
        Condition {
            kind: self.kind,
            exprs: self.exprs,
            _marker: PhantomData,
        }
    }

    pub fn into_expr(self) -> Option<Expr<T>> {
        let mut iter = self.exprs.into_iter();
        let first = iter.next()?;
        Some(Expr::new(iter.fold(first, |acc, expr| ExprNode::Bool {
            left: Box::new(acc),
            op: match self.kind {
                ConditionKind::Any => BoolOp::Or,
                ConditionKind::All => BoolOp::And,
            },
            right: Box::new(expr),
        })))
    }
}
