#![allow(non_upper_case_globals)]

use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{model, Database, Executor};

#[model(table = "unicode_character_samples")]
pub struct UnicodeCharacterSample {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub label: String,
    pub value: String,
    pub nullable_value: Option<String>,
    pub codepoint: i32,
    pub nullable_codepoint: Option<i32>,
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn setup_schema<E: Executor + Send + Sync>(ex: &E, unicode_fast: bool) -> Result<(), dbkit::Error> {
    let value_type = if unicode_fast { "TEXT COLLATE pg_unicode_fast" } else { "TEXT" };
    let statement = format!(
        "CREATE TEMP TABLE unicode_character_samples (\
            id BIGSERIAL PRIMARY KEY,\
            label TEXT NOT NULL,\
            value {value_type} NOT NULL,\
            nullable_value {value_type},\
            codepoint INTEGER NOT NULL,\
            nullable_codepoint INTEGER\
        )"
    );
    ex.execute(&statement, PgArguments::default()).await?;
    Ok(())
}

async fn seed<E: Executor + Send + Sync>(
    ex: &E,
    label: &str,
    value: &str,
    nullable_value: Option<&str>,
    codepoint: i32,
    nullable_codepoint: Option<i32>,
) -> Result<(), dbkit::Error> {
    UnicodeCharacterSample::insert(UnicodeCharacterSampleInsert {
        label: label.to_string(),
        value: value.to_string(),
        nullable_value: nullable_value.map(str::to_string),
        codepoint,
        nullable_codepoint,
    })
    .execute(ex)
    .await?;
    Ok(())
}

async fn server_encoding<E: Executor + Send + Sync>(ex: &E) -> Result<String, dbkit::Error> {
    let value: (String,) = ex
        .fetch_optional("SELECT current_setting('server_encoding')", PgArguments::default())
        .await?
        .expect("server encoding");
    Ok(value.0)
}

async fn server_version_num<E: Executor + Send + Sync>(ex: &E) -> Result<i32, dbkit::Error> {
    let value: (i32,) = ex
        .fetch_optional("SELECT current_setting('server_version_num')::INTEGER", PgArguments::default())
        .await?
        .expect("server version");
    Ok(value.0)
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct StringResult {
    value: String,
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct I32Result {
    value: i32,
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct NullableResult {
    normalized: Option<String>,
    first_codepoint: Option<i32>,
    character: Option<String>,
    ascii: Option<String>,
}

#[tokio::test]
async fn normalization_and_codepoint_conversion_follow_postgresql_unicode_semantics() -> Result<(), dbkit::Error> {
    use dbkit::func::NormalizationForm::{Nfc, Nfd, Nfkc, Nfkd};

    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    assert_eq!(server_encoding(&tx).await?, "UTF8");
    setup_schema(&tx, false).await?;
    seed(&tx, "row", "A", None, 65, None).await?;

    for (input, form, expected) in [
        ("e\u{301}", Nfc, "é"),
        ("é", Nfd, "e\u{301}"),
        ("①", Nfkc, "1"),
        ("ﬁ", Nfkd, "fi"),
        ("", Nfc, ""),
        ("already normalized", Nfc, "already normalized"),
        ("'%_\\.*[]()", Nfc, "'%_\\.*[]()"),
    ] {
        let result: StringResult = UnicodeCharacterSample::query()
            .select_only()
            .column_as(dbkit::func::normalize(input, form), "value")
            .into_model()
            .one(&tx)
            .await?
            .expect("normalization result");
        assert_eq!(result.value, expected, "input: {input:?}");
    }

    for (input, expected) in [("A", 65), ("🦀", 0x1f980), (" ", 32), ("", 0)] {
        let result: I32Result = UnicodeCharacterSample::query()
            .select_only()
            .column_as(dbkit::func::first_codepoint(input), "value")
            .into_model()
            .one(&tx)
            .await?
            .expect("codepoint result");
        assert_eq!(result.value, expected, "input: {input:?}");
    }

    for (input, expected) in [(65_i32, "A"), (0x1f980, "🦀")] {
        let result: StringResult = UnicodeCharacterSample::query()
            .select_only()
            .column_as(dbkit::func::from_codepoint(input), "value")
            .into_model()
            .one(&tx)
            .await?
            .expect("character result");
        assert_eq!(result.value, expected, "codepoint: {input}");
    }

    let nulls: NullableResult = UnicodeCharacterSample::query()
        .select_only()
        .column_as(dbkit::func::normalize(UnicodeCharacterSample::nullable_value, Nfc), "normalized")
        .column_as(
            dbkit::func::first_codepoint(UnicodeCharacterSample::nullable_value),
            "first_codepoint",
        )
        .column_as(dbkit::func::from_codepoint(UnicodeCharacterSample::nullable_codepoint), "character")
        .column_as(dbkit::func::to_ascii(UnicodeCharacterSample::nullable_value), "ascii")
        .into_model()
        .one(&tx)
        .await?
        .expect("null propagation result");
    assert_eq!(
        (nulls.normalized, nulls.first_codepoint, nulls.character, nulls.ascii),
        (None, None, None, None)
    );

    Ok(())
}

#[tokio::test]
async fn from_codepoint_rejects_invalid_unicode_values() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;

    for (label, codepoint) in [("zero", 0), ("negative", -1), ("surrogate", 0xd800), ("out of range", 0x110000)] {
        let tx = db.begin().await?;
        assert_eq!(server_encoding(&tx).await?, "UTF8");
        setup_schema(&tx, false).await?;
        seed(&tx, label, "value", None, codepoint, None).await?;

        let result: Result<Vec<StringResult>, dbkit::Error> = UnicodeCharacterSample::query()
            .select_only()
            .column_as(dbkit::func::from_codepoint(UnicodeCharacterSample::codepoint), "value")
            .into_model()
            .all(&tx)
            .await;
        let error = result.expect_err(label);
        assert!(
            matches!(&error, dbkit::Error::Sqlx(sqlx_error) if sqlx_error.as_database_error().is_some()),
            "{label} should be rejected by PostgreSQL, got: {error:?}"
        );
    }

    Ok(())
}

#[tokio::test]
async fn to_ascii_reports_utf8_as_an_unsupported_source_encoding() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    assert_eq!(server_encoding(&tx).await?, "UTF8");
    setup_schema(&tx, false).await?;
    seed(&tx, "accented", "Karél", None, 65, None).await?;

    let result: Result<Vec<StringResult>, dbkit::Error> = UnicodeCharacterSample::query()
        .select_only()
        .column_as(dbkit::func::to_ascii(UnicodeCharacterSample::value), "value")
        .into_model()
        .all(&tx)
        .await;
    let error = result.expect_err("UTF8 is not a TO_ASCII source encoding");
    assert!(
        error.to_string().contains("encoding conversion from UTF8 to ASCII not supported"),
        "unexpected error: {error:?}"
    );

    Ok(())
}

#[tokio::test]
#[ignore = "requires a LATIN1, LATIN2, LATIN9, or WIN1250 PostgreSQL database"]
async fn to_ascii_removes_accents_on_a_supported_database_encoding() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    let encoding = server_encoding(&tx).await?;
    assert!(
        ["LATIN1", "LATIN2", "LATIN9", "WIN1250"].contains(&encoding.as_str()),
        "TO_ASCII accent-removal coverage requires a supported database encoding, got {encoding}"
    );
    setup_schema(&tx, false).await?;
    seed(&tx, "accented", "Karél", None, 65, None).await?;

    let result: StringResult = UnicodeCharacterSample::query()
        .select_only()
        .column_as(dbkit::func::to_ascii(UnicodeCharacterSample::value), "value")
        .into_model()
        .one(&tx)
        .await?
        .expect("ASCII result");
    assert_eq!(result.value, "Karel");

    Ok(())
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct Postgres18Result {
    label: String,
    folded: String,
    normalized_fold: String,
    assigned: bool,
}

#[derive(dbkit::sqlx::FromRow, Debug)]
struct Postgres18NullResult {
    folded: Option<String>,
    assigned: Option<bool>,
}

#[tokio::test]
#[ignore = "requires PostgreSQL 18+; repository CI baseline is PostgreSQL 16"]
async fn postgres_18_case_folding_and_unicode_assignment_are_explicitly_version_gated() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    assert!(server_version_num(&tx).await? >= 180000, "this test requires PostgreSQL 18+");
    assert_eq!(server_encoding(&tx).await?, "UTF8");
    setup_schema(&tx, true).await?;

    for (label, value) in [
        ("simple", "ABC"),
        ("length changing", "Straße"),
        ("normalization", "ǰ"),
        ("empty", ""),
        ("unassigned", "\u{0378}"),
    ] {
        seed(&tx, label, value, None, 65, None).await?;
    }

    let rows: Vec<Postgres18Result> = UnicodeCharacterSample::query()
        .select_only()
        .column(UnicodeCharacterSample::label)
        .column_as(dbkit::func::case_fold(UnicodeCharacterSample::value), "folded")
        .column_as(
            dbkit::func::normalize(
                dbkit::func::case_fold(UnicodeCharacterSample::value),
                dbkit::func::NormalizationForm::Nfc,
            ),
            "normalized_fold",
        )
        .column_as(dbkit::func::is_unicode_assigned(UnicodeCharacterSample::value), "assigned")
        .order_by(dbkit::Order::asc(UnicodeCharacterSample::id))
        .into_model()
        .all(&tx)
        .await?;

    let values: Vec<_> = rows
        .into_iter()
        .map(|row| (row.label, row.folded, row.normalized_fold, row.assigned))
        .collect();
    assert_eq!(
        values,
        vec![
            ("simple".to_string(), "abc".to_string(), "abc".to_string(), true),
            ("length changing".to_string(), "strasse".to_string(), "strasse".to_string(), true,),
            ("normalization".to_string(), "j\u{030c}".to_string(), "ǰ".to_string(), true,),
            ("empty".to_string(), "".to_string(), "".to_string(), true),
            ("unassigned".to_string(), "\u{0378}".to_string(), "\u{0378}".to_string(), false,),
        ]
    );

    let nulls: Postgres18NullResult = UnicodeCharacterSample::query()
        .select_only()
        .column_as(dbkit::func::case_fold(UnicodeCharacterSample::nullable_value), "folded")
        .column_as(dbkit::func::is_unicode_assigned(UnicodeCharacterSample::nullable_value), "assigned")
        .order_by(dbkit::Order::asc(UnicodeCharacterSample::id))
        .into_model()
        .one(&tx)
        .await?
        .expect("PostgreSQL 18 null result");
    assert_eq!((nulls.folded, nulls.assigned), (None, None));

    Ok(())
}
