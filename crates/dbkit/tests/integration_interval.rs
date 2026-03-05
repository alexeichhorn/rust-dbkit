#![allow(non_upper_case_globals)]

use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{model, Database, Executor, Order};

#[model(table = "interval_rows")]
pub struct IntervalRow {
    #[key]
    pub id: i64,
    pub base_interval_hours: i32,
    pub backoff_minutes: Option<i32>,
    pub lease_window: dbkit::PgInterval,
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn setup_schema<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    ex.execute(
        "CREATE TEMP TABLE interval_rows (\
            id BIGINT PRIMARY KEY,\
            base_interval_hours INTEGER NOT NULL,\
            backoff_minutes INTEGER NULL,\
            lease_window INTERVAL NOT NULL\
        )",
        PgArguments::default(),
    )
    .await?;

    ex.execute(
        "INSERT INTO interval_rows (id, base_interval_hours, backoff_minutes, lease_window) VALUES \
            (1, 2, NULL, MAKE_INTERVAL(hours => 2)),\
            (2, 3, NULL, MAKE_INTERVAL(hours => 3)),\
            (3, 99, 30, MAKE_INTERVAL(mins => 30)),\
            (4, 99, NULL, MAKE_INTERVAL(mins => 15))",
        PgArguments::default(),
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn interval_hours_equality_matches_interval_columns() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let exact_two_hours = IntervalRow::query()
        .filter(dbkit::interval::hours(2_i32).eq_col(IntervalRow::lease_window))
        .all(&tx)
        .await?;
    assert_eq!(exact_two_hours.len(), 1);
    assert_eq!(exact_two_hours[0].id, 1);

    let matches_own_hour_column = IntervalRow::query()
        .filter(dbkit::interval::hours(IntervalRow::base_interval_hours).eq_col(IntervalRow::lease_window))
        .order_by(Order::asc(IntervalRow::id))
        .all(&tx)
        .await?;
    let ids: Vec<i64> = matches_own_hour_column.iter().map(|row| row.id).collect();
    assert_eq!(ids, vec![1, 2]);

    Ok(())
}

#[tokio::test]
async fn interval_minutes_with_coalesce_matches_expected_rows() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let matched = IntervalRow::query()
        .filter(dbkit::interval::minutes(dbkit::func::coalesce(IntervalRow::backoff_minutes, 15_i32)).eq_col(IntervalRow::lease_window))
        .order_by(Order::asc(IntervalRow::id))
        .all(&tx)
        .await?;

    let ids: Vec<i64> = matched.iter().map(|row| row.id).collect();
    assert_eq!(ids, vec![3, 4]);

    Ok(())
}

#[tokio::test]
async fn interval_helpers_can_drive_ordering() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let ordered = IntervalRow::query()
        .filter(IntervalRow::id.in_([1_i64, 2_i64]))
        .order_by(Order::asc(dbkit::interval::hours(IntervalRow::base_interval_hours)))
        .all(&tx)
        .await?;

    let ids: Vec<i64> = ordered.iter().map(|row| row.id).collect();
    assert_eq!(ids, vec![1, 2]);

    Ok(())
}
