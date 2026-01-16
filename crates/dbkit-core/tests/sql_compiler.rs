use dbkit_core::{expr::Value, Column, Expr, Select, Table};

#[derive(Debug)]
struct User;

fn user_table() -> Table {
    Table::new("users")
}

fn user_id() -> Column<User, i64> {
    Column::new(user_table(), "id")
}

fn user_email() -> Column<User, String> {
    Column::new(user_table(), "email")
}

#[test]
fn compiles_basic_filter() {
    let expr = user_email().eq("a@b.com");
    let sql = expr_sql(expr);
    assert_eq!(sql.sql, "(users.email = $1)");
    assert_eq!(sql.binds, vec![Value::String("a@b.com".to_string())]);
}

#[test]
fn compiles_bool_composition() {
    let expr = user_id().gt(10).and(user_email().ilike("%test%"));
    let sql = expr_sql(expr);
    assert_eq!(
        sql.sql,
        "((users.id > $1) AND (users.email ILIKE $2))"
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
    assert_eq!(sql.sql, "(users.id IN ($1, $2, $3))");
    assert_eq!(
        sql.binds,
        vec![Value::I64(1), Value::I64(2), Value::I64(3)]
    );
}

#[test]
fn compiles_is_null_expression() {
    let expr = user_email().is_null();
    let sql = expr_sql(expr);
    assert_eq!(sql.sql, "(users.email IS NULL)");
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
