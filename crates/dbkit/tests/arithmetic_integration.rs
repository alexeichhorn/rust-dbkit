#![allow(non_upper_case_globals)]

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{model, Database, Executor, Expr, Order, PgInterval};

#[model(table = "records")]
pub struct Record {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub left_value: i32,
    pub right_value: i32,
    pub real_value: f32,
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
    divided: Option<i32>,
    floating_divided: Option<f64>,
    literal_minus_nullable: Option<i32>,
    nullable_time_required_interval: Option<NaiveDateTime>,
    required_time_nullable_interval: Option<NaiveDateTime>,
    nullable_time_nullable_interval: Option<NaiveDateTime>,
    literal_time_plus_nullable_interval: Option<NaiveDateTime>,
    literal_time_minus_nullable_interval: Option<NaiveDateTime>,
}

#[derive(Debug, sqlx::FromRow)]
struct DivisionResult {
    id: i64,
    integer_quotient: i32,
    floating_quotient: f64,
}

#[derive(Debug, sqlx::FromRow)]
struct MixedRealDivisionResult {
    integer_by_real: f64,
    real_by_integer: f64,
    real_by_real: f32,
}

#[derive(Debug, sqlx::FromRow)]
struct NullableRhsComparisonResult {
    less: Option<bool>,
    less_or_equal: Option<bool>,
    greater: Option<bool>,
    greater_or_equal: Option<bool>,
    nullable_left: Option<bool>,
    required: bool,
    coalesced: bool,
}

#[derive(Debug, sqlx::FromRow)]
struct DirectNullableRhsComparisonResult {
    less: Option<bool>,
    less_or_equal: Option<bool>,
    greater: Option<bool>,
    greater_or_equal: Option<bool>,
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
            real_value REAL NOT NULL,\
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
        real_value: 2.0,
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
async fn division_roundtrips_and_sorts_before_pagination() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let day = NaiveDate::from_ymd_opt(2024, 4, 1).expect("day");
    let occurred_at = NaiveDateTime::new(day, NaiveTime::from_hms_opt(9, 0, 0).expect("time"));
    let first = seed_record(&tx, -5, 2, 0, occurred_at).await?;
    let _second = seed_record(&tx, 9, 4, 0, occurred_at).await?;
    let third = seed_record(&tx, 11, 5, 0, occurred_at).await?;

    let floating_quotient: dbkit::Expr<f64> = Record::left_value.cast::<f64>() / Record::right_value;
    let rows: Vec<DivisionResult> = Record::query()
        .select_only()
        .column(Record::id)
        .column_as(Record::left_value / Record::right_value, "integer_quotient")
        .column_as(floating_quotient.clone(), "floating_quotient")
        .order_by(Order::asc(floating_quotient))
        .limit(2)
        .into_model()
        .all(&tx)
        .await?;

    assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), vec![first.id, third.id]);
    assert_eq!(rows.iter().map(|row| row.integer_quotient).collect::<Vec<_>>(), vec![-2, 2]);
    assert_eq!(rows.iter().map(|row| row.floating_quotient).collect::<Vec<_>>(), vec![-2.5, 2.2]);

    Ok(())
}

#[tokio::test]
async fn mixed_integer_and_real_division_roundtrips_as_double_precision() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let day = NaiveDate::from_ymd_opt(2024, 4, 1).expect("day");
    let occurred_at = NaiveDateTime::new(day, NaiveTime::from_hms_opt(9, 0, 0).expect("time"));
    let row = seed_record(&tx, 5, 2, 0, occurred_at).await?;

    let integer_by_real: Expr<f64> = Record::left_value / Record::real_value;
    let real_by_integer: Expr<f64> = Record::real_value / Record::left_value;
    let real_by_real: Expr<f32> = Record::real_value / Record::real_value;
    let result: MixedRealDivisionResult = Record::query()
        .select_only()
        .column_as(integer_by_real, "integer_by_real")
        .column_as(real_by_integer, "real_by_integer")
        .column_as(real_by_real, "real_by_real")
        .filter(Record::id.eq(row.id))
        .into_model()
        .one(&tx)
        .await?
        .ok_or(dbkit::Error::NotFound)?;

    assert_eq!(result.integer_by_real, 2.5);
    assert_eq!(result.real_by_integer, 0.4);
    assert_eq!(result.real_by_real, 1.0);

    Ok(())
}

#[tokio::test]
async fn division_by_zero_returns_postgres_error() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let day = NaiveDate::from_ymd_opt(2024, 4, 1).expect("day");
    let occurred_at = NaiveDateTime::new(day, NaiveTime::from_hms_opt(9, 0, 0).expect("time"));
    seed_record(&tx, 1, 0, 0, occurred_at).await?;

    let error = Record::query()
        .filter((Record::left_value / Record::right_value).gt(0_i32))
        .all(&tx)
        .await
        .expect_err("division by zero must fail");

    match error {
        dbkit::Error::Sqlx(dbkit::sqlx::Error::Database(error)) => {
            assert_eq!(error.code().as_deref(), Some("22012"));
        }
        error => panic!("unexpected error: {error}"),
    }

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
        .column_as(
            NullableArithmeticRecord::nullable_left / NullableArithmeticRecord::nullable_right,
            "divided",
        )
        .column_as(
            NullableArithmeticRecord::nullable_left.cast::<f64>() / NullableArithmeticRecord::nullable_right,
            "floating_divided",
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
        .column_as(
            base + dbkit::interval::hours(NullableArithmeticRecord::nullable_left),
            "literal_time_plus_nullable_interval",
        )
        .column_as(
            base - dbkit::interval::hours(NullableArithmeticRecord::nullable_right),
            "literal_time_minus_nullable_interval",
        )
        .order_by(Order::asc(NullableArithmeticRecord::id))
        .into_model()
        .all(&tx)
        .await?;

    assert_eq!(rows.len(), 4);

    assert_eq!(rows[0].added, Some(14));
    assert_eq!(rows[0].subtracted, Some(8));
    assert_eq!(rows[0].multiplied, Some(8));
    assert_eq!(rows[0].divided, Some(2));
    assert_eq!(rows[0].floating_divided, Some(2.0));
    assert_eq!(rows[0].literal_minus_nullable, Some(96));
    assert_eq!(rows[0].nullable_time_required_interval, Some(base + Duration::hours(1)));
    assert_eq!(rows[0].required_time_nullable_interval, Some(base - Duration::hours(2)));
    assert_eq!(rows[0].nullable_time_nullable_interval, Some(base + Duration::hours(2)));
    assert_eq!(rows[0].literal_time_plus_nullable_interval, Some(base + Duration::hours(4)));
    assert_eq!(rows[0].literal_time_minus_nullable_interval, Some(base - Duration::hours(2)));

    assert_eq!(rows[1].added, None);
    assert_eq!(rows[1].subtracted, Some(8));
    assert_eq!(rows[1].multiplied, None);
    assert_eq!(rows[1].divided, None);
    assert_eq!(rows[1].floating_divided, None);
    assert_eq!(rows[1].literal_minus_nullable, None);
    assert_eq!(rows[1].nullable_time_required_interval, None);
    assert_eq!(rows[1].required_time_nullable_interval, Some(base - Duration::hours(2)));
    assert_eq!(rows[1].nullable_time_nullable_interval, None);
    assert_eq!(rows[1].literal_time_plus_nullable_interval, None);
    assert_eq!(rows[1].literal_time_minus_nullable_interval, Some(base - Duration::hours(2)));

    assert_eq!(rows[2].added, Some(14));
    assert_eq!(rows[2].subtracted, None);
    assert_eq!(rows[2].multiplied, None);
    assert_eq!(rows[2].divided, None);
    assert_eq!(rows[2].floating_divided, None);
    assert_eq!(rows[2].literal_minus_nullable, Some(96));
    assert_eq!(rows[2].nullable_time_required_interval, Some(base + Duration::hours(1)));
    assert_eq!(rows[2].required_time_nullable_interval, None);
    assert_eq!(rows[2].nullable_time_nullable_interval, None);
    assert_eq!(rows[2].literal_time_plus_nullable_interval, Some(base + Duration::hours(4)));
    assert_eq!(rows[2].literal_time_minus_nullable_interval, None);

    assert_eq!(rows[3].added, None);
    assert_eq!(rows[3].subtracted, None);
    assert_eq!(rows[3].multiplied, None);
    assert_eq!(rows[3].divided, None);
    assert_eq!(rows[3].floating_divided, None);
    assert_eq!(rows[3].literal_minus_nullable, None);
    assert_eq!(rows[3].nullable_time_required_interval, None);
    assert_eq!(rows[3].required_time_nullable_interval, None);
    assert_eq!(rows[3].nullable_time_nullable_interval, None);
    assert_eq!(rows[3].literal_time_plus_nullable_interval, None);
    assert_eq!(rows[3].literal_time_minus_nullable_interval, None);

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

#[tokio::test]
async fn nullable_ordered_column_comparisons_exclude_null_operands() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_nullable_arithmetic_schema(&tx).await?;

    tx.execute(
        "INSERT INTO nullable_arithmetic_records \
            (required_value, nullable_left, nullable_right, required_at, nullable_at, required_interval, nullable_interval) \
         VALUES \
            (0, 4, 2, NOW(), NULL, INTERVAL '1 hour', NULL), \
            (0, 1, 2, NOW(), NULL, INTERVAL '1 hour', NULL), \
            (0, 3, 3, NOW(), NULL, INTERVAL '1 hour', NULL), \
            (0, NULL, 2, NOW(), NULL, INTERVAL '1 hour', NULL), \
            (0, 4, NULL, NOW(), NULL, INTERVAL '1 hour', NULL), \
            (0, NULL, NULL, NOW(), NULL, INTERVAL '1 hour', NULL)",
        PgArguments::default(),
    )
    .await?;

    let less = NullableArithmeticRecord::query()
        .filter(NullableArithmeticRecord::nullable_left.lt_col(NullableArithmeticRecord::nullable_right))
        .order_by(Order::asc(NullableArithmeticRecord::id))
        .all(&tx)
        .await?;
    assert_eq!(less.iter().map(|row| row.id).collect::<Vec<_>>(), vec![2]);

    let less_or_equal = NullableArithmeticRecord::query()
        .filter(NullableArithmeticRecord::nullable_left.le_col(NullableArithmeticRecord::nullable_right))
        .order_by(Order::asc(NullableArithmeticRecord::id))
        .all(&tx)
        .await?;
    assert_eq!(less_or_equal.iter().map(|row| row.id).collect::<Vec<_>>(), vec![2, 3]);

    let greater = NullableArithmeticRecord::query()
        .filter(NullableArithmeticRecord::nullable_left.gt_col(NullableArithmeticRecord::nullable_right))
        .order_by(Order::asc(NullableArithmeticRecord::id))
        .all(&tx)
        .await?;
    assert_eq!(greater.iter().map(|row| row.id).collect::<Vec<_>>(), vec![1]);

    let greater_or_equal = NullableArithmeticRecord::query()
        .filter(NullableArithmeticRecord::nullable_left.ge_col(NullableArithmeticRecord::nullable_right))
        .order_by(Order::asc(NullableArithmeticRecord::id))
        .all(&tx)
        .await?;
    assert_eq!(greater_or_equal.iter().map(|row| row.id).collect::<Vec<_>>(), vec![1, 3]);

    Ok(())
}

#[tokio::test]
async fn required_columns_compare_to_nullable_rhs_expressions() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_nullable_arithmetic_schema(&tx).await?;

    tx.execute(
        "INSERT INTO nullable_arithmetic_records \
            (required_value, nullable_left, nullable_right, required_at, nullable_at, required_interval, nullable_interval) \
         VALUES \
            (5, 2, NULL, NOW(), NULL, INTERVAL '1 hour', NULL), \
            (3, 2, NULL, NOW(), NULL, INTERVAL '1 hour', NULL), \
            (2, 2, NULL, NOW(), NULL, INTERVAL '1 hour', NULL), \
            (5, NULL, NULL, NOW(), NULL, INTERVAL '1 hour', NULL)",
        PgArguments::default(),
    )
    .await?;

    let nullable_rhs = NullableArithmeticRecord::nullable_left + 1_i32;
    let required_rhs = NullableArithmeticRecord::required_value + 1_i32;
    let rows: Vec<NullableRhsComparisonResult> = NullableArithmeticRecord::query()
        .select_only()
        .column_as(NullableArithmeticRecord::required_value.lt(nullable_rhs.clone()), "less")
        .column_as(NullableArithmeticRecord::required_value.le(nullable_rhs.clone()), "less_or_equal")
        .column_as(NullableArithmeticRecord::required_value.gt(nullable_rhs.clone()), "greater")
        .column_as(NullableArithmeticRecord::required_value.ge(nullable_rhs), "greater_or_equal")
        .column_as(NullableArithmeticRecord::nullable_left.lt(required_rhs.clone()), "nullable_left")
        .column_as(NullableArithmeticRecord::required_value.lt(required_rhs), "required")
        .column_as(
            NullableArithmeticRecord::required_value.lt(dbkit::func::coalesce(NullableArithmeticRecord::nullable_left, 0_i32)),
            "coalesced",
        )
        .order_by(Order::asc(NullableArithmeticRecord::id))
        .into_model()
        .all(&tx)
        .await?;

    assert_eq!(
        rows.into_iter()
            .map(|row| (
                row.less,
                row.less_or_equal,
                row.greater,
                row.greater_or_equal,
                row.nullable_left,
                row.required,
                row.coalesced,
            ))
            .collect::<Vec<_>>(),
        vec![
            (Some(false), Some(false), Some(true), Some(true), Some(true), true, false),
            (Some(false), Some(true), Some(false), Some(true), Some(true), true, false),
            (Some(true), Some(true), Some(false), Some(false), Some(true), true, false),
            (None, None, None, None, None, true, false),
        ]
    );

    let direct_rows: Vec<DirectNullableRhsComparisonResult> = NullableArithmeticRecord::query()
        .select_only()
        .column_as(
            NullableArithmeticRecord::required_value.lt(NullableArithmeticRecord::nullable_left),
            "less",
        )
        .column_as(
            NullableArithmeticRecord::required_value.le(NullableArithmeticRecord::nullable_left),
            "less_or_equal",
        )
        .column_as(
            NullableArithmeticRecord::required_value.gt(NullableArithmeticRecord::nullable_left),
            "greater",
        )
        .column_as(
            NullableArithmeticRecord::required_value.ge(NullableArithmeticRecord::nullable_left),
            "greater_or_equal",
        )
        .order_by(Order::asc(NullableArithmeticRecord::id))
        .into_model()
        .all(&tx)
        .await?;

    assert_eq!(
        direct_rows
            .into_iter()
            .map(|row| (row.less, row.less_or_equal, row.greater, row.greater_or_equal))
            .collect::<Vec<_>>(),
        vec![
            (Some(false), Some(false), Some(true), Some(true)),
            (Some(false), Some(false), Some(true), Some(true)),
            (Some(false), Some(true), Some(false), Some(true)),
            (None, None, None, None),
        ]
    );

    let greater = NullableArithmeticRecord::query()
        .filter(NullableArithmeticRecord::required_value.gt(NullableArithmeticRecord::nullable_left + 1_i32))
        .all(&tx)
        .await?;
    assert_eq!(greater.iter().map(|row| row.id).collect::<Vec<_>>(), vec![1]);

    let less_or_equal = NullableArithmeticRecord::query()
        .filter(NullableArithmeticRecord::required_value.le(NullableArithmeticRecord::nullable_left + 1_i32))
        .order_by(Order::asc(NullableArithmeticRecord::id))
        .all(&tx)
        .await?;
    assert_eq!(less_or_equal.iter().map(|row| row.id).collect::<Vec<_>>(), vec![2, 3]);

    Ok(())
}
