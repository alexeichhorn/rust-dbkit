use dbkit::executor::BoxFuture;
use dbkit::{model, Error, Executor, SelectExt};
use dbkit::sqlx::postgres::PgArguments;

#[model(table = "users")]
struct User {
    #[key]
    id: i64,
    name: String,
    #[has_many]
    todos: dbkit::HasMany<Todo>,
}

#[model(table = "todos")]
struct Todo {
    #[key]
    id: i64,
    user_id: i64,
    title: String,
    #[belongs_to(key = user_id, references = id)]
    user: dbkit::BelongsTo<User>,
    #[many_to_many(through = TodoTag, left_key = todo_id, right_key = tag_id)]
    tags: dbkit::ManyToMany<Tag>,
}

#[model(table = "tags")]
struct Tag {
    #[key]
    id: i64,
    name: String,
    #[many_to_many(through = TodoTag, left_key = tag_id, right_key = todo_id)]
    todos: dbkit::ManyToMany<Todo>,
}

#[model(table = "todo_tags")]
struct TodoTag {
    #[key]
    todo_id: i64,
    #[key]
    tag_id: i64,
}

struct CaptureExecutor {
    sqls: std::sync::Mutex<Vec<String>>,
}

impl CaptureExecutor {
    fn new() -> Self {
        Self {
            sqls: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl Executor for CaptureExecutor {
    fn fetch_all<'e, T>(
        &'e self,
        sql: &'e str,
        _args: PgArguments,
    ) -> BoxFuture<'e, Result<Vec<T>, Error>>
    where
        T: for<'r> dbkit::sqlx::FromRow<'r, dbkit::sqlx::postgres::PgRow> + Send + Unpin + 'e,
    {
        self.sqls.lock().expect("lock").push(sql.to_string());
        Box::pin(async move { Ok(Vec::new()) })
    }

    fn fetch_optional<'e, T>(
        &'e self,
        sql: &'e str,
        _args: PgArguments,
    ) -> BoxFuture<'e, Result<Option<T>, Error>>
    where
        T: for<'r> dbkit::sqlx::FromRow<'r, dbkit::sqlx::postgres::PgRow> + Send + Unpin + 'e,
    {
        self.sqls.lock().expect("lock").push(sql.to_string());
        Box::pin(async move { Ok(None) })
    }

    fn fetch_rows<'e>(&'e self, sql: &'e str, _args: PgArguments) -> BoxFuture<'e, Result<Vec<dbkit::sqlx::postgres::PgRow>, Error>> {
        self.sqls.lock().expect("lock").push(sql.to_string());
        Box::pin(async move { Ok(Vec::new()) })
    }

    fn execute<'e>(&'e self, sql: &'e str, _args: PgArguments) -> BoxFuture<'e, Result<u64, Error>> {
        self.sqls.lock().expect("lock").push(sql.to_string());
        Box::pin(async move { Ok(0) })
    }
}

#[tokio::test]
async fn joined_has_many_uses_single_join_query() -> Result<(), dbkit::Error> {
    let ex = CaptureExecutor::new();
    let _rows: Vec<UserModel<Vec<Todo>>> = User::query()
        .with(User::todos.joined())
        .all(&ex)
        .await?;

    let sqls = ex.sqls.lock().expect("lock");
    assert_eq!(sqls.len(), 1);
    let sql = &sqls[0];
    assert!(sql.contains("JOIN"), "sql missing JOIN: {sql}");
    assert!(sql.contains("todos"), "sql missing todos join: {sql}");

    Ok(())
}

#[tokio::test]
async fn joined_belongs_to_uses_single_join_query() -> Result<(), dbkit::Error> {
    let ex = CaptureExecutor::new();
    let _rows: Vec<TodoModel<Option<User>>> = Todo::query()
        .with(Todo::user.joined())
        .all(&ex)
        .await?;

    let sqls = ex.sqls.lock().expect("lock");
    assert_eq!(sqls.len(), 1);
    let sql = &sqls[0];
    assert!(sql.contains("JOIN"), "sql missing JOIN: {sql}");
    assert!(sql.contains("users"), "sql missing users join: {sql}");

    Ok(())
}

#[tokio::test]
async fn joined_many_to_many_uses_single_join_query() -> Result<(), dbkit::Error> {
    let ex = CaptureExecutor::new();
    let _rows: Vec<TodoModel<dbkit::NotLoaded, Vec<Tag>>> = Todo::query()
        .with(Todo::tags.joined())
        .all(&ex)
        .await?;

    let sqls = ex.sqls.lock().expect("lock");
    assert_eq!(sqls.len(), 1);
    let sql = &sqls[0];
    assert!(sql.contains("JOIN"), "sql missing JOIN: {sql}");
    assert!(sql.contains("todo_tags"), "sql missing join table: {sql}");
    assert!(sql.contains("tags"), "sql missing tags join: {sql}");

    Ok(())
}

#[tokio::test]
async fn joined_nested_many_to_many_uses_single_join_query() -> Result<(), dbkit::Error> {
    let ex = CaptureExecutor::new();
    let _rows: Vec<UserModel<Vec<TodoModel<dbkit::NotLoaded, Vec<Tag>>>>> = User::query()
        .with(User::todos.joined().with(Todo::tags.joined()))
        .all(&ex)
        .await?;

    let sqls = ex.sqls.lock().expect("lock");
    assert_eq!(sqls.len(), 1);
    let sql = &sqls[0];
    assert!(sql.contains("JOIN"), "sql missing JOIN: {sql}");
    assert!(sql.contains("todos"), "sql missing todos join: {sql}");
    assert!(sql.contains("todo_tags"), "sql missing join table: {sql}");
    assert!(sql.contains("tags"), "sql missing tags join: {sql}");

    Ok(())
}
