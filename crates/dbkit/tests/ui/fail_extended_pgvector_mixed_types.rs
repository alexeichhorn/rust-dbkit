use dbkit::{func, model, PgHalfVector, PgSparseVector, PgVector};

#[model(table = "embeddings")]
pub struct Embedding {
    #[key]
    pub id: i64,
    pub dense: PgVector<3>,
    pub half: PgHalfVector<3>,
    pub sparse: Option<PgSparseVector<3>>,
}

fn main() {
    let dense = PgVector::<3>::new([1.0; 3]).unwrap();
    let half = PgHalfVector::<3>::from_f32([1.0; 3]).unwrap();
    let sparse = PgSparseVector::<3>::new([(0, 1.0)]).unwrap();
    let _ = func::l2_distance(Embedding::dense, half.clone()); //~ E0277
    let _ = func::l2_distance(Embedding::half, dense.clone()); //~ E0277
    let _ = func::cosine_distance(Embedding::half, sparse.clone()); //~ E0277
    let _ = func::cosine_distance(Embedding::sparse, half); //~ E0277
    let _ = func::inner_product_distance(Embedding::dense, sparse); //~ E0277
    let _ = func::inner_product_distance(Embedding::sparse, dense); //~ E0277
}
