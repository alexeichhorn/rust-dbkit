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
