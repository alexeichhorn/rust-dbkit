use dbkit_core::{Delete, Insert, Table, Update, Value};
use dbkit_core::{Column, ColumnRef};

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
fn compiles_delete_with_filter() {
    let query = Delete::new(user_table()).filter(user_id().eq(42_i64));

    let sql = query.compile();
    assert_eq!(sql.sql, "DELETE FROM users WHERE (users.id = $1)");
    assert_eq!(sql.binds, vec![Value::I64(42)]);
}
