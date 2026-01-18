use crate::expr::{Expr, ExprNode, IntoExpr};

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
