use dbkit_core::{func, Column, Order, Select, Table, Value};

#[test]
fn value_from_pgvector() {
    let embedding = dbkit_core::PgVector::<3>::new([0.1, 0.2, 0.3]).expect("finite vector");

    assert_eq!(Value::from(embedding), Value::Vector(vec![0.1_f32, 0.2_f32, 0.3_f32]));
}

#[test]
fn pgvector_rejects_nan_and_infinity() {
    assert!(dbkit_core::PgVector::<3>::new([0.0, f32::NAN, 1.0]).is_err());
    assert!(dbkit_core::PgVector::<3>::new([0.0, f32::INFINITY, 1.0]).is_err());
    assert!(dbkit_core::PgVector::<3>::new([0.0, f32::NEG_INFINITY, 1.0]).is_err());
}

#[test]
fn select_binds_pgvector_eq_and_null_filters() {
    let table = Table::new("embedding_rows");
    let embedding_col: Column<(), dbkit_core::PgVector<3>> = Column::new(table, "embedding");
    let optional_embedding_col: Column<(), Option<dbkit_core::PgVector<3>>> = Column::new(table, "embedding_optional");

    let query = dbkit_core::PgVector::<3>::new([0.5, 0.25, 0.125]).expect("vector");

    let compiled = Select::<()>::new(table)
        .filter(embedding_col.eq(query.clone()))
        .filter(optional_embedding_col.eq(None::<dbkit_core::PgVector<3>>))
        .compile();

    assert!(compiled.sql.contains("embedding_rows.embedding = $1::vector"));
    assert!(
        compiled.sql.contains("embedding_rows.embedding_optional IS NULL"),
        "unexpected SQL: {}",
        compiled.sql
    );
    assert_eq!(compiled.binds, vec![Value::from(query)]);
}

#[test]
fn select_compiles_vector_distance_operators_and_bind_order() {
    let table = Table::new("embedding_rows");
    let embedding_col: Column<(), dbkit_core::PgVector<3>> = Column::new(table, "embedding");

    let query = dbkit_core::PgVector::<3>::new([1.0, 0.0, 0.0]).expect("vector");

    let compiled = Select::<()>::new(table)
        .filter(func::l2_distance(embedding_col.clone(), query.clone()).lt(0.30_f32))
        .filter(func::cosine_distance(embedding_col.clone(), query.clone()).lt(0.20_f32))
        .filter(func::inner_product(embedding_col.clone(), query.clone()).gt(0.50_f32))
        .filter(func::l1_distance(embedding_col.clone(), query.clone()).lt(0.80_f32))
        .order_by(Order::asc(func::l2_distance(embedding_col.clone(), query.clone())))
        .order_by(Order::asc(func::cosine_distance(embedding_col.clone(), query.clone())))
        .order_by(Order::asc(func::inner_product(embedding_col, query.clone())))
        .compile();

    assert!(compiled.sql.contains("embedding_rows.embedding <-> $1::vector"));
    assert!(compiled.sql.contains("embedding_rows.embedding <=> $1::vector"));
    assert!(compiled.sql.contains("INNER_PRODUCT(embedding_rows.embedding, $1::vector)"));
    assert!(compiled.sql.contains("embedding_rows.embedding <+> $1::vector"));
    assert!(compiled.sql.contains("ORDER BY (embedding_rows.embedding <-> $1::vector) ASC"));
    assert!(compiled.sql.contains("(embedding_rows.embedding <=> $1::vector) ASC"));
    assert!(compiled.sql.contains("INNER_PRODUCT(embedding_rows.embedding, $1::vector) ASC"));
    assert!(!compiled.sql.contains("L2_DISTANCE("), "unexpected SQL: {}", compiled.sql);
    assert!(!compiled.sql.contains("COSINE_DISTANCE("), "unexpected SQL: {}", compiled.sql);
    assert!(!compiled.sql.contains("L1_DISTANCE("), "unexpected SQL: {}", compiled.sql);

    assert_eq!(
        compiled.binds,
        vec![
            Value::from(query.clone()),
            Value::F32(0.30),
            Value::F32(0.20),
            Value::F32(0.50),
            Value::F32(0.80),
        ]
    );
}

#[test]
fn select_compiles_inner_product_distance_operator_for_ann_order_by() {
    let table = Table::new("embedding_rows");
    let embedding_col: Column<(), dbkit_core::PgVector<3>> = Column::new(table, "embedding");
    let query = dbkit_core::PgVector::<3>::new([0.2, 0.1, 0.7]).expect("vector");

    let compiled = Select::<()>::new(table)
        .order_by(Order::asc(func::inner_product_distance(embedding_col, query)))
        .limit(10)
        .compile();

    assert!(compiled.sql.contains("ORDER BY (embedding_rows.embedding <#> $1::vector) ASC"));
    assert!(!compiled.sql.contains("INNER_PRODUCT("), "unexpected SQL: {}", compiled.sql);
}

#[test]
fn pgvector_try_from_vec_validates_exact_dimension() {
    let ok = dbkit_core::PgVector::<3>::try_from(vec![1.0, 2.0, 3.0]);
    assert!(ok.is_ok());

    let too_short = dbkit_core::PgVector::<3>::try_from(vec![1.0, 2.0]);
    assert!(too_short.is_err());

    let too_long = dbkit_core::PgVector::<3>::try_from(vec![1.0, 2.0, 3.0, 4.0]);
    assert!(too_long.is_err());
}

#[test]
fn select_compiles_optional_vector_distance_operator() {
    let table = Table::new("embedding_rows");
    let optional_embedding_col: Column<(), Option<dbkit_core::PgVector<3>>> = Column::new(table, "embedding_optional");
    let query = dbkit_core::PgVector::<3>::new([0.2, 0.3, 0.4]).expect("vector");

    let compiled = Select::<()>::new(table)
        .filter(optional_embedding_col.clone().is_not_null())
        .order_by(Order::asc(func::cosine_distance(optional_embedding_col, query)))
        .limit(5)
        .compile();

    assert!(compiled.sql.contains("embedding_rows.embedding_optional IS NOT NULL"));
    assert!(compiled
        .sql
        .contains("ORDER BY (embedding_rows.embedding_optional <=> $1::vector) ASC"));
    assert!(!compiled.sql.contains("COSINE_DISTANCE("), "unexpected SQL: {}", compiled.sql);
}
