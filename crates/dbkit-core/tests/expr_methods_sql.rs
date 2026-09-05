use dbkit_core::{func, Column, Expr, IntoExpr, Order, Select, Table, Value};

#[derive(Debug)]
struct Sample;

fn column<T>(name: &'static str) -> Column<Sample, T> {
    Column::new(Table::new("samples"), name)
}

#[test]
fn unwrap_or_accepts_literals_columns_and_expressions() {
    let compiled = Select::<Sample>::new(Table::new("samples"))
        .select_only()
        .column_as(column::<Option<String>>("label").unwrap_or("unknown"), "label")
        .column_as(column::<Option<i32>>("count").unwrap_or(column::<i32>("fallback")), "count")
        .column_as(
            (column::<Option<i32>>("count") + 1_i32).unwrap_or(column::<i32>("fallback") + 2_i32),
            "computed",
        )
        .column_as(column::<Option<String>>("label").eq("ready").unwrap_or(false), "ready")
        .compile();

    assert_eq!(
        compiled.sql,
        "SELECT COALESCE(samples.label, $1) AS label, COALESCE(samples.count, samples.fallback) AS count, COALESCE((samples.count + $2), (samples.fallback + $3)) AS computed, COALESCE((samples.label = $4), $5) AS ready FROM samples"
    );
    assert_eq!(
        compiled.binds,
        vec![
            Value::String("unknown".into()),
            Value::I32(1),
            Value::I32(2),
            Value::String("ready".into()),
            Value::Bool(false)
        ]
    );
}

#[test]
fn unwrap_or_default_preserves_literal_types() {
    let compiled = Select::<Sample>::new(Table::new("samples"))
        .select_only()
        .column_as(column::<Option<bool>>("enabled").unwrap_or_default(), "enabled")
        .column_as(column::<Option<i16>>("small").unwrap_or_default(), "small")
        .column_as(column::<Option<i32>>("medium").unwrap_or_default(), "medium")
        .column_as(column::<Option<i64>>("large").unwrap_or_default(), "large")
        .column_as(column::<Option<f32>>("real").unwrap_or_default(), "real")
        .column_as(column::<Option<f64>>("double").unwrap_or_default(), "double")
        .column_as(column::<Option<String>>("label").unwrap_or_default(), "label")
        .column_as(column::<Option<bool>>("enabled").into_expr().unwrap_or_default(), "expression")
        .compile();

    assert_eq!(
        compiled.sql,
        "SELECT COALESCE(samples.enabled, $1) AS enabled, COALESCE(samples.small, $2) AS small, COALESCE(samples.medium, $3) AS medium, COALESCE(samples.large, $4) AS large, COALESCE(samples.real, $5) AS real, COALESCE(samples.double, $6) AS double, COALESCE(samples.label, $7) AS label, COALESCE(samples.enabled, $1) AS expression FROM samples"
    );
    assert_eq!(
        compiled.binds,
        vec![
            Value::Bool(false),
            Value::I16(0),
            Value::I32(0),
            Value::I64(0),
            Value::F32(0.0),
            Value::F64(0.0),
            Value::String(String::new())
        ]
    );
}

#[test]
fn unwrap_or_default_uses_the_types_default_without_requiring_clone() {
    struct RetryLimit(i32);

    impl Default for RetryLimit {
        fn default() -> Self {
            Self(3)
        }
    }

    impl IntoExpr<RetryLimit> for RetryLimit {
        fn into_expr(self) -> Expr<RetryLimit> {
            Expr::new(dbkit_core::ExprNode::Value(Value::I32(self.0)))
        }
    }

    let compiled = Select::<Sample>::new(Table::new("samples"))
        .select_only()
        .column_as(column::<Option<RetryLimit>>("limit").unwrap_or_default(), "column_default")
        .column_as(
            column::<Option<RetryLimit>>("limit").into_expr().unwrap_or_default(),
            "expression_default",
        )
        .compile();

    assert_eq!(
        compiled.sql,
        "SELECT COALESCE(samples.limit, $1) AS column_default, COALESCE(samples.limit, $1) AS expression_default FROM samples"
    );
    assert_eq!(compiled.binds, vec![Value::I32(3)]);
}

#[test]
fn string_methods_chain_in_projections_filters_and_ordering() {
    let normalized = column::<Option<String>>("label").trim().lower().unwrap_or("missing");
    let compiled = Select::<Sample>::new(Table::new("samples"))
        .select_only()
        .column_as(normalized.clone(), "normalized")
        .filter(normalized.clone().starts_with("a"))
        .order_by(Order::asc(normalized))
        .compile();

    assert_eq!(
        compiled.sql,
        "SELECT COALESCE(LOWER(TRIM(samples.label)), $1) AS normalized FROM samples WHERE STARTS_WITH(COALESCE(LOWER(TRIM(samples.label)), $1), $2) ORDER BY COALESCE(LOWER(TRIM(samples.label)), $1) ASC"
    );
    assert_eq!(compiled.binds, vec![Value::String("missing".into()), Value::String("a".into())]);
}

#[test]
fn string_methods_match_existing_functions_for_required_and_nullable_inputs() {
    macro_rules! assert_same_sql {
        ($method:expr, $function:expr) => {
            assert_eq!(
                Select::<Sample>::new(Table::new("samples"))
                    .select_only()
                    .column_as($method, "value")
                    .compile(),
                Select::<Sample>::new(Table::new("samples"))
                    .select_only()
                    .column_as($function, "value")
                    .compile(),
            );
        };
    }

    assert_same_sql!(column::<String>("label").trim(), func::trim(column::<String>("label")));
    assert_same_sql!(
        column::<Option<String>>("label").trim(),
        func::trim(column::<Option<String>>("label"))
    );
    assert_same_sql!(column::<String>("label").lower(), func::lower(column::<String>("label")));
    assert_same_sql!(
        column::<Option<String>>("label").lower(),
        func::lower(column::<Option<String>>("label"))
    );
    assert_same_sql!(
        func::upper(column::<String>("label")).trim(),
        func::trim(func::upper(column::<String>("label")))
    );
    assert_same_sql!(
        func::upper(column::<Option<String>>("label")).lower(),
        func::lower(func::upper(column::<Option<String>>("label")))
    );
    assert_same_sql!(
        column::<String>("label").starts_with(column::<String>("prefix")),
        func::starts_with(column::<String>("label"), column::<String>("prefix"))
    );
    assert_same_sql!(
        column::<Option<String>>("label").starts_with(column::<Option<String>>("prefix")),
        func::starts_with(column::<Option<String>>("label"), column::<Option<String>>("prefix"))
    );
    assert_same_sql!(
        column::<String>("label").lower().starts_with(column::<String>("prefix").lower()),
        func::starts_with(func::lower(column::<String>("label")), func::lower(column::<String>("prefix")))
    );
}

#[test]
fn starts_with_binds_literal_prefixes_without_like_escaping() {
    let prefix = "'%_\\🦀";
    let compiled = Select::<Sample>::new(Table::new("samples"))
        .select_only()
        .column_as(column::<String>("label").starts_with(prefix), "literal")
        .column_as(column::<Option<String>>("label").starts_with(String::new()), "empty")
        .compile();

    assert_eq!(
        compiled.sql,
        "SELECT STARTS_WITH(samples.label, $1) AS literal, STARTS_WITH(samples.label, $2) AS empty FROM samples"
    );
    assert_eq!(compiled.binds, vec![Value::String(prefix.into()), Value::String(String::new())]);
}

#[test]
fn methods_wrap_aggregates_after_their_filter_clause() {
    let compiled = Select::<Sample>::new(Table::new("samples"))
        .select_only()
        .column_as(func::sum(column::<i64>("amount")).unwrap_or_default(), "total")
        .column_as(
            func::sum(column::<i64>("amount"))
                .filter(column::<i64>("amount").gt(0_i64))
                .unwrap_or(0_i64),
            "positive_total",
        )
        .column_as(
            func::min(column::<String>("label")).trim().lower().unwrap_or("empty"),
            "first_label",
        )
        .compile();

    assert_eq!(
        compiled.sql,
        "SELECT COALESCE(SUM(samples.amount), $1) AS total, COALESCE(SUM(samples.amount) FILTER (WHERE (samples.amount > $1)), $1) AS positive_total, COALESCE(LOWER(TRIM(MIN(samples.label))), $2) AS first_label FROM samples"
    );
    assert_eq!(compiled.binds, vec![Value::I64(0), Value::String("empty".into())]);
}
