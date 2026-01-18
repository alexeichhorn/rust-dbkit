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
    Uuid(uuid::Uuid),
    DateTime(chrono::NaiveDateTime),
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
