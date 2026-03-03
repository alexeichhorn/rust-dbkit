#![allow(non_upper_case_globals)]

use dbkit::executor::build_arguments;
use dbkit::model;
use dbkit::{func, Order, Value};

#[model(table = "embedding_rows")]
pub struct EmbeddingRow {
    #[key]
    pub id: i64,
    pub label: String,
    pub embedding: dbkit::PgVector<3>,
    pub embedding_optional: Option<dbkit::PgVector<3>>,
}

#[test]
fn build_arguments_accepts_pgvector() {
    let embedding = dbkit::PgVector::<3>::new([0.1, 0.2, 0.3]).expect("vector");

    let values = vec![Value::from(embedding)];
    let result = build_arguments(&values);

    assert!(result.is_ok());
}

#[test]
fn query_with_pgvector_distance_functions_has_expected_sql_shape() {
    let query = dbkit::PgVector::<3>::new([1.0, 0.0, 0.0]).expect("vector");

    let sql = EmbeddingRow::query()
        .filter(func::l2_distance(EmbeddingRow::embedding, query.clone()).lt(0.35_f32))
        .filter(func::cosine_distance(EmbeddingRow::embedding, query.clone()).lt(0.2_f32))
        .order_by(Order::asc(func::l2_distance(
            EmbeddingRow::embedding,
            query.clone(),
        )))
        .order_by(Order::asc(func::cosine_distance(
            EmbeddingRow::embedding,
            query,
        )))
        .debug_sql();

    assert!(sql.contains("L2_DISTANCE(embedding_rows.embedding, $1::vector)"));
    assert!(sql.contains("COSINE_DISTANCE(embedding_rows.embedding, $1::vector)"));
    assert!(sql.contains("ORDER BY L2_DISTANCE(embedding_rows.embedding, $1::vector) ASC"));
    assert!(sql.contains("COSINE_DISTANCE(embedding_rows.embedding, $1::vector) ASC"));
}

#[test]
fn insert_and_update_builders_accept_pgvector_and_optional_pgvector() {
    let embedding = dbkit::PgVector::<3>::new([0.11, 0.22, 0.33]).expect("vector");
    let replacement = dbkit::PgVector::<3>::new([0.44, 0.55, 0.66]).expect("vector");

    let insert_sql = EmbeddingRow::insert(EmbeddingRowInsert {
        id: 1,
        label: "hello".to_string(),
        embedding: embedding.clone(),
        embedding_optional: None,
    })
    .returning_all()
    .compile()
    .sql;

    let update_sql = EmbeddingRow::update()
        .set(EmbeddingRow::embedding, replacement)
        .set(EmbeddingRow::embedding_optional, Some(embedding.clone()))
        .filter(EmbeddingRow::id.eq(1_i64))
        .compile()
        .sql;

    let clear_optional_sql = EmbeddingRow::update()
        .set(EmbeddingRow::embedding_optional, None::<dbkit::PgVector<3>>)
        .filter(EmbeddingRow::id.eq(1_i64))
        .compile()
        .sql;

    assert!(insert_sql.contains("INSERT INTO embedding_rows"));
    assert!(update_sql.contains("UPDATE embedding_rows SET embedding = $1"));
    assert!(clear_optional_sql.contains("embedding_optional = NULL"));
}

#[test]
fn pgvector_constructors_validate_non_finite_inputs() {
    assert!(dbkit::PgVector::<3>::new([0.0, f32::NAN, 1.0]).is_err());
    assert!(dbkit::PgVector::<3>::new([0.0, f32::INFINITY, 1.0]).is_err());
}
