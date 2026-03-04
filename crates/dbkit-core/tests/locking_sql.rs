use dbkit_core::{
    expr::{ExprNode, Value},
    Column, Join, JoinKind, Order, Select, SelectItem, Table,
};

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
    let query = Select::<User>::new(user_table()).for_update();

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.* FROM users FOR UPDATE");
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_for_update_skip_locked_clause() {
    let query = Select::<User>::new(user_table()).for_update().skip_locked();

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.* FROM users FOR UPDATE SKIP LOCKED");
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_for_update_nowait_clause() {
    let query = Select::<User>::new(user_table()).for_update().nowait();

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.* FROM users FOR UPDATE NOWAIT");
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_locking_after_order_limit_offset() {
    let query = Select::<User>::new(user_table())
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
    let query = Select::<User>::new(user_table())
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
fn compiles_locking_with_join_filter() {
    let query = Select::<User>::new(user_table())
        .join_on(todo_table(), user_id().eq_col(todo_user_id()))
        .filter(todo_title().eq("Senior Rust Engineer"))
        .for_update()
        .nowait();

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT users.* FROM users JOIN todos ON (users.id = todos.user_id) WHERE (todos.title = $1) FOR UPDATE NOWAIT"
    );
    assert_eq!(sql.binds, vec![Value::String("Senior Rust Engineer".to_string())]);
}

#[test]
fn compiles_select_only_with_locking_clause() {
    let query = Select::<User>::new(user_table()).select_only().column(user_id()).for_update();

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.id FROM users FOR UPDATE");
    assert!(sql.binds.is_empty());
}

#[test]
fn for_update_is_idempotent() {
    let query = Select::<User>::new(user_table()).for_update().for_update();

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.* FROM users FOR UPDATE");
    assert!(sql.binds.is_empty());
}

#[test]
fn skip_locked_is_idempotent() {
    let query = Select::<User>::new(user_table()).for_update().skip_locked().skip_locked();

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.* FROM users FOR UPDATE SKIP LOCKED");
    assert!(sql.binds.is_empty());
}

#[test]
fn nowait_is_idempotent() {
    let query = Select::<User>::new(user_table()).for_update().nowait().nowait();

    let sql = query.compile();
    assert_eq!(sql.sql, "SELECT users.* FROM users FOR UPDATE NOWAIT");
    assert!(sql.binds.is_empty());
}

#[test]
fn compile_without_pagination_omits_locking_for_count_exists_subqueries() {
    let query = Select::<User>::new(user_table())
        .filter(user_email().eq("worker@example.com"))
        .order_by(Order::desc(user_id()))
        .limit(5)
        .offset(10)
        .for_update()
        .skip_locked();

    let sql = query.compile_without_pagination();
    assert_eq!(sql.sql, "SELECT users.* FROM users WHERE (users.email = $1)");
    assert_eq!(sql.binds, vec![Value::String("worker@example.com".to_string())]);
}

#[test]
fn compile_with_extra_preserves_locking_clause() {
    let query = Select::<User>::new(user_table()).filter(user_id().eq(42)).for_update().nowait();

    let extra_columns = vec![SelectItem {
        expr: ExprNode::Column(todo_title().as_ref()),
        alias: Some("todo_title".to_string()),
    }];
    let extra_joins = vec![Join {
        table: todo_table(),
        on: user_id().eq_col(todo_user_id()),
        kind: JoinKind::Inner,
    }];

    let sql = query.compile_with_extra(&extra_columns, &extra_joins);
    assert_eq!(
        sql.sql,
        "SELECT users.*, todos.title AS todo_title FROM users JOIN todos ON (users.id = todos.user_id) WHERE (users.id = $1) FOR UPDATE NOWAIT"
    );
    assert_eq!(sql.binds, vec![Value::I64(42)]);
}

#[test]
fn compile_with_extra_left_join_scopes_lock_to_base_table() {
    let query = Select::<User>::new(user_table()).filter(user_id().eq(7)).for_update().nowait();

    let extra_columns = vec![SelectItem {
        expr: ExprNode::Column(todo_title().as_ref()),
        alias: Some("todo_title".to_string()),
    }];
    let extra_joins = vec![Join {
        table: todo_table(),
        on: user_id().eq_col(todo_user_id()),
        kind: JoinKind::Left,
    }];

    let sql = query.compile_with_extra(&extra_columns, &extra_joins);
    assert_eq!(
        sql.sql,
        "SELECT users.*, todos.title AS todo_title FROM users LEFT JOIN todos ON (users.id = todos.user_id) WHERE (users.id = $1) FOR UPDATE OF users NOWAIT"
    );
    assert_eq!(sql.binds, vec![Value::I64(7)]);
}

#[test]
fn compiles_direct_left_join_for_update_scopes_lock_to_base_table() {
    let query = Select::<User>::new(user_table())
        .left_join_on(todo_table(), user_id().eq_col(todo_user_id()))
        .for_update();

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT users.* FROM users LEFT JOIN todos ON (users.id = todos.user_id) FOR UPDATE OF users"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_direct_left_join_for_update_skip_locked_scopes_lock_to_base_table() {
    let query = Select::<User>::new(user_table())
        .left_join_on(todo_table(), user_id().eq_col(todo_user_id()))
        .for_update()
        .skip_locked();

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT users.* FROM users LEFT JOIN todos ON (users.id = todos.user_id) FOR UPDATE OF users SKIP LOCKED"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_direct_left_join_for_update_nowait_scopes_lock_to_base_table() {
    let query = Select::<User>::new(user_table())
        .left_join_on(todo_table(), user_id().eq_col(todo_user_id()))
        .for_update()
        .nowait();

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT users.* FROM users LEFT JOIN todos ON (users.id = todos.user_id) FOR UPDATE OF users NOWAIT"
    );
    assert!(sql.binds.is_empty());
}
