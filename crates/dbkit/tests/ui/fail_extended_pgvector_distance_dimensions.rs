use dbkit::{func, model, PgBit, PgHalfVector, PgSparseVector};

#[model(table = "embeddings")]
pub struct Embedding {
    #[key]
    pub id: i64,
    pub half: PgHalfVector<3>,
    pub sparse: Option<PgSparseVector<3>>,
    pub bits: Option<PgBit<3>>,
}

fn main() {
    let half = PgHalfVector::<4>::from_f32([1.0; 4]).unwrap();
    let sparse = PgSparseVector::<4>::new([(0, 1.0)]).unwrap();
    let bits = PgBit::<4>::new([true; 4]).unwrap();
    let _ = func::l2_distance(Embedding::half, half); //~ E0277
    let _ = func::cosine_distance(Embedding::sparse, sparse); //~ E0277
    let _ = func::hamming_distance(Embedding::bits, bits.clone()); //~ E0277
    let _ = func::jaccard_distance(bits, Embedding::bits); //~ E0277
}
