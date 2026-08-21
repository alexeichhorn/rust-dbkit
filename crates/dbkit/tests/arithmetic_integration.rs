#![allow(non_upper_case_globals)]

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{model, Database, Executor, Order, PgInterval};

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

#[model(table = "nullable_arithmetic_records")]
pub struct NullableArithmeticRecord {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub required_value: i32,
    pub nullable_left: Option<i32>,
    pub nullable_right: Option<i32>,
    pub required_at: NaiveDateTime,
    pub nullable_at: Option<NaiveDateTime>,
    pub required_interval: PgInterval,
    pub nullable_interval: Option<PgInterval>,
}

#[derive(Debug, sqlx::FromRow)]
struct NullableArithmeticResult {
    added: Option<i32>,
    subtracted: Option<i32>,
    multiplied: Option<i32>,
    literal_minus_nullable: Option<i32>,
    nullable_time_required_interval: Option<NaiveDateTime>,
    required_time_nullable_interval: Option<NaiveDateTime>,
    nullable_time_nullable_interval: Option<NaiveDateTime>,
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

async fn setup_nullable_arithmetic_schema<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    ex.execute(
        "CREATE TEMP TABLE nullable_arithmetic_records (\
            id BIGSERIAL PRIMARY KEY,\
            required_value INTEGER NOT NULL,\
            nullable_left INTEGER,\
            nullable_right INTEGER,\
            required_at TIMESTAMP NOT NULL,\
            nullable_at TIMESTAMP,\
            required_interval INTERVAL NOT NULL,\
            nullable_interval INTERVAL\
        )",
        PgArguments::default(),
    )
    .await?;

    Ok(())
}

fn hours(value: i64) -> PgInterval {
    PgInterval {
        months: 0,
        days: 0,
        microseconds: value * 3_600_000_000,
    }
}

async fn seed_compact_record<E: Executor + Send + Sync>(ex: &E, left_units: i16, right_units: i16) -> Result<CompactRecord, dbkit::Error> {
    let row = CompactRecord::insert(CompactRecordInsert { left_units, right_units })
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

#[tokio::test]
async fn nullable_arithmetic_propagates_null_from_either_operand() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_nullable_arithmetic_schema(&tx).await?;

    let day = NaiveDate::from_ymd_opt(2024, 4, 3).expect("day");
    let base = NaiveDateTime::new(day, NaiveTime::from_hms_opt(12, 0, 0).expect("time"));

    NullableArithmeticRecord::insert_many(vec![
        NullableArithmeticRecordInsert {
            required_value: 10,
            nullable_left: Some(4),
            nullable_right: Some(2),
            required_at: base,
            nullable_at: Some(base),
            required_interval: hours(1),
            nullable_interval: Some(hours(2)),
        },
        NullableArithmeticRecordInsert {
            required_value: 10,
            nullable_left: None,
            nullable_right: Some(2),
            required_at: base,
            nullable_at: None,
            required_interval: hours(1),
            nullable_interval: Some(hours(2)),
        },
        NullableArithmeticRecordInsert {
            required_value: 10,
            nullable_left: Some(4),
            nullable_right: None,
            required_at: base,
            nullable_at: Some(base),
            required_interval: hours(1),
            nullable_interval: None,
        },
        NullableArithmeticRecordInsert {
            required_value: 10,
            nullable_left: None,
            nullable_right: None,
            required_at: base,
            nullable_at: None,
            required_interval: hours(1),
            nullable_interval: None,
        },
    ])
    .execute(&tx)
    .await?;

    let rows: Vec<NullableArithmeticResult> = NullableArithmeticRecord::query()
        .select_only()
        .column_as(
            NullableArithmeticRecord::nullable_left + NullableArithmeticRecord::required_value,
            "added",
        )
        .column_as(
            NullableArithmeticRecord::required_value - NullableArithmeticRecord::nullable_right,
            "subtracted",
        )
        .column_as(
            NullableArithmeticRecord::nullable_left * NullableArithmeticRecord::nullable_right,
            "multiplied",
        )
        .column_as(100 - NullableArithmeticRecord::nullable_left, "literal_minus_nullable")
        .column_as(
            NullableArithmeticRecord::nullable_at + NullableArithmeticRecord::required_interval,
            "nullable_time_required_interval",
        )
        .column_as(
            NullableArithmeticRecord::required_at - NullableArithmeticRecord::nullable_interval,
            "required_time_nullable_interval",
        )
        .column_as(
            NullableArithmeticRecord::nullable_at + NullableArithmeticRecord::nullable_interval,
            "nullable_time_nullable_interval",
        )
        .order_by(Order::asc(NullableArithmeticRecord::id))
        .into_model()
        .all(&tx)
        .await?;

    assert_eq!(rows.len(), 4);

    assert_eq!(rows[0].added, Some(14));
    assert_eq!(rows[0].subtracted, Some(8));
    assert_eq!(rows[0].multiplied, Some(8));
    assert_eq!(rows[0].literal_minus_nullable, Some(96));
    assert_eq!(rows[0].nullable_time_required_interval, Some(base + Duration::hours(1)));
    assert_eq!(rows[0].required_time_nullable_interval, Some(base - Duration::hours(2)));
    assert_eq!(rows[0].nullable_time_nullable_interval, Some(base + Duration::hours(2)));

    assert_eq!(rows[1].added, None);
    assert_eq!(rows[1].subtracted, Some(8));
    assert_eq!(rows[1].multiplied, None);
    assert_eq!(rows[1].literal_minus_nullable, None);
    assert_eq!(rows[1].nullable_time_required_interval, None);
    assert_eq!(rows[1].required_time_nullable_interval, Some(base - Duration::hours(2)));
    assert_eq!(rows[1].nullable_time_nullable_interval, None);

    assert_eq!(rows[2].added, Some(14));
    assert_eq!(rows[2].subtracted, None);
    assert_eq!(rows[2].multiplied, None);
    assert_eq!(rows[2].literal_minus_nullable, Some(96));
    assert_eq!(rows[2].nullable_time_required_interval, Some(base + Duration::hours(1)));
    assert_eq!(rows[2].required_time_nullable_interval, None);
    assert_eq!(rows[2].nullable_time_nullable_interval, None);

    assert_eq!(rows[3].added, None);
    assert_eq!(rows[3].subtracted, None);
    assert_eq!(rows[3].multiplied, None);
    assert_eq!(rows[3].literal_minus_nullable, None);
    assert_eq!(rows[3].nullable_time_required_interval, None);
    assert_eq!(rows[3].required_time_nullable_interval, None);
    assert_eq!(rows[3].nullable_time_nullable_interval, None);

    let matching = NullableArithmeticRecord::query()
        .filter((100 - NullableArithmeticRecord::nullable_left).eq(96_i32))
        .filter((NullableArithmeticRecord::nullable_left + NullableArithmeticRecord::required_value).eq(14_i32))
        .filter((NullableArithmeticRecord::nullable_at + NullableArithmeticRecord::required_interval).eq(base + Duration::hours(1)))
        .order_by(Order::asc(NullableArithmeticRecord::id))
        .all(&tx)
        .await?;
    assert_eq!(matching.iter().map(|row| row.id).collect::<Vec<_>>(), vec![1, 3]);

    let null_results = NullableArithmeticRecord::query()
        .filter((100 - NullableArithmeticRecord::nullable_left).is_null())
        .order_by(Order::asc(NullableArithmeticRecord::id))
        .all(&tx)
        .await?;
    assert_eq!(null_results.iter().map(|row| row.id).collect::<Vec<_>>(), vec![2, 4]);

    Ok(())
}
