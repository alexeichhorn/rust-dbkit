#![allow(non_upper_case_globals)]

use chrono::NaiveDateTime;
use dbkit::{func, interval, model, Expr, Order};

#[model(table = "work_runs")]
pub struct WorkRun {
    #[key]
    pub id: i64,
    pub status: String,
    pub attempts: i32,
    pub updated_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

fn due_window_filter(now: NaiveDateTime, stale_timeout_seconds: i64, retry_base_seconds: f64, retry_cap_seconds: f64) -> Expr<bool> {
    let retry_exponent: Expr<i32> = func::least(func::greatest(WorkRun::attempts - 1_i32, 0_i32), 10_i32);
    let retry_seconds: Expr<f64> = func::least(retry_cap_seconds, func::power(2.0_f64, retry_exponent.clone()) * retry_base_seconds);

    WorkRun::status
        .eq("pending")
        .or(WorkRun::status
            .eq("running")
            .and(WorkRun::updated_at.le(now - interval::seconds(stale_timeout_seconds))))
        .or(WorkRun::status
            .eq("failed")
            .and(WorkRun::updated_at.le(now - interval::seconds(retry_seconds))))
}

#[test]
fn query_with_dynamic_interval_math_has_expected_sql_shape() {
    let now = chrono::DateTime::from_timestamp(1_700_000_000, 0).expect("now").naive_utc();

    let sql = WorkRun::query()
        .filter(due_window_filter(now, 300_i64, 60.0_f64, 3_600.0_f64))
        .order_by(Order::asc(WorkRun::attempts))
        .order_by(Order::asc(WorkRun::created_at))
        .for_update()
        .skip_locked()
        .limit(1)
        .debug_sql();

    assert!(sql.contains("FROM work_runs"), "unexpected SQL: {sql}");
    assert!(sql.contains("LEAST("), "unexpected SQL: {sql}");
    assert!(sql.contains("GREATEST("), "unexpected SQL: {sql}");
    assert!(sql.contains("POWER("), "unexpected SQL: {sql}");
    assert!(sql.contains("MAKE_INTERVAL(secs =>"), "unexpected SQL: {sql}");
    assert!(
        sql.contains("ORDER BY work_runs.attempts ASC, work_runs.created_at ASC"),
        "unexpected SQL: {sql}"
    );
    assert!(sql.ends_with("FOR UPDATE SKIP LOCKED"), "unexpected SQL: {sql}");
}

#[test]
fn dynamic_interval_math_api_preserves_expected_expression_types() {
    let retry_exponent: Expr<i32> = func::least(func::greatest(WorkRun::attempts - 1_i32, 0_i32), 10_i32);
    let retry_seconds: Expr<f64> = func::power(2.0_f64, retry_exponent.clone()) * 60.0_f64;
    let _stale_window = interval::seconds(300_i64);
    let _retry_window = interval::seconds(retry_seconds);
}
