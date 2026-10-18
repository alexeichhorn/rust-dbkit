//@check-pass
use dbkit::{func, model, Expr, PgBit, PgBitError, PgHalfVector, PgHalfVectorError, PgSparseVector, PgSparseVectorError};

#[model(table = "embeddings")]
pub struct Embedding {
    #[key]
    pub id: i64,
    pub half: PgHalfVector<3>,
    pub sparse: PgSparseVector<1000>,
    pub bits: PgBit<9>,
    pub optional_half: Option<PgHalfVector<3>>,
    pub optional_sparse: Option<PgSparseVector<1000>>,
    pub optional_bits: Option<PgBit<9>>,
}

fn assert_error<T: std::error::Error>() {}

fn main() {
    assert_error::<PgHalfVectorError>();
    assert_error::<PgSparseVectorError>();
    assert_error::<PgBitError>();

    let half = PgHalfVector::<3>::from_f32([1.0, 0.0, 0.0]).unwrap();
    let sparse = PgSparseVector::<1000>::new([(0, 1.0)]).unwrap();
    let bits: PgBit<9> = "100000001".parse().unwrap();
    let _: Expr<f64> = func::cosine_distance(Embedding::half, half.clone());
    let _: Expr<Option<f64>> = func::l2_distance(Embedding::optional_sparse, sparse.clone());
    let _: Expr<f64> = func::hamming_distance(Embedding::bits, bits.clone());
    let _: Expr<Option<f64>> = func::jaccard_distance(bits.clone(), Embedding::optional_bits);

    let _insert = Embedding::insert(EmbeddingInsert {
        id: 1,
        half: half.clone(),
        sparse: sparse.clone(),
        bits: bits.clone(),
        optional_half: Some(half.clone()),
        optional_sparse: Some(sparse.clone()),
        optional_bits: Some(bits.clone()),
    })
    .returning_all();

    let _query = Embedding::query()
        .filter(Embedding::half.eq(half.clone()))
        .filter(Embedding::sparse.eq(sparse.clone()))
        .filter(Embedding::bits.eq(bits.clone()))
        .filter(Embedding::optional_half.eq(None::<PgHalfVector<3>>))
        .filter(Embedding::optional_sparse.eq(None::<PgSparseVector<1000>>))
        .filter(Embedding::optional_bits.eq(None::<PgBit<9>>));

    let _update = Embedding::update()
        .set(Embedding::half, half.clone())
        .set(Embedding::sparse, sparse.clone())
        .set(Embedding::bits, bits.clone())
        .set(Embedding::optional_half, None::<PgHalfVector<3>>)
        .set(Embedding::optional_sparse, None::<PgSparseVector<1000>>)
        .set(Embedding::optional_bits, None::<PgBit<9>>);

    let mut active = Embedding::new_active();
    active.id = 1_i64.into();
    active.half = half.clone().into();
    active.sparse = sparse.clone().into();
    active.bits = bits.clone().into();
    active.optional_half = Some(half).into();
    active.optional_sparse = Some(sparse).into();
    active.optional_bits = Some(bits).into();
}
