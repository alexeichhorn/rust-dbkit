//@check-pass
use chrono::NaiveDateTime;
use dbkit::{model, Order};

#[model(table = "records")]
pub struct Record {
    #[key]
    pub id: i64,
    pub left_value: i32,
    pub right_value: i32,
    pub baseline_value: i32,
    pub occurred_at: NaiveDateTime,
}

fn main() {
    let cutoff = chrono::DateTime::from_timestamp(1_700_000_000, 0)
        .expect("cutoff")
        .naive_utc();

    let _query = Record::query()
        .filter((Record::left_value + 1_i32).lt_col(Record::baseline_value))
        .filter((Record::right_value - Record::left_value).ge(0_i32))
        .filter((Record::occurred_at + dbkit::interval::hours(1_i32)).le(cutoff))
        .order_by(Order::desc(Record::baseline_value + Record::left_value))
        .order_by(Order::asc(Record::occurred_at - dbkit::interval::hours(Record::left_value)))
        .limit(25)
        .debug_sql();

    let _projection = Record::query()
        .select_only()
        .column_as(Record::baseline_value + Record::left_value, "computed_value")
        .debug_sql();
}
