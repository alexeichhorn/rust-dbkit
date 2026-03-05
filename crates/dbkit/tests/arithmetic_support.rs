#![allow(non_upper_case_globals)]

use chrono::NaiveDateTime;
use dbkit::{model, Expr, ExprNode, IntoExpr, Order};

#[model(table = "records")]
pub struct Record {
    #[key]
    pub id: i64,
    pub left_value: i64,
    pub right_value: i64,
    pub baseline_value: i64,
    pub occurred_at: NaiveDateTime,
}

#[derive(Debug, Clone, Copy)]
struct OffsetValue;

impl dbkit::SqlInterval for OffsetValue {}

fn make_offset(arg: impl IntoExpr<i64>) -> Expr<OffsetValue> {
    let expr = arg.into_expr();
    Expr::new(ExprNode::Func {
        name: "MAKE_OFFSET",
        args: vec![expr.node],
    })
}

#[test]
fn query_with_numeric_arithmetic_has_expected_sql_shape() {
    let sql = Record::query()
        .filter((Record::left_value + 1_i64).lt_col(Record::baseline_value))
        .filter((Record::right_value - Record::left_value).gt(0_i64))
        .order_by(Order::desc(Record::baseline_value + Record::left_value))
        .debug_sql();

    assert!(
        sql.contains("(records.left_value + $1) < records.baseline_value"),
        "unexpected SQL: {sql}"
    );
    assert!(
        sql.contains("(records.right_value - records.left_value) > $2"),
        "unexpected SQL: {sql}"
    );
    assert!(
        sql.contains("ORDER BY (records.baseline_value + records.left_value) DESC"),
        "unexpected SQL: {sql}"
    );
}

#[test]
fn query_with_timestamp_offset_arithmetic_has_expected_sql_shape() {
    let cutoff = chrono::DateTime::from_timestamp(1_700_000_000, 0).expect("cutoff").naive_utc();

    let sql = Record::query()
        .filter((Record::occurred_at + make_offset(Record::left_value)).le(cutoff))
        .order_by(Order::asc(Record::occurred_at - make_offset(1_i64)))
        .debug_sql();

    assert!(
        sql.contains("(records.occurred_at + MAKE_OFFSET(records.left_value)) <= $1"),
        "unexpected SQL: {sql}"
    );
    assert!(
        sql.contains("ORDER BY (records.occurred_at - MAKE_OFFSET($2)) ASC"),
        "unexpected SQL: {sql}"
    );
}

#[test]
fn select_only_accepts_arithmetic_projection_aliases() {
    let sql = Record::query()
        .select_only()
        .column_as(Record::baseline_value + Record::left_value, "computed_value")
        .order_by(Order::desc_alias("computed_value"))
        .debug_sql();

    assert!(sql.contains("SELECT (records.baseline_value + records.left_value) AS computed_value FROM records"));
    assert!(sql.contains("ORDER BY computed_value DESC"));
}
