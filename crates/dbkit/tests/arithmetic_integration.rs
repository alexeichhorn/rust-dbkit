#![allow(non_upper_case_globals)]

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{model, Database, Executor, Expr, ExprNode, IntoExpr, Order};

#[model(table = "records")]
pub struct Record {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub left_value: i64,
    pub right_value: i64,
    pub baseline_value: i64,
    pub occurred_at: NaiveDateTime,
}

#[derive(Debug, Clone, Copy)]
struct OffsetValue;

impl dbkit::SqlInterval for OffsetValue {}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

fn make_offset(arg: impl IntoExpr<i64>) -> Expr<OffsetValue> {
    let expr = arg.into_expr();
    Expr::new(ExprNode::Func {
        name: "pg_temp.make_offset",
        args: vec![expr.node],
    })
}

async fn setup_schema<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    ex.execute(
        "CREATE OR REPLACE FUNCTION pg_temp.make_offset(value BIGINT) \
         RETURNS INTERVAL \
         LANGUAGE SQL \
         IMMUTABLE \
         AS $$ SELECT make_interval(hours => value::int) $$;",
        PgArguments::default(),
    )
    .await?;

    ex.execute(
        "CREATE TEMP TABLE records (\
            id BIGSERIAL PRIMARY KEY,\
            left_value BIGINT NOT NULL,\
            right_value BIGINT NOT NULL,\
            baseline_value BIGINT NOT NULL,\
            occurred_at TIMESTAMP NOT NULL\
        )",
        PgArguments::default(),
    )
    .await?;

    Ok(())
}

async fn seed_record<E: Executor + Send + Sync>(
    ex: &E,
    left_value: i64,
    right_value: i64,
    baseline_value: i64,
    occurred_at: NaiveDateTime,
) -> Result<Record, dbkit::Error> {
    let row = Record::insert(RecordInsert {
        left_value,
        right_value,
        baseline_value,
        occurred_at,
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted record");
    Ok(row)
}

#[tokio::test]
async fn arithmetic_numeric_filters_and_ordering_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let day = NaiveDate::from_ymd_opt(2024, 4, 1).expect("day");
    let row1 = seed_record(
        &tx,
        2,
        8,
        10,
        NaiveDateTime::new(day, NaiveTime::from_hms_opt(9, 0, 0).expect("time")),
    )
    .await?;
    let row2 = seed_record(
        &tx,
        4,
        9,
        20,
        NaiveDateTime::new(day, NaiveTime::from_hms_opt(10, 0, 0).expect("time")),
    )
    .await?;
    let _row3 = seed_record(
        &tx,
        7,
        8,
        8,
        NaiveDateTime::new(day, NaiveTime::from_hms_opt(11, 0, 0).expect("time")),
    )
    .await?;

    let rows = Record::query()
        .filter((Record::left_value + 1_i64).lt_col(Record::baseline_value))
        .filter((Record::right_value - Record::left_value).gt(3_i64))
        .order_by(Order::desc(Record::baseline_value + Record::left_value))
        .all(&tx)
        .await?;

    let ids: Vec<i64> = rows.iter().map(|row| row.id).collect();
    assert_eq!(ids, vec![row2.id, row1.id]);

    Ok(())
}

#[tokio::test]
async fn arithmetic_temporal_offset_filter_and_ordering_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let day = NaiveDate::from_ymd_opt(2024, 4, 2).expect("day");
    let base = NaiveDateTime::new(day, NaiveTime::from_hms_opt(8, 0, 0).expect("time"));
    let row1 = seed_record(&tx, 1, 0, 0, base).await?;
    let row2 = seed_record(&tx, 3, 0, 0, base + Duration::hours(1)).await?;
    let _row3 = seed_record(&tx, 5, 0, 0, base + Duration::hours(4)).await?;

    let cutoff = base + Duration::hours(4);
    let rows = Record::query()
        .filter((Record::occurred_at + make_offset(Record::left_value)).le(cutoff))
        .order_by(Order::asc(Record::occurred_at - make_offset(1_i64)))
        .all(&tx)
        .await?;

    let ids: Vec<i64> = rows.iter().map(|row| row.id).collect();
    assert_eq!(ids, vec![row1.id, row2.id]);

    Ok(())
}
