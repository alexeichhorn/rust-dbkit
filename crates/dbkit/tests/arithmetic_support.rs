#![allow(non_upper_case_globals)]

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

#[model(table = "compact_records")]
pub struct CompactRecord {
    #[key]
    pub id: i64,
    pub left_units: i16,
    pub right_units: i16,
}

#[test]
fn query_with_numeric_arithmetic_has_expected_sql_shape() {
    let sql = Record::query()
        .filter((Record::left_value + 1_i32).lt_col(Record::baseline_value))
        .filter((Record::right_value - Record::left_value).gt(0_i32))
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
        .filter((Record::occurred_at + dbkit::interval::hours(Record::left_value)).le(cutoff))
        .order_by(Order::asc(Record::occurred_at - dbkit::interval::hours(1_i32)))
        .debug_sql();

    assert!(
        sql.contains("(records.occurred_at + MAKE_INTERVAL(hours => records.left_value)) <= $1"),
        "unexpected SQL: {sql}"
    );
    assert!(
        sql.contains("ORDER BY (records.occurred_at - MAKE_INTERVAL(hours => $2)) ASC"),
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

#[test]
fn smallint_arithmetic_promotes_filters_to_integer_operands() {
    // This pins the Postgres result type, not just the SQL text shape.
    let sql = CompactRecord::query()
        .filter((CompactRecord::left_units + CompactRecord::right_units).gt(9_i32))
        .filter((CompactRecord::left_units - CompactRecord::right_units).lt(4_i32))
        .debug_sql();

    assert!(
        sql.contains("(compact_records.left_units + compact_records.right_units) > $1"),
        "unexpected SQL: {sql}"
    );
    assert!(
        sql.contains("(compact_records.left_units - compact_records.right_units) < $2"),
        "unexpected SQL: {sql}"
    );
}

#[test]
fn smallint_arithmetic_projection_is_usable_as_integer_expression() {
    // If this assignment fails, the DSL is still treating SMALLINT arithmetic as i16.
    let total_units: dbkit::Expr<i32> = CompactRecord::left_units + CompactRecord::right_units;

    let sql = CompactRecord::query()
        .select_only()
        .column_as(total_units, "total_units")
        .order_by(Order::desc(CompactRecord::left_units - CompactRecord::right_units))
        .debug_sql();

    assert!(
        sql.contains("SELECT (compact_records.left_units + compact_records.right_units) AS total_units FROM compact_records"),
        "unexpected SQL: {sql}"
    );
    assert!(
        sql.contains("ORDER BY (compact_records.left_units - compact_records.right_units) DESC"),
        "unexpected SQL: {sql}"
    );
}
