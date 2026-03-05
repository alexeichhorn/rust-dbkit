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
    pub left_units: i16,
    pub right_units: i16,
    pub occurred_at: NaiveDateTime,
}

fn main() {
    let cutoff = chrono::DateTime::from_timestamp(1_700_000_000, 0)
        .expect("cutoff")
        .naive_utc();
    // PostgreSQL promotes SMALLINT +/- SMALLINT to INTEGER.
    let _promoted_sum: dbkit::Expr<i32> = Record::left_units + Record::right_units;
    let _promoted_delta: dbkit::Expr<i32> = Record::left_units - Record::right_units;

    let _query = Record::query()
        .filter((Record::left_value + 1_i32).lt_col(Record::baseline_value))
        .filter((Record::right_value - Record::left_value).ge(0_i32))
        .filter((Record::left_units + Record::right_units).gt(3_i32))
        .filter((Record::left_units - Record::right_units).lt(5_i32))
        .filter((Record::occurred_at + dbkit::interval::hours(1_i32)).le(cutoff))
        .order_by(Order::desc(Record::baseline_value + Record::left_value))
        .order_by(Order::desc(Record::left_units + Record::right_units))
        .order_by(Order::asc(Record::occurred_at - dbkit::interval::hours(Record::left_value)))
        .limit(25)
        .debug_sql();

    let _projection = Record::query()
        .select_only()
        .column_as(Record::baseline_value + Record::left_value, "computed_value")
        .column_as(Record::left_units + Record::right_units, "total_units")
        .debug_sql();
}
