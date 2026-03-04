use dbkit::model;

#[model(table = "embedding_rows")]
pub struct EmbeddingRow {
    #[key]
    pub id: i64,
    pub embedding: dbkit::PgVector<3>,
}

fn main() {
    let wrong = dbkit::PgVector::<4>::new([1.0, 0.0, 0.0, 0.0]).expect("vector");

    let _query = EmbeddingRow::query().filter(EmbeddingRow::embedding.eq(wrong)); //~ E0277
}
