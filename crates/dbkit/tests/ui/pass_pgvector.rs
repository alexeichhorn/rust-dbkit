//@check-pass
use dbkit::{model, Expr};

#[model(table = "embedding_rows")]
pub struct EmbeddingRow {
    #[key]
    pub id: i64,
    pub label: String,
    pub embedding: dbkit::PgVector<3>,
    pub embedding_optional: Option<dbkit::PgVector<3>>,
}

fn assert_f32(_: Expr<f32>) {}

fn assert_nullable_f32(_: Expr<Option<f32>>) {}

fn main() {
    let query = dbkit::PgVector::<3>::new([1.0, 0.0, 0.0]).expect("vector");
    let replacement = dbkit::PgVector::<3>::new([0.0, 1.0, 0.0]).expect("vector");

    assert_f32(dbkit::func::l2_distance(EmbeddingRow::embedding, query.clone()));
    assert_f32(dbkit::func::cosine_distance(EmbeddingRow::embedding, query.clone()));
    assert_f32(dbkit::func::inner_product(EmbeddingRow::embedding, query.clone()));
    assert_f32(dbkit::func::l1_distance(EmbeddingRow::embedding, query.clone()));
    assert_f32(dbkit::func::inner_product_distance(EmbeddingRow::embedding, query.clone()));
    assert_nullable_f32(dbkit::func::l2_distance(EmbeddingRow::embedding_optional, query.clone()));
    assert_nullable_f32(dbkit::func::cosine_distance(EmbeddingRow::embedding_optional, query.clone()));
    assert_nullable_f32(dbkit::func::inner_product(EmbeddingRow::embedding_optional, query.clone()));
    assert_nullable_f32(dbkit::func::l1_distance(EmbeddingRow::embedding_optional, query.clone()));
    assert_nullable_f32(dbkit::func::inner_product_distance(EmbeddingRow::embedding_optional, query.clone()));
    assert_nullable_f32(dbkit::func::l2_distance(query.clone(), EmbeddingRow::embedding_optional));
    assert_nullable_f32(dbkit::func::cosine_distance(
        EmbeddingRow::embedding_optional,
        EmbeddingRow::embedding_optional,
    ));

    let _query = EmbeddingRow::query()
        .filter(EmbeddingRow::embedding.eq(query.clone()))
        .filter(EmbeddingRow::embedding_optional.eq(None::<dbkit::PgVector<3>>))
        .filter(dbkit::func::l2_distance(EmbeddingRow::embedding, query.clone()).lt(0.5_f32))
        .filter(dbkit::func::cosine_distance(EmbeddingRow::embedding, query.clone()).lt(0.2_f32))
        .filter(dbkit::func::inner_product(EmbeddingRow::embedding, query.clone()).gt(0.8_f32))
        .order_by(dbkit::Order::asc(dbkit::func::l1_distance(EmbeddingRow::embedding, query)));

    let _insert = EmbeddingRow::insert(EmbeddingRowInsert {
        id: 1,
        label: "hello".to_string(),
        embedding: replacement.clone(),
        embedding_optional: Some(replacement.clone()),
    });

    let _update = EmbeddingRow::update()
        .set(EmbeddingRow::embedding, replacement.clone())
        .set(EmbeddingRow::embedding_optional, Some(replacement.clone()));

    let mut active = EmbeddingRow::new_active();
    active.id = 1_i64.into();
    active.label = "active".to_string().into();
    active.embedding = replacement.into();
    active.embedding_optional = None::<dbkit::PgVector<3>>.into();

    let _ = active;
}
