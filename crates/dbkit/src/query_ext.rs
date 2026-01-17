use crate::executor::{build_arguments, BoxFuture};
use crate::runtime::RunLoads;
use crate::{Delete, Error, Executor, Insert, Select, Update};

pub trait SelectExt<Out, Loads> {
    fn all<'e, E>(self, ex: &'e mut E) -> BoxFuture<'e, Result<Vec<Out>, Error>>
    where
        E: Executor + Send + 'e,
        Loads: RunLoads<Out> + Send + Sync + 'e,
        Out: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + 'e;

    fn one<'e, E>(self, ex: &'e mut E) -> BoxFuture<'e, Result<Option<Out>, Error>>
    where
        E: Executor + Send + 'e,
        Loads: RunLoads<Out> + Send + Sync + 'e,
        Out: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + 'e;
}

impl<Out, Loads> SelectExt<Out, Loads> for Select<Out, Loads> {
    fn all<'e, E>(self, ex: &'e mut E) -> BoxFuture<'e, Result<Vec<Out>, Error>>
    where
        E: Executor + Send + 'e,
        Loads: RunLoads<Out> + Send + Sync + 'e,
        Out: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + 'e,
    {
        let (compiled, loads) = self.into_parts();
        Box::pin(async move {
            let args = build_arguments(&compiled.binds)?;
            let mut rows = ex.fetch_all::<Out>(&compiled.sql, args).await?;
            loads.run(ex, &mut rows).await?;
            Ok(rows)
        })
    }

    fn one<'e, E>(self, ex: &'e mut E) -> BoxFuture<'e, Result<Option<Out>, Error>>
    where
        E: Executor + Send + 'e,
        Loads: RunLoads<Out> + Send + Sync + 'e,
        Out: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + 'e,
    {
        let (compiled, loads) = self.into_parts();
        Box::pin(async move {
            let args = build_arguments(&compiled.binds)?;
            let row = ex.fetch_optional::<Out>(&compiled.sql, args).await?;
            let Some(value) = row else {
                return Ok(None);
            };
            let mut rows = vec![value];
            loads.run(ex, &mut rows).await?;
            Ok(rows.pop())
        })
    }
}

pub trait InsertExt<Out> {
    fn execute<'e, E>(self, ex: &'e mut E) -> BoxFuture<'e, Result<u64, Error>>
    where
        E: Executor + Send + 'e;

    fn one<'e, E>(self, ex: &'e mut E) -> BoxFuture<'e, Result<Option<Out>, Error>>
    where
        E: Executor + Send + 'e,
        Out: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + 'e;
}

impl<Out> InsertExt<Out> for Insert<Out> {
    fn execute<'e, E>(self, ex: &'e mut E) -> BoxFuture<'e, Result<u64, Error>>
    where
        E: Executor + Send + 'e,
    {
        let compiled = self.compile();
        Box::pin(async move {
            let args = build_arguments(&compiled.binds)?;
            ex.execute(&compiled.sql, args).await
        })
    }

    fn one<'e, E>(self, ex: &'e mut E) -> BoxFuture<'e, Result<Option<Out>, Error>>
    where
        E: Executor + Send + 'e,
        Out: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + 'e,
    {
        let compiled = self.compile();
        Box::pin(async move {
            let args = build_arguments(&compiled.binds)?;
            let row = ex.fetch_optional::<Out>(&compiled.sql, args).await?;
            Ok(row)
        })
    }
}

pub trait UpdateExt<Out> {
    fn execute<'e, E>(self, ex: &'e mut E) -> BoxFuture<'e, Result<u64, Error>>
    where
        E: Executor + Send + 'e;

    fn all<'e, E>(self, ex: &'e mut E) -> BoxFuture<'e, Result<Vec<Out>, Error>>
    where
        E: Executor + Send + 'e,
        Out: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + 'e;
}

impl<Out> UpdateExt<Out> for Update<Out> {
    fn execute<'e, E>(self, ex: &'e mut E) -> BoxFuture<'e, Result<u64, Error>>
    where
        E: Executor + Send + 'e,
    {
        let compiled = self.compile();
        Box::pin(async move {
            let args = build_arguments(&compiled.binds)?;
            ex.execute(&compiled.sql, args).await
        })
    }

    fn all<'e, E>(self, ex: &'e mut E) -> BoxFuture<'e, Result<Vec<Out>, Error>>
    where
        E: Executor + Send + 'e,
        Out: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + 'e,
    {
        let compiled = self.compile();
        Box::pin(async move {
            let args = build_arguments(&compiled.binds)?;
            let rows = ex.fetch_all::<Out>(&compiled.sql, args).await?;
            Ok(rows)
        })
    }
}

pub trait DeleteExt {
    fn execute<'e, E>(self, ex: &'e mut E) -> BoxFuture<'e, Result<u64, Error>>
    where
        E: Executor + Send + 'e;
}

impl DeleteExt for Delete {
    fn execute<'e, E>(self, ex: &'e mut E) -> BoxFuture<'e, Result<u64, Error>>
    where
        E: Executor + Send + 'e,
    {
        let compiled = self.compile();
        Box::pin(async move {
            let args = build_arguments(&compiled.binds)?;
            ex.execute(&compiled.sql, args).await
        })
    }
}
