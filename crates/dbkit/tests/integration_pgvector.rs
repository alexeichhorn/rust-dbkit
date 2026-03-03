#![allow(non_upper_case_globals)]

use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{func, model, Database, Executor, Order};

#[model(table = "embedding_rows")]
pub struct EmbeddingRow {
    #[key]
    pub id: i64,
    pub label: String,
    pub embedding: dbkit::PgVector<3>,
    pub embedding_optional: Option<dbkit::PgVector<3>>,
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn setup_schema<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    ex.execute(
        "CREATE EXTENSION IF NOT EXISTS vector",
        PgArguments::default(),
    )
    .await?;

    ex.execute(
        "CREATE TEMP TABLE embedding_rows (\
            id BIGINT PRIMARY KEY,\
            label TEXT NOT NULL,\
            embedding VECTOR(3) NOT NULL,\
            embedding_optional VECTOR(3) NULL\
        )",
        PgArguments::default(),
    )
    .await?;

    Ok(())
}

async fn seed_row<E: Executor + Send + Sync>(
    ex: &E,
    id: i64,
    label: &str,
    embedding: dbkit::PgVector<3>,
    embedding_optional: Option<dbkit::PgVector<3>>,
) -> Result<EmbeddingRow, dbkit::Error> {
    let row = EmbeddingRow::insert(EmbeddingRowInsert {
        id,
        label: label.to_string(),
        embedding,
        embedding_optional,
    })
    .returning_all()
    .one(ex)
    .await?
    .expect("inserted embedding row");
    Ok(row)
}

#[tokio::test]
async fn pgvector_roundtrip_eq_filter_and_active_model_nulling() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let embedding = dbkit::PgVector::<3>::new([0.1, 0.2, 0.3]).expect("embedding");
    let optional = dbkit::PgVector::<3>::new([0.3, 0.2, 0.1]).expect("optional embedding");

    let inserted = seed_row(&tx, 1, "row-1", embedding.clone(), Some(optional.clone())).await?;

    assert_eq!(inserted.id, 1);
    assert_eq!(inserted.embedding, embedding);
    assert_eq!(inserted.embedding_optional, Some(optional));

    let found = EmbeddingRow::query()
        .filter(EmbeddingRow::embedding.eq(embedding.clone()))
        .one(&tx)
        .await?
        .expect("row by vector equality");

    assert_eq!(found.id, 1);

    let mut active = found.into_active();
    active.embedding_optional = None::<dbkit::PgVector<3>>.into();
    let cleared = active.update(&tx).await?;

    assert!(cleared.embedding_optional.is_none());

    let only_null_optional = EmbeddingRow::query()
        .filter(EmbeddingRow::embedding_optional.eq(None::<dbkit::PgVector<3>>))
        .all(&tx)
        .await?;
    assert_eq!(only_null_optional.len(), 1);
    assert_eq!(only_null_optional[0].id, 1);

    Ok(())
}

#[tokio::test]
async fn pgvector_distance_functions_support_threshold_and_ordering() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    seed_row(
        &tx,
        1,
        "perfect",
        dbkit::PgVector::<3>::new([1.0, 0.0, 0.0]).expect("vector"),
        None,
    )
    .await?;
    seed_row(
        &tx,
        2,
        "close",
        dbkit::PgVector::<3>::new([0.9, 0.1, 0.0]).expect("vector"),
        None,
    )
    .await?;
    seed_row(
        &tx,
        3,
        "far",
        dbkit::PgVector::<3>::new([0.0, 1.0, 0.0]).expect("vector"),
        None,
    )
    .await?;

    let query = dbkit::PgVector::<3>::new([1.0, 0.0, 0.0]).expect("query vector");

    let nearest = EmbeddingRow::query()
        .filter(func::l2_distance(EmbeddingRow::embedding, query.clone()).lt(1.5_f32))
        .order_by(Order::asc(func::l2_distance(
            EmbeddingRow::embedding,
            query.clone(),
        )))
        .all(&tx)
        .await?;

    assert_eq!(nearest.len(), 3);
    assert_eq!(nearest[0].id, 1);
    assert_eq!(nearest[1].id, 2);
    assert_eq!(nearest[2].id, 3);

    let high_similarity = EmbeddingRow::query()
        .filter(func::cosine_distance(EmbeddingRow::embedding, query.clone()).lt(0.01_f32))
        .filter(func::inner_product(EmbeddingRow::embedding, query.clone()).gt(0.80_f32))
        .order_by(Order::asc(func::l1_distance(
            EmbeddingRow::embedding,
            query,
        )))
        .all(&tx)
        .await?;

    assert_eq!(high_similarity.len(), 2);
    assert_eq!(high_similarity[0].id, 1);
    assert_eq!(high_similarity[1].id, 2);

    Ok(())
}

#[tokio::test]
async fn pgvector_top_n_by_inner_product_desc_returns_expected_rank() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    seed_row(
        &tx,
        1,
        "perfect",
        dbkit::PgVector::<3>::new([1.0, 0.0, 0.0]).expect("vector"),
        Some(dbkit::PgVector::<3>::new([1.0, 0.0, 0.0]).expect("vector")),
    )
    .await?;
    seed_row(
        &tx,
        2,
        "close",
        dbkit::PgVector::<3>::new([0.9, 0.1, 0.0]).expect("vector"),
        Some(dbkit::PgVector::<3>::new([0.9, 0.1, 0.0]).expect("vector")),
    )
    .await?;
    seed_row(
        &tx,
        3,
        "orthogonal",
        dbkit::PgVector::<3>::new([0.0, 1.0, 0.0]).expect("vector"),
        Some(dbkit::PgVector::<3>::new([0.0, 1.0, 0.0]).expect("vector")),
    )
    .await?;
    seed_row(
        &tx,
        4,
        "opposite",
        dbkit::PgVector::<3>::new([-1.0, 0.0, 0.0]).expect("vector"),
        Some(dbkit::PgVector::<3>::new([-1.0, 0.0, 0.0]).expect("vector")),
    )
    .await?;

    let query = dbkit::PgVector::<3>::new([1.0, 0.0, 0.0]).expect("query vector");

    let top2 = EmbeddingRow::query()
        .order_by(Order::desc(func::inner_product(
            EmbeddingRow::embedding,
            query,
        )))
        .limit(2)
        .all(&tx)
        .await?;

    assert_eq!(top2.len(), 2);
    assert_eq!(top2[0].id, 1);
    assert_eq!(top2[1].id, 2);

    Ok(())
}

#[tokio::test]
async fn pgvector_top_n_optional_embeddings_uses_non_null_filter_for_determinism(
) -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    seed_row(
        &tx,
        1,
        "has-strong-optional",
        dbkit::PgVector::<3>::new([0.0, 0.0, 0.0]).expect("vector"),
        Some(dbkit::PgVector::<3>::new([1.0, 0.0, 0.0]).expect("vector")),
    )
    .await?;
    seed_row(
        &tx,
        2,
        "missing-optional",
        dbkit::PgVector::<3>::new([0.0, 0.0, 0.0]).expect("vector"),
        None,
    )
    .await?;
    seed_row(
        &tx,
        3,
        "has-close-optional",
        dbkit::PgVector::<3>::new([0.0, 0.0, 0.0]).expect("vector"),
        Some(dbkit::PgVector::<3>::new([0.9, 0.1, 0.0]).expect("vector")),
    )
    .await?;
    seed_row(
        &tx,
        4,
        "has-far-optional",
        dbkit::PgVector::<3>::new([0.0, 0.0, 0.0]).expect("vector"),
        Some(dbkit::PgVector::<3>::new([0.0, 1.0, 0.0]).expect("vector")),
    )
    .await?;

    let query = dbkit::PgVector::<3>::new([1.0, 0.0, 0.0]).expect("query vector");

    // We explicitly filter NULLs to avoid relying on PostgreSQL NULL sort defaults in DESC.
    let top2 = EmbeddingRow::query()
        .filter(EmbeddingRow::embedding_optional.is_not_null())
        .order_by(Order::desc(func::inner_product(
            EmbeddingRow::embedding_optional,
            query,
        )))
        .limit(2)
        .all(&tx)
        .await?;

    assert_eq!(top2.len(), 2);
    assert_eq!(top2[0].id, 1);
    assert_eq!(top2[1].id, 3);

    Ok(())
}
