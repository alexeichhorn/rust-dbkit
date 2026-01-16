use dbkit_core::{rel::{BelongsTo, HasMany}, Column, Select, Table, Value};

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

fn todo_user_id() -> Column<Todo, i64> {
    Column::new(todo_table(), "user_id")
}

#[test]
fn compiles_join_from_relation() {
    let rel = HasMany::new(user_table(), todo_table(), user_id().as_ref(), todo_user_id().as_ref());
    let query: Select<User> = Select::new(user_table()).join(rel);

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT users.* FROM users JOIN todos ON (todos.user_id = users.id)"
    );
    assert_eq!(sql.binds, Vec::<Value>::new());
}

#[test]
fn compiles_left_join_from_relation() {
    let rel = HasMany::new(user_table(), todo_table(), user_id().as_ref(), todo_user_id().as_ref());
    let query: Select<User> = Select::new(user_table()).left_join(rel);

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT users.* FROM users LEFT JOIN todos ON (todos.user_id = users.id)"
    );
    assert_eq!(sql.binds, Vec::<Value>::new());
}

#[test]
fn compiles_join_belongs_to_relation() {
    let rel = BelongsTo::new(todo_table(), user_table(), todo_user_id().as_ref(), user_id().as_ref());
    let query: Select<Todo> = Select::new(todo_table()).join(rel);

    let sql = query.compile();
    assert_eq!(
        sql.sql,
        "SELECT todos.* FROM todos JOIN users ON (todos.user_id = users.id)"
    );
    assert_eq!(sql.binds, Vec::<Value>::new());
}
