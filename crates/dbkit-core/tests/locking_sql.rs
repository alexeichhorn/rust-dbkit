use dbkit_core::{expr::Value, Column, Order, Select, Table};

#[derive(Debug)]
struct User;

#[derive(Debug)]
struct Todo;

fn user_table() -> Table {
    Table::new("users")
}

fn todo_table() -> Table {
    Table::new("todos")
}

fn user_id() -> Column<User, i64> {
    Column::new(user_table(), "id")
}

fn user_email() -> Column<User, String> {
    Column::new(user_table(), "email")
}

fn todo_user_id() -> Column<Todo, i64> {
    Column::new(todo_table(), "user_id")
}

fn todo_title() -> Column<Todo, String> {
    Column::new(todo_table(), "title")
}

#[test]
fn compiles_for_update_clause() {
    let query: Select<User> = Select::new(user_table()).for_update();

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.* FROM users FOR UPDATE");
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_for_update_skip_locked_clause() {
    let query: Select<User> = Select::new(user_table()).for_update().skip_locked();

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.* FROM users FOR UPDATE SKIP LOCKED");
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_for_update_nowait_clause() {
    let query: Select<User> = Select::new(user_table()).for_update().nowait();

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.* FROM users FOR UPDATE NOWAIT");
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_locking_after_order_limit_offset() {
    let query: Select<User> = Select::new(user_table())
        .filter(user_email().ilike("%@example.com"))
        .order_by(Order::asc(user_id()))
        .limit(20)
        .offset(40)
        .for_update()
        .skip_locked();

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT users.* FROM users WHERE (users.email ILIKE $1) ORDER BY users.id ASC LIMIT 20 OFFSET 40 FOR UPDATE SKIP LOCKED"
    );
    assert_eq!(sql.binds, vec![Value::String("%@example.com".to_string())]);
}

#[test]
fn compiles_locking_when_for_update_called_before_other_clauses() {
    let query: Select<User> = Select::new(user_table())
        .for_update()
        .filter(user_email().ilike("%@example.com"))
        .order_by(Order::desc(user_id()))
        .limit(10)
        .offset(5)
        .skip_locked();

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT users.* FROM users WHERE (users.email ILIKE $1) ORDER BY users.id DESC LIMIT 10 OFFSET 5 FOR UPDATE SKIP LOCKED"
    );
    assert_eq!(sql.binds, vec![Value::String("%@example.com".to_string())]);
}

#[test]
fn compiles_locking_with_join_filter_distinct() {
    let query: Select<User> = Select::new(user_table())
        .join_on(todo_table(), user_id().eq_col(todo_user_id()))
        .filter(todo_title().eq("Senior Rust Engineer"))
        .distinct()
        .for_update()
        .nowait();

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT DISTINCT users.* FROM users JOIN todos ON (users.id = todos.user_id) WHERE (todos.title = $1) FOR UPDATE NOWAIT"
    );
    assert_eq!(
        sql.binds,
        vec![Value::String("Senior Rust Engineer".to_string())]
    );
}

#[test]
fn compiles_select_only_with_locking_clause() {
    let query: Select<User> = Select::new(user_table())
        .select_only()
        .column(user_id())
        .for_update();

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.id FROM users FOR UPDATE");
    assert!(sql.binds.is_empty());
}

#[test]
fn for_update_is_idempotent() {
    let query: Select<User> = Select::new(user_table()).for_update().for_update();

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.* FROM users FOR UPDATE");
    assert!(sql.binds.is_empty());
}

#[test]
fn skip_locked_is_idempotent() {
    let query: Select<User> = Select::new(user_table())
        .for_update()
        .skip_locked()
        .skip_locked();

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.* FROM users FOR UPDATE SKIP LOCKED");
    assert!(sql.binds.is_empty());
}

#[test]
fn nowait_is_idempotent() {
    let query: Select<User> = Select::new(user_table()).for_update().nowait().nowait();

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.* FROM users FOR UPDATE NOWAIT");
    assert!(sql.binds.is_empty());
}
