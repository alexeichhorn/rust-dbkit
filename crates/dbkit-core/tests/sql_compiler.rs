use chrono::NaiveDateTime;
use dbkit_core::{expr::Value, func, Column, Condition, Expr, Order, Select, Table};

#[derive(Debug)]
struct User;

#[derive(Debug)]
struct Event;

#[derive(Debug)]
struct Sale;

#[derive(Debug)]
struct WindowRow;

#[derive(Debug)]
struct CompactRow;

#[derive(Debug)]
struct TextSample;

fn user_table() -> Table {
    Table::new("users")
}

fn user_id() -> Column<User, i64> {
    Column::new(user_table(), "id")
}

fn user_email() -> Column<User, String> {
    Column::new(user_table(), "email")
}

fn user_backup_email() -> Column<User, String> {
    Column::new(user_table(), "backup_email")
}

fn event_table() -> Table {
    Table::new("events")
}

fn event_starts_at() -> Column<Event, NaiveDateTime> {
    Column::new(event_table(), "starts_at")
}

fn sales_table() -> Table {
    Table::new("sales")
}

fn sales_id() -> Column<Sale, i64> {
    Column::new(sales_table(), "id")
}

fn sales_region() -> Column<Sale, String> {
    Column::new(sales_table(), "region")
}

fn sales_amount() -> Column<Sale, i64> {
    Column::new(sales_table(), "amount")
}

fn sales_created_at() -> Column<Sale, NaiveDateTime> {
    Column::new(sales_table(), "created_at")
}

fn window_table() -> Table {
    Table::new("window_rows")
}

fn window_anchor_at() -> Column<WindowRow, NaiveDateTime> {
    Column::new(window_table(), "anchor_at")
}

fn window_offset_units() -> Column<WindowRow, i32> {
    Column::new(window_table(), "offset_units")
}

fn compact_table() -> Table {
    Table::new("compact_rows")
}

fn compact_left_units() -> Column<CompactRow, i16> {
    Column::new(compact_table(), "left_units")
}

fn compact_right_units() -> Column<CompactRow, i16> {
    Column::new(compact_table(), "right_units")
}

fn text_samples_table() -> Table {
    Table::new("text_samples")
}

fn text_sample_id() -> Column<TextSample, i64> {
    Column::new(text_samples_table(), "id")
}

fn text_sample_body() -> Column<TextSample, Option<String>> {
    Column::new(text_samples_table(), "body")
}

fn text_sample_title() -> Column<TextSample, String> {
    Column::new(text_samples_table(), "title")
}

fn text_sample_width() -> Column<TextSample, i32> {
    Column::new(text_samples_table(), "width")
}

#[test]
fn compiles_basic_filter() {
    let expr = user_email().eq("a@b.com");
    let sql = expr_sql(expr);
    assert_eq!(sql.sql, "SELECT users.* FROM users WHERE (users.email = $1)");
    assert_eq!(sql.binds, vec![Value::String("a@b.com".to_string())]);
}

#[test]
fn compiles_bool_composition() {
    let expr = user_id().gt(10_i64).and(user_email().ilike("%test%"));
    let sql = expr_sql(expr);
    assert_eq!(
        sql.sql,
        "SELECT users.* FROM users WHERE ((users.id > $1) AND (users.email ILIKE $2))"
    );
    assert_eq!(sql.binds, vec![Value::I64(10), Value::String("%test%".to_string())]);
}

#[test]
fn compiles_in_expression() {
    let expr = user_id().in_([1_i64, 2, 3]);
    let sql = expr_sql(expr);
    assert_eq!(sql.sql, "SELECT users.* FROM users WHERE (users.id IN ($1, $2, $3))");
    assert_eq!(sql.binds, vec![Value::I64(1), Value::I64(2), Value::I64(3)]);
}

#[test]
fn compiles_is_null_expression() {
    let expr = user_email().is_null();
    let sql = expr_sql(expr);
    assert_eq!(sql.sql, "SELECT users.* FROM users WHERE (users.email IS NULL)");
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_eq_none_as_is_null() {
    let expr = user_email().eq(None);
    let sql = expr_sql(expr);
    assert_eq!(sql.sql, "SELECT users.* FROM users WHERE (users.email IS NULL)");
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_ne_none_as_is_not_null() {
    let expr = user_email().ne(None);
    let sql = expr_sql(expr);
    assert_eq!(sql.sql, "SELECT users.* FROM users WHERE (users.email IS NOT NULL)");
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_upper_function_filter() {
    let expr = func::upper(user_email()).eq("TEST");
    let sql = expr_sql(expr);
    assert_eq!(sql.sql, "SELECT users.* FROM users WHERE (UPPER(users.email) = $1)");
    assert_eq!(sql.binds, vec![Value::String("TEST".to_string())]);
}

#[test]
fn compiles_lower_function_with_literal_input() {
    let lowered: Expr<String> = func::lower("MiXeD");
    let query: Select<TextSample> = Select::new(text_samples_table()).select_only().column_as(lowered, "lowered");

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT LOWER($1) AS lowered FROM text_samples");
    assert_eq!(sql.binds, vec![Value::String("MiXeD".to_string())]);
}

#[test]
fn compiles_coalesce_function_filter() {
    let expr = func::coalesce(user_email(), "unknown").eq("ALPHA");
    let sql = expr_sql(expr);
    assert_eq!(sql.sql, "SELECT users.* FROM users WHERE (COALESCE(users.email, $1) = $2)");
    assert_eq!(
        sql.binds,
        vec![Value::String("unknown".to_string()), Value::String("ALPHA".to_string()),]
    );
}

#[test]
fn compiles_coalesce_two_columns() {
    let expr = func::coalesce(user_email(), user_backup_email()).eq("alpha@db.com");
    let sql = expr_sql(expr);
    assert_eq!(
        sql.sql,
        "SELECT users.* FROM users WHERE (COALESCE(users.email, users.backup_email) = $1)"
    );
    assert_eq!(sql.binds, vec![Value::String("alpha@db.com".to_string())]);
}

#[test]
fn compiles_date_trunc_function_filter() {
    let dt = NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).expect("dt");
    let expr = func::date_trunc("day", event_starts_at()).eq(dt);
    let query: Select<Event> = Select::new(event_table()).filter(expr);
    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT events.* FROM events WHERE (DATE_TRUNC($1, events.starts_at) = $2)");
    assert_eq!(sql.binds, vec![Value::String("day".to_string()), Value::DateTime(dt)]);
}

#[test]
fn compiles_nested_functions() {
    let expr = func::upper(func::coalesce(user_email(), "unknown")).eq("ALPHA");
    let sql = expr_sql(expr);
    assert_eq!(sql.sql, "SELECT users.* FROM users WHERE (UPPER(COALESCE(users.email, $1)) = $2)");
    assert_eq!(
        sql.binds,
        vec![Value::String("unknown".to_string()), Value::String("ALPHA".to_string()),]
    );
}

#[test]
fn compiles_trim_function_filter() {
    let expr = func::trim(text_sample_title()).eq("alpha");
    let query: Select<TextSample> = Select::new(text_samples_table()).filter(expr);
    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT text_samples.* FROM text_samples WHERE (TRIM(text_samples.title) = $1)"
    );
    assert_eq!(sql.binds, vec![Value::String("alpha".to_string())]);
}

#[test]
fn compiles_nested_char_length_trim_filter_on_nullable_text() {
    let expr = func::char_length(func::trim(text_sample_body())).ge(5_i32);
    let query: Select<TextSample> = Select::new(text_samples_table())
        .filter(text_sample_body().is_not_null())
        .filter(expr);
    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT text_samples.* FROM text_samples WHERE (text_samples.body IS NOT NULL) AND (CHAR_LENGTH(TRIM(text_samples.body)) >= $1)"
    );
    assert_eq!(sql.binds, vec![Value::I32(5)]);
}

#[test]
fn compiles_byte_and_bit_length_with_postgresql_names() {
    let literal_bytes: Expr<i32> = func::byte_length("é");
    let query: Select<TextSample> = Select::new(text_samples_table())
        .select_only()
        .column_as(literal_bytes, "literal_bytes")
        .column_as(func::byte_length(text_sample_title()), "title_bytes")
        .column_as(func::bit_length(func::trim(text_sample_body())), "trimmed_body_bits");

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT OCTET_LENGTH($1) AS literal_bytes, OCTET_LENGTH(text_samples.title) AS title_bytes, BIT_LENGTH(TRIM(text_samples.body)) AS trimmed_body_bits FROM text_samples"
    );
    assert_eq!(sql.binds, vec![Value::String("é".to_string())]);
}

#[test]
fn compiles_position_and_starts_with_with_expression_arguments() {
    let query: Select<TextSample> = Select::new(text_samples_table())
        .select_only()
        .column_as(
            func::position(func::lower(text_sample_title()), func::trim(text_sample_body())),
            "normalized_position",
        )
        .column_as(
            func::starts_with(func::lower(text_sample_title()), func::lower(text_sample_body())),
            "normalized_prefix",
        );

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT STRPOS(LOWER(text_samples.title), TRIM(text_samples.body)) AS normalized_position, STARTS_WITH(LOWER(text_samples.title), LOWER(text_samples.body)) AS normalized_prefix FROM text_samples"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_string_search_and_lengths_in_projections_filters_and_orderings() {
    let unsafe_search = "'%_\\needle";
    let query: Select<TextSample> = Select::new(text_samples_table())
        .select_only()
        .column_as(func::byte_length(text_sample_body()), "body_bytes")
        .column_as(func::bit_length(text_sample_title()), "title_bits")
        .column_as(func::position(text_sample_title(), unsafe_search), "unsafe_position")
        .column_as(func::starts_with(text_sample_title(), unsafe_search), "unsafe_prefix")
        .filter(func::starts_with(text_sample_title(), "prefix").eq(true))
        .filter(func::position(text_sample_title(), "needle").gt(0_i32))
        .order_by(Order::desc(func::byte_length(text_sample_body())))
        .order_by(Order::asc(func::position(text_sample_title(), "needle")));

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT OCTET_LENGTH(text_samples.body) AS body_bytes, BIT_LENGTH(text_samples.title) AS title_bits, STRPOS(text_samples.title, $1) AS unsafe_position, STARTS_WITH(text_samples.title, $1) AS unsafe_prefix FROM text_samples WHERE (STARTS_WITH(text_samples.title, $2) = $3) AND (STRPOS(text_samples.title, $4) > $5) ORDER BY OCTET_LENGTH(text_samples.body) DESC, STRPOS(text_samples.title, $4) ASC"
    );
    assert_eq!(
        sql.binds,
        vec![
            Value::String(unsafe_search.to_string()),
            Value::String("prefix".to_string()),
            Value::Bool(true),
            Value::String("needle".to_string()),
            Value::I32(0),
        ]
    );
}

#[test]
fn compiles_trimmed_nullable_text_selection() {
    let query: Select<TextSample> = Select::new(text_samples_table())
        .select_only()
        .column(text_sample_id())
        .column_as(func::trim(text_sample_body()), "trimmed_body")
        .column_as(func::char_length(func::trim(text_sample_body())), "trimmed_body_len");

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT text_samples.id, TRIM(text_samples.body) AS trimmed_body, CHAR_LENGTH(TRIM(text_samples.body)) AS trimmed_body_len FROM text_samples"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_string_normalization_in_projections_filter_and_ordering() {
    let trimmed_body: Expr<Option<String>> = func::trim_end_chars(text_sample_body(), "!?");
    let query: Select<TextSample> = Select::new(text_samples_table())
        .select_only()
        .column_as(func::lower(text_sample_title()), "lowered_title")
        .column_as(func::trim_chars(text_sample_title(), "xy"), "trimmed_title")
        .column_as(func::trim_start(text_sample_title()), "left_trimmed_title")
        .column_as(func::trim_start_chars(text_sample_title(), "@"), "handle")
        .column_as(func::trim_end(text_sample_body()), "right_trimmed_body")
        .column_as(trimmed_body, "punctuation_trimmed_body")
        .filter(func::trim_chars(text_sample_title(), ".").eq("alpha"))
        .order_by(Order::asc(func::trim_start_chars(text_sample_title(), "#")))
        .order_by(Order::desc(func::trim_end(text_sample_body())));

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT LOWER(text_samples.title) AS lowered_title, TRIM(BOTH $1 FROM text_samples.title) AS trimmed_title, TRIM(LEADING FROM text_samples.title) AS left_trimmed_title, TRIM(LEADING $2 FROM text_samples.title) AS handle, TRIM(TRAILING FROM text_samples.body) AS right_trimmed_body, TRIM(TRAILING $3 FROM text_samples.body) AS punctuation_trimmed_body FROM text_samples WHERE (TRIM(BOTH $4 FROM text_samples.title) = $5) ORDER BY TRIM(LEADING $6 FROM text_samples.title) ASC, TRIM(TRAILING FROM text_samples.body) DESC"
    );
    assert_eq!(
        sql.binds,
        vec![
            Value::String("xy".to_string()),
            Value::String("@".to_string()),
            Value::String("!?".to_string()),
            Value::String(".".to_string()),
            Value::String("alpha".to_string()),
            Value::String("#".to_string()),
        ]
    );
}

#[test]
fn custom_trim_characters_are_bound_in_sql_order() {
    let expr = func::trim_chars(func::trim_start_chars(text_sample_title(), "'\\"), "[]").eq("alpha");
    let query: Select<TextSample> = Select::new(text_samples_table()).filter(expr);

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT text_samples.* FROM text_samples WHERE (TRIM(BOTH $1 FROM TRIM(LEADING $2 FROM text_samples.title)) = $3)"
    );
    assert_eq!(
        sql.binds,
        vec![
            Value::String("[]".to_string()),
            Value::String("'\\".to_string()),
            Value::String("alpha".to_string()),
        ]
    );
}

#[test]
fn compiles_normalized_handle_lookup() {
    let handle = "alice";
    let normalized = func::lower(func::trim(func::trim_start_chars(func::trim(text_sample_title()), "@")));
    let query: Select<TextSample> = Select::new(text_samples_table()).filter(normalized.eq(handle));

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT text_samples.* FROM text_samples WHERE (LOWER(TRIM(TRIM(LEADING $1 FROM TRIM(text_samples.title)))) = $2)"
    );
    assert_eq!(sql.binds, vec![Value::String("@".to_string()), Value::String(handle.to_string())]);
}

#[test]
fn compiles_string_extraction_and_sizing_functions_with_expected_types() {
    let left_title: Expr<String> = func::left(text_sample_title(), 2_i32);
    let right_body: Expr<Option<String>> = func::right(text_sample_body(), text_sample_width());
    let substring_title: Expr<String> = func::substring(func::lower(text_sample_title()), 2_i32, text_sample_width());
    let repeated_literal: Expr<String> = func::repeat("ab", 3_i32);
    let padded_title: Expr<String> = func::pad_start(text_sample_title(), 8_i32, "xy");
    let padded_body: Expr<Option<String>> = func::pad_end(text_sample_body(), func::char_length(text_sample_title()), func::lower("Z"));

    let query: Select<TextSample> = Select::new(text_samples_table())
        .select_only()
        .column_as(left_title, "left_title")
        .column_as(right_body, "right_body")
        .column_as(substring_title, "substring_title")
        .column_as(repeated_literal, "repeated_literal")
        .column_as(padded_title, "padded_title")
        .column_as(padded_body, "padded_body");

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT LEFT(text_samples.title, $1) AS left_title, RIGHT(text_samples.body, text_samples.width) AS right_body, SUBSTRING(LOWER(text_samples.title), $1, text_samples.width) AS substring_title, REPEAT($2, $3) AS repeated_literal, LPAD(text_samples.title, $4, $5) AS padded_title, RPAD(text_samples.body, CHAR_LENGTH(text_samples.title), LOWER($6)) AS padded_body FROM text_samples"
    );
    assert_eq!(
        sql.binds,
        vec![
            Value::I32(2),
            Value::String("ab".to_string()),
            Value::I32(3),
            Value::I32(8),
            Value::String("xy".to_string()),
            Value::String("Z".to_string()),
        ]
    );
}

#[test]
fn compiles_nested_string_functions_in_projection_filter_and_ordering() {
    let fill = "'%_\\";
    let query: Select<TextSample> = Select::new(text_samples_table())
        .select_only()
        .column_as(
            func::substring(func::left(func::lower(text_sample_title()), 5_i32), 2_i32, 3_i32),
            "slice",
        )
        .filter(func::repeat(func::right(func::trim(text_sample_title()), 2_i32), 2_i32).eq("abab"))
        .order_by(Order::asc(func::pad_end(func::left(text_sample_body(), 4_i32), 8_i32, fill)));

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT SUBSTRING(LEFT(LOWER(text_samples.title), $1), $2, $3) AS slice FROM text_samples WHERE (REPEAT(RIGHT(TRIM(text_samples.title), $2), $2) = $4) ORDER BY RPAD(LEFT(text_samples.body, $5), $6, $7) ASC"
    );
    assert_eq!(
        sql.binds,
        vec![
            Value::I32(5),
            Value::I32(2),
            Value::I32(3),
            Value::String("abab".to_string()),
            Value::I32(4),
            Value::I32(8),
            Value::String(fill.to_string()),
        ]
    );
}

#[test]
fn compiles_text_transformation_functions_with_expected_types_and_postgresql_names() {
    let titled: Expr<String> = func::title_case(text_sample_title());
    let reversed: Expr<Option<String>> = func::reverse(text_sample_body());
    let replaced: Expr<String> = func::replace(func::lower(text_sample_title()), "FROM_LITERAL", "TO_LITERAL");
    let nullable_replace: Expr<Option<String>> = func::replace(text_sample_title(), text_sample_body(), "fallback");
    let ranged: Expr<String> = func::replace_range(text_sample_title(), func::upper("replacement"), text_sample_width(), 2_i32);
    let nullable_range: Expr<Option<String>> = func::replace_range(text_sample_title(), text_sample_body(), 1_i32, text_sample_width());
    let translated: Expr<String> = func::translate_chars(text_sample_title(), "abc", "xyz");
    let nullable_translation: Expr<Option<String>> = func::translate_chars(text_sample_title(), "abc", text_sample_body());

    let query: Select<TextSample> = Select::new(text_samples_table())
        .select_only()
        .column_as(titled, "titled")
        .column_as(reversed, "reversed")
        .column_as(replaced, "replaced")
        .column_as(nullable_replace, "nullable_replace")
        .column_as(ranged, "ranged")
        .column_as(nullable_range, "nullable_range")
        .column_as(translated, "translated")
        .column_as(nullable_translation, "nullable_translation");

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT INITCAP(text_samples.title) AS titled, REVERSE(text_samples.body) AS reversed, REPLACE(LOWER(text_samples.title), $1, $2) AS replaced, REPLACE(text_samples.title, text_samples.body, $3) AS nullable_replace, OVERLAY(text_samples.title, UPPER($4), text_samples.width, $5) AS ranged, OVERLAY(text_samples.title, text_samples.body, $6, text_samples.width) AS nullable_range, TRANSLATE(text_samples.title, $7, $8) AS translated, TRANSLATE(text_samples.title, $7, text_samples.body) AS nullable_translation FROM text_samples"
    );
    assert_eq!(
        sql.binds,
        vec![
            Value::String("FROM_LITERAL".to_string()),
            Value::String("TO_LITERAL".to_string()),
            Value::String("fallback".to_string()),
            Value::String("replacement".to_string()),
            Value::I32(2),
            Value::I32(1),
            Value::String("abc".to_string()),
            Value::String("xyz".to_string()),
        ]
    );
}

#[test]
fn text_transformation_arguments_remain_bound_when_composed() {
    let unsafe_text = "'%_\\[]().*+";
    let transformed = func::reverse(func::translate_chars(
        func::replace(text_sample_title(), unsafe_text, "<$&>"),
        "[]",
        "{}",
    ));
    let query: Select<TextSample> = Select::new(text_samples_table())
        .select_only()
        .column_as(func::title_case(transformed), "transformed")
        .filter(func::replace_range(text_sample_body(), unsafe_text, 1_i32, 2_i32).eq("expected"))
        .order_by(Order::asc(func::reverse(func::translate_chars(text_sample_body(), "%_", ""))));

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT INITCAP(REVERSE(TRANSLATE(REPLACE(text_samples.title, $1, $2), $3, $4))) AS transformed FROM text_samples WHERE (OVERLAY(text_samples.body, $1, $5, $6) = $7) ORDER BY REVERSE(TRANSLATE(text_samples.body, $8, $9)) ASC"
    );
    assert_eq!(
        sql.binds,
        vec![
            Value::String(unsafe_text.to_string()),
            Value::String("<$&>".to_string()),
            Value::String("[]".to_string()),
            Value::String("{}".to_string()),
            Value::I32(1),
            Value::I32(2),
            Value::String("expected".to_string()),
            Value::String("%_".to_string()),
            Value::String("".to_string()),
        ]
    );
}

#[test]
fn compiles_select_only_with_columns() {
    let query: Select<User> = Select::new(user_table()).select_only().column(user_email()).column(user_id());

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.email, users.id FROM users");
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_select_only_with_column_as() {
    let query: Select<User> = Select::new(user_table()).select_only().column_as(user_email(), "email_addr");

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.email AS email_addr FROM users");
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_select_only_with_func_column() {
    let query: Select<User> = Select::new(user_table()).select_only().column(func::upper(user_email()));

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT UPPER(users.email) FROM users");
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_group_by_and_having() {
    let query = Select::<User>::new(user_table())
        .select_only()
        .column(user_email())
        .column_as(func::count(user_id()), "cnt")
        .group_by(user_email())
        .having(func::count(user_id()).gt(1_i64));

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT users.email, COUNT(users.id) AS cnt FROM users GROUP BY users.email HAVING (COUNT(users.id) > $1)"
    );
    assert_eq!(sql.binds, vec![Value::I64(1)]);
}

#[test]
fn compiles_filtered_aggregate_projections_without_group_by() {
    let discounted_us_sale = sales_region().eq("us").and(sales_amount().le(50_i64));
    let query = Select::<Sale>::new(sales_table())
        .select_only()
        .column_as(func::count(sales_id()), "active_sales")
        .column_as(func::count(sales_id()).filter(discounted_us_sale.clone()), "discounted_us_sales")
        .column_as(
            func::min(sales_created_at()).filter(discounted_us_sale),
            "oldest_discounted_us_sale_at",
        )
        .column_as(func::count(sales_id()).filter(sales_region().eq("missing")), "missing_sales")
        .filter(sales_amount().gt(0_i64));

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT COUNT(sales.id) AS active_sales, COUNT(sales.id) FILTER (WHERE ((sales.region = $1) AND (sales.amount <= $2))) AS discounted_us_sales, MIN(sales.created_at) FILTER (WHERE ((sales.region = $1) AND (sales.amount <= $2))) AS oldest_discounted_us_sale_at, COUNT(sales.id) FILTER (WHERE (sales.region = $3)) AS missing_sales FROM sales WHERE (sales.amount > $4)"
    );
    assert_eq!(
        sql.binds,
        vec![
            Value::String("us".to_string()),
            Value::I64(50),
            Value::String("missing".to_string()),
            Value::I64(0),
        ]
    );
}

#[test]
fn compiles_filtered_aggregate_in_grouped_projection_and_having() {
    let query = Select::<Sale>::new(sales_table())
        .select_only()
        .column(sales_region())
        .column_as(func::count(sales_id()).filter(sales_amount().ge(50_i64)), "large_sales")
        .group_by(sales_region())
        .having(func::count(sales_id()).filter(sales_amount().ge(50_i64)).gt(1_i64));

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT sales.region, COUNT(sales.id) FILTER (WHERE (sales.amount >= $1)) AS large_sales FROM sales GROUP BY sales.region HAVING (COUNT(sales.id) FILTER (WHERE (sales.amount >= $1)) > $2)"
    );
    assert_eq!(sql.binds, vec![Value::I64(50), Value::I64(1)]);
}

#[test]
fn compiles_scalar_function_wrapping_filtered_aggregate() {
    let query = Select::<Sale>::new(sales_table()).select_only().column_as(
        func::coalesce(func::sum(sales_amount()).filter(sales_region().eq("us")), 0_i64),
        "us_total",
    );

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT COALESCE(SUM(sales.amount) FILTER (WHERE (sales.region = $1)), $2) AS us_total FROM sales"
    );
    assert_eq!(sql.binds, vec![Value::String("us".to_string()), Value::I64(0)]);
}

#[test]
fn compiles_select_only_with_join_and_group_by() {
    let todos_table = Table::new("todos");
    let todo_user_id: Column<User, i64> = Column::new(todos_table, "user_id");
    let todo_id: Column<User, i64> = Column::new(todos_table, "id");

    let query = Select::<User>::new(user_table())
        .select_only()
        .column(user_id())
        .column_as(func::count(todo_id), "todo_count")
        .join_on(todos_table, user_id().eq_col(todo_user_id))
        .group_by(user_id());

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT users.id, COUNT(todos.id) AS todo_count FROM users JOIN todos ON (users.id = todos.user_id) GROUP BY users.id"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_group_by_expression() {
    let query = Select::<Sale>::new(sales_table())
        .select_only()
        .column_as(func::date_trunc("day", sales_created_at()), "bucket")
        .column_as(func::sum(sales_amount()), "total")
        .group_by(func::date_trunc("day", sales_created_at()));

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT DATE_TRUNC($1, sales.created_at) AS bucket, SUM(sales.amount) AS total FROM sales GROUP BY DATE_TRUNC($1, sales.created_at)"
    );
    assert_eq!(sql.binds, vec![Value::String("day".to_string())]);
}

#[test]
fn compiles_min_max_aggregate_projections() {
    let query = Select::<Sale>::new(sales_table())
        .select_only()
        .column_as(sales_region(), "region")
        .column_as(func::min(sales_created_at()), "first_sale_at")
        .column_as(func::max(sales_created_at()), "last_sale_at")
        .column_as(func::min(sales_amount()), "min_amount")
        .column_as(func::max(sales_amount()), "max_amount")
        .group_by(sales_region())
        .having(func::max(sales_amount()).gt(100_i64))
        .order_by(Order::asc(func::min(sales_created_at())));

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT sales.region AS region, MIN(sales.created_at) AS first_sale_at, MAX(sales.created_at) AS last_sale_at, MIN(sales.amount) AS min_amount, MAX(sales.amount) AS max_amount FROM sales GROUP BY sales.region HAVING (MAX(sales.amount) > $1) ORDER BY MIN(sales.created_at) ASC"
    );
    assert_eq!(sql.binds, vec![Value::I64(100)]);
}

#[test]
fn compiles_min_max_for_nullable_text_without_nested_option_type() {
    let min_body = func::min(text_sample_body());
    let max_body = func::max(text_sample_body());

    let query = Select::<TextSample>::new(text_samples_table())
        .select_only()
        .column_as(min_body, "min_body")
        .column_as(max_body, "max_body");

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT MIN(text_samples.body) AS min_body, MAX(text_samples.body) AS max_body FROM text_samples"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_order_by_expression() {
    let query: Select<Sale> = Select::new(sales_table())
        .select_only()
        .column_as(func::date_trunc("day", sales_created_at()), "bucket")
        .order_by(Order::desc(func::date_trunc("day", sales_created_at())));

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT DATE_TRUNC($1, sales.created_at) AS bucket FROM sales ORDER BY DATE_TRUNC($1, sales.created_at) DESC"
    );
    assert_eq!(sql.binds, vec![Value::String("day".to_string())]);
}

#[test]
fn compiles_order_by_alias() {
    let query: Select<User> = Select::new(user_table())
        .select_only()
        .column_as(user_email(), "email_addr")
        .order_by(Order::asc_alias("email_addr"));

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.email AS email_addr FROM users ORDER BY email_addr ASC");
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_select_query() {
    let query: Select<User> = Select::new(user_table()).filter(user_email().like("%example%")).limit(5).offset(10);

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.* FROM users WHERE (users.email LIKE $1) LIMIT 5 OFFSET 10");
    assert_eq!(sql.binds, vec![Value::String("%example%".to_string())]);
}

#[test]
fn compiles_between_expression() {
    let query: Select<User> = Select::new(user_table()).filter(user_id().between(1_i64, 5_i64));

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.* FROM users WHERE ((users.id >= $1) AND (users.id <= $2))");
    assert_eq!(sql.binds, vec![Value::I64(1), Value::I64(5)]);
}

#[test]
fn compiles_between_on_func_expression() {
    let start = NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).expect("start");
    let end = NaiveDateTime::from_timestamp_opt(1_700_000_100, 0).expect("end");
    let query: Select<Sale> = Select::new(sales_table()).filter(func::date_trunc("day", sales_created_at()).between(start, end));

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT sales.* FROM sales WHERE ((DATE_TRUNC($1, sales.created_at) >= $2) AND (DATE_TRUNC($1, sales.created_at) <= $3))"
    );
    assert_eq!(
        sql.binds,
        vec![Value::String("day".to_string()), Value::DateTime(start), Value::DateTime(end)]
    );
}

#[test]
fn compiles_add_operator_filter() {
    let expr = (sales_amount() + 5_i64).gt(10_i64);
    let query: Select<Sale> = Select::new(sales_table()).filter(expr);

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT sales.* FROM sales WHERE ((sales.amount + $1) > $2)");
    assert_eq!(sql.binds, vec![Value::I64(5), Value::I64(10)]);
}

#[test]
fn compiles_sub_operator_filter() {
    let expr = (sales_amount() - 7_i64).le(100_i64);
    let query: Select<Sale> = Select::new(sales_table()).filter(expr);

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT sales.* FROM sales WHERE ((sales.amount - $1) <= $2)");
    assert_eq!(sql.binds, vec![Value::I64(7), Value::I64(100)]);
}

#[test]
fn compiles_nested_arithmetic_expression_with_stable_parentheses() {
    let expr = (((sales_amount() + 5_i64) - sales_id()) + 2_i64).ge(20_i64);
    let query: Select<Sale> = Select::new(sales_table()).filter(expr);

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT sales.* FROM sales WHERE ((((sales.amount + $1) - sales.id) + $2) >= $3)"
    );
    assert_eq!(sql.binds, vec![Value::I64(5), Value::I64(2), Value::I64(20)]);
}

#[test]
fn compiles_arithmetic_expression_in_projection_and_ordering() {
    let query: Select<Sale> = Select::new(sales_table())
        .select_only()
        .column_as(sales_amount() + sales_id(), "projected_total")
        .order_by(Order::desc(sales_amount() - 10_i64));

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT (sales.amount + sales.id) AS projected_total FROM sales ORDER BY (sales.amount - $1) DESC"
    );
    assert_eq!(sql.binds, vec![Value::I64(10)]);
}

#[test]
fn compiles_timestamp_plus_custom_offset_function_filter() {
    let cutoff = chrono::DateTime::from_timestamp(1_700_000_000, 0).expect("cutoff").naive_utc();
    let expr = (window_anchor_at() + dbkit_core::interval::hours(window_offset_units())).le(cutoff);
    let query: Select<WindowRow> = Select::new(window_table()).filter(expr);

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT window_rows.* FROM window_rows WHERE ((window_rows.anchor_at + MAKE_INTERVAL(hours => window_rows.offset_units)) <= $1)"
    );
    assert_eq!(sql.binds, vec![Value::DateTime(cutoff)]);
}

#[test]
fn compiles_smallint_add_filter_against_integer_rhs() {
    // PostgreSQL promotes SMALLINT + SMALLINT to INTEGER, so the expression must
    // accept i32 comparison operands even though both source columns are i16.
    let expr = (compact_left_units() + compact_right_units()).gt(10_i32);
    let query: Select<CompactRow> = Select::new(compact_table()).filter(expr);

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT compact_rows.* FROM compact_rows WHERE ((compact_rows.left_units + compact_rows.right_units) > $1)"
    );
    assert_eq!(sql.binds, vec![Value::I32(10)]);
}

#[test]
fn compiles_smallint_sub_filter_against_integer_rhs() {
    // PostgreSQL applies the same promotion rule for subtraction.
    let expr = (compact_left_units() - compact_right_units()).le(3_i32);
    let query: Select<CompactRow> = Select::new(compact_table()).filter(expr);

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT compact_rows.* FROM compact_rows WHERE ((compact_rows.left_units - compact_rows.right_units) <= $1)"
    );
    assert_eq!(sql.binds, vec![Value::I32(3)]);
}

#[test]
fn compiles_smallint_arithmetic_projection_with_integer_expression_type() {
    // Projection typing matters too: follow-up filters/ordering should see the
    // arithmetic result as INTEGER rather than narrowing it back to SMALLINT.
    let projected_total: Expr<i32> = compact_left_units() + compact_right_units();
    let projected_delta: Expr<i32> = compact_left_units() - compact_right_units();
    let query: Select<CompactRow> = Select::new(compact_table())
        .select_only()
        .column_as(projected_total, "total_units")
        .order_by(Order::desc(projected_delta));

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT (compact_rows.left_units + compact_rows.right_units) AS total_units FROM compact_rows ORDER BY (compact_rows.left_units - compact_rows.right_units) DESC"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn condition_any_empty_returns_none() {
    let cond = Condition::any();
    assert!(cond.into_expr().is_none());
}

#[test]
fn condition_all_empty_returns_none() {
    let cond = Condition::all();
    assert!(cond.into_expr().is_none());
}

#[test]
fn compiles_condition_any_or() {
    let cond = Condition::any().add(user_email().like("%example%")).add(user_id().gt(10_i64));

    let query: Select<User> = Select::new(user_table()).filter(cond.into_expr().expect("expr"));
    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT users.* FROM users WHERE ((users.email LIKE $1) OR (users.id > $2))"
    );
    assert_eq!(sql.binds, vec![Value::String("%example%".to_string()), Value::I64(10)]);
}

#[test]
fn compiles_condition_all_and() {
    let cond = Condition::all().add(user_email().like("%example%")).add(user_id().gt(10_i64));

    let query: Select<User> = Select::new(user_table()).filter(cond.into_expr().expect("expr"));
    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT users.* FROM users WHERE ((users.email LIKE $1) AND (users.id > $2))"
    );
    assert_eq!(sql.binds, vec![Value::String("%example%".to_string()), Value::I64(10)]);
}

fn expr_sql(expr: Expr<bool>) -> dbkit_core::CompiledSql {
    let query: Select<User> = Select::new(user_table()).filter(expr);
    query.compile()
}
