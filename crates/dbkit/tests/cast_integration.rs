use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{model, Database, Executor, PgInterval};
use uuid::Uuid;

#[model(table = "cast_records")]
pub struct CastRecord {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub enabled: bool,
    pub integer_value: i32,
    pub float_value: f64,
    pub numeric_text: String,
    pub uuid_text: String,
    pub external_id: Uuid,
    pub recorded_at: NaiveDateTime,
    pub recorded_at_utc: DateTime<Utc>,
    pub day: NaiveDate,
    pub clock_time: NaiveTime,
    pub elapsed: PgInterval,
    pub nullable_integer: Option<i32>,
    pub nullable_text: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct CastResult {
    integer_to_small: i16,
    integer_to_big: i64,
    integer_to_float: f64,
    float_to_integer: i32,
    integer_to_bool: bool,
    bool_to_integer: i32,
    numeric_text_to_float: f64,
    integer_to_text: String,
    text_to_uuid: Uuid,
    uuid_to_text: String,
    date_to_timestamp: NaiveDateTime,
    date_to_timestamptz: DateTime<Utc>,
    timestamp_to_date: NaiveDate,
    timestamp_to_time: NaiveTime,
    timestamp_to_timestamptz: DateTime<Utc>,
    timestamptz_to_timestamp: NaiveDateTime,
    time_to_interval: PgInterval,
    interval_to_time: NaiveTime,
    nullable_integer_to_float: Option<f64>,
    nullable_text_to_uuid: Option<Uuid>,
}

#[derive(Debug, sqlx::FromRow)]
struct FloatResult {
    value: f64,
}

#[derive(Debug, sqlx::FromRow)]
struct SmallintResult {
    value: i16,
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn setup_schema<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    ex.execute(
        "CREATE TEMP TABLE cast_records (\
            id BIGSERIAL PRIMARY KEY,\
            enabled BOOLEAN NOT NULL,\
            integer_value INTEGER NOT NULL,\
            float_value DOUBLE PRECISION NOT NULL,\
            numeric_text TEXT NOT NULL,\
            uuid_text TEXT NOT NULL,\
            external_id UUID NOT NULL,\
            recorded_at TIMESTAMP NOT NULL,\
            recorded_at_utc TIMESTAMPTZ NOT NULL,\
            day DATE NOT NULL,\
            clock_time TIME NOT NULL,\
            elapsed INTERVAL NOT NULL,\
            nullable_integer INTEGER,\
            nullable_text TEXT\
        )",
        PgArguments::default(),
    )
    .await?;

    Ok(())
}

async fn seed_record<E: Executor + Send + Sync>(ex: &E, numeric_text: &str, float_value: f64) -> Result<CastRecord, dbkit::Error> {
    let external_id = Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").expect("valid UUID");
    let day = NaiveDate::from_ymd_opt(2025, 2, 3).expect("valid date");
    let clock_time = NaiveTime::from_hms_opt(12, 34, 56).expect("valid time");
    let recorded_at = NaiveDateTime::new(day, clock_time);
    let recorded_at_utc = DateTime::<Utc>::from_naive_utc_and_offset(recorded_at, Utc);

    CastRecord::insert(CastRecordInsert {
        enabled: true,
        integer_value: 7,
        float_value,
        numeric_text: numeric_text.to_string(),
        uuid_text: external_id.to_string(),
        external_id,
        recorded_at,
        recorded_at_utc,
        day,
        clock_time,
        elapsed: PgInterval {
            months: 0,
            days: 0,
            microseconds: 2 * 3_600_000_000,
        },
        nullable_integer: None,
        nullable_text: None,
    })
    .returning_all()
    .one(ex)
    .await?
    .ok_or(dbkit::Error::NotFound)
}

fn assert_postgres_code(error: dbkit::Error, expected: &str) {
    match error {
        dbkit::Error::Sqlx(dbkit::sqlx::Error::Database(error)) => {
            assert_eq!(error.code().as_deref(), Some(expected));
        }
        error => panic!("unexpected error: {error}"),
    }
}

#[tokio::test]
async fn casts_roundtrip_across_numeric_text_uuid_and_temporal_types() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    tx.execute("SET LOCAL TIME ZONE 'UTC'", PgArguments::default()).await?;
    setup_schema(&tx).await?;
    let row = seed_record(&tx, "12.5", 2.6).await?;

    let result: CastResult = CastRecord::query()
        .select_only()
        .column_as(CastRecord::integer_value.cast::<i16>(), "integer_to_small")
        .column_as(CastRecord::integer_value.cast::<i64>(), "integer_to_big")
        .column_as(CastRecord::integer_value.cast::<f64>(), "integer_to_float")
        .column_as(CastRecord::float_value.cast::<i32>(), "float_to_integer")
        .column_as(CastRecord::integer_value.cast::<bool>(), "integer_to_bool")
        .column_as(CastRecord::enabled.cast::<i32>(), "bool_to_integer")
        .column_as(CastRecord::numeric_text.cast::<f64>(), "numeric_text_to_float")
        .column_as(CastRecord::integer_value.cast::<String>(), "integer_to_text")
        .column_as(CastRecord::uuid_text.cast::<Uuid>(), "text_to_uuid")
        .column_as(CastRecord::external_id.cast::<String>(), "uuid_to_text")
        .column_as(CastRecord::day.cast::<NaiveDateTime>(), "date_to_timestamp")
        .column_as(CastRecord::day.cast::<DateTime<Utc>>(), "date_to_timestamptz")
        .column_as(CastRecord::recorded_at.cast::<NaiveDate>(), "timestamp_to_date")
        .column_as(CastRecord::recorded_at.cast::<NaiveTime>(), "timestamp_to_time")
        .column_as(CastRecord::recorded_at.cast::<DateTime<Utc>>(), "timestamp_to_timestamptz")
        .column_as(CastRecord::recorded_at_utc.cast::<NaiveDateTime>(), "timestamptz_to_timestamp")
        .column_as(CastRecord::clock_time.cast::<PgInterval>(), "time_to_interval")
        .column_as(CastRecord::elapsed.cast::<NaiveTime>(), "interval_to_time")
        .column_as(CastRecord::nullable_integer.cast::<f64>(), "nullable_integer_to_float")
        .column_as(CastRecord::nullable_text.cast::<Uuid>(), "nullable_text_to_uuid")
        .filter(CastRecord::id.eq(row.id))
        .into_model()
        .one(&tx)
        .await?
        .ok_or(dbkit::Error::NotFound)?;

    assert_eq!(result.integer_to_small, 7);
    assert_eq!(result.integer_to_big, 7);
    assert_eq!(result.integer_to_float, 7.0);
    assert_eq!(result.float_to_integer, 3);
    assert!(result.integer_to_bool);
    assert_eq!(result.bool_to_integer, 1);
    assert_eq!(result.numeric_text_to_float, 12.5);
    assert_eq!(result.integer_to_text, "7");
    assert_eq!(result.text_to_uuid, row.external_id);
    assert_eq!(result.uuid_to_text, row.external_id.to_string());
    assert_eq!(result.date_to_timestamp, row.day.and_hms_opt(0, 0, 0).expect("midnight"));
    assert_eq!(
        result.date_to_timestamptz,
        DateTime::<Utc>::from_naive_utc_and_offset(row.day.and_hms_opt(0, 0, 0).expect("midnight"), Utc)
    );
    assert_eq!(result.timestamp_to_date, row.day);
    assert_eq!(result.timestamp_to_time, row.clock_time);
    assert_eq!(result.timestamp_to_timestamptz, row.recorded_at_utc);
    assert_eq!(result.timestamptz_to_timestamp, row.recorded_at);
    assert_eq!(
        result.time_to_interval,
        PgInterval {
            months: 0,
            days: 0,
            microseconds: 45_296_000_000,
        }
    );
    assert_eq!(result.interval_to_time, NaiveTime::from_hms_opt(2, 0, 0).expect("valid time"));
    assert_eq!(result.nullable_integer_to_float, None);
    assert_eq!(result.nullable_text_to_uuid, None);

    Ok(())
}

#[tokio::test]
async fn invalid_text_cast_returns_postgres_error() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    let row = seed_record(&tx, "not-a-number", 2.6).await?;

    let error = CastRecord::query()
        .select_only()
        .column_as(CastRecord::numeric_text.cast::<f64>(), "value")
        .filter(CastRecord::id.eq(row.id))
        .into_model::<FloatResult>()
        .one(&tx)
        .await
        .expect_err("invalid text cast must fail");

    assert_postgres_code(error, "22P02");
    Ok(())
}

#[tokio::test]
async fn overflowing_numeric_cast_returns_postgres_error() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    let row = seed_record(&tx, "12.5", 100_000.0).await?;

    let error = CastRecord::query()
        .select_only()
        .column_as(CastRecord::float_value.cast::<i16>(), "value")
        .filter(CastRecord::id.eq(row.id))
        .into_model::<SmallintResult>()
        .one(&tx)
        .await
        .expect_err("overflowing cast must fail");

    assert_postgres_code(error, "22003");
    Ok(())
}
