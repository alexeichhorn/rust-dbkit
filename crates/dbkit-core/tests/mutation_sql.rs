use dbkit_core::{Delete, Insert, Table, Update, Value};
use dbkit_core::{Column, ColumnRef};

#[derive(Debug)]
struct User;
#[derive(Debug)]
struct RunPayload;

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

fn run_payload_table() -> Table {
    Table::new("run_payloads")
}

fn run_target_id() -> Column<RunPayload, i64> {
    Column::new(run_payload_table(), "target_id")
}

fn run_id() -> Column<RunPayload, i64> {
    Column::new(run_payload_table(), "run_id")
}

fn run_payload() -> Column<RunPayload, String> {
    Column::new(run_payload_table(), "payload")
}

fn run_source() -> Column<RunPayload, String> {
    Column::new(run_payload_table(), "source")
}

fn run_version() -> Column<RunPayload, i64> {
    Column::new(run_payload_table(), "version")
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
fn compiles_insert_on_conflict_do_nothing_with_single_target() {
    let query: Insert<User> = Insert::new(user_table())
        .value(user_email(), "a@b.com")
        .value(user_name(), "Alice")
        .on_conflict_do_nothing(user_email())
        .returning_all();

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "INSERT INTO users (email, name) VALUES ($1, $2) ON CONFLICT (email) DO NOTHING RETURNING users.*"
    );
    assert_eq!(
        sql.binds,
        vec![
            Value::String("a@b.com".to_string()),
            Value::String("Alice".to_string()),
        ]
    );
}

#[test]
fn compiles_insert_many_on_conflict_do_nothing_with_composite_target() {
    let query: Insert<RunPayload> = Insert::new(run_payload_table())
        .row(|row| {
            row.value(run_target_id(), 10_i64)
                .value(run_id(), 20_i64)
                .value(run_payload(), "v1")
                .value(run_source(), "seed")
                .value(run_version(), 1_i64)
        })
        .row(|row| {
            row.value(run_target_id(), 11_i64)
                .value(run_id(), 21_i64)
                .value(run_payload(), "v2")
                .value(run_source(), "seed-2")
                .value(run_version(), 2_i64)
        })
        .on_conflict_do_nothing((run_target_id(), run_id()))
        .returning_all();

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "INSERT INTO run_payloads (target_id, run_id, payload, source, version) VALUES ($1, $2, $3, $4, $5), ($6, $7, $8, $9, $10) ON CONFLICT (target_id, run_id) DO NOTHING RETURNING run_payloads.*"
    );
    assert_eq!(
        sql.binds,
        vec![
            Value::I64(10),
            Value::I64(20),
            Value::String("v1".to_string()),
            Value::String("seed".to_string()),
            Value::I64(1),
            Value::I64(11),
            Value::I64(21),
            Value::String("v2".to_string()),
            Value::String("seed-2".to_string()),
            Value::I64(2),
        ]
    );
}

#[test]
fn compiles_insert_on_conflict_do_update_with_composite_target_and_selected_overwrite_columns() {
    let query: Insert<RunPayload> = Insert::new(run_payload_table())
        .value(run_target_id(), 42_i64)
        .value(run_id(), 7_i64)
        .value(run_payload(), "new-payload")
        .value(run_source(), "new-source")
        .value(run_version(), 3_i64)
        .on_conflict_do_update(
            (run_target_id(), run_id()),
            (run_payload(), run_version()),
        )
        .returning_all();

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "INSERT INTO run_payloads (target_id, run_id, payload, source, version) VALUES ($1, $2, $3, $4, $5) ON CONFLICT (target_id, run_id) DO UPDATE SET payload = EXCLUDED.payload, version = EXCLUDED.version RETURNING run_payloads.*"
    );
    assert_eq!(
        sql.binds,
        vec![
            Value::I64(42),
            Value::I64(7),
            Value::String("new-payload".to_string()),
            Value::String("new-source".to_string()),
            Value::I64(3),
        ]
    );
}

#[test]
fn compiles_insert_on_conflict_do_update_with_null_insert_values_and_preserves_bind_order() {
    let query: Insert<User> = Insert::new(user_table())
        .value(user_email(), None::<String>)
        .value(user_name(), "fallback-name")
        .on_conflict_do_update(user_email(), user_name())
        .returning_all();

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "INSERT INTO users (email, name) VALUES (NULL, $1) ON CONFLICT (email) DO UPDATE SET name = EXCLUDED.name RETURNING users.*"
    );
    assert_eq!(sql.binds, vec![Value::String("fallback-name".to_string())]);
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
