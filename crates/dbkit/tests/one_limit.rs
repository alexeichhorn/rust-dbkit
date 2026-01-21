use dbkit::executor::BoxFuture;
use dbkit::{model, Error, Executor, SelectExt};
use dbkit::sqlx::postgres::PgArguments;

#[model(table = "users")]
struct User {
    #[key]
    id: i64,
}

struct CaptureExecutor {
    last_sql: Option<String>,
}

impl CaptureExecutor {
    fn new() -> Self {
        Self { last_sql: None }
    }
}

impl Executor for CaptureExecutor {
    fn fetch_all<'e, T>(
        &'e mut self,
        _sql: &'e str,
        _args: PgArguments,
    ) -> BoxFuture<'e, Result<Vec<T>, Error>>
    where
        T: for<'r> dbkit::sqlx::FromRow<'r, dbkit::sqlx::postgres::PgRow> + Send + Unpin + 'e,
    {
        Box::pin(async move { Ok(Vec::new()) })
    }

    fn fetch_optional<'e, T>(
        &'e mut self,
        sql: &'e str,
        _args: PgArguments,
    ) -> BoxFuture<'e, Result<Option<T>, Error>>
    where
        T: for<'r> dbkit::sqlx::FromRow<'r, dbkit::sqlx::postgres::PgRow> + Send + Unpin + 'e,
    {
        self.last_sql = Some(sql.to_string());
        Box::pin(async move { Ok(None) })
    }

    fn fetch_rows<'e>(
        &'e mut self,
        _sql: &'e str,
        _args: PgArguments,
    ) -> BoxFuture<'e, Result<Vec<dbkit::sqlx::postgres::PgRow>, Error>> {
        Box::pin(async move { Ok(Vec::new()) })
    }

    fn execute<'e>(&'e mut self, _sql: &'e str, _args: PgArguments) -> BoxFuture<'e, Result<u64, Error>> {
        Box::pin(async move { Ok(0) })
    }
}

#[tokio::test]
async fn one_applies_limit_one() -> Result<(), dbkit::Error> {
    let mut ex = CaptureExecutor::new();
    let _ = User::query().one(&mut ex).await?;
    let sql = ex.last_sql.expect("sql");
    assert!(sql.contains("LIMIT 1"), "sql missing LIMIT 1: {sql}");
    Ok(())
}
