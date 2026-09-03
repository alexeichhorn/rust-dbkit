#![allow(non_upper_case_globals)]

use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{model, Database, Executor, Order, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, dbkit::sqlx::Type)]
#[sqlx(transparent)]
#[repr(transparent)]
struct PermissionBits(i64);

impl PermissionBits {
    const READ: Self = Self(1 << 0);
    const WRITE: Self = Self(1 << 1);
    const EXECUTE: Self = Self(1 << 2);
}

impl From<PermissionBits> for Value {
    fn from(value: PermissionBits) -> Self {
        Self::I64(value.0)
    }
}

impl From<PermissionBits> for i64 {
    fn from(value: PermissionBits) -> Self {
        value.0
    }
}

#[model(table = "bitwise_samples")]
struct BitwiseSample {
    #[key]
    #[autoincrement]
    id: i64,
    small_value: i16,
    medium_value: i32,
    large_value: i64,
    nullable_large: Option<i64>,
    shift_count: i32,
    nullable_shift_count: Option<i32>,
    permissions: PermissionBits,
}

#[derive(Debug, dbkit::sqlx::FromRow)]
struct OperatorProjection {
    and_value: i32,
    or_value: i32,
    xor_value: i32,
    inverted: i32,
    shifted_left: i32,
    shifted_right: i32,
}

#[derive(Debug, dbkit::sqlx::FromRow)]
struct MixedWidthProjection {
    small_and_medium: i32,
    medium_or_large: i64,
    large_xor_small: i64,
}

#[derive(Debug, dbkit::sqlx::FromRow)]
struct EdgeProjection {
    zero_and: i64,
    zero_or: i64,
    all_bits: i64,
    literal_all_bits: i64,
    sign_bit: i64,
    signed_right_shift: i64,
    unchanged_by_zero_shift: i64,
}

#[derive(Debug, dbkit::sqlx::FromRow)]
struct NullableProjection {
    and_value: Option<i64>,
    or_value: Option<i64>,
    xor_value: Option<i64>,
    inverted: Option<i64>,
    shifted_by_required: Option<i64>,
    shifted_by_nullable: Option<i64>,
}

#[derive(Debug, dbkit::sqlx::FromRow)]
struct PermissionProjection {
    selected: PermissionBits,
    expanded: PermissionBits,
    inverted: PermissionBits,
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn setup_schema<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    ex.execute(
        "CREATE TEMP TABLE bitwise_samples (\
            id BIGSERIAL PRIMARY KEY,\
            small_value SMALLINT NOT NULL,\
            medium_value INTEGER NOT NULL,\
            large_value BIGINT NOT NULL,\
            nullable_large BIGINT,\
            shift_count INTEGER NOT NULL,\
            nullable_shift_count INTEGER,\
            permissions BIGINT NOT NULL\
        )",
        PgArguments::default(),
    )
    .await?;

    Ok(())
}

async fn seed_sample<E: Executor + Send + Sync>(
    ex: &E,
    small_value: i16,
    medium_value: i32,
    large_value: i64,
    nullable_large: Option<i64>,
    shift_count: i32,
    nullable_shift_count: Option<i32>,
    permissions: PermissionBits,
) -> Result<BitwiseSample, dbkit::Error> {
    Ok(BitwiseSample::insert(BitwiseSampleInsert {
        small_value,
        medium_value,
        large_value,
        nullable_large,
        shift_count,
        nullable_shift_count,
        permissions,
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted bitwise sample"))
}

#[tokio::test]
async fn all_scalar_operators_roundtrip() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_sample(&tx, 0, 0b1010, 0, None, 2, None, PermissionBits(0)).await?;

    let result: OperatorProjection = BitwiseSample::query()
        .select_only()
        .column_as(BitwiseSample::medium_value & 0b1100_i32, "and_value")
        .column_as(BitwiseSample::medium_value | 0b0101_i32, "or_value")
        .column_as(BitwiseSample::medium_value ^ 0b1111_i32, "xor_value")
        .column_as(!BitwiseSample::medium_value, "inverted")
        .column_as(BitwiseSample::medium_value << BitwiseSample::shift_count, "shifted_left")
        .column_as(BitwiseSample::medium_value >> 1_i16, "shifted_right")
        .into_model()
        .one(&tx)
        .await?
        .expect("operator projection");

    assert_eq!(result.and_value, 0b1000);
    assert_eq!(result.or_value, 0b1111);
    assert_eq!(result.xor_value, 0b0101);
    assert_eq!(result.inverted, !0b1010_i32);
    assert_eq!(result.shifted_left, 0b101000);
    assert_eq!(result.shifted_right, 0b0101);

    Ok(())
}

#[tokio::test]
async fn mixed_integer_widths_promote_to_the_wider_type() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_sample(&tx, 0b1100, 0b1010, 0b1_0001, None, 0, None, PermissionBits(0)).await?;

    // PostgreSQL widens mixed integral operands instead of truncating either side.
    let result: MixedWidthProjection = BitwiseSample::query()
        .select_only()
        .column_as(BitwiseSample::small_value & BitwiseSample::medium_value, "small_and_medium")
        .column_as(BitwiseSample::medium_value | BitwiseSample::large_value, "medium_or_large")
        .column_as(BitwiseSample::large_value ^ BitwiseSample::small_value, "large_xor_small")
        .into_model()
        .one(&tx)
        .await?
        .expect("mixed-width projection");

    assert_eq!(result.small_and_medium, 0b1000);
    assert_eq!(result.medium_or_large, 0b1_1011);
    assert_eq!(result.large_xor_small, 0b1_1101);

    Ok(())
}

#[tokio::test]
async fn zero_all_bits_and_signed_shifts_follow_postgres_semantics() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_sample(&tx, 0, 0, -8, None, 0, None, PermissionBits(0)).await?;

    let result: EdgeProjection = BitwiseSample::query()
        .select_only()
        .column_as(BitwiseSample::large_value & 0_i64, "zero_and")
        .column_as(BitwiseSample::large_value | 0_i64, "zero_or")
        .column_as(BitwiseSample::large_value & -1_i64, "all_bits")
        .column_as(!0_i64 & BitwiseSample::large_value, "literal_all_bits")
        .column_as(1_i64 << 63_i32, "sign_bit")
        .column_as(BitwiseSample::large_value >> 2_i32, "signed_right_shift")
        .column_as(BitwiseSample::large_value << 0_i32, "unchanged_by_zero_shift")
        .into_model()
        .one(&tx)
        .await?
        .expect("edge projection");

    assert_eq!(result.zero_and, 0);
    assert_eq!(result.zero_or, -8);
    assert_eq!(result.all_bits, -8);
    assert_eq!(result.literal_all_bits, -8);
    assert_eq!(result.sign_bit, i64::MIN);
    assert_eq!(result.signed_right_shift, -2);
    assert_eq!(result.unchanged_by_zero_shift, -8);

    Ok(())
}

#[tokio::test]
async fn null_propagates_from_values_and_shift_counts() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_sample(&tx, 0, 0, 0b1010, Some(0b1100), 2, Some(1), PermissionBits(0)).await?;
    seed_sample(&tx, 0, 0, 0b1010, None, 2, None, PermissionBits(0)).await?;

    let results: Vec<NullableProjection> = BitwiseSample::query()
        .select_only()
        .column_as(BitwiseSample::nullable_large & 0b1010_i64, "and_value")
        .column_as(BitwiseSample::nullable_large | BitwiseSample::large_value, "or_value")
        .column_as(BitwiseSample::large_value ^ BitwiseSample::nullable_large, "xor_value")
        .column_as(!BitwiseSample::nullable_large, "inverted")
        .column_as(BitwiseSample::nullable_large << BitwiseSample::shift_count, "shifted_by_required")
        .column_as(
            BitwiseSample::large_value >> BitwiseSample::nullable_shift_count,
            "shifted_by_nullable",
        )
        .order_by(Order::asc(BitwiseSample::id))
        .into_model()
        .all(&tx)
        .await?;

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].and_value, Some(0b1000));
    assert_eq!(results[0].or_value, Some(0b1110));
    assert_eq!(results[0].xor_value, Some(0b0110));
    assert_eq!(results[0].inverted, Some(!0b1100_i64));
    assert_eq!(results[0].shifted_by_required, Some(0b11_0000));
    assert_eq!(results[0].shifted_by_nullable, Some(0b0101));

    assert_eq!(results[1].and_value, None);
    assert_eq!(results[1].or_value, None);
    assert_eq!(results[1].xor_value, None);
    assert_eq!(results[1].inverted, None);
    assert_eq!(results[1].shifted_by_required, None);
    assert_eq!(results[1].shifted_by_nullable, None);

    Ok(())
}

#[tokio::test]
async fn composed_expressions_filter_and_order_rows() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let first = seed_sample(&tx, 0, 0, 0b0011, None, 0, None, PermissionBits(0)).await?;
    let second = seed_sample(&tx, 0, 0, 0b0110, None, 0, None, PermissionBits(0)).await?;
    let third = seed_sample(&tx, 0, 0, 0b1100, None, 0, None, PermissionBits(0)).await?;

    let score = ((BitwiseSample::large_value & 0b1111_i64) ^ 0b0011_i64) << 1_i32;
    let rows = BitwiseSample::query()
        .filter(score.clone().gt(0b1000_i64))
        .order_by(Order::desc(score))
        .all(&tx)
        .await?;

    assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), vec![third.id, second.id]);
    assert!(!rows.iter().any(|row| row.id == first.id));

    Ok(())
}

#[tokio::test]
async fn custom_integer_backed_type_filters_and_roundtrips() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let future_permission = PermissionBits(1 << 40);
    seed_sample(
        &tx,
        0,
        0,
        0,
        None,
        0,
        None,
        PermissionBits(PermissionBits::READ.0 | future_permission.0),
    )
    .await?;
    seed_sample(&tx, 0, 0, 0, None, 0, None, PermissionBits::EXECUTE).await?;

    let result: PermissionProjection = BitwiseSample::query()
        .filter((BitwiseSample::permissions & future_permission).ne(PermissionBits(0)))
        .select_only()
        .column_as(BitwiseSample::permissions & future_permission, "selected")
        .column_as(BitwiseSample::permissions | PermissionBits::WRITE, "expanded")
        .column_as(!BitwiseSample::permissions, "inverted")
        .into_model()
        .one(&tx)
        .await?
        .expect("custom bitwise projection");

    assert_eq!(result.selected, future_permission);
    assert_eq!(
        result.expanded,
        PermissionBits(PermissionBits::READ.0 | PermissionBits::WRITE.0 | future_permission.0)
    );
    assert_eq!(result.inverted, PermissionBits(!(PermissionBits::READ.0 | future_permission.0)));

    Ok(())
}
