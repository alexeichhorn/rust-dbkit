use std::future::Future;
use std::pin::Pin;

use sqlx::postgres::{PgArguments, PgRow};
use sqlx::Arguments;

use crate::Error;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Executor {
    fn fetch_all<'e, T>(&'e self, sql: &'e str, args: PgArguments) -> BoxFuture<'e, Result<Vec<T>, Error>>
    where
        T: for<'r> sqlx::FromRow<'r, PgRow> + Send + Unpin + 'e;

    fn fetch_optional<'e, T>(&'e self, sql: &'e str, args: PgArguments) -> BoxFuture<'e, Result<Option<T>, Error>>
    where
        T: for<'r> sqlx::FromRow<'r, PgRow> + Send + Unpin + 'e;

    fn fetch_rows<'e>(&'e self, sql: &'e str, args: PgArguments) -> BoxFuture<'e, Result<Vec<PgRow>, Error>>;

    fn execute<'e>(&'e self, sql: &'e str, args: PgArguments) -> BoxFuture<'e, Result<u64, Error>>;
}

impl Executor for crate::Database {
    fn fetch_all<'e, T>(&'e self, sql: &'e str, args: PgArguments) -> BoxFuture<'e, Result<Vec<T>, Error>>
    where
        T: for<'r> sqlx::FromRow<'r, PgRow> + Send + Unpin + 'e,
    {
        let pool = self.pool().clone();
        Box::pin(async move {
            let rows = sqlx::query_as_with::<sqlx::Postgres, T, _>(sql, args)
                .fetch_all(&pool)
                .await?;
            Ok(rows)
        })
    }

    fn fetch_optional<'e, T>(
        &'e self,
        sql: &'e str,
        args: PgArguments,
    ) -> BoxFuture<'e, Result<Option<T>, Error>>
    where
        T: for<'r> sqlx::FromRow<'r, PgRow> + Send + Unpin + 'e,
    {
        let pool = self.pool().clone();
        Box::pin(async move {
            let row = sqlx::query_as_with::<sqlx::Postgres, T, _>(sql, args)
                .fetch_optional(&pool)
                .await?;
            Ok(row)
        })
    }

    fn fetch_rows<'e>(&'e self, sql: &'e str, args: PgArguments) -> BoxFuture<'e, Result<Vec<PgRow>, Error>> {
        let pool = self.pool().clone();
        Box::pin(async move {
            let rows = sqlx::query_with::<sqlx::Postgres, _>(sql, args)
                .fetch_all(&pool)
                .await?;
            Ok(rows)
        })
    }

    fn execute<'e>(&'e self, sql: &'e str, args: PgArguments) -> BoxFuture<'e, Result<u64, Error>> {
        let pool = self.pool().clone();
        Box::pin(async move {
            let result = sqlx::query_with::<sqlx::Postgres, _>(sql, args)
                .execute(&pool)
                .await?;
            Ok(result.rows_affected())
        })
    }
}

impl Executor for sqlx::Pool<sqlx::Postgres> {
    fn fetch_all<'e, T>(&'e self, sql: &'e str, args: PgArguments) -> BoxFuture<'e, Result<Vec<T>, Error>>
    where
        T: for<'r> sqlx::FromRow<'r, PgRow> + Send + Unpin + 'e,
    {
        let pool = self.clone();
        Box::pin(async move {
            let rows = sqlx::query_as_with::<sqlx::Postgres, T, _>(sql, args)
                .fetch_all(&pool)
                .await?;
            Ok(rows)
        })
    }

    fn fetch_optional<'e, T>(
        &'e self,
        sql: &'e str,
        args: PgArguments,
    ) -> BoxFuture<'e, Result<Option<T>, Error>>
    where
        T: for<'r> sqlx::FromRow<'r, PgRow> + Send + Unpin + 'e,
    {
        let pool = self.clone();
        Box::pin(async move {
            let row = sqlx::query_as_with::<sqlx::Postgres, T, _>(sql, args)
                .fetch_optional(&pool)
                .await?;
            Ok(row)
        })
    }

    fn fetch_rows<'e>(&'e self, sql: &'e str, args: PgArguments) -> BoxFuture<'e, Result<Vec<PgRow>, Error>> {
        let pool = self.clone();
        Box::pin(async move {
            let rows = sqlx::query_with::<sqlx::Postgres, _>(sql, args)
                .fetch_all(&pool)
                .await?;
            Ok(rows)
        })
    }

    fn execute<'e>(&'e self, sql: &'e str, args: PgArguments) -> BoxFuture<'e, Result<u64, Error>> {
        let pool = self.clone();
        Box::pin(async move {
            let result = sqlx::query_with::<sqlx::Postgres, _>(sql, args)
                .execute(&pool)
                .await?;
            Ok(result.rows_affected())
        })
    }
}

impl<'t> Executor for crate::database::DbTransaction<'t> {
    fn fetch_all<'e, T>(&'e self, sql: &'e str, args: PgArguments) -> BoxFuture<'e, Result<Vec<T>, Error>>
    where
        T: for<'r> sqlx::FromRow<'r, PgRow> + Send + Unpin + 'e,
    {
        let inner = self.inner.clone();
        Box::pin(async move {
            let mut guard = inner.lock().await;
            let tx = guard
                .as_mut()
                .ok_or_else(|| Error::Decode("transaction already completed".to_string()))?;
            let conn = tx.as_mut();
            let rows = sqlx::query_as_with::<sqlx::Postgres, T, _>(sql, args)
                .fetch_all(conn)
                .await?;
            Ok(rows)
        })
    }

    fn fetch_optional<'e, T>(
        &'e self,
        sql: &'e str,
        args: PgArguments,
    ) -> BoxFuture<'e, Result<Option<T>, Error>>
    where
        T: for<'r> sqlx::FromRow<'r, PgRow> + Send + Unpin + 'e,
    {
        let inner = self.inner.clone();
        Box::pin(async move {
            let mut guard = inner.lock().await;
            let tx = guard
                .as_mut()
                .ok_or_else(|| Error::Decode("transaction already completed".to_string()))?;
            let conn = tx.as_mut();
            let row = sqlx::query_as_with::<sqlx::Postgres, T, _>(sql, args)
                .fetch_optional(conn)
                .await?;
            Ok(row)
        })
    }

    fn fetch_rows<'e>(&'e self, sql: &'e str, args: PgArguments) -> BoxFuture<'e, Result<Vec<PgRow>, Error>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            let mut guard = inner.lock().await;
            let tx = guard
                .as_mut()
                .ok_or_else(|| Error::Decode("transaction already completed".to_string()))?;
            let conn = tx.as_mut();
            let rows = sqlx::query_with::<sqlx::Postgres, _>(sql, args)
                .fetch_all(conn)
                .await?;
            Ok(rows)
        })
    }

    fn execute<'e>(&'e self, sql: &'e str, args: PgArguments) -> BoxFuture<'e, Result<u64, Error>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            let mut guard = inner.lock().await;
            let tx = guard
                .as_mut()
                .ok_or_else(|| Error::Decode("transaction already completed".to_string()))?;
            let conn = tx.as_mut();
            let result = sqlx::query_with::<sqlx::Postgres, _>(sql, args)
                .execute(conn)
                .await?;
            Ok(result.rows_affected())
        })
    }
}

pub fn build_arguments(binds: &[crate::Value]) -> Result<PgArguments, Error> {
    let mut args = PgArguments::default();
    for value in binds {
        match value {
            crate::Value::Null => {
                return Err(Error::Decode("cannot bind NULL without type".to_string()))
            }
            crate::Value::Bool(value) => args.add(*value),
            crate::Value::I16(value) => args.add(*value),
            crate::Value::I32(value) => args.add(*value),
            crate::Value::I64(value) => args.add(*value),
            crate::Value::F32(value) => args.add(*value),
            crate::Value::F64(value) => args.add(*value),
            crate::Value::String(value) => args.add(value.clone()),
            crate::Value::Array(value) => args.add(value.clone()),
            crate::Value::Json(value) => args.add(value.clone()),
            crate::Value::Uuid(value) => args.add(*value),
            crate::Value::DateTime(value) => args.add(value.clone()),
            crate::Value::DateTimeUtc(value) => args.add(value.clone()),
            crate::Value::Date(value) => args.add(value.clone()),
            crate::Value::Time(value) => args.add(value.clone()),
            crate::Value::Vector(value) => args.add(dbkit_core::types::vector_sql_literal(value)),
        }
    }
    Ok(args)
}
