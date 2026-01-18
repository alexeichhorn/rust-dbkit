use dbkit_core::{Delete, Insert, Table, Update, Value};
use dbkit_core::{Column, ColumnRef};

#[derive(Debug)]
struct User;

fn user_table() -> Table {
    Table::new("users")
}

fn order_line_table() -> Table {
    Table::new("order_lines")
}

fn user_id() -> Column<User, i64> {
    Column::new(user_table(), "id")
}

fn user_email() -> Column<User, String> {
    Column::new(user_table(), "email")
}

fn user_name() -> Column<User, String> {
    Column::new(user_table(), "name")
}

fn order_id() -> Column<User, i64> {
    Column::new(order_line_table(), "order_id")
}

fn line_id() -> Column<User, i64> {
    Column::new(order_line_table(), "line_id")
}

fn line_note() -> Column<User, String> {
    Column::new(order_line_table(), "note")
}

#[test]
fn compiles_insert_returning_all() {
    let query: Insert<User> = Insert::new(user_table())
        .value(user_email(), "a@b.com")
        .returning_all();

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "INSERT INTO users (email) VALUES ($1) RETURNING users.*"
    );
    assert_eq!(sql.binds, vec![Value::String("a@b.com".to_string())]);
}

#[test]
fn compiles_insert_with_null() {
    let query: Insert<User> = Insert::new(user_table())
        .value(user_email(), None)
        .returning_all();

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "INSERT INTO users (email) VALUES (NULL) RETURNING users.*"
    );
    assert!(sql.binds.is_empty());
}

#[test]
fn compiles_insert_many_rows() {
    let query: Insert<User> = Insert::new(user_table())
        .row(|row| row.value(user_email(), "a@b.com").value(user_name(), "Alice"))
        .row(|row| row.value(user_email(), None::<String>).value(user_name(), "Bob"))
        .returning_all();

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "INSERT INTO users (email, name) VALUES ($1, $2), (NULL, $3) RETURNING users.*"
    );
    assert_eq!(
        sql.binds,
        vec![
            Value::String("a@b.com".to_string()),
            Value::String("Alice".to_string()),
            Value::String("Bob".to_string()),
        ]
    );
}

#[test]
fn compiles_update_with_filter() {
    let query: Update<User> = Update::new(user_table())
        .set(user_email(), "new@b.com")
        .filter(user_id().eq(1_i64))
        .returning(vec![ColumnRef::new(user_table(), "id")]);

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "UPDATE users SET email = $1 WHERE (users.id = $2) RETURNING users.id"
    );
    assert_eq!(
        sql.binds,
        vec![Value::String("new@b.com".to_string()), Value::I64(1)]
    );
}

#[test]
fn compiles_update_returning_all_single_field() {
    let query: Update<User> = Update::new(user_table())
        .set(user_name(), "Updated")
        .filter(user_id().eq(1_i64))
        .returning_all();

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "UPDATE users SET name = $1 WHERE (users.id = $2) RETURNING users.*"
    );
    assert_eq!(
        sql.binds,
        vec![Value::String("Updated".to_string()), Value::I64(1)]
    );
}

#[test]
fn compiles_update_with_composite_key_filters() {
    let query: Update<User> = Update::new(order_line_table())
        .set(line_note(), "Updated")
        .filter(order_id().eq(1_i64))
        .filter(line_id().eq(2_i64))
        .returning_all();

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "UPDATE order_lines SET note = $1 WHERE (order_lines.order_id = $2) AND (order_lines.line_id = $3) RETURNING order_lines.*"
    );
    assert_eq!(
        sql.binds,
        vec![Value::String("Updated".to_string()), Value::I64(1), Value::I64(2)]
    );
}

#[test]
fn compiles_update_set_null() {
    let query: Update<User> = Update::new(user_table())
        .set(user_email(), None)
        .filter(user_id().eq(1_i64))
        .returning_all();

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "UPDATE users SET email = NULL WHERE (users.id = $1) RETURNING users.*"
    );
    assert_eq!(sql.binds, vec![Value::I64(1)]);
}

#[test]
fn compiles_delete_with_filter() {
    let query = Delete::new(user_table()).filter(user_id().eq(42_i64));

    let sql = query.compile();
    assert_eq!(sql.sql, "DELETE FROM users WHERE (users.id = $1)");
    assert_eq!(sql.binds, vec![Value::I64(42)]);
}
