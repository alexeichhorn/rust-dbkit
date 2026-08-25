use chrono::{DateTime, NaiveDateTime, Utc};
use dbkit_core::{func, Column, Expr, Order, PgInterval, Select, Table, Value};

#[derive(Debug)]
struct Schedule;

fn schedule_table() -> Table {
    Table::new("schedules")
}

fn schedule_base_interval_hours() -> Column<Schedule, i32> {
    Column::new(schedule_table(), "base_interval_hours")
}

fn schedule_backoff_minutes() -> Column<Schedule, i32> {
    Column::new(schedule_table(), "backoff_minutes")
}

fn schedule_optional_interval_units() -> Column<Schedule, Option<i32>> {
    Column::new(schedule_table(), "optional_interval_units")
}

fn schedule_optional_seconds() -> Column<Schedule, Option<f64>> {
    Column::new(schedule_table(), "optional_seconds")
}

fn schedule_retry_interval() -> Column<Schedule, dbkit_core::PgInterval> {
    Column::new(schedule_table(), "retry_interval")
}

#[test]
fn compiles_interval_hours_with_literal() {
    let query: Select<Schedule> = Select::new(schedule_table())
        .select_only()
        .column_as(dbkit_core::interval::hours(6_i32), "lease_window");

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT MAKE_INTERVAL(hours => $1) AS lease_window FROM schedules");
    assert_eq!(sql.binds, vec![Value::I32(6)]);
}

#[test]
fn compiles_interval_hours_with_column() {
    let query: Select<Schedule> = Select::new(schedule_table())
        .select_only()
        .column_as(dbkit_core::interval::hours(schedule_base_interval_hours()), "lease_window");

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT MAKE_INTERVAL(hours => schedules.base_interval_hours) AS lease_window FROM schedules"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_interval_hours_with_nested_expression_part() {
    let query: Select<Schedule> = Select::new(schedule_table()).select_only().column_as(
        dbkit_core::interval::hours(func::coalesce(schedule_base_interval_hours(), 24_i32)),
        "lease_window",
    );

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT MAKE_INTERVAL(hours => COALESCE(schedules.base_interval_hours, $1)) AS lease_window FROM schedules"
    );
    assert_eq!(sql.binds, vec![Value::I32(24)]);
}

#[test]
fn compiles_interval_minutes_with_negative_literal() {
    let query: Select<Schedule> = Select::new(schedule_table())
        .select_only()
        .column_as(dbkit_core::interval::minutes(-30_i32), "retry_after");

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT MAKE_INTERVAL(mins => $1) AS retry_after FROM schedules");
    assert_eq!(sql.binds, vec![Value::I32(-30)]);
}

#[test]
fn compiles_interval_days_with_literal() {
    let query: Select<Schedule> = Select::new(schedule_table())
        .select_only()
        .column_as(dbkit_core::interval::days(2_i32), "cooldown");

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT MAKE_INTERVAL(days => $1) AS cooldown FROM schedules");
    assert_eq!(sql.binds, vec![Value::I32(2)]);
}

#[test]
fn compiles_interval_seconds_with_fractional_literal() {
    let query: Select<Schedule> = Select::new(schedule_table())
        .select_only()
        .column_as(dbkit_core::interval::seconds(1.5_f64), "jitter");

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT MAKE_INTERVAL(secs => $1) AS jitter FROM schedules");
    assert_eq!(sql.binds, vec![Value::F64(1.5)]);
}

#[test]
fn compiles_interval_constructors_with_nullable_columns() {
    let days: Expr<Option<PgInterval>> = dbkit_core::interval::days(schedule_optional_interval_units());
    let hours: Expr<Option<PgInterval>> = dbkit_core::interval::hours(schedule_optional_interval_units());
    let minutes: Expr<Option<PgInterval>> = dbkit_core::interval::minutes(schedule_optional_interval_units());
    let seconds: Expr<Option<PgInterval>> = dbkit_core::interval::seconds(schedule_optional_seconds());

    let query: Select<Schedule> = Select::new(schedule_table())
        .select_only()
        .column_as(days, "days")
        .column_as(hours, "hours")
        .column_as(minutes, "minutes")
        .column_as(seconds, "seconds");

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT MAKE_INTERVAL(days => schedules.optional_interval_units) AS days, \
         MAKE_INTERVAL(hours => schedules.optional_interval_units) AS hours, \
         MAKE_INTERVAL(mins => schedules.optional_interval_units) AS minutes, \
         MAKE_INTERVAL(secs => schedules.optional_seconds) AS seconds FROM schedules"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_timestamp_literals_with_nullable_interval_expressions() {
    let utc: DateTime<Utc> = DateTime::from_timestamp(1_700_000_000, 0).expect("timestamp");
    let naive = utc.naive_utc();

    let naive_added: Expr<Option<NaiveDateTime>> = naive + dbkit_core::interval::hours(schedule_optional_interval_units());
    let naive_subtracted: Expr<Option<NaiveDateTime>> = naive - dbkit_core::interval::hours(schedule_optional_interval_units());
    let utc_added: Expr<Option<DateTime<Utc>>> = utc + dbkit_core::interval::hours(schedule_optional_interval_units());
    let utc_subtracted: Expr<Option<DateTime<Utc>>> = utc - dbkit_core::interval::hours(schedule_optional_interval_units());

    let query: Select<Schedule> = Select::new(schedule_table())
        .select_only()
        .column_as(naive_added, "naive_added")
        .column_as(naive_subtracted, "naive_subtracted")
        .column_as(utc_added, "utc_added")
        .column_as(utc_subtracted, "utc_subtracted");

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT ($1 + MAKE_INTERVAL(hours => schedules.optional_interval_units)) AS naive_added, \
         ($1 - MAKE_INTERVAL(hours => schedules.optional_interval_units)) AS naive_subtracted, \
         ($2 + MAKE_INTERVAL(hours => schedules.optional_interval_units)) AS utc_added, \
         ($2 - MAKE_INTERVAL(hours => schedules.optional_interval_units)) AS utc_subtracted FROM schedules"
    );
    assert_eq!(sql.binds, vec![Value::DateTime(naive), Value::DateTimeUtc(utc)]);
}

#[test]
fn interval_expression_can_compare_to_interval_columns() {
    let query: Select<Schedule> =
        Select::new(schedule_table()).filter(dbkit_core::interval::hours(1_i32).eq_col(schedule_retry_interval()));

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT schedules.* FROM schedules WHERE (MAKE_INTERVAL(hours => $1) = schedules.retry_interval)"
    );
    assert_eq!(sql.binds, vec![Value::I32(1)]);
}

#[test]
fn interval_expression_can_be_used_in_order_by() {
    let query: Select<Schedule> = Select::new(schedule_table())
        .order_by(Order::asc(dbkit_core::interval::minutes(schedule_backoff_minutes())))
        .limit(10);

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT schedules.* FROM schedules ORDER BY MAKE_INTERVAL(mins => schedules.backoff_minutes) ASC LIMIT 10"
    );
    assert!(sql.binds.is_empty());
}
