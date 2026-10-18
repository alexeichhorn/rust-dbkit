#![allow(non_upper_case_globals)]

use dbkit::prelude::*;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::sqlx::{self, Row};
use dbkit::{func, model, Database, Executor, Expr, Order, PgBit, PgHalfVector, PgSparseVector};

#[model(table = "extended_embeddings")]
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

#[derive(dbkit::sqlx::FromRow)]
struct DistanceRow {
    id: i64,
    distance: f64,
}

fn db_url() -> String {
    let _ = dotenvy::dotenv();
    std::env::var("DB_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .expect("DB_URL or DATABASE_URL must be set for integration tests")
}

async fn setup_schema<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    ex.execute("CREATE EXTENSION IF NOT EXISTS vector", PgArguments::default()).await?;
    ex.execute(
        "CREATE TEMP TABLE extended_embeddings (
            id BIGINT PRIMARY KEY,
            half HALFVEC(3) NOT NULL,
            sparse SPARSEVEC(1000) NOT NULL,
            bits BIT(9) NOT NULL,
            optional_half HALFVEC(3),
            optional_sparse SPARSEVEC(1000),
            optional_bits BIT(9)
        )",
        PgArguments::default(),
    )
    .await?;
    Ok(())
}

async fn seed_distances<E: Executor + Send + Sync>(ex: &E) -> Result<(), dbkit::Error> {
    // SQL literals keep the distance expectations independent of our encoders.
    ex.execute(
        "INSERT INTO extended_embeddings (id, half, sparse, bits) VALUES
            (1, '[1,0,0]', '{1:1}/1000', '110000000'),
            (2, '[0.5,0.5,0]', '{1:0.5,1000:0.5}/1000', '100000000'),
            (3, '[0,1,0]', '{1000:1}/1000', '111110000'),
            (4, '[-1,0,0]', '{1:-1}/1000', '000000001')",
        PgArguments::default(),
    )
    .await?;
    Ok(())
}

#[tokio::test]
async fn extended_vectors_roundtrip_insert_update_equality_and_nulls() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;

    let half = PgHalfVector::<3>::from_f32([0.1, 0.5, -0.25]).unwrap();
    let sparse = PgSparseVector::<1000>::new([(999, -0.25), (0, 0.5)]).unwrap();
    let bits: PgBit<9> = "010000001".parse().unwrap();
    let inserted = Embedding::insert(EmbeddingInsert {
        id: 1,
        half: half.clone(),
        sparse: sparse.clone(),
        bits: bits.clone(),
        optional_half: Some(half.clone()),
        optional_sparse: Some(sparse.clone()),
        optional_bits: Some(bits.clone()),
    })
    .returning_all()
    .one(&tx)
    .await?
    .unwrap();
    assert_eq!(inserted.half, half);
    assert_eq!(inserted.sparse, sparse);
    assert_eq!(inserted.bits, bits);
    assert_eq!(inserted.optional_half, Some(half.clone()));
    assert_eq!(inserted.optional_sparse, Some(sparse.clone()));
    assert_eq!(inserted.optional_bits, Some(bits.clone()));

    let (stored_half, stored_sparse, stored_bits): (String, String, String) = tx
        .fetch_optional(
            "SELECT half::text, sparse::text, bits::text FROM extended_embeddings WHERE id = 1",
            PgArguments::default(),
        )
        .await?
        .unwrap();
    assert_eq!(stored_half.parse::<PgHalfVector<3>>().unwrap(), half);
    assert_eq!(stored_sparse, "{1:0.5,1000:-0.25}/1000");
    assert_eq!(stored_bits, "010000001");

    let found = Embedding::query()
        .filter(Embedding::half.eq(half.clone()))
        .filter(Embedding::sparse.eq(sparse.clone()))
        .filter(Embedding::bits.eq(bits.clone()))
        .one(&tx)
        .await?
        .unwrap();
    assert_eq!(found.id, 1);

    let replacement_half = PgHalfVector::<3>::from_f32([0.0, 1.0, 0.0]).unwrap();
    let replacement_sparse = PgSparseVector::<1000>::new([]).unwrap();
    let replacement_bits = PgBit::<9>::new([false; 9]).unwrap();
    Embedding::update()
        .set(Embedding::half, replacement_half.clone())
        .set(Embedding::sparse, replacement_sparse.clone())
        .set(Embedding::bits, replacement_bits.clone())
        .filter(Embedding::id.eq(1_i64))
        .execute(&tx)
        .await?;
    let updated = Embedding::query().filter(Embedding::id.eq(1_i64)).one(&tx).await?.unwrap();
    assert_eq!(updated.half, replacement_half);
    assert_eq!(updated.sparse, replacement_sparse);
    assert_eq!(updated.bits, replacement_bits);

    let mut active = updated.into_active();
    active.optional_half = None::<PgHalfVector<3>>.into();
    active.optional_sparse = None::<PgSparseVector<1000>>.into();
    active.optional_bits = None::<PgBit<9>>.into();
    let cleared = active.update(&tx).await?;
    assert!(cleared.optional_half.is_none());
    assert!(cleared.optional_sparse.is_none());
    assert!(cleared.optional_bits.is_none());
    let null_rows = Embedding::query()
        .filter(Embedding::optional_half.eq(None::<PgHalfVector<3>>))
        .filter(Embedding::optional_sparse.eq(None::<PgSparseVector<1000>>))
        .filter(Embedding::optional_bits.eq(None::<PgBit<9>>))
        .all(&tx)
        .await?;
    assert_eq!(null_rows.len(), 1);
    assert_eq!(null_rows[0].id, 1);

    active = cleared.into_active();
    active.optional_half = Some(half.clone()).into();
    active.optional_sparse = Some(sparse.clone()).into();
    active.optional_bits = Some(bits.clone()).into();
    let restored = active.update(&tx).await?;
    assert_eq!(restored.optional_half, Some(half));
    assert_eq!(restored.optional_sparse, Some(sparse));
    assert_eq!(restored.optional_bits, Some(bits));
    tx.rollback().await
}

macro_rules! numeric_distances {
    ($name:ident, $column:ident, $optional:ident, $query:expr) => {
        #[tokio::test]
        async fn $name() -> Result<(), dbkit::Error> {
            let db = Database::connect(&db_url()).await?;
            let tx = db.begin().await?;
            setup_schema(&tx).await?;
            seed_distances(&tx).await?;
            let query = $query;

            let cases = [
                (
                    func::l2_distance(Embedding::$column, query.clone()),
                    [0.0, 0.5_f64.sqrt(), 2.0_f64.sqrt(), 2.0],
                    false,
                ),
                (
                    func::cosine_distance(Embedding::$column, query.clone()),
                    [0.0, 1.0 - 0.5_f64.sqrt(), 1.0, 2.0],
                    false,
                ),
                (
                    func::l1_distance(Embedding::$column, query.clone()),
                    [0.0, 1.0, 2.0, 2.0],
                    false,
                ),
                (
                    func::inner_product_distance(Embedding::$column, query.clone()),
                    [-1.0, -0.5, 0.0, 1.0],
                    false,
                ),
                (
                    func::inner_product(Embedding::$column, query.clone()),
                    [1.0, 0.5, 0.0, -1.0],
                    true,
                ),
            ];
            for (distance, expected, descending) in cases {
                let order = if descending {
                    Order::desc(distance.clone())
                } else {
                    Order::asc(distance.clone())
                };
                let rows: Vec<DistanceRow> = Embedding::query()
                    .select_only()
                    .column(Embedding::id)
                    .column_as(distance, "distance")
                    .order_by(order)
                    .order_by(Order::asc(Embedding::id))
                    .into_model()
                    .all(&tx)
                    .await?;
                assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 2, 3, 4]);
                for (row, expected) in rows.iter().zip(expected) {
                    assert!(
                        (row.distance - expected).abs() < 1e-6,
                        "id {}: {} != {expected}",
                        row.id,
                        row.distance
                    );
                }
            }

            let closest = Embedding::query()
                .filter(func::inner_product_distance(Embedding::$column, query.clone()).lt(-0.4_f32))
                .order_by(Order::asc(func::l2_distance(Embedding::$column, query.clone())))
                .limit(2)
                .all(&tx)
                .await?;
            assert_eq!(closest.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 2]);

            for distance in [
                func::l2_distance(Embedding::$optional, query.clone()),
                func::cosine_distance(Embedding::$optional, query.clone()),
                func::l1_distance(Embedding::$optional, query.clone()),
                func::inner_product_distance(Embedding::$optional, query.clone()),
                func::inner_product(Embedding::$optional, query),
            ] {
                let rows = Embedding::query().filter(distance.eq(None)).all(&tx).await?;
                assert_eq!(rows.len(), 4);
            }
            tx.rollback().await
        }
    };
}

numeric_distances!(
    half_distances_match_postgres_values_and_rankings,
    half,
    optional_half,
    PgHalfVector::<3>::from_f32([1.0, 0.0, 0.0]).unwrap()
);
numeric_distances!(
    sparse_distances_match_postgres_values_and_rankings,
    sparse,
    optional_sparse,
    PgSparseVector::<1000>::new([(0, 1.0)]).unwrap()
);

#[tokio::test]
async fn bit_distances_match_postgres_values_rankings_and_nulls() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_distances(&tx).await?;
    let query: PgBit<9> = "110000000".parse().unwrap();

    for (distance, expected) in [
        (func::hamming_distance(Embedding::bits, query.clone()), [0.0, 1.0, 3.0, 3.0]),
        (func::jaccard_distance(Embedding::bits, query.clone()), [0.0, 0.5, 0.6, 1.0]),
    ] {
        let rows: Vec<DistanceRow> = Embedding::query()
            .select_only()
            .column(Embedding::id)
            .column_as(distance.clone(), "distance")
            .order_by(Order::asc(distance))
            .order_by(Order::asc(Embedding::id))
            .into_model()
            .all(&tx)
            .await?;
        assert_eq!(rows.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 2, 3, 4]);
        for (row, expected) in rows.iter().zip(expected) {
            assert!(
                (row.distance - expected).abs() < 1e-6,
                "id {}: {} != {expected}",
                row.id,
                row.distance
            );
        }
    }

    let closest = Embedding::query()
        .filter(func::hamming_distance(Embedding::bits, query.clone()).lt(2.0_f32))
        .filter(func::jaccard_distance(Embedding::bits, query.clone()).lt(0.75_f32))
        .order_by(Order::asc(func::hamming_distance(Embedding::bits, query.clone())))
        .limit(2)
        .all(&tx)
        .await?;
    assert_eq!(closest.iter().map(|row| row.id).collect::<Vec<_>>(), [1, 2]);
    for distance in [
        func::hamming_distance(Embedding::optional_bits, query.clone()),
        func::jaccard_distance(Embedding::optional_bits, query),
    ] {
        assert_eq!(Embedding::query().filter(distance.eq(None)).all(&tx).await?.len(), 4);
    }
    tx.rollback().await
}

#[tokio::test]
async fn hamming_distance_decodes_as_its_expression_type() -> Result<(), dbkit::Error> {
    let query: PgBit<9> = "110000000".parse().unwrap();
    let distance = func::hamming_distance(Embedding::bits, query);
    assert_eq!(project_second_row_distance(distance).await?, 1.0);
    Ok(())
}

#[tokio::test]
async fn jaccard_distance_decodes_as_its_expression_type() -> Result<(), dbkit::Error> {
    let query: PgBit<9> = "110000000".parse().unwrap();
    let distance = func::jaccard_distance(Embedding::bits, query);
    assert_eq!(project_second_row_distance(distance).await?, 0.5);
    Ok(())
}

// Infer the decoder from Expr<T> so the projection cannot hide a result-type mismatch.
async fn project_second_row_distance<T>(distance: Expr<T>) -> Result<T, dbkit::Error>
where
    T: for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Send + Unpin + 'static,
{
    let db = Database::connect(&db_url()).await?;
    let tx = db.begin().await?;
    setup_schema(&tx).await?;
    seed_distances(&tx).await?;

    let (value,) = Embedding::query()
        .select_only()
        .column_as(distance, "distance")
        .filter(Embedding::id.eq(2_i64))
        .into_model::<(T,)>()
        .one(&tx)
        .await?
        .expect("seeded second row");
    tx.rollback().await?;
    Ok(value)
}

#[tokio::test]
async fn decoders_validate_dimensions_in_binary_and_text_protocols() -> Result<(), dbkit::Error> {
    let db = Database::connect(&db_url()).await?;
    let mut tx = db.pool().begin().await?;
    sqlx::query("CREATE EXTENSION IF NOT EXISTS vector").execute(&mut *tx).await?;

    // Prepared queries return binary values; raw_sql uses PostgreSQL's text protocol.
    let sql = "SELECT '[0.5,1,-0.25]'::halfvec";
    for row in [
        sqlx::query(sql).fetch_one(&mut *tx).await?,
        sqlx::raw_sql(sql).fetch_one(&mut *tx).await?,
    ] {
        assert_eq!(
            row.try_get::<PgHalfVector<3>, _>(0)?,
            PgHalfVector::<3>::from_f32([0.5, 1.0, -0.25]).unwrap()
        );
        assert!(row.try_get::<PgHalfVector<2>, _>(0).is_err());
    }
    let sql = "SELECT '{1:0.5,1000:-0.25}/1000'::sparsevec";
    for row in [
        sqlx::query(sql).fetch_one(&mut *tx).await?,
        sqlx::raw_sql(sql).fetch_one(&mut *tx).await?,
    ] {
        assert_eq!(
            row.try_get::<PgSparseVector<1000>, _>(0)?,
            PgSparseVector::<1000>::new([(0, 0.5), (999, -0.25)]).unwrap()
        );
        assert!(row.try_get::<PgSparseVector<999>, _>(0).is_err());
    }
    let sql = "SELECT '010000001'::bit(9)";
    for row in [
        sqlx::query(sql).fetch_one(&mut *tx).await?,
        sqlx::raw_sql(sql).fetch_one(&mut *tx).await?,
    ] {
        assert_eq!(row.try_get::<PgBit<9>, _>(0)?, "010000001".parse::<PgBit<9>>().unwrap());
        assert!(row.try_get::<PgBit<8>, _>(0).is_err());
    }
    tx.rollback().await?;
    Ok(())
}
