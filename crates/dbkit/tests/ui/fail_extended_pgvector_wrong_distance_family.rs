use dbkit::{func, model, PgBit, PgHalfVector, PgSparseVector, PgVector};

#[model(table = "embeddings")]
pub struct Embedding {
    #[key]
    pub id: i64,
    pub dense: PgVector<3>,
    pub half: PgHalfVector<3>,
    pub sparse: Option<PgSparseVector<3>>,
    pub bits: PgBit<3>,
}

fn main() {
    let bits = PgBit::<3>::new([true; 3]).unwrap();
    let _ = func::l2_distance(Embedding::bits, bits.clone()); //~ E0277
    let _ = func::cosine_distance(Embedding::bits, bits.clone()); //~ E0277
    let _ = func::l1_distance(Embedding::bits, bits.clone()); //~ E0277
    let _ = func::inner_product(Embedding::bits, bits.clone()); //~ E0277
    let _ = func::inner_product_distance(Embedding::bits, bits); //~ E0277
    let _ = func::hamming_distance(Embedding::dense, Embedding::dense); //~ E0277
    let _ = func::jaccard_distance(Embedding::dense, Embedding::dense); //~ E0277
    let _ = func::hamming_distance(Embedding::half, Embedding::half); //~ E0277
    let _ = func::jaccard_distance(Embedding::half, Embedding::half); //~ E0277
    let _ = func::hamming_distance(Embedding::sparse, Embedding::sparse); //~ E0277
    let _ = func::jaccard_distance(Embedding::sparse, Embedding::sparse); //~ E0277
}
