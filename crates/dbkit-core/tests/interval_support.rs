use dbkit_core::{func, Column, Order, Select, Table, Value};

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
    let query: Select<Schedule> = Select::new(schedule_table())
        .select_only()
        .column_as(dbkit_core::interval::hours(func::coalesce(schedule_base_interval_hours(), 24_i32)), "lease_window");

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT MAKE_INTERVAL(hours => COALESCE(schedules.base_interval_hours, $1)) AS lease_window FROM schedules"
    );
    assert_eq!(sql.binds, vec![Value::I32(24)]);
}

#[test]
fn compiles_interval_minutes_with_negative_literal() {
    let query: Select<Schedule> = Select::new(schedule_table()).select_only().column_as(
        dbkit_core::interval::minutes(-30_i32),
        "retry_after",
    );

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT MAKE_INTERVAL(mins => $1) AS retry_after FROM schedules");
    assert_eq!(sql.binds, vec![Value::I32(-30)]);
}

#[test]
fn compiles_interval_days_with_literal() {
    let query: Select<Schedule> = Select::new(schedule_table()).select_only().column_as(
        dbkit_core::interval::days(2_i32),
        "cooldown",
    );

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
fn interval_expression_can_compare_to_interval_columns() {
    let query: Select<Schedule> = Select::new(schedule_table()).filter(dbkit_core::interval::hours(1_i32).eq_col(schedule_retry_interval()));

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
