use std::marker::PhantomData;
use std::ops::{Add, Mul, Sub};

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

impl<T> ColumnValue<T> for Option<T>
where
    T: Into<Value>,
{
    fn into_value(self) -> Option<Value> {
        self.map(Into::into)
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

impl<T> From<Option<T>> for Value
where
    T: Into<Value>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(v) => v.into(),
            None => Self::Null,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
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
pub enum BoolOp {
    And,
    Or,
}

#[derive(Debug, Clone, Copy)]
pub enum UnaryOp {
    Not,
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

#[derive(Debug, Clone)]
pub enum ExprNode {
    Column(ColumnRef),
    Value(Value),
    Func {
        name: &'static str,
        args: Vec<ExprNode>,
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
    IsNull {
        expr: Box<ExprNode>,
        negated: bool,
    },
    Like {
        expr: Box<ExprNode>,
        pattern: Value,
        case_insensitive: bool,
    },
}

#[derive(Debug, Clone)]
pub struct Expr<T> {
    pub node: ExprNode,
    _marker: PhantomData<T>,
}

impl<T> Expr<T> {
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

pub trait ComparisonValue<T> {
    fn into_comparison_expr(self) -> Expr<T>;
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

pub trait NumericExprType {}

impl<T> IntoExpr<T> for Expr<T> {
    fn into_expr(self) -> Expr<T> {
        self
    }
}

impl<T> ExprOperand for Expr<T> {
    type Value = T;

    fn into_operand_expr(self) -> Expr<Self::Value> {
        self
    }
}

impl<T> ComparisonValue<T> for Expr<T> {
    fn into_comparison_expr(self) -> Expr<T> {
        self
    }
}

impl<T> ComparisonValue<Option<T>> for Expr<T> {
    fn into_comparison_expr(self) -> Expr<Option<T>> {
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

impl<M, T> ComparisonValue<T> for Column<M, T> {
    fn into_comparison_expr(self) -> Expr<T> {
        self.into_expr()
    }
}

impl<M, T> ComparisonValue<Option<T>> for Column<M, T> {
    fn into_comparison_expr(self) -> Expr<Option<T>> {
        Expr::new(ExprNode::Column(self.as_ref()))
    }
}

macro_rules! impl_value_expr_traits {
    ($ty:ty, $value_ty:ty) => {
        impl ComparisonValue<$value_ty> for $ty {
            fn into_comparison_expr(self) -> Expr<$value_ty> {
                self.into_expr()
            }
        }

        impl ComparisonValue<Option<$value_ty>> for $ty {
            fn into_comparison_expr(self) -> Expr<Option<$value_ty>> {
                Expr::new(self.into_expr().node)
            }
        }
    };
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

impl_value_expr_traits!(String, String);

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

impl_value_expr_traits!(&str, String);

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

impl_value_expr_traits!(bool, bool);

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

impl_value_expr_traits!(i16, i16);
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

impl_value_expr_traits!(i32, i32);
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

impl_value_expr_traits!(i64, i64);
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

impl_value_expr_traits!(f32, f32);
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

impl_value_expr_traits!(f64, f64);
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

impl_value_expr_traits!(uuid::Uuid, uuid::Uuid);

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

impl_value_expr_traits!(chrono::NaiveDateTime, chrono::NaiveDateTime);

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

impl_value_expr_traits!(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>);

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

impl_value_expr_traits!(chrono::NaiveDate, chrono::NaiveDate);

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

impl_value_expr_traits!(chrono::NaiveTime, chrono::NaiveTime);

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

impl_value_expr_traits!(PgInterval, PgInterval);

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

impl_value_expr_traits!(Vec<String>, Vec<String>);

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

impl_value_expr_traits!(serde_json::Value, serde_json::Value);

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

macro_rules! impl_numeric_arithmetic {
    ($($ty:ty),* $(,)?) => {
        $(
            impl SqlAdd<$ty> for $ty {
                type Output = $ty;
            }

            impl SqlSub<$ty> for $ty {
                type Output = $ty;
            }

            impl SqlMul<$ty> for $ty {
                type Output = $ty;
            }
        )*
    };
}

impl SqlAdd<i16> for i16 {
    type Output = i32;
}

impl SqlSub<i16> for i16 {
    type Output = i32;
}

impl SqlMul<i16> for i16 {
    type Output = i32;
}

impl_numeric_arithmetic!(i32, i64, f32, f64);

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

impl Add<Expr<PgInterval>> for chrono::NaiveDateTime {
    type Output = Expr<chrono::NaiveDateTime>;

    fn add(self, rhs: Expr<PgInterval>) -> Self::Output {
        arithmetic_expr(self.into_expr().node, BinaryOp::Add, rhs.node)
    }
}

impl Sub<Expr<PgInterval>> for chrono::NaiveDateTime {
    type Output = Expr<chrono::NaiveDateTime>;

    fn sub(self, rhs: Expr<PgInterval>) -> Self::Output {
        arithmetic_expr(self.into_expr().node, BinaryOp::Sub, rhs.node)
    }
}

impl Add<Expr<PgInterval>> for chrono::DateTime<chrono::Utc> {
    type Output = Expr<chrono::DateTime<chrono::Utc>>;

    fn add(self, rhs: Expr<PgInterval>) -> Self::Output {
        arithmetic_expr(self.into_expr().node, BinaryOp::Add, rhs.node)
    }
}

impl Sub<Expr<PgInterval>> for chrono::DateTime<chrono::Utc> {
    type Output = Expr<chrono::DateTime<chrono::Utc>>;

    fn sub(self, rhs: Expr<PgInterval>) -> Self::Output {
        arithmetic_expr(self.into_expr().node, BinaryOp::Sub, rhs.node)
    }
}

fn arithmetic_expr<Out>(left: ExprNode, op: BinaryOp, right: ExprNode) -> Expr<Out> {
    Expr::new(ExprNode::Binary {
        left: Box::new(left),
        op,
        right: Box::new(right),
    })
}

impl<Lhs, RhsExpr> Add<RhsExpr> for Expr<Lhs>
where
    RhsExpr: ExprOperand,
    Lhs: SqlAdd<RhsExpr::Value>,
{
    type Output = Expr<<Lhs as SqlAdd<RhsExpr::Value>>::Output>;

    fn add(self, rhs: RhsExpr) -> Self::Output {
        arithmetic_expr(self.node, BinaryOp::Add, rhs.into_operand_expr().node)
    }
}

impl<Lhs, RhsExpr> Sub<RhsExpr> for Expr<Lhs>
where
    RhsExpr: ExprOperand,
    Lhs: SqlSub<RhsExpr::Value>,
{
    type Output = Expr<<Lhs as SqlSub<RhsExpr::Value>>::Output>;

    fn sub(self, rhs: RhsExpr) -> Self::Output {
        arithmetic_expr(self.node, BinaryOp::Sub, rhs.into_operand_expr().node)
    }
}

impl<Lhs, RhsExpr> Mul<RhsExpr> for Expr<Lhs>
where
    RhsExpr: ExprOperand,
    Lhs: SqlMul<RhsExpr::Value>,
{
    type Output = Expr<<Lhs as SqlMul<RhsExpr::Value>>::Output>;

    fn mul(self, rhs: RhsExpr) -> Self::Output {
        arithmetic_expr(self.node, BinaryOp::Mul, rhs.into_operand_expr().node)
    }
}

impl<M, Lhs, RhsExpr> Add<RhsExpr> for Column<M, Lhs>
where
    RhsExpr: ExprOperand,
    Lhs: SqlAdd<RhsExpr::Value>,
{
    type Output = Expr<<Lhs as SqlAdd<RhsExpr::Value>>::Output>;

    fn add(self, rhs: RhsExpr) -> Self::Output {
        arithmetic_expr(ExprNode::Column(self.as_ref()), BinaryOp::Add, rhs.into_operand_expr().node)
    }
}

impl<M, Lhs, RhsExpr> Sub<RhsExpr> for Column<M, Lhs>
where
    RhsExpr: ExprOperand,
    Lhs: SqlSub<RhsExpr::Value>,
{
    type Output = Expr<<Lhs as SqlSub<RhsExpr::Value>>::Output>;

    fn sub(self, rhs: RhsExpr) -> Self::Output {
        arithmetic_expr(ExprNode::Column(self.as_ref()), BinaryOp::Sub, rhs.into_operand_expr().node)
    }
}

impl<M, Lhs, RhsExpr> Mul<RhsExpr> for Column<M, Lhs>
where
    RhsExpr: ExprOperand,
    Lhs: SqlMul<RhsExpr::Value>,
{
    type Output = Expr<<Lhs as SqlMul<RhsExpr::Value>>::Output>;

    fn mul(self, rhs: RhsExpr) -> Self::Output {
        arithmetic_expr(ExprNode::Column(self.as_ref()), BinaryOp::Mul, rhs.into_operand_expr().node)
    }
}

impl<T> Expr<T>
where
    T: 'static,
{
    pub fn eq<V>(self, value: V) -> Expr<bool>
    where
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

    pub fn eq_col<M2>(self, other: Column<M2, T>) -> Expr<bool> {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Eq,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn ne<V>(self, value: V) -> Expr<bool>
    where
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

    pub fn ne_col<M2>(self, other: Column<M2, T>) -> Expr<bool> {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Ne,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn is_distinct_from_col<M2>(self, other: Column<M2, T>) -> Expr<bool> {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::IsDistinctFrom,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn is_not_distinct_from_col<M2>(self, other: Column<M2, T>) -> Expr<bool> {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::IsNotDistinctFrom,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn lt<V>(self, value: V) -> Expr<bool>
    where
        V: ComparisonValue<T>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Lt,
            right: Box::new(value.into_comparison_expr().node),
        })
    }

    pub fn lt_col<M2>(self, other: Column<M2, T>) -> Expr<bool> {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Lt,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn le<V>(self, value: V) -> Expr<bool>
    where
        V: ComparisonValue<T>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Le,
            right: Box::new(value.into_comparison_expr().node),
        })
    }

    pub fn le_col<M2>(self, other: Column<M2, T>) -> Expr<bool> {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Le,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn gt<V>(self, value: V) -> Expr<bool>
    where
        V: ComparisonValue<T>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Gt,
            right: Box::new(value.into_comparison_expr().node),
        })
    }

    pub fn gt_col<M2>(self, other: Column<M2, T>) -> Expr<bool> {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Gt,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn ge<V>(self, value: V) -> Expr<bool>
    where
        V: ComparisonValue<T>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Ge,
            right: Box::new(value.into_comparison_expr().node),
        })
    }

    pub fn ge_col<M2>(self, other: Column<M2, T>) -> Expr<bool> {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Ge,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn between<L, U>(self, low: L, high: U) -> Expr<bool>
    where
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

    pub fn like<V>(self, pattern: V) -> Expr<bool>
    where
        V: ColumnValue<T>,
    {
        Expr::new(ExprNode::Like {
            expr: Box::new(self.node),
            pattern: pattern.into_value().unwrap_or(Value::Null),
            case_insensitive: false,
        })
    }

    pub fn ilike<V>(self, pattern: V) -> Expr<bool>
    where
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

    pub fn in_<I, V>(self, values: I) -> Expr<bool>
    where
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

impl Expr<bool> {
    pub fn and(self, other: Expr<bool>) -> Expr<bool> {
        Expr::new(ExprNode::Bool {
            left: Box::new(self.node),
            op: BoolOp::And,
            right: Box::new(other.node),
        })
    }

    pub fn or(self, other: Expr<bool>) -> Expr<bool> {
        Expr::new(ExprNode::Bool {
            left: Box::new(self.node),
            op: BoolOp::Or,
            right: Box::new(other.node),
        })
    }

    pub fn not(self) -> Expr<bool> {
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
    pub fn eq<V>(self, value: V) -> Expr<bool>
    where
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

    pub fn eq_col<M2>(self, other: Column<M2, T>) -> Expr<bool> {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Eq,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn ne_col<M2>(self, other: Column<M2, T>) -> Expr<bool> {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Ne,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn is_distinct_from_col<M2>(self, other: Column<M2, T>) -> Expr<bool> {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::IsDistinctFrom,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn is_not_distinct_from_col<M2>(self, other: Column<M2, T>) -> Expr<bool> {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::IsNotDistinctFrom,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn ne<V>(self, value: V) -> Expr<bool>
    where
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

    pub fn lt<V>(self, value: V) -> Expr<bool>
    where
        V: ComparisonValue<T>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Lt,
            right: Box::new(value.into_comparison_expr().node),
        })
    }

    pub fn lt_col<M2>(self, other: Column<M2, T>) -> Expr<bool> {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Lt,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn le<V>(self, value: V) -> Expr<bool>
    where
        V: ComparisonValue<T>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Le,
            right: Box::new(value.into_comparison_expr().node),
        })
    }

    pub fn le_col<M2>(self, other: Column<M2, T>) -> Expr<bool> {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Le,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn gt<V>(self, value: V) -> Expr<bool>
    where
        V: ComparisonValue<T>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Gt,
            right: Box::new(value.into_comparison_expr().node),
        })
    }

    pub fn gt_col<M2>(self, other: Column<M2, T>) -> Expr<bool> {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Gt,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn ge<V>(self, value: V) -> Expr<bool>
    where
        V: ComparisonValue<T>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Ge,
            right: Box::new(value.into_comparison_expr().node),
        })
    }

    pub fn ge_col<M2>(self, other: Column<M2, T>) -> Expr<bool> {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Ge,
            right: Box::new(ExprNode::Column(other.as_ref())),
        })
    }

    pub fn between<L, U>(self, low: L, high: U) -> Expr<bool>
    where
        L: Into<Value>,
        U: Into<Value>,
    {
        let left = ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Ge,
            right: Box::new(ExprNode::Value(low.into())),
        };
        let right = ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Le,
            right: Box::new(ExprNode::Value(high.into())),
        };
        Expr::new(ExprNode::Bool {
            left: Box::new(left),
            op: BoolOp::And,
            right: Box::new(right),
        })
    }

    pub fn like<V>(self, pattern: V) -> Expr<bool>
    where
        V: Into<Value>,
    {
        Expr::new(ExprNode::Like {
            expr: Box::new(ExprNode::Column(self.as_ref())),
            pattern: pattern.into(),
            case_insensitive: false,
        })
    }

    pub fn ilike<V>(self, pattern: V) -> Expr<bool>
    where
        V: Into<Value>,
    {
        Expr::new(ExprNode::Like {
            expr: Box::new(ExprNode::Column(self.as_ref())),
            pattern: pattern.into(),
            case_insensitive: true,
        })
    }

    pub fn in_<I, V>(self, values: I) -> Expr<bool>
    where
        I: IntoIterator<Item = V>,
        V: Into<Value>,
    {
        Expr::new(ExprNode::In {
            expr: Box::new(ExprNode::Column(self.as_ref())),
            values: values.into_iter().map(Into::into).collect(),
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
pub struct Condition {
    kind: ConditionKind,
    exprs: Vec<Expr<bool>>,
}

impl Condition {
    pub fn any() -> Self {
        Self {
            kind: ConditionKind::Any,
            exprs: Vec::new(),
        }
    }

    pub fn all() -> Self {
        Self {
            kind: ConditionKind::All,
            exprs: Vec::new(),
        }
    }

    pub fn add(mut self, expr: Expr<bool>) -> Self {
        self.exprs.push(expr);
        self
    }

    pub fn into_expr(self) -> Option<Expr<bool>> {
        let mut iter = self.exprs.into_iter();
        let first = iter.next()?;
        Some(iter.fold(first, |acc, expr| match self.kind {
            ConditionKind::Any => acc.or(expr),
            ConditionKind::All => acc.and(expr),
        }))
    }
}
