use chrono::NaiveDateTime;
use dbkit_core::{expr::Value, func, Column, Expr, Select, Table};

#[derive(Debug)]
struct User;

#[derive(Debug)]
struct Event;

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
