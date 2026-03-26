use chrono::NaiveDateTime;
use dbkit_core::{func, interval, Column, Order, Select, Table};

#[derive(Debug)]
struct WorkRun;

fn work_runs_table() -> Table {
    Table::new("work_runs")
}

fn work_run_status() -> Column<WorkRun, String> {
    Column::new(work_runs_table(), "status")
}

fn work_run_attempts() -> Column<WorkRun, i32> {
    Column::new(work_runs_table(), "attempts")
}

fn work_run_updated_at() -> Column<WorkRun, NaiveDateTime> {
    Column::new(work_runs_table(), "updated_at")
}

fn work_run_created_at() -> Column<WorkRun, NaiveDateTime> {
    Column::new(work_runs_table(), "created_at")
}

#[test]
fn compiles_query_with_dynamic_interval_math() {
    let now = chrono::DateTime::from_timestamp(1_700_000_000, 0)
        .expect("now")
        .naive_utc();
    let stale_timeout_seconds = 300_i64;
    let retry_base_seconds = 60.0_f64;
    let retry_cap_seconds = 3_600.0_f64;

    let retry_exponent = func::least(
        func::greatest(work_run_attempts() - 1_i32, 0_i32),
        10_i32,
    );
    let retry_seconds = func::least(
        retry_cap_seconds,
        func::power(2.0_f64, retry_exponent) * retry_base_seconds,
    );

    let sql = Select::<WorkRun>::new(work_runs_table())
        .filter(
            work_run_status()
                .eq("pending")
                .or(
                    work_run_status()
                        .eq("running")
                        .and(work_run_updated_at().le(now - interval::seconds(stale_timeout_seconds))),
                )
                .or(
                    work_run_status()
                        .eq("failed")
                        .and(work_run_updated_at().le(now - interval::seconds(retry_seconds))),
                ),
        )
        .order_by(Order::asc(work_run_attempts()))
        .order_by(Order::asc(work_run_created_at()))
        .for_update()
        .skip_locked()
        .limit(1)
        .compile();

    assert!(
        sql.sql.contains("FROM work_runs"),
        "unexpected SQL: {}",
        sql.sql
    );
    assert!(
        sql.sql.contains("MAKE_INTERVAL(secs => $"),
        "unexpected SQL: {}",
        sql.sql
    );
    assert!(
        sql.sql.contains("MAKE_INTERVAL(secs => LEAST("),
        "unexpected SQL: {}",
        sql.sql
    );
    assert!(
        sql.sql.contains("POWER($"),
        "unexpected SQL: {}",
        sql.sql
    );
    assert!(
        sql.sql.contains("GREATEST((work_runs.attempts - $"),
        "unexpected SQL: {}",
        sql.sql
    );
    assert!(
        sql.sql.contains("ORDER BY work_runs.attempts ASC, work_runs.created_at ASC"),
        "unexpected SQL: {}",
        sql.sql
    );
    assert!(
        sql.sql.ends_with("FOR UPDATE SKIP LOCKED"),
        "unexpected SQL: {}",
        sql.sql
    );
}
