#![allow(non_upper_case_globals)]

use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use dbkit::prelude::*;
use dbkit::executor::BoxFuture;
use dbkit::sqlx::postgres::{PgArguments, PgPoolOptions, PgRow};
use dbkit::{model, row, DbEnum, Executor, Order};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, DbEnum)]
#[dbkit(type_name = "lookup_scope", rename_all = "snake_case")]
pub enum LookupScope {
    Public,
    Internal,
}

#[model(table = "lookup_rows")]
pub struct LookupRow {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub scope: LookupScope,
    pub external_key: String,
    pub locale: String,
    pub label: String,
}

#[model(table = "revision_snapshots")]
pub struct RevisionSnapshot {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub series_id: i64,
    pub revision: i32,
    pub captured_at: NaiveDateTime,
    pub note: String,
}

struct TestTx<'t> {
    inner: Arc<Mutex<Option<sqlx::Transaction<'t, sqlx::Postgres>>>>,
}

impl<'t> TestTx<'t> {
    fn new(tx: sqlx::Transaction<'t, sqlx::Postgres>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Some(tx))),
        }
    }

    async fn rollback(self) -> Result<(), dbkit::Error> {
        let tx = {
            let mut guard = self.inner.lock().await;
            guard
                .take()
                .ok_or_else(|| dbkit::Error::Decode("transaction already completed".to_string()))?
        };
        tx.rollback().await?;
        Ok(())
    }
}

// This wrapper is intentionally test-local.
//
// These integration tests create temporary enum/table objects with stable names and may reuse pooled
// connections across cases. SQLx prepared statement caching can then hit Postgres plan-cache errors like
// "cached plan must not change result type" when a later test recreates temp objects under the same names.
//
// The row-value feature itself works with enums; the problem is the temp-schema test setup combined with
// prepared statement reuse on the same connection. To keep the production/runtime executor behavior unchanged,
// only this test wrapper disables statement persistence.
impl<'t> dbkit::Executor for TestTx<'t> {
    fn fetch_all<'e, T>(&'e self, sql: &'e str, args: PgArguments) -> BoxFuture<'e, Result<Vec<T>, dbkit::Error>>
    where
        T: for<'r> sqlx::FromRow<'r, PgRow> + Send + Unpin + 'e,
    {
        let inner = self.inner.clone();
        Box::pin(async move {
            let mut guard = inner.lock().await;
            let tx = guard
                .as_mut()
                .ok_or_else(|| dbkit::Error::Decode("transaction already completed".to_string()))?;
            let conn = tx.as_mut();
            let rows = sqlx::query_as_with::<sqlx::Postgres, T, _>(sql, args)
                .persistent(false)
                .fetch_all(conn)
                .await?;
            Ok(rows)
        })
    }

    fn fetch_optional<'e, T>(&'e self, sql: &'e str, args: PgArguments) -> BoxFuture<'e, Result<Option<T>, dbkit::Error>>
    where
        T: for<'r> sqlx::FromRow<'r, PgRow> + Send + Unpin + 'e,
    {
        let inner = self.inner.clone();
        Box::pin(async move {
            let mut guard = inner.lock().await;
            let tx = guard
                .as_mut()
                .ok_or_else(|| dbkit::Error::Decode("transaction already completed".to_string()))?;
            let conn = tx.as_mut();
            let row = sqlx::query_as_with::<sqlx::Postgres, T, _>(sql, args)
                .persistent(false)
                .fetch_optional(conn)
                .await?;
            Ok(row)
        })
    }

    fn fetch_rows<'e>(&'e self, sql: &'e str, args: PgArguments) -> BoxFuture<'e, Result<Vec<PgRow>, dbkit::Error>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            let mut guard = inner.lock().await;
            let tx = guard
                .as_mut()
                .ok_or_else(|| dbkit::Error::Decode("transaction already completed".to_string()))?;
            let conn = tx.as_mut();
            let rows = sqlx::query_with::<sqlx::Postgres, _>(sql, args)
                .persistent(false)
                .fetch_all(conn)
                .await?;
            Ok(rows)
        })
    }

    fn execute<'e>(&'e self, sql: &'e str, args: PgArguments) -> BoxFuture<'e, Result<u64, dbkit::Error>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            let mut guard = inner.lock().await;
            let tx = guard
                .as_mut()
                .ok_or_else(|| dbkit::Error::Decode("transaction already completed".to_string()))?;
            let conn = tx.as_mut();
            let result = sqlx::query_with::<sqlx::Postgres, _>(sql, args)
                .persistent(false)
                .execute(conn)
                .await?;
            Ok(result.rows_affected())
        })
    }
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn setup_schema<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    ex.execute(
        "CREATE TYPE pg_temp.lookup_scope AS ENUM ('public', 'internal')",
        PgArguments::default(),
    )
    .await?;
    ex.execute(
        "CREATE TEMP TABLE lookup_rows (\
            id BIGSERIAL PRIMARY KEY,\
            scope lookup_scope NOT NULL,\
            external_key TEXT NOT NULL,\
            locale TEXT NOT NULL,\
            label TEXT NOT NULL,\
            UNIQUE (scope, external_key, locale)\
        )",
        PgArguments::default(),
    )
    .await?;
    ex.execute(
        "CREATE TEMP TABLE revision_snapshots (\
            id BIGSERIAL PRIMARY KEY,\
            series_id BIGINT NOT NULL,\
            revision INTEGER NOT NULL,\
            captured_at TIMESTAMP NOT NULL,\
            note TEXT NOT NULL,\
            UNIQUE (series_id, revision, captured_at)\
        )",
        PgArguments::default(),
    )
    .await?;
    Ok(())
}

async fn seed_row<E: Executor + Send + Sync>(
    ex: &E,
    scope: LookupScope,
    external_key: &str,
    locale: &str,
    label: &str,
) -> Result<LookupRow, dbkit::Error> {
    let row = LookupRow::insert(LookupRowInsert {
        scope,
        external_key: external_key.to_string(),
        locale: locale.to_string(),
        label: label.to_string(),
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted lookup row");
    Ok(row)
}

async fn seed_revision_snapshot<E: Executor + Send + Sync>(
    ex: &E,
    series_id: i64,
    revision: i32,
    captured_at: NaiveDateTime,
    note: &str,
) -> Result<RevisionSnapshot, dbkit::Error> {
    let row = RevisionSnapshot::insert(RevisionSnapshotInsert {
        series_id,
        revision,
        captured_at,
        note: note.to_string(),
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted revision snapshot");
    Ok(row)
}

#[tokio::test]
async fn row_value_in_returns_rows_matching_requested_composite_keys() -> Result<(), dbkit::Error> {
    let pool = PgPoolOptions::new().connect(&db_url()).await?;
    let tx = TestTx::new(pool.begin().await?);
    setup_schema(&tx).await?;

    let alpha_en = seed_row(&tx, LookupScope::Public, "alpha", "en", "Alpha").await?;
    let _alpha_de = seed_row(&tx, LookupScope::Public, "alpha", "de", "Alpha DE").await?;
    let beta_en = seed_row(&tx, LookupScope::Internal, "beta", "en", "Beta").await?;
    let _gamma_en = seed_row(&tx, LookupScope::Internal, "gamma", "en", "Gamma").await?;

    let rows: Vec<LookupRow> = LookupRow::query()
        .filter(
            row((LookupRow::scope, LookupRow::external_key, LookupRow::locale))
                .in_([(LookupScope::Public, "alpha", "en"), (LookupScope::Internal, "beta", "en")]),
        )
        .order_by(Order::asc(LookupRow::id))
        .all(&tx)
        .await?;

    assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), vec![alpha_en.id, beta_en.id]);
    assert_eq!(rows.iter().map(|row| row.label.as_str()).collect::<Vec<_>>(), vec!["Alpha", "Beta"]);

    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn row_value_in_empty_input_returns_no_rows() -> Result<(), dbkit::Error> {
    let pool = PgPoolOptions::new().connect(&db_url()).await?;
    let tx = TestTx::new(pool.begin().await?);
    setup_schema(&tx).await?;

    seed_row(&tx, LookupScope::Public, "alpha", "en", "Alpha").await?;
    seed_row(&tx, LookupScope::Internal, "beta", "en", "Beta").await?;

    let rows: Vec<LookupRow> = LookupRow::query()
        .filter(row((LookupRow::scope, LookupRow::external_key, LookupRow::locale)).in_(std::iter::empty::<(LookupScope, &str, &str)>()))
        .all(&tx)
        .await?;

    assert!(rows.is_empty());

    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn row_value_in_duplicate_requested_keys_do_not_duplicate_rows() -> Result<(), dbkit::Error> {
    let pool = PgPoolOptions::new().connect(&db_url()).await?;
    let tx = TestTx::new(pool.begin().await?);
    setup_schema(&tx).await?;

    let alpha_en = seed_row(&tx, LookupScope::Public, "alpha", "en", "Alpha").await?;
    let beta_en = seed_row(&tx, LookupScope::Internal, "beta", "en", "Beta").await?;

    let rows: Vec<LookupRow> = LookupRow::query()
        .filter(row((LookupRow::scope, LookupRow::external_key, LookupRow::locale)).in_([
            (LookupScope::Public, "alpha", "en"),
            (LookupScope::Public, "alpha", "en"),
            (LookupScope::Internal, "beta", "en"),
        ]))
        .order_by(Order::asc(LookupRow::id))
        .all(&tx)
        .await?;

    assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), vec![alpha_en.id, beta_en.id]);

    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn row_value_in_composes_with_additional_filters() -> Result<(), dbkit::Error> {
    let pool = PgPoolOptions::new().connect(&db_url()).await?;
    let tx = TestTx::new(pool.begin().await?);
    setup_schema(&tx).await?;

    let alpha_en = seed_row(&tx, LookupScope::Public, "alpha", "en", "Alpha").await?;
    seed_row(&tx, LookupScope::Internal, "beta", "en", "Beta").await?;

    let rows: Vec<LookupRow> = LookupRow::query()
        .filter(LookupRow::scope.eq(LookupScope::Public))
        .filter(
            row((LookupRow::scope, LookupRow::external_key, LookupRow::locale))
                .in_([(LookupScope::Public, "alpha", "en"), (LookupScope::Internal, "beta", "en")]),
        )
        .order_by(Order::asc(LookupRow::id))
        .all(&tx)
        .await?;

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, alpha_en.id);
    assert_eq!(rows[0].label, "Alpha");

    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn row_value_in_supports_integer_and_timestamp_columns() -> Result<(), dbkit::Error> {
    let pool = PgPoolOptions::new().connect(&db_url()).await?;
    let tx = TestTx::new(pool.begin().await?);
    setup_schema(&tx).await?;

    let first = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(2024, 4, 1).expect("date"),
        NaiveTime::from_hms_opt(9, 0, 0).expect("time"),
    );
    let second = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(2024, 4, 1).expect("date"),
        NaiveTime::from_hms_opt(10, 0, 0).expect("time"),
    );
    let third = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(2024, 4, 2).expect("date"),
        NaiveTime::from_hms_opt(9, 0, 0).expect("time"),
    );

    let first_row = seed_revision_snapshot(&tx, 42, 1, first, "First").await?;
    let second_row = seed_revision_snapshot(&tx, 42, 2, second, "Second").await?;
    let _other_series = seed_revision_snapshot(&tx, 7, 1, first, "Other series").await?;
    let _other_time = seed_revision_snapshot(&tx, 42, 3, third, "Other time").await?;

    let rows: Vec<RevisionSnapshot> = RevisionSnapshot::query()
        .filter(
            row((
                RevisionSnapshot::series_id,
                RevisionSnapshot::revision,
                RevisionSnapshot::captured_at,
            ))
            .in_([(42_i64, 1_i32, first), (42_i64, 2_i32, second)]),
        )
        .order_by(Order::asc(RevisionSnapshot::id))
        .all(&tx)
        .await?;

    assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), vec![first_row.id, second_row.id]);
    assert_eq!(
        rows.iter().map(|row| row.note.as_str()).collect::<Vec<_>>(),
        vec!["First", "Second"]
    );

    tx.rollback().await?;
    Ok(())
}
