use dbkit_core::{
    func, Column, Expr, Order, PgBit, PgBitError, PgHalfVector, PgHalfVectorError, PgSparseVector, PgSparseVectorError, Select, Table,
    Value,
};

#[test]
fn half_vector_rounds_before_use_and_preserves_native_half_values() {
    let half = PgHalfVector::<3>::from_f32([0.1, 1.0 + 1.0 / 2048.0, 65504.0]).unwrap();
    let values: Vec<f32> = half.as_slice().iter().map(|value| value.to_f32()).collect();
    assert_eq!(values, [0.099_975_586, 1.0, 65504.0]);
    assert_eq!(PgHalfVector::<3>::new(*half.as_array()).unwrap(), half);
    assert_eq!(half.to_string().parse::<PgHalfVector<3>>().unwrap(), half);

    let smallest = PgHalfVector::<1>::from_f32([2.0_f32.powi(-24)]).unwrap();
    assert_eq!(smallest.as_slice()[0].to_f32(), 2.0_f32.powi(-24));
}

#[test]
fn half_vector_rejects_non_finite_values_overflow_and_wrong_dimensions() {
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 65520.0, -65520.0] {
        let _: PgHalfVectorError = PgHalfVector::<1>::from_f32([value]).unwrap_err();
    }
    let native = PgHalfVector::<2>::from_f32([1.0, 0.0]).unwrap();
    assert!(PgHalfVector::<1>::new([native.as_array()[0] / native.as_array()[1]]).is_err());
    for text in ["[1]", "[1,2,3]", "1,2", "[1,]", "[NaN,0]", "[Infinity,0]", "[65520,0]"] {
        assert!(text.parse::<PgHalfVector<2>>().is_err(), "accepted {text}");
    }
    assert!(PgHalfVector::<0>::from_f32([]).is_err());
}

#[test]
fn sparse_vector_normalizes_entries_and_translates_postgres_indices() {
    let sparse = PgSparseVector::<1000>::new([(999, -0.25), (42, 0.75), (7, 0.0), (0, 0.5)]).unwrap();
    assert_eq!(sparse.entries(), &[(0, 0.5), (42, 0.75), (999, -0.25)]);
    assert_eq!(sparse.to_string(), "{1:0.5,43:0.75,1000:-0.25}/1000");
    assert_eq!(sparse.to_string().parse::<PgSparseVector<1000>>().unwrap(), sparse);
    assert_eq!("{1000:-0.25,1:0.5,43:0.75}/1000".parse::<PgSparseVector<1000>>().unwrap(), sparse);

    let zero = PgSparseVector::<1000>::new([]).unwrap();
    assert!(zero.entries().is_empty());
    assert_eq!(zero.to_string(), "{}/1000");
    assert_eq!("{}/1000".parse::<PgSparseVector<1000>>().unwrap(), zero);
}

#[test]
fn sparse_vector_rejects_ambiguous_or_invalid_entries() {
    for entries in [vec![(3, 1.0)], vec![(0, 1.0), (0, 2.0)], vec![(0, 0.0), (0, 1.0)]] {
        let _: PgSparseVectorError = PgSparseVector::<3>::new(entries).unwrap_err();
    }
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(PgSparseVector::<3>::new([(0, value)]).is_err());
    }
    for text in [
        "{0:1}/3",
        "{4:1}/3",
        "{1:1,1:2}/3",
        "{1:NaN}/3",
        "{1:Infinity}/3",
        "{1:1}/4",
        "{1:1}",
        "{1}/3",
    ] {
        assert!(text.parse::<PgSparseVector<3>>().is_err(), "accepted {text}");
    }
    assert!(PgSparseVector::<0>::new([]).is_err());
}

#[test]
fn bits_preserve_leading_zeros_and_non_byte_aligned_lengths() {
    let bits = PgBit::<9>::new([false, true, false, false, false, false, false, false, true]).unwrap();
    assert_eq!(bits.as_bytes(), &[0b0100_0000, 0b1000_0000]);
    assert_eq!(bits.to_string(), "010000001");
    assert_eq!(bits.to_string().parse::<PgBit<9>>().unwrap(), bits);
    assert_eq!(PgBit::<1>::new([true]).unwrap().to_string(), "1");
    assert_eq!(PgBit::<8>::new([false; 8]).unwrap().as_bytes(), &[0]);
}

#[test]
fn bits_reject_invalid_digits_and_wrong_dimensions() {
    for text in ["", "101", "10101", "1021", "true", "10 1"] {
        let _: PgBitError = text.parse::<PgBit<4>>().unwrap_err();
    }
    assert!(PgBit::<0>::new([]).is_err());
}

#[test]
fn constructors_enforce_postgres_storage_limits() {
    assert!(matches!(
        PgHalfVector::<16_001>::from_f32([0.0; 16_001]),
        Err(PgHalfVectorError::InvalidDimension(16_001))
    ));
    assert!(matches!(
        PgSparseVector::<1_000_000_001>::new([]),
        Err(PgSparseVectorError::InvalidDimension(1_000_000_001))
    ));
    assert!(matches!(
        PgSparseVector::<16_001>::new((0..16_001).map(|index| (index, 1.0))),
        Err(PgSparseVectorError::TooManyEntries(16_001))
    ));
    assert!(matches!(
        "".parse::<PgBit<2_147_483_641>>(),
        Err(PgBitError::InvalidDimension(2_147_483_641))
    ));
}

macro_rules! numeric_vector_queries {
    ($test:ident, $ty:ty, $query:expr, $cast:literal) => {
        #[test]
        fn $test() {
            let table = Table::new("embeddings");
            let column: Column<(), $ty> = Column::new(table, "embedding");
            let nullable: Column<(), Option<$ty>> = Column::new(table, "optional_embedding");
            let query = $query;
            let compiled = Select::<()>::new(table)
                .filter(column.eq(query.clone()))
                .filter(nullable.eq(None::<$ty>))
                .order_by(Order::asc(func::l2_distance(column, query.clone())))
                .order_by(Order::asc(func::cosine_distance(column, query.clone())))
                .order_by(Order::asc(func::l1_distance(column, query.clone())))
                .order_by(Order::asc(func::inner_product_distance(column, query.clone())))
                .order_by(Order::desc(func::inner_product(column, query.clone())))
                .compile();

            let cast = $cast;
            for op in ["=", "<->", "<=>", "<+>", "<#>"] {
                assert!(
                    compiled.sql.contains(&format!("embeddings.embedding {op} $1::{cast}")),
                    "{}",
                    compiled.sql
                );
            }
            assert!(compiled
                .sql
                .contains(&format!("INNER_PRODUCT(embeddings.embedding, $1::{cast})")));
            assert!(compiled.sql.contains("embeddings.optional_embedding IS NULL"));
            assert_eq!(compiled.binds, vec![Value::from(query.clone())]);

            macro_rules! check_output {
                ($function:ident) => {
                    let _: Expr<f64> = func::$function(column, query.clone());
                    let _: Expr<Option<f64>> = func::$function(nullable, query.clone());
                    let _: Expr<Option<f64>> = func::$function(query.clone(), nullable);
                    let _: Expr<Option<f64>> = func::$function(nullable, nullable);
                };
            }
            check_output!(l2_distance);
            check_output!(cosine_distance);
            check_output!(l1_distance);
            check_output!(inner_product_distance);
            check_output!(inner_product);
        }
    };
}

numeric_vector_queries!(
    half_vector_queries_preserve_operators_and_nullability,
    PgHalfVector<3>,
    PgHalfVector::<3>::from_f32([1.0, 0.5, 0.0]).unwrap(),
    "halfvec"
);
numeric_vector_queries!(
    sparse_vector_queries_preserve_operators_and_nullability,
    PgSparseVector<1000>,
    PgSparseVector::<1000>::new([(0, 1.0), (999, 0.5)]).unwrap(),
    "sparsevec"
);

#[test]
fn bit_queries_preserve_dimensions_operators_and_nullability() {
    let table = Table::new("embeddings");
    let column: Column<(), PgBit<9>> = Column::new(table, "bits");
    let nullable: Column<(), Option<PgBit<9>>> = Column::new(table, "optional_bits");
    let query: PgBit<9> = "010000001".parse().unwrap();
    let compiled = Select::<()>::new(table)
        .filter(column.eq(query.clone()))
        .filter(nullable.eq(None::<PgBit<9>>))
        .order_by(Order::asc(func::hamming_distance(column, query.clone())))
        .order_by(Order::asc(func::jaccard_distance(column, query.clone())))
        .compile();
    for op in ["=", "<~>", "<%>"] {
        assert!(
            compiled.sql.contains(&format!("embeddings.bits {op} $1::bit(9)")),
            "{}",
            compiled.sql
        );
    }
    assert!(compiled.sql.contains("embeddings.optional_bits IS NULL"));
    assert_eq!(compiled.binds, vec![Value::from(query.clone())]);

    let _: Expr<f64> = func::hamming_distance(column, query.clone());
    let _: Expr<f64> = func::jaccard_distance(column, query.clone());
    let _: Expr<Option<f64>> = func::hamming_distance(nullable, query.clone());
    let _: Expr<Option<f64>> = func::hamming_distance(query.clone(), nullable);
    let _: Expr<Option<f64>> = func::hamming_distance(nullable, nullable);
    let _: Expr<Option<f64>> = func::jaccard_distance(nullable, query.clone());
    let _: Expr<Option<f64>> = func::jaccard_distance(query, nullable);
    let _: Expr<Option<f64>> = func::jaccard_distance(nullable, nullable);
}

#[test]
fn bit_bind_deduplication_includes_the_bit_length() {
    let table = Table::new("embeddings");
    let short: Column<(), PgBit<7>> = Column::new(table, "short_bits");
    let long: Column<(), PgBit<8>> = Column::new(table, "long_bits");
    let compiled = Select::<()>::new(table)
        .filter(short.eq("1000000".parse::<PgBit<7>>().unwrap()))
        .filter(long.eq("10000000".parse::<PgBit<8>>().unwrap()))
        .compile();
    assert!(compiled.sql.contains("short_bits = $1::bit(7)"));
    assert!(compiled.sql.contains("long_bits = $2::bit(8)"));
    assert_eq!(compiled.binds.len(), 2);
    assert_ne!(compiled.binds[0], compiled.binds[1]);
}
