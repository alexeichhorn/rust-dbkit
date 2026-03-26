#![allow(non_upper_case_globals)]

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{func, interval, model, Database, Executor, Expr, Order};

#[model(table = "work_runs")]
pub struct WorkRun {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub status: String,
    pub attempts: i32,
    pub updated_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

fn due_window_filter(
    now: NaiveDateTime,
    stale_timeout_seconds: i64,
    retry_base_seconds: f64,
    retry_cap_seconds: f64,
) -> Expr<bool> {
    let retry_exponent: Expr<i32> = func::least(func::greatest(WorkRun::attempts - 1_i32, 0_i32), 10_i32);
    let retry_seconds: Expr<f64> =
        func::least(retry_cap_seconds, func::power(2.0_f64, retry_exponent.clone()) * retry_base_seconds);

    WorkRun::status
        .eq("pending")
        .or(
            WorkRun::status
                .eq("running")
                .and(WorkRun::updated_at.le(now - interval::seconds(stale_timeout_seconds))),
        )
        .or(
            WorkRun::status
                .eq("failed")
                .and(WorkRun::updated_at.le(now - interval::seconds(retry_seconds))),
        )
}

async fn setup_schema<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    ex.execute(
        "CREATE TEMP TABLE work_runs (\
            id BIGSERIAL PRIMARY KEY,\
            status TEXT NOT NULL,\
            attempts INTEGER NOT NULL,\
            updated_at TIMESTAMP NOT NULL,\
            created_at TIMESTAMP NOT NULL\
        )",
        PgArguments::default(),
    )
    .await?;

    Ok(())
}

async fn seed_run<E: Executor + Send + Sync>(
    ex: &E,
    status: &str,
    attempts: i32,
    updated_at: NaiveDateTime,
    created_at: NaiveDateTime,
) -> Result<WorkRun, dbkit::Error> {
    let row = WorkRun::insert(WorkRunInsert {
        status: status.to_string(),
        attempts,
        updated_at,
        created_at,
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted run");
    Ok(row)
}

#[tokio::test]
async fn query_with_dynamic_interval_math_prefers_lowest_attempt_due_row() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let day = NaiveDate::from_ymd_opt(2024, 4, 3).expect("day");
    let now = NaiveDateTime::new(day, NaiveTime::from_hms_opt(12, 0, 0).expect("time"));

    let _pending = seed_run(&tx, "pending", 5, now - Duration::seconds(5), now - Duration::minutes(30)).await?;
    let _stale_running = seed_run(&tx, "running", 4, now - Duration::minutes(10), now - Duration::minutes(40)).await?;
    let failed_due = seed_run(&tx, "failed", 3, now - Duration::minutes(5), now - Duration::minutes(20)).await?;
    let _failed_not_due = seed_run(&tx, "failed", 2, now - Duration::seconds(90), now - Duration::minutes(10)).await?;
    let _fresh_running = seed_run(&tx, "running", 0, now - Duration::seconds(45), now - Duration::minutes(15)).await?;

    let claimed = WorkRun::query()
        .filter(due_window_filter(now, 300_i64, 60.0_f64, 3_600.0_f64))
        .order_by(Order::asc(WorkRun::attempts))
        .order_by(Order::asc(WorkRun::created_at))
        .for_update()
        .skip_locked()
        .limit(1)
        .one(&tx)
        .await?
        .expect("claimed run");

    assert_eq!(claimed.id, failed_due.id);

    Ok(())
}

#[tokio::test]
async fn query_with_dynamic_interval_math_matches_stale_rows_from_integer_second_windows() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let day = NaiveDate::from_ymd_opt(2024, 4, 4).expect("day");
    let now = NaiveDateTime::new(day, NaiveTime::from_hms_opt(9, 0, 0).expect("time"));

    let stale_running = seed_run(&tx, "running", 1, now - Duration::minutes(8), now - Duration::minutes(20)).await?;
    let _fresh_running = seed_run(&tx, "running", 0, now - Duration::seconds(45), now - Duration::minutes(15)).await?;
    let _failed_not_due = seed_run(&tx, "failed", 4, now - Duration::minutes(3), now - Duration::minutes(10)).await?;

    let claimed = WorkRun::query()
        .filter(due_window_filter(now, 300_i64, 60.0_f64, 3_600.0_f64))
        .order_by(Order::asc(WorkRun::attempts))
        .order_by(Order::asc(WorkRun::created_at))
        .for_update()
        .skip_locked()
        .limit(1)
        .one(&tx)
        .await?
        .expect("claimed run");

    assert_eq!(claimed.id, stale_running.id);

    Ok(())
}
