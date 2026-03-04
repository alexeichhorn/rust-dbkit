use dbkit::model;

#[model(table = "embedding_rows")]
pub struct EmbeddingRow {
    #[key]
    pub id: i64,
    pub embedding: dbkit::PgVector<3>,
}

fn main() {
    let raw = vec![1.0_f32, 0.0, 0.0];

    let _query = EmbeddingRow::query().filter(EmbeddingRow::embedding.eq(raw)); //~ E0277
}
