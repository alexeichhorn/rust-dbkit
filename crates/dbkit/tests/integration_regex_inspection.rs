#![allow(non_upper_case_globals)]

use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{model, Database, Executor};

#[model(table = "regex_samples")]
pub struct RegexSample {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub label: String,
    pub source: Option<String>,
    pub pattern: Option<String>,
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn setup_schema<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    ex.execute(
        "CREATE TEMP TABLE regex_samples (\
            id BIGSERIAL PRIMARY KEY,\
            label TEXT NOT NULL,\
            source TEXT NULL,\
            pattern TEXT NULL\
        )",
        PgArguments::default(),
    )
    .await?;
    Ok(())
}

async fn seed_sample<E: Executor + Send + Sync>(
    ex: &E,
    label: &str,
    source: Option<&str>,
    pattern: Option<&str>,
) -> Result<(), dbkit::Error> {
    RegexSample::insert(RegexSampleInsert {
        label: label.to_string(),
        source: source.map(str::to_string),
        pattern: pattern.map(str::to_string),
    })
    .execute(ex)
    .await?;
    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug, PartialEq)]
struct InspectionResult {
    label: String,
    is_match: Option<bool>,
    count: Option<i32>,
    position: Option<i32>,
    captures: Option<Vec<Option<String>>>,
    extract: Option<String>,
}

#[tokio::test]
async fn regex_inspection_follows_postgresql_core_semantics() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let cases = [
        ("repeated", Some("abcabc"), Some("abc")),
        ("overlap", Some("ababa"), Some("aba")),
        ("zero_length", Some("abc"), Some("")),
        ("empty", Some(""), Some("")),
        ("anchors_groups", Some("abc"), Some("^(a)(b)?(c)$")),
        ("alternation_unicode", Some("é🙂é"), Some("🙂|界")),
        ("case_sensitive", Some("ABC"), Some("abc")),
        ("no_match", Some("abc"), Some("z+")),
        ("null_source", None, Some("a")),
        ("null_pattern", Some("abc"), None),
    ];
    for (label, value, regex) in cases {
        seed_sample(&tx, label, value, regex).await?;
    }

    let rows: Vec<InspectionResult> = RegexSample::query()
        .select_only()
        .column(RegexSample::label)
        .column_as(dbkit::func::regex_is_match(RegexSample::source, RegexSample::pattern), "is_match")
        .column_as(dbkit::func::regex_count(RegexSample::source, RegexSample::pattern), "count")
        .column_as(dbkit::func::regex_position(RegexSample::source, RegexSample::pattern), "position")
        .column_as(dbkit::func::regex_captures(RegexSample::source, RegexSample::pattern), "captures")
        .column_as(dbkit::func::regex_extract(RegexSample::source, RegexSample::pattern), "extract")
        .order_by(dbkit::Order::asc(RegexSample::id))
        .into_model()
        .all(&tx)
        .await?;

    assert_eq!(
        rows,
        vec![
            InspectionResult {
                label: "repeated".to_string(),
                is_match: Some(true),
                count: Some(2),
                position: Some(1),
                captures: Some(vec![Some("abc".to_string())]),
                extract: Some("abc".to_string()),
            },
            InspectionResult {
                label: "overlap".to_string(),
                is_match: Some(true),
                count: Some(1),
                position: Some(1),
                captures: Some(vec![Some("aba".to_string())]),
                extract: Some("aba".to_string()),
            },
            InspectionResult {
                label: "zero_length".to_string(),
                is_match: Some(true),
                count: Some(4),
                position: Some(1),
                captures: Some(vec![Some(String::new())]),
                extract: Some(String::new()),
            },
            InspectionResult {
                label: "empty".to_string(),
                is_match: Some(true),
                count: Some(1),
                position: Some(1),
                captures: Some(vec![Some(String::new())]),
                extract: Some(String::new()),
            },
            InspectionResult {
                label: "anchors_groups".to_string(),
                is_match: Some(true),
                count: Some(1),
                position: Some(1),
                captures: Some(vec![Some("a".to_string()), Some("b".to_string()), Some("c".to_string())]),
                extract: Some("abc".to_string()),
            },
            InspectionResult {
                label: "alternation_unicode".to_string(),
                is_match: Some(true),
                count: Some(1),
                position: Some(2),
                captures: Some(vec![Some("🙂".to_string())]),
                extract: Some("🙂".to_string()),
            },
            InspectionResult {
                label: "case_sensitive".to_string(),
                is_match: Some(false),
                count: Some(0),
                position: Some(0),
                captures: None,
                extract: None,
            },
            InspectionResult {
                label: "no_match".to_string(),
                is_match: Some(false),
                count: Some(0),
                position: Some(0),
                captures: None,
                extract: None,
            },
            InspectionResult {
                label: "null_source".to_string(),
                is_match: None,
                count: None,
                position: None,
                captures: None,
                extract: None,
            },
            InspectionResult {
                label: "null_pattern".to_string(),
                is_match: None,
                count: None,
                position: None,
                captures: None,
                extract: None,
            },
        ]
    );

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct CaptureShapeResult {
    no_groups: Option<Vec<Option<String>>>,
    multiple_groups: Option<Vec<Option<String>>>,
    optional_group: Option<Vec<Option<String>>>,
    no_match: Option<Vec<Option<String>>>,
    null_input: Option<Vec<Option<String>>>,
    whole_extract: Option<String>,
    first_extract: Option<String>,
}

#[tokio::test]
async fn regex_capture_shape_distinguishes_sql_null_from_null_elements() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_sample(&tx, "shape", None, None).await?;

    let result: CaptureShapeResult = RegexSample::query()
        .select_only()
        .column_as(dbkit::func::regex_captures("foobarbeque", "barbeque"), "no_groups")
        .column_as(dbkit::func::regex_captures("foobarbeque", "(bar)(.*)(beque)"), "multiple_groups")
        .column_as(dbkit::func::regex_captures("foobarbeque", "(bar)(.+)?(beque)"), "optional_group")
        .column_as(dbkit::func::regex_captures("abc", "z+"), "no_match")
        .column_as(dbkit::func::regex_captures(RegexSample::source, "a"), "null_input")
        .column_as(dbkit::func::regex_extract("foobarbeque", "(bar)(beque)"), "whole_extract")
        .column_as(dbkit::func::regex_extract("abcabc", "abc"), "first_extract")
        .into_model()
        .one(&tx)
        .await?
        .expect("capture shape result");

    assert_eq!(result.no_groups, Some(vec![Some("barbeque".to_string())]));
    assert_eq!(
        result.multiple_groups,
        Some(vec![Some("bar".to_string()), Some(String::new()), Some("beque".to_string())])
    );
    assert_eq!(
        result.optional_group,
        Some(vec![Some("bar".to_string()), None, Some("beque".to_string())])
    );
    assert_eq!(result.no_match, None);
    assert_eq!(result.null_input, None);
    assert_eq!(result.whole_extract.as_deref(), Some("barbeque"));
    assert_eq!(result.first_extract.as_deref(), Some("abc"));

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct BoundRegexResult {
    is_match: bool,
    count: i32,
    position: i32,
    captures: Option<Vec<Option<String>>>,
    extract: Option<String>,
}

#[tokio::test]
async fn regex_expression_and_pattern_remain_bound_values() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_sample(&tx, "bound", None, None).await?;

    let expression = "'%_\\.*+";
    let pattern = r"^'%_\\\.\*\+$";
    let result: BoundRegexResult = RegexSample::query()
        .select_only()
        .column_as(dbkit::func::regex_is_match(expression, pattern), "is_match")
        .column_as(dbkit::func::regex_count(expression, pattern), "count")
        .column_as(dbkit::func::regex_position(expression, pattern), "position")
        .column_as(dbkit::func::regex_captures(expression, pattern), "captures")
        .column_as(dbkit::func::regex_extract(expression, pattern), "extract")
        .into_model()
        .one(&tx)
        .await?
        .expect("bound regex result");

    assert!(result.is_match);
    assert_eq!(result.count, 1);
    assert_eq!(result.position, 1);
    assert_eq!(result.captures, Some(vec![Some(expression.to_string())]));
    assert_eq!(result.extract.as_deref(), Some(expression));

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct InvalidPatternResult {
    value: bool,
}

#[tokio::test]
async fn invalid_regex_is_a_postgresql_error() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_sample(&tx, "invalid", None, None).await?;

    let error = RegexSample::query()
        .select_only()
        .column_as(dbkit::func::regex_is_match("abc", "("), "value")
        .into_model::<InvalidPatternResult>()
        .one(&tx)
        .await
        .expect_err("PostgreSQL must reject an invalid regular expression");
    assert!(
        error.to_string().contains("invalid regular expression"),
        "expected PostgreSQL regex error, got: {error:?}"
    );

    Ok(())
}
