use crate::expr::{Expr, ExprNode, IntervalField, IntoExpr};
use crate::PgInterval;

#[doc(hidden)]
pub trait IntervalExprType {
    type Output;
}

macro_rules! impl_interval_expr_type {
    ($($ty:ty),* $(,)?) => {
        $(
            impl IntervalExprType for $ty {
                type Output = PgInterval;
            }

            impl IntervalExprType for Option<$ty> {
                type Output = Option<PgInterval>;
            }
        )*
    };
}

impl_interval_expr_type!(i16, i32, i64, f32, f64);

#[doc(hidden)]
pub trait IntegerIntervalExprType: IntervalExprType {}

impl IntegerIntervalExprType for i32 {}
impl IntegerIntervalExprType for Option<i32> {}

fn interval_part<T>(field: IntervalField, value: impl IntoExpr<T>) -> Expr<<T as IntervalExprType>::Output>
where
    T: IntervalExprType,
{
    let value = value.into_expr();
    Expr::new(ExprNode::MakeInterval {
        field,
        value: Box::new(value.node),
    })
}

pub fn days<T>(value: impl IntoExpr<T>) -> Expr<<T as IntervalExprType>::Output>
where
    T: IntegerIntervalExprType,
{
    interval_part(IntervalField::Days, value)
}

pub fn hours<T>(value: impl IntoExpr<T>) -> Expr<<T as IntervalExprType>::Output>
where
    T: IntegerIntervalExprType,
{
    interval_part(IntervalField::Hours, value)
}

pub fn minutes<T>(value: impl IntoExpr<T>) -> Expr<<T as IntervalExprType>::Output>
where
    T: IntegerIntervalExprType,
{
    interval_part(IntervalField::Minutes, value)
}

pub fn seconds<T>(value: impl IntoExpr<T>) -> Expr<<T as IntervalExprType>::Output>
where
    T: IntervalExprType,
{
    interval_part(IntervalField::Seconds, value)
}
