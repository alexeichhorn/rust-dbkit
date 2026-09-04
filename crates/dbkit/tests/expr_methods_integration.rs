use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{func, model, Database, Executor, IntoExpr, Order};

#[model(table = "method_samples")]
pub struct MethodSample {
    #[key]
    pub id: i64,
    pub text: String,
    pub optional_text: Option<String>,
    pub optional_prefix: Option<String>,
    pub optional_count: Option<i32>,
    pub optional_bool: Option<bool>,
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn setup<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    ex.execute(
        "CREATE TEMP TABLE method_samples (\
            id BIGINT PRIMARY KEY,\
            text TEXT NOT NULL,\
            optional_text TEXT,\
            optional_prefix TEXT,\
            optional_count INTEGER DEFAULT 91,\
            optional_bool BOOLEAN\
        )",
        PgArguments::default(),
    )
    .await?;

    for (id, text, optional_text, prefix, count, enabled) in [
        (1, "ALPHA", None, None, None, None),
        (2, "", Some(""), Some(""), Some(0), Some(false)),
        (3, "    ", Some("    "), Some(" "), Some(-4), Some(true)),
        (4, "  ALPHA  ", Some("  ALPHA  "), Some("  A"), Some(7), Some(true)),
        (5, "%_literal", Some("%_literal"), Some("%_"), Some(2), Some(false)),
        (6, "\tALPHA\n", Some("\tALPHA\n"), Some("\tA"), Some(3), Some(false)),
    ] {
        MethodSample::insert(MethodSampleInsert {
            id,
            text: text.into(),
            optional_text: optional_text.map(str::to_string),
            optional_prefix: prefix.map(str::to_string),
            optional_count: count,
            optional_bool: enabled,
        })
        .execute(ex)
        .await?;
    }
    Ok(())
}

#[derive(Debug, dbkit::sqlx::FromRow)]
struct FallbackResult {
    literal: String,
    column_fallback: String,
    expression_fallback: String,
    default_text: String,
    default_count: i32,
    default_bool: bool,
    computed_count: i32,
}

#[tokio::test]
async fn fallbacks_replace_only_null_and_use_rust_defaults() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let rows: Vec<FallbackResult> = MethodSample::query()
        .select_only()
        .column_as(MethodSample::optional_text.unwrap_or("fallback"), "literal")
        .column_as(MethodSample::optional_text.unwrap_or(MethodSample::text), "column_fallback")
        .column_as(
            MethodSample::optional_text.into_expr().unwrap_or(MethodSample::text.lower()),
            "expression_fallback",
        )
        .column_as(MethodSample::optional_text.unwrap_or_default(), "default_text")
        .column_as(MethodSample::optional_count.unwrap_or_default(), "default_count")
        .column_as(MethodSample::optional_bool.into_expr().unwrap_or_default(), "default_bool")
        .column_as((MethodSample::optional_count + 2_i32).unwrap_or(99), "computed_count")
        .order_by(Order::asc(MethodSample::id))
        .into_model()
        .all(&tx)
        .await?;

    // The column's SQL DEFAULT is 91; unwrap_or_default must bind Rust's i32 default, 0.
    let expected = [
        ("fallback", "ALPHA", "alpha", "", 0, false, 99),
        ("", "", "", "", 0, false, 2),
        ("    ", "    ", "    ", "    ", -4, true, -2),
        ("  ALPHA  ", "  ALPHA  ", "  ALPHA  ", "  ALPHA  ", 7, true, 9),
        ("%_literal", "%_literal", "%_literal", "%_literal", 2, false, 4),
        ("\tALPHA\n", "\tALPHA\n", "\tALPHA\n", "\tALPHA\n", 3, false, 5),
    ];
    assert_eq!(rows.len(), expected.len());
    for (row, expected) in rows.iter().zip(expected) {
        assert_eq!(
            (
                row.literal.as_str(),
                row.column_fallback.as_str(),
                row.expression_fallback.as_str(),
                row.default_text.as_str(),
                row.default_count,
                row.default_bool,
                row.computed_count
            ),
            expected,
        );
    }

    let positive = MethodSample::query()
        .filter(MethodSample::optional_count.gt(0).unwrap_or(false))
        .order_by(Order::asc(MethodSample::id))
        .all(&tx)
        .await?;
    assert_eq!(positive.iter().map(|row| row.id).collect::<Vec<_>>(), vec![4, 5, 6]);
    Ok(())
}

#[derive(Debug, dbkit::sqlx::FromRow)]
struct StringResult {
    required: String,
    optional: Option<String>,
    required_with_optional_prefix: Option<bool>,
    optional_with_required_prefix: Option<bool>,
    optional_with_optional_prefix: Option<bool>,
}

#[tokio::test]
async fn string_chains_preserve_nullability_and_postgres_space_trimming() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    let rows: Vec<StringResult> = MethodSample::query()
        .select_only()
        .column_as(MethodSample::text.trim().lower(), "required")
        .column_as(MethodSample::optional_text.trim().lower(), "optional")
        .column_as(
            MethodSample::text.starts_with(MethodSample::optional_prefix),
            "required_with_optional_prefix",
        )
        .column_as(MethodSample::optional_text.starts_with(""), "optional_with_required_prefix")
        .column_as(
            MethodSample::optional_text
                .lower()
                .starts_with(MethodSample::optional_prefix.lower()),
            "optional_with_optional_prefix",
        )
        .order_by(Order::asc(MethodSample::id))
        .into_model()
        .all(&tx)
        .await?;

    // SQL TRIM removes spaces by default, not the tabs/newlines handled by Rust str::trim.
    let expected = [
        ("alpha", None, None, None, None),
        ("", Some(""), Some(true), Some(true), Some(true)),
        ("", Some(""), Some(true), Some(true), Some(true)),
        ("alpha", Some("alpha"), Some(true), Some(true), Some(true)),
        ("%_literal", Some("%_literal"), Some(true), Some(true), Some(true)),
        ("\talpha\n", Some("\talpha\n"), Some(true), Some(true), Some(true)),
    ];
    assert_eq!(rows.len(), expected.len());
    for (row, expected) in rows.iter().zip(expected) {
        assert_eq!(
            (
                row.required.as_str(),
                row.optional.as_deref(),
                row.required_with_optional_prefix,
                row.optional_with_required_prefix,
                row.optional_with_optional_prefix
            ),
            expected,
        );
    }

    let normalized = MethodSample::optional_text.trim().lower().unwrap_or_default();
    let matches = MethodSample::query()
        .filter(normalized.clone().starts_with("alpha"))
        .order_by(Order::asc(normalized))
        .all(&tx)
        .await?;
    assert_eq!(matches.iter().map(|row| row.id).collect::<Vec<_>>(), vec![4]);
    Ok(())
}

#[derive(Debug, dbkit::sqlx::FromRow)]
struct PrefixResult {
    matches: bool,
}

#[tokio::test]
async fn starts_with_uses_literal_case_sensitive_prefixes() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    for (text, prefix, expected) in [
        ("Alpha", "Al", true),
        ("Alpha", "al", false),
        ("", "", true),
        ("", "a", false),
        ("a", "", true),
        ("a", "ab", false),
        ("%_literal", "%_", true),
        ("anything", "%", false),
        ("anything", "_", false),
        ("'\\literal", "'\\", true),
        ("🦀café", "🦀", true),
        ("café", "caf", true),
    ] {
        let result: PrefixResult = MethodSample::query()
            .filter(MethodSample::id.eq(1_i64))
            .select_only()
            .column_as(text.into_expr().starts_with(prefix), "matches")
            .into_model()
            .one(&tx)
            .await?
            .expect("seeded sample");
        assert_eq!(result.matches, expected, "text: {text:?}, prefix: {prefix:?}");
    }
    Ok(())
}

#[derive(Debug, dbkit::sqlx::FromRow)]
struct AggregateResult {
    // PostgreSQL returns NUMERIC for SUM(BIGINT), including when wrapped in COALESCE.
    total: dbkit::sqlx::types::BigDecimal,
    default_total: dbkit::sqlx::types::BigDecimal,
    filtered_total: dbkit::sqlx::types::BigDecimal,
    first_text: String,
}

#[tokio::test]
async fn aggregate_methods_handle_empty_inputs_and_filtered_aggregates() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup(&tx).await?;

    for (minimum_id, expected) in [(0_i64, (21, 21, 15, "")), (100_i64, (42, 0, 0, "empty"))] {
        let result: AggregateResult = MethodSample::query()
            .filter(MethodSample::id.gt(minimum_id))
            .select_only()
            .column_as(func::sum(MethodSample::id).unwrap_or(42_i64), "total")
            .column_as(func::sum(MethodSample::id).unwrap_or_default(), "default_total")
            .column_as(
                func::sum(MethodSample::id).filter(MethodSample::id.gt(3_i64)).unwrap_or_default(),
                "filtered_total",
            )
            .column_as(func::min(MethodSample::text).trim().lower().unwrap_or("empty"), "first_text")
            .into_model()
            .one(&tx)
            .await?
            .expect("aggregate result even without input rows");
        assert_eq!(
            (
                result.total,
                result.default_total,
                result.filtered_total,
                result.first_text.as_str()
            ),
            (expected.0.into(), expected.1.into(), expected.2.into(), expected.3)
        );
    }
    Ok(())
}
