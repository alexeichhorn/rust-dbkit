//@check-pass
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

fn main() {
    let cutoff = chrono::DateTime::from_timestamp(1_700_000_000, 0)
        .expect("cutoff")
        .naive_utc();

    let _query = Record::query()
        .filter((Record::left_value + 1_i64).lt_col(Record::baseline_value))
        .filter((Record::right_value - Record::left_value).ge(0_i64))
        .filter((Record::occurred_at + make_offset(1_i64)).le(cutoff))
        .order_by(Order::desc(Record::baseline_value + Record::left_value))
        .order_by(Order::asc(Record::occurred_at - make_offset(Record::left_value)))
        .limit(25)
        .debug_sql();

    let _projection = Record::query()
        .select_only()
        .column_as(Record::baseline_value + Record::left_value, "computed_value")
        .debug_sql();
}
