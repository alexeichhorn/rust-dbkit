use dbkit_core::{expr::Value, func, Column, Expr, Order, Select, Table};

#[derive(Debug)]
struct TextSample;

fn text_samples_table() -> Table {
    Table::new("text_samples")
}

fn title() -> Column<TextSample, String> {
    Column::new(text_samples_table(), "title")
}

fn body() -> Column<TextSample, Option<String>> {
    Column::new(text_samples_table(), "body")
}

fn pattern() -> Column<TextSample, String> {
    Column::new(text_samples_table(), "pattern")
}

fn nullable_pattern() -> Column<TextSample, Option<String>> {
    Column::new(text_samples_table(), "nullable_pattern")
}

#[test]
fn compiles_regex_inspection_names_and_output_types() {
    let is_match: Expr<bool> = func::regex_is_match(title(), pattern());
    let nullable_match: Expr<Option<bool>> = func::regex_is_match(body(), pattern());
    let count: Expr<i32> = func::regex_count(title(), pattern());
    let nullable_count: Expr<Option<i32>> = func::regex_count(title(), nullable_pattern());
    let position: Expr<i32> = func::regex_position(title(), pattern());
    let nullable_position: Expr<Option<i32>> = func::regex_position(body(), nullable_pattern());
    let captures: Expr<Option<Vec<Option<String>>>> = func::regex_captures(title(), pattern());
    let nullable_captures: Expr<Option<Vec<Option<String>>>> = func::regex_captures(body(), nullable_pattern());
    let extract: Expr<Option<String>> = func::regex_extract(title(), pattern());
    let nullable_extract: Expr<Option<String>> = func::regex_extract(body(), nullable_pattern());

    let query: Select<TextSample> = Select::new(text_samples_table())
        .select_only()
        .column_as(is_match, "is_match")
        .column_as(nullable_match, "nullable_match")
        .column_as(count, "count")
        .column_as(nullable_count, "nullable_count")
        .column_as(position, "position")
        .column_as(nullable_position, "nullable_position")
        .column_as(captures, "captures")
        .column_as(nullable_captures, "nullable_captures")
        .column_as(extract, "extract")
        .column_as(nullable_extract, "nullable_extract");

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT REGEXP_LIKE(text_samples.title, text_samples.pattern) AS is_match, REGEXP_LIKE(text_samples.body, text_samples.pattern) AS nullable_match, REGEXP_COUNT(text_samples.title, text_samples.pattern) AS count, REGEXP_COUNT(text_samples.title, text_samples.nullable_pattern) AS nullable_count, REGEXP_INSTR(text_samples.title, text_samples.pattern) AS position, REGEXP_INSTR(text_samples.body, text_samples.nullable_pattern) AS nullable_position, REGEXP_MATCH(text_samples.title, text_samples.pattern) AS captures, REGEXP_MATCH(text_samples.body, text_samples.nullable_pattern) AS nullable_captures, REGEXP_SUBSTR(text_samples.title, text_samples.pattern) AS extract, REGEXP_SUBSTR(text_samples.body, text_samples.nullable_pattern) AS nullable_extract FROM text_samples"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_bound_regexes_and_nested_expressions_in_query_clauses() {
    let unsafe_pattern = r"^'%_\\\.\*\+$";
    let extracted = func::regex_extract(func::lower(title()), r"[0-9]+");
    let query: Select<TextSample> = Select::new(text_samples_table())
        .select_only()
        .column_as(func::regex_count(func::trim(body()), unsafe_pattern), "count")
        .column_as(extracted.clone(), "number")
        .filter(func::regex_is_match(title(), r"^(foo|bar)$").eq(true))
        .filter(func::regex_position(body(), extracted.clone()).gt(0_i32))
        .order_by(Order::asc(func::regex_position(func::lower(body()), r"[[:alpha:]]+")))
        .order_by(Order::desc(func::regex_captures(title(), unsafe_pattern)));

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT REGEXP_COUNT(TRIM(text_samples.body), $1) AS count, REGEXP_SUBSTR(LOWER(text_samples.title), $2) AS number FROM text_samples WHERE (REGEXP_LIKE(text_samples.title, $3) = $4) AND (REGEXP_INSTR(text_samples.body, REGEXP_SUBSTR(LOWER(text_samples.title), $2)) > $5) ORDER BY REGEXP_INSTR(LOWER(text_samples.body), $6) ASC, REGEXP_MATCH(text_samples.title, $1) DESC"
    );
    assert_eq!(
        sql.binds,
        vec![
            Value::String(unsafe_pattern.to_string()),
            Value::String("[0-9]+".to_string()),
            Value::String("^(foo|bar)$".to_string()),
            Value::Bool(true),
            Value::I32(0),
            Value::String("[[:alpha:]]+".to_string()),
        ]
    );
}

#[test]
fn regex_metacharacters_and_sql_punctuation_remain_bind_values() {
    let expression = "'%_\\.*+); DROP TABLE text_samples; --";
    let pattern = r"^'%_\\\.\*\+\); DROP TABLE text_samples; --$";
    let query: Select<TextSample> = Select::new(text_samples_table())
        .select_only()
        .column_as(func::regex_is_match(expression, pattern), "is_match")
        .column_as(func::regex_count(expression, pattern), "count")
        .column_as(func::regex_position(expression, pattern), "position")
        .column_as(func::regex_captures(expression, pattern), "captures")
        .column_as(func::regex_extract(expression, pattern), "extract");

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT REGEXP_LIKE($1, $2) AS is_match, REGEXP_COUNT($1, $2) AS count, REGEXP_INSTR($1, $2) AS position, REGEXP_MATCH($1, $2) AS captures, REGEXP_SUBSTR($1, $2) AS extract FROM text_samples"
    );
    assert_eq!(
        sql.binds,
        vec![Value::String(expression.to_string()), Value::String(pattern.to_string())]
    );
}
