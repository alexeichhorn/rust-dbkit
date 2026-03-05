#![allow(non_upper_case_globals)]

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{model, Database, Executor, Order};

#[model(table = "records")]
pub struct Record {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub left_value: i32,
    pub right_value: i32,
    pub baseline_value: i32,
    pub occurred_at: NaiveDateTime,
}

#[model(table = "compact_records")]
pub struct CompactRecord {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub left_units: i16,
    pub right_units: i16,
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn setup_schema<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    ex.execute(
        "CREATE TEMP TABLE records (\
            id BIGSERIAL PRIMARY KEY,\
            left_value INTEGER NOT NULL,\
            right_value INTEGER NOT NULL,\
            baseline_value INTEGER NOT NULL,\
            occurred_at TIMESTAMP NOT NULL\
        )",
        PgArguments::default(),
    )
    .await?;

    Ok(())
}

async fn seed_record<E: Executor + Send + Sync>(
    ex: &E,
    left_value: i32,
    right_value: i32,
    baseline_value: i32,
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

async fn setup_compact_schema<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    ex.execute(
        "CREATE TEMP TABLE compact_records (\
            id BIGSERIAL PRIMARY KEY,\
            left_units SMALLINT NOT NULL,\
            right_units SMALLINT NOT NULL\
        )",
        PgArguments::default(),
    )
    .await?;

    Ok(())
}

async fn seed_compact_record<E: Executor + Send + Sync>(
    ex: &E,
    left_units: i16,
    right_units: i16,
) -> Result<CompactRecord, dbkit::Error> {
    let row = CompactRecord::insert(CompactRecordInsert {
        left_units,
        right_units,
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted compact record");
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
        .filter((Record::left_value + 1_i32).lt_col(Record::baseline_value))
        .filter((Record::right_value - Record::left_value).gt(3_i32))
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
        .filter((Record::occurred_at + dbkit::interval::hours(Record::left_value)).le(cutoff))
        .order_by(Order::asc(Record::occurred_at - dbkit::interval::hours(1_i32)))
        .all(&tx)
        .await?;

    let ids: Vec<i64> = rows.iter().map(|row| row.id).collect();
    assert_eq!(ids, vec![row1.id, row2.id]);

    Ok(())
}

#[tokio::test]
async fn smallint_arithmetic_filters_roundtrip_with_integer_rhs() -> Result<(), dbkit::Error> {
    // This roundtrip guards the database-facing contract: SMALLINT arithmetic
    // must compose with INTEGER predicates and ordering in real Postgres.
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_compact_schema(&tx).await?;

    let row1 = seed_compact_record(&tx, 5, 5).await?;
    let row2 = seed_compact_record(&tx, 6, 5).await?;
    let _row3 = seed_compact_record(&tx, 8, 5).await?;

    let rows = CompactRecord::query()
        .filter((CompactRecord::left_units + CompactRecord::right_units).gt(9_i32))
        .filter((CompactRecord::left_units - CompactRecord::right_units).lt(2_i32))
        .order_by(Order::asc(CompactRecord::left_units - CompactRecord::right_units))
        .all(&tx)
        .await?;

    let ids: Vec<i64> = rows.iter().map(|row| row.id).collect();
    assert_eq!(ids, vec![row1.id, row2.id]);

    Ok(())
}
