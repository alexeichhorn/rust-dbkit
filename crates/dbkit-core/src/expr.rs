use std::marker::PhantomData;

use crate::schema::{Column, ColumnRef};

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
    Eq,
    Ne,
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

#[derive(Debug, Clone)]
pub enum ExprNode {
    Column(ColumnRef),
    Value(Value),
    Func {
        name: &'static str,
        args: Vec<ExprNode>,
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

impl<T> IntoExpr<T> for Expr<T> {
    fn into_expr(self) -> Expr<T> {
        self
    }
}

impl<M, T> IntoExpr<T> for Column<M, T> {
    fn into_expr(self) -> Expr<T> {
        Expr::new(ExprNode::Column(self.as_ref()))
    }
}

impl IntoExpr<String> for String {
    fn into_expr(self) -> Expr<String> {
        Expr::new(ExprNode::Value(Value::String(self)))
    }
}

impl IntoExpr<String> for &str {
    fn into_expr(self) -> Expr<String> {
        Expr::new(ExprNode::Value(Value::String(self.to_string())))
    }
}

impl IntoExpr<bool> for bool {
    fn into_expr(self) -> Expr<bool> {
        Expr::new(ExprNode::Value(Value::Bool(self)))
    }
}

impl IntoExpr<i16> for i16 {
    fn into_expr(self) -> Expr<i16> {
        Expr::new(ExprNode::Value(Value::I16(self)))
    }
}

impl IntoExpr<i32> for i32 {
    fn into_expr(self) -> Expr<i32> {
        Expr::new(ExprNode::Value(Value::I32(self)))
    }
}

impl IntoExpr<i64> for i64 {
    fn into_expr(self) -> Expr<i64> {
        Expr::new(ExprNode::Value(Value::I64(self)))
    }
}

impl IntoExpr<f32> for f32 {
    fn into_expr(self) -> Expr<f32> {
        Expr::new(ExprNode::Value(Value::F32(self)))
    }
}

impl IntoExpr<f64> for f64 {
    fn into_expr(self) -> Expr<f64> {
        Expr::new(ExprNode::Value(Value::F64(self)))
    }
}

impl IntoExpr<uuid::Uuid> for uuid::Uuid {
    fn into_expr(self) -> Expr<uuid::Uuid> {
        Expr::new(ExprNode::Value(Value::Uuid(self)))
    }
}

impl IntoExpr<chrono::NaiveDateTime> for chrono::NaiveDateTime {
    fn into_expr(self) -> Expr<chrono::NaiveDateTime> {
        Expr::new(ExprNode::Value(Value::DateTime(self)))
    }
}

impl IntoExpr<chrono::DateTime<chrono::Utc>> for chrono::DateTime<chrono::Utc> {
    fn into_expr(self) -> Expr<chrono::DateTime<chrono::Utc>> {
        Expr::new(ExprNode::Value(Value::DateTimeUtc(self)))
    }
}

impl IntoExpr<chrono::NaiveDate> for chrono::NaiveDate {
    fn into_expr(self) -> Expr<chrono::NaiveDate> {
        Expr::new(ExprNode::Value(Value::Date(self)))
    }
}

impl IntoExpr<chrono::NaiveTime> for chrono::NaiveTime {
    fn into_expr(self) -> Expr<chrono::NaiveTime> {
        Expr::new(ExprNode::Value(Value::Time(self)))
    }
}

impl IntoExpr<Vec<String>> for Vec<String> {
    fn into_expr(self) -> Expr<Vec<String>> {
        Expr::new(ExprNode::Value(Value::Array(self)))
    }
}

impl IntoExpr<serde_json::Value> for serde_json::Value {
    fn into_expr(self) -> Expr<serde_json::Value> {
        Expr::new(ExprNode::Value(Value::Json(self)))
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

    pub fn ne<V>(self, value: V) -> Expr<bool>
    where
        V: ColumnValue<T>,
    {
        match value.into_value() {
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

    pub fn lt<V>(self, value: V) -> Expr<bool>
    where
        V: ColumnValue<T>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Lt,
            right: Box::new(ExprNode::Value(value.into_value().unwrap_or(Value::Null))),
        })
    }

    pub fn le<V>(self, value: V) -> Expr<bool>
    where
        V: ColumnValue<T>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Le,
            right: Box::new(ExprNode::Value(value.into_value().unwrap_or(Value::Null))),
        })
    }

    pub fn gt<V>(self, value: V) -> Expr<bool>
    where
        V: ColumnValue<T>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Gt,
            right: Box::new(ExprNode::Value(value.into_value().unwrap_or(Value::Null))),
        })
    }

    pub fn ge<V>(self, value: V) -> Expr<bool>
    where
        V: ColumnValue<T>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(self.node),
            op: BinaryOp::Ge,
            right: Box::new(ExprNode::Value(value.into_value().unwrap_or(Value::Null))),
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

    pub fn ne<V>(self, value: V) -> Expr<bool>
    where
        V: ColumnValue<T>,
    {
        match value.into_value() {
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
        V: Into<Value>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Lt,
            right: Box::new(ExprNode::Value(value.into())),
        })
    }

    pub fn le<V>(self, value: V) -> Expr<bool>
    where
        V: Into<Value>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Le,
            right: Box::new(ExprNode::Value(value.into())),
        })
    }

    pub fn gt<V>(self, value: V) -> Expr<bool>
    where
        V: Into<Value>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Gt,
            right: Box::new(ExprNode::Value(value.into())),
        })
    }

    pub fn ge<V>(self, value: V) -> Expr<bool>
    where
        V: Into<Value>,
    {
        Expr::new(ExprNode::Binary {
            left: Box::new(ExprNode::Column(self.as_ref())),
            op: BinaryOp::Ge,
            right: Box::new(ExprNode::Value(value.into())),
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
