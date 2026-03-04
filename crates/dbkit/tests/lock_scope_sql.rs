use dbkit::executor::BoxFuture;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{model, Error, Executor, SelectExt};

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
    fn fetch_all<'e, T>(&'e self, sql: &'e str, _args: PgArguments) -> BoxFuture<'e, Result<Vec<T>, Error>>
    where
        T: for<'r> dbkit::sqlx::FromRow<'r, dbkit::sqlx::postgres::PgRow> + Send + Unpin + 'e,
    {
        self.sqls.lock().expect("lock").push(sql.to_string());
        Box::pin(async move { Ok(Vec::new()) })
    }

    fn fetch_optional<'e, T>(&'e self, sql: &'e str, _args: PgArguments) -> BoxFuture<'e, Result<Option<T>, Error>>
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
async fn left_join_for_update_nowait_scopes_lock_to_base_table() -> Result<(), dbkit::Error> {
    let ex = CaptureExecutor::new();
    let _rows: Vec<User> = User::query().left_join(User::todos).for_update().nowait().all(&ex).await?;

    let sqls = ex.sqls.lock().expect("lock");
    assert_eq!(sqls.len(), 1);
    let sql = &sqls[0];
    assert!(
        sql.contains("FOR UPDATE OF users NOWAIT"),
        "sql should scope lock to base table: {sql}"
    );

    Ok(())
}
