use dbkit::{model, PgBit, PgHalfVector, PgSparseVector};

#[model(table = "embeddings")]
pub struct Embedding {
    #[key]
    pub id: i64,
    pub half: PgHalfVector<3>,
    pub sparse: PgSparseVector<3>,
    pub bits: PgBit<3>,
}

fn main() {
    let half = PgHalfVector::<4>::from_f32([1.0; 4]).unwrap();
    let sparse = PgSparseVector::<4>::new([(0, 1.0)]).unwrap();
    let bits = PgBit::<4>::new([true; 4]).unwrap();
    let _ = Embedding::half.eq(half); //~ E0277
    let _ = Embedding::sparse.eq(sparse); //~ E0277
    let _ = Embedding::bits.eq(bits); //~ E0277
}
