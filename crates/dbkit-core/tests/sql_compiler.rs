use chrono::NaiveDateTime;
use dbkit_core::{expr::Value, func, Column, Expr, Order, Select, Table};

#[derive(Debug)]
struct User;

#[derive(Debug)]
struct Event;

#[derive(Debug)]
struct Sale;

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
    assert_eq!(
        sql.binds,
        vec![Value::I64(10), Value::String("%test%".to_string())]
    );
}

#[test]
fn compiles_in_expression() {
    let expr = user_id().in_([1_i64, 2, 3]);
    let sql = expr_sql(expr);
    assert_eq!(
        sql.sql,
        "SELECT users.* FROM users WHERE (users.id IN ($1, $2, $3))"
    );
    assert_eq!(
        sql.binds,
        vec![Value::I64(1), Value::I64(2), Value::I64(3)]
    );
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
    assert_eq!(
        sql.sql,
        "SELECT users.* FROM users WHERE (users.email IS NOT NULL)"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_upper_function_filter() {
    let expr = func::upper(user_email()).eq("TEST");
    let sql = expr_sql(expr);
    assert_eq!(
        sql.sql,
        "SELECT users.* FROM users WHERE (UPPER(users.email) = $1)"
    );
    assert_eq!(sql.binds, vec![Value::String("TEST".to_string())]);
}

#[test]
fn compiles_coalesce_function_filter() {
    let expr = func::coalesce(user_email(), "unknown").eq("ALPHA");
    let sql = expr_sql(expr);
    assert_eq!(
        sql.sql,
        "SELECT users.* FROM users WHERE (COALESCE(users.email, $1) = $2)"
    );
    assert_eq!(
        sql.binds,
        vec![
            Value::String("unknown".to_string()),
            Value::String("ALPHA".to_string()),
        ]
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
    assert_eq!(
        sql.binds,
        vec![Value::String("alpha@db.com".to_string())]
    );
}

#[test]
fn compiles_date_trunc_function_filter() {
    let dt = NaiveDateTime::from_timestamp_opt(1_700_000_000, 0).expect("dt");
    let expr = func::date_trunc("day", event_starts_at()).eq(dt);
    let query: Select<Event> = Select::new(event_table()).filter(expr);
    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT events.* FROM events WHERE (DATE_TRUNC($1, events.starts_at) = $2)"
    );
    assert_eq!(
        sql.binds,
        vec![Value::String("day".to_string()), Value::DateTime(dt)]
    );
}

#[test]
fn compiles_nested_functions() {
    let expr = func::upper(func::coalesce(user_email(), "unknown")).eq("ALPHA");
    let sql = expr_sql(expr);
    assert_eq!(
        sql.sql,
        "SELECT users.* FROM users WHERE (UPPER(COALESCE(users.email, $1)) = $2)"
    );
    assert_eq!(
        sql.binds,
        vec![
            Value::String("unknown".to_string()),
            Value::String("ALPHA".to_string()),
        ]
    );
}

#[test]
fn compiles_select_only_with_columns() {
    let query: Select<User> = Select::new(user_table())
        .select_only()
        .column(user_email())
        .column(user_id());

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.email, users.id FROM users");
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_select_only_with_column_as() {
    let query: Select<User> = Select::new(user_table())
        .select_only()
        .column_as(user_email(), "email_addr");

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.email AS email_addr FROM users");
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_select_only_with_func_column() {
    let query: Select<User> = Select::new(user_table())
        .select_only()
        .column(func::upper(user_email()));

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT UPPER(users.email) FROM users");
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_group_by_and_having() {
    let query: Select<User> = Select::new(user_table())
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

    let query: Select<User> = Select::new(user_table())
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
    let query: Select<Sale> = Select::new(sales_table())
        .select_only()
        .column_as(func::date_trunc("day", sales_created_at()), "bucket")
        .column_as(func::sum(sales_amount()), "total")
        .group_by(func::date_trunc("day", sales_created_at()));

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT DATE_TRUNC($1, sales.created_at) AS bucket, SUM(sales.amount) AS total FROM sales GROUP BY DATE_TRUNC($1, sales.created_at)"
    );
    assert_eq!(
        sql.binds,
        vec![Value::String("day".to_string())]
    );
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
    assert_eq!(
        sql.sql,
        "SELECT users.email AS email_addr FROM users ORDER BY email_addr ASC"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_select_query() {
    let query: Select<User> = Select::new(user_table())
        .filter(user_email().like("%example%"))
        .limit(5)
        .offset(10);

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT users.* FROM users WHERE (users.email LIKE $1) LIMIT 5 OFFSET 10"
    );
    assert_eq!(sql.binds, vec![Value::String("%example%".to_string())]);
}

fn expr_sql(expr: Expr<bool>) -> dbkit_core::CompiledSql {
    let query: Select<User> = Select::new(user_table()).filter(expr);
    query.compile()
}
