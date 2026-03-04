use dbkit::executor::BoxFuture;
use dbkit::sqlx::postgres::PgArguments;
use dbkit::{model, Error, Executor};

#[model(table = "users")]
struct User {
    #[key]
    id: i64,
    name: String,
    email: String,
}

struct CaptureExecutor {
    last_sql: std::sync::Mutex<Option<String>>,
}

impl CaptureExecutor {
    fn new() -> Self {
        Self {
            last_sql: std::sync::Mutex::new(None),
        }
    }
}

impl Executor for CaptureExecutor {
    fn fetch_all<'e, T>(&'e self, sql: &'e str, _args: PgArguments) -> BoxFuture<'e, Result<Vec<T>, Error>>
    where
        T: for<'r> dbkit::sqlx::FromRow<'r, dbkit::sqlx::postgres::PgRow> + Send + Unpin + 'e,
    {
        *self.last_sql.lock().expect("lock") = Some(sql.to_string());
        Box::pin(async move { Ok(Vec::new()) })
    }

    fn fetch_optional<'e, T>(&'e self, _sql: &'e str, _args: PgArguments) -> BoxFuture<'e, Result<Option<T>, Error>>
    where
        T: for<'r> dbkit::sqlx::FromRow<'r, dbkit::sqlx::postgres::PgRow> + Send + Unpin + 'e,
    {
        Box::pin(async move { Ok(None) })
    }

    fn fetch_rows<'e>(&'e self, _sql: &'e str, _args: PgArguments) -> BoxFuture<'e, Result<Vec<dbkit::sqlx::postgres::PgRow>, Error>> {
        Box::pin(async move { Ok(Vec::new()) })
    }

    fn execute<'e>(&'e self, _sql: &'e str, _args: PgArguments) -> BoxFuture<'e, Result<u64, Error>> {
        Box::pin(async move { Ok(0) })
    }
}

#[tokio::test]
async fn active_update_only_sets_changed_fields() -> Result<(), dbkit::Error> {
    let ex = CaptureExecutor::new();
    let user = User {
        id: 1,
        name: "Old".to_string(),
        email: "old@db.com".to_string(),
    };
    let mut active = user.into_active();
    active.name = "New".into();

    let _ = active.update(&ex).await;
    let sql = ex.last_sql.lock().expect("lock").clone().expect("sql");
    assert!(sql.contains("SET name"), "sql missing SET name: {sql}");
    assert!(!sql.contains("email ="), "sql should not set email: {sql}");

    Ok(())
}

#[tokio::test]
async fn active_update_no_changes_skips_sql() -> Result<(), dbkit::Error> {
    let ex = CaptureExecutor::new();
    let user = User {
        id: 1,
        name: "Old".to_string(),
        email: "old@db.com".to_string(),
    };
    let active = user.into_active();

    let result = active.update(&ex).await;
    assert!(result.is_err());
    assert!(ex.last_sql.lock().expect("lock").is_none(), "update should not execute SQL");

    Ok(())
}
