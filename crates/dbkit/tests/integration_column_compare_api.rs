#![allow(non_upper_case_globals)]

use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{model, Database, Executor};

#[model(table = "jobs_col_compare")]
pub struct JobColCompare {
    #[key]
    #[autoincrement]
    pub id: i64,
    pub content_hash: String,
    pub last_content_hash: String,
    pub embedding_hash: Option<String>,
    pub embedding: Option<String>,
    pub retry_count: i64,
    pub max_retries: i64,
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn setup_schema<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    ex.execute(
        "CREATE TEMP TABLE jobs_col_compare (\
            id BIGSERIAL PRIMARY KEY,\
            content_hash TEXT NOT NULL,\
            last_content_hash TEXT NOT NULL,\
            embedding_hash TEXT,\
            embedding TEXT,\
            retry_count BIGINT NOT NULL,\
            max_retries BIGINT NOT NULL\
        )",
        PgArguments::default(),
    )
    .await?;

    Ok(())
}

#[tokio::test]
async fn ne_col_filters_changed_hash_rows() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    JobColCompare::insert_many(vec![
        JobColCompareInsert {
            content_hash: "h1".to_string(),
            last_content_hash: "h1".to_string(),
            embedding_hash: Some("h1".to_string()),
            embedding: Some("v1".to_string()),
            retry_count: 0,
            max_retries: 3,
        },
        JobColCompareInsert {
            content_hash: "h2".to_string(),
            last_content_hash: "h1".to_string(),
            embedding_hash: Some("h1".to_string()),
            embedding: Some("v2".to_string()),
            retry_count: 1,
            max_retries: 3,
        },
    ])
    .execute(&tx)
    .await?;

    let changed = JobColCompare::query()
        .filter(JobColCompare::content_hash.ne_col(JobColCompare::last_content_hash))
        .all(&tx)
        .await?;

    assert_eq!(changed.len(), 1);
    assert_eq!(changed[0].content_hash, "h2");
    assert_eq!(changed[0].last_content_hash, "h1");

    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn lt_col_and_ge_col_work_for_numeric_column_comparisons() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    JobColCompare::insert_many(vec![
        JobColCompareInsert {
            content_hash: "h1".to_string(),
            last_content_hash: "h1".to_string(),
            embedding_hash: Some("h1".to_string()),
            embedding: Some("v1".to_string()),
            retry_count: 1,
            max_retries: 3,
        },
        JobColCompareInsert {
            content_hash: "h2".to_string(),
            last_content_hash: "h2".to_string(),
            embedding_hash: Some("h2".to_string()),
            embedding: Some("v2".to_string()),
            retry_count: 3,
            max_retries: 3,
        },
        JobColCompareInsert {
            content_hash: "h3".to_string(),
            last_content_hash: "h3".to_string(),
            embedding_hash: Some("h3".to_string()),
            embedding: Some("v3".to_string()),
            retry_count: 5,
            max_retries: 3,
        },
    ])
    .execute(&tx)
    .await?;

    let retryable = JobColCompare::query()
        .filter(JobColCompare::retry_count.lt_col(JobColCompare::max_retries))
        .all(&tx)
        .await?;
    assert_eq!(retryable.len(), 1);
    assert_eq!(retryable[0].content_hash, "h1");

    let exhausted_or_over = JobColCompare::query()
        .filter(JobColCompare::retry_count.ge_col(JobColCompare::max_retries))
        .all(&tx)
        .await?;
    assert_eq!(exhausted_or_over.len(), 2);

    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn stale_embedding_predicate_catches_nulls_and_hash_mismatch() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    JobColCompare::insert_many(vec![
        JobColCompareInsert {
            // fresh
            content_hash: "fresh".to_string(),
            last_content_hash: "fresh".to_string(),
            embedding_hash: Some("fresh".to_string()),
            embedding: Some("vec-fresh".to_string()),
            retry_count: 0,
            max_retries: 3,
        },
        JobColCompareInsert {
            // missing embedding payload
            content_hash: "missing-embedding".to_string(),
            last_content_hash: "missing-embedding".to_string(),
            embedding_hash: Some("missing-embedding".to_string()),
            embedding: None,
            retry_count: 0,
            max_retries: 3,
        },
        JobColCompareInsert {
            // missing embedding hash
            content_hash: "missing-hash".to_string(),
            last_content_hash: "missing-hash".to_string(),
            embedding_hash: None,
            embedding: Some("vec-present".to_string()),
            retry_count: 0,
            max_retries: 3,
        },
        JobColCompareInsert {
            // stale hash mismatch (non-null on both sides)
            content_hash: "new-content".to_string(),
            last_content_hash: "new-content".to_string(),
            embedding_hash: Some("old-content".to_string()),
            embedding: Some("vec-stale".to_string()),
            retry_count: 0,
            max_retries: 3,
        },
    ])
    .execute(&tx)
    .await?;

    let stale = JobColCompare::query()
        .filter(
            JobColCompare::embedding
                .is_null()
                .or(JobColCompare::embedding_hash.is_distinct_from_col(JobColCompare::content_hash)),
        )
        .all(&tx)
        .await?;

    // Should return all except the fresh row.
    assert_eq!(stale.len(), 3);

    let content_hashes: Vec<String> = stale.into_iter().map(|row| row.content_hash).collect();
    assert!(content_hashes.contains(&"missing-embedding".to_string()));
    assert!(content_hashes.contains(&"missing-hash".to_string()));
    assert!(content_hashes.contains(&"new-content".to_string()));
    assert!(!content_hashes.contains(&"fresh".to_string()));

    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn stale_embedding_predicate_with_coalesce_catches_nulls_and_hash_mismatch() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    JobColCompare::insert_many(vec![
        JobColCompareInsert {
            // fresh
            content_hash: "fresh".to_string(),
            last_content_hash: "fresh".to_string(),
            embedding_hash: Some("fresh".to_string()),
            embedding: Some("vec-fresh".to_string()),
            retry_count: 0,
            max_retries: 3,
        },
        JobColCompareInsert {
            // missing embedding payload
            content_hash: "missing-embedding".to_string(),
            last_content_hash: "missing-embedding".to_string(),
            embedding_hash: Some("missing-embedding".to_string()),
            embedding: None,
            retry_count: 0,
            max_retries: 3,
        },
        JobColCompareInsert {
            // missing embedding hash
            content_hash: "missing-hash".to_string(),
            last_content_hash: "missing-hash".to_string(),
            embedding_hash: None,
            embedding: Some("vec-present".to_string()),
            retry_count: 0,
            max_retries: 3,
        },
        JobColCompareInsert {
            // stale hash mismatch (non-null on both sides)
            content_hash: "new-content".to_string(),
            last_content_hash: "new-content".to_string(),
            embedding_hash: Some("old-content".to_string()),
            embedding: Some("vec-stale".to_string()),
            retry_count: 0,
            max_retries: 3,
        },
    ])
    .execute(&tx)
    .await?;

    let stale = JobColCompare::query()
        .filter(
            JobColCompare::embedding
                .is_null()
                .or(JobColCompare::embedding_hash.is_null())
                .or(dbkit::func::coalesce(JobColCompare::embedding_hash, "").ne_col(JobColCompare::content_hash)),
        )
        .all(&tx)
        .await?;

    // Should return all except the fresh row.
    assert_eq!(stale.len(), 3);

    let content_hashes: Vec<String> = stale.into_iter().map(|row| row.content_hash).collect();
    assert!(content_hashes.contains(&"missing-embedding".to_string()));
    assert!(content_hashes.contains(&"missing-hash".to_string()));
    assert!(content_hashes.contains(&"new-content".to_string()));
    assert!(!content_hashes.contains(&"fresh".to_string()));

    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn is_distinct_from_col_matches_null_safe_truth_table_for_nullable_columns() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    JobColCompare::insert_many(vec![
        JobColCompareInsert {
            // equal (non-null): false
            content_hash: "eq_non_null".to_string(),
            last_content_hash: "eq_non_null".to_string(),
            embedding_hash: Some("same".to_string()),
            embedding: Some("same".to_string()),
            retry_count: 0,
            max_retries: 3,
        },
        JobColCompareInsert {
            // different (non-null): true
            content_hash: "diff_non_null".to_string(),
            last_content_hash: "diff_non_null".to_string(),
            embedding_hash: Some("a".to_string()),
            embedding: Some("b".to_string()),
            retry_count: 0,
            max_retries: 3,
        },
        JobColCompareInsert {
            // null vs value: true
            content_hash: "null_vs_value".to_string(),
            last_content_hash: "null_vs_value".to_string(),
            embedding_hash: None,
            embedding: Some("b".to_string()),
            retry_count: 0,
            max_retries: 3,
        },
        JobColCompareInsert {
            // value vs null: true
            content_hash: "value_vs_null".to_string(),
            last_content_hash: "value_vs_null".to_string(),
            embedding_hash: Some("a".to_string()),
            embedding: None,
            retry_count: 0,
            max_retries: 3,
        },
        JobColCompareInsert {
            // null vs null: false
            content_hash: "null_vs_null".to_string(),
            last_content_hash: "null_vs_null".to_string(),
            embedding_hash: None,
            embedding: None,
            retry_count: 0,
            max_retries: 3,
        },
    ])
    .execute(&tx)
    .await?;

    let rows = JobColCompare::query()
        .filter(JobColCompare::embedding_hash.is_distinct_from_col(JobColCompare::embedding))
        .all(&tx)
        .await?;

    let content_hashes: Vec<String> = rows.into_iter().map(|row| row.content_hash).collect();
    assert_eq!(content_hashes.len(), 3);
    assert!(content_hashes.contains(&"diff_non_null".to_string()));
    assert!(content_hashes.contains(&"null_vs_value".to_string()));
    assert!(content_hashes.contains(&"value_vs_null".to_string()));
    assert!(!content_hashes.contains(&"eq_non_null".to_string()));
    assert!(!content_hashes.contains(&"null_vs_null".to_string()));

    tx.rollback().await?;
    Ok(())
}

#[tokio::test]
async fn is_not_distinct_from_col_matches_null_safe_truth_table_for_nullable_columns() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    JobColCompare::insert_many(vec![
        JobColCompareInsert {
            // equal (non-null): true
            content_hash: "eq_non_null".to_string(),
            last_content_hash: "eq_non_null".to_string(),
            embedding_hash: Some("same".to_string()),
            embedding: Some("same".to_string()),
            retry_count: 0,
            max_retries: 3,
        },
        JobColCompareInsert {
            // different (non-null): false
            content_hash: "diff_non_null".to_string(),
            last_content_hash: "diff_non_null".to_string(),
            embedding_hash: Some("a".to_string()),
            embedding: Some("b".to_string()),
            retry_count: 0,
            max_retries: 3,
        },
        JobColCompareInsert {
            // null vs value: false
            content_hash: "null_vs_value".to_string(),
            last_content_hash: "null_vs_value".to_string(),
            embedding_hash: None,
            embedding: Some("b".to_string()),
            retry_count: 0,
            max_retries: 3,
        },
        JobColCompareInsert {
            // value vs null: false
            content_hash: "value_vs_null".to_string(),
            last_content_hash: "value_vs_null".to_string(),
            embedding_hash: Some("a".to_string()),
            embedding: None,
            retry_count: 0,
            max_retries: 3,
        },
        JobColCompareInsert {
            // null vs null: true
            content_hash: "null_vs_null".to_string(),
            last_content_hash: "null_vs_null".to_string(),
            embedding_hash: None,
            embedding: None,
            retry_count: 0,
            max_retries: 3,
        },
    ])
    .execute(&tx)
    .await?;

    let rows = JobColCompare::query()
        .filter(JobColCompare::embedding_hash.is_not_distinct_from_col(JobColCompare::embedding))
        .all(&tx)
        .await?;

    let content_hashes: Vec<String> = rows.into_iter().map(|row| row.content_hash).collect();
    assert_eq!(content_hashes.len(), 2);
    assert!(content_hashes.contains(&"eq_non_null".to_string()));
    assert!(content_hashes.contains(&"null_vs_null".to_string()));
    assert!(!content_hashes.contains(&"diff_non_null".to_string()));
    assert!(!content_hashes.contains(&"null_vs_value".to_string()));
    assert!(!content_hashes.contains(&"value_vs_null".to_string()));

    tx.rollback().await?;
    Ok(())
}
