use crate::expr::{Expr, ExprNode, IntervalField, IntoExpr, NumericExprType};
use crate::PgInterval;

fn interval_part<T>(field: IntervalField, value: impl IntoExpr<T>) -> Expr<PgInterval> {
    let value = value.into_expr();
    Expr::new(ExprNode::MakeInterval {
        field,
        value: Box::new(value.node),
    })
}

pub fn days(value: impl IntoExpr<i32>) -> Expr<PgInterval> {
    interval_part(IntervalField::Days, value)
}

pub fn hours(value: impl IntoExpr<i32>) -> Expr<PgInterval> {
    interval_part(IntervalField::Hours, value)
}

pub fn minutes(value: impl IntoExpr<i32>) -> Expr<PgInterval> {
    interval_part(IntervalField::Minutes, value)
}

pub fn seconds<T>(value: impl IntoExpr<T>) -> Expr<PgInterval>
where
    T: NumericExprType,
{
    interval_part(IntervalField::Seconds, value)
}
