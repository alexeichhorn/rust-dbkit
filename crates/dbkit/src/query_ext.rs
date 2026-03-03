use crate::executor::{build_arguments, BoxFuture};
use crate::joined::{JoinOps, JoinedFlag, Ops};
use crate::runtime::RunLoads;
use crate::{Delete, Error, Executor, Insert, Select, Update};

pub trait SelectExt<Out, Loads> {
    fn all<'e, E>(self, ex: &'e E) -> BoxFuture<'e, Result<Vec<Out>, Error>>
    where
        E: Executor + Send + Sync + 'e,
        Loads: RunLoads<Out> + Send + Sync + 'e,
        Out: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + 'e;

    fn one<'e, E>(self, ex: &'e E) -> BoxFuture<'e, Result<Option<Out>, Error>>
    where
        E: Executor + Send + Sync + 'e,
        Loads: RunLoads<Out> + Send + Sync + 'e,
        Out: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + 'e;

    fn count<'e, E>(&self, ex: &'e E) -> BoxFuture<'e, Result<i64, Error>>
    where
        E: Executor + Send + Sync + 'e;

    fn exists<'e, E>(&self, ex: &'e E) -> BoxFuture<'e, Result<bool, Error>>
    where
        E: Executor + Send + Sync + 'e;

    fn paginate<'e, E>(self, page: u64, per_page: u64, ex: &'e E) -> BoxFuture<'e, Result<Page<Out>, Error>>
    where
        E: Executor + Send + Sync + 'e,
        Loads: RunLoads<Out> + Send + Sync + 'e,
        Out: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + 'e;
}

impl<Out, Loads, Lock, DistinctState> SelectExt<Out, Loads> for Select<Out, Loads, Lock, DistinctState>
where
    Loads: JoinedFlag,
    Ops<<Loads as JoinedFlag>::Flag, Out, Loads>: JoinOps<Out = Out, Loads = Loads>,
    Lock: Send + 'static,
    DistinctState: Send + 'static,
{
    fn all<'e, E>(self, ex: &'e E) -> BoxFuture<'e, Result<Vec<Out>, Error>>
    where
        E: Executor + Send + Sync + 'e,
        Loads: RunLoads<Out> + Send + Sync + 'e,
        Out: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + 'e,
    {
        <Ops<<Loads as JoinedFlag>::Flag, Out, Loads> as JoinOps>::all::<E, Lock, DistinctState>(self, ex)
    }

    fn one<'e, E>(self, ex: &'e E) -> BoxFuture<'e, Result<Option<Out>, Error>>
    where
        E: Executor + Send + Sync + 'e,
        Loads: RunLoads<Out> + Send + Sync + 'e,
        Out: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + 'e,
    {
        <Ops<<Loads as JoinedFlag>::Flag, Out, Loads> as JoinOps>::one::<E, Lock, DistinctState>(self, ex)
    }

    fn count<'e, E>(&self, ex: &'e E) -> BoxFuture<'e, Result<i64, Error>>
    where
        E: Executor + Send + Sync + 'e,
    {
        let compiled = self.compile_without_pagination();
        let sql = format!("SELECT COUNT(*) AS count FROM ({}) AS sub", compiled.sql);
        Box::pin(async move {
            let args = build_arguments(&compiled.binds)?;
            let row = ex.fetch_optional::<(i64,)>(&sql, args).await?;
            Ok(row.map(|(count,)| count).unwrap_or(0))
        })
    }

    fn exists<'e, E>(&self, ex: &'e E) -> BoxFuture<'e, Result<bool, Error>>
    where
        E: Executor + Send + Sync + 'e,
    {
        let compiled = self.compile_without_pagination();
        let sql = format!("SELECT EXISTS({})", compiled.sql);
        Box::pin(async move {
            let args = build_arguments(&compiled.binds)?;
            let row = ex.fetch_optional::<(bool,)>(&sql, args).await?;
            Ok(row.map(|(value,)| value).unwrap_or(false))
        })
    }

    fn paginate<'e, E>(self, page: u64, per_page: u64, ex: &'e E) -> BoxFuture<'e, Result<Page<Out>, Error>>
    where
        E: Executor + Send + Sync + 'e,
        Loads: RunLoads<Out> + Send + Sync + 'e,
        Out: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + 'e,
    {
        Box::pin(async move {
            let page = if page == 0 { 1 } else { page };
            let per_page = if per_page == 0 { 1 } else { per_page };
            let total = self.count(ex).await?;
            let offset = (page - 1).saturating_mul(per_page);
            let items = self.limit(per_page).offset(offset).all(ex).await?;
            Ok(Page {
                items,
                page,
                per_page,
                total,
            })
        })
    }
}

#[derive(Debug, Clone)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub page: u64,
    pub per_page: u64,
    pub total: i64,
}

impl<T> Page<T> {
    pub fn total_pages(&self) -> u64 {
        if self.per_page == 0 || self.total <= 0 {
            return 0;
        }
        let total = self.total as u64;
        (total + self.per_page - 1) / self.per_page
    }
}

pub trait InsertExt<Out> {
    fn execute<'e, E>(self, ex: &'e E) -> BoxFuture<'e, Result<u64, Error>>
    where
        E: Executor + Send + Sync + 'e;

    fn one<'e, E>(self, ex: &'e E) -> BoxFuture<'e, Result<Option<Out>, Error>>
    where
        E: Executor + Send + Sync + 'e,
        Out: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + 'e;
}

impl<Out> InsertExt<Out> for Insert<Out> {
    fn execute<'e, E>(self, ex: &'e E) -> BoxFuture<'e, Result<u64, Error>>
    where
        E: Executor + Send + Sync + 'e,
    {
        let compiled = self.compile();
        Box::pin(async move {
            let args = build_arguments(&compiled.binds)?;
            ex.execute(&compiled.sql, args).await
        })
    }

    fn one<'e, E>(self, ex: &'e E) -> BoxFuture<'e, Result<Option<Out>, Error>>
    where
        E: Executor + Send + Sync + 'e,
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
    fn execute<'e, E>(self, ex: &'e E) -> BoxFuture<'e, Result<u64, Error>>
    where
        E: Executor + Send + Sync + 'e;

    fn all<'e, E>(self, ex: &'e E) -> BoxFuture<'e, Result<Vec<Out>, Error>>
    where
        E: Executor + Send + Sync + 'e,
        Out: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin + 'e;
}

impl<Out> UpdateExt<Out> for Update<Out> {
    fn execute<'e, E>(self, ex: &'e E) -> BoxFuture<'e, Result<u64, Error>>
    where
        E: Executor + Send + Sync + 'e,
    {
        let compiled = self.compile();
        Box::pin(async move {
            let args = build_arguments(&compiled.binds)?;
            ex.execute(&compiled.sql, args).await
        })
    }

    fn all<'e, E>(self, ex: &'e E) -> BoxFuture<'e, Result<Vec<Out>, Error>>
    where
        E: Executor + Send + Sync + 'e,
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
    fn execute<'e, E>(self, ex: &'e E) -> BoxFuture<'e, Result<u64, Error>>
    where
        E: Executor + Send + Sync + 'e;
}

impl DeleteExt for Delete {
    fn execute<'e, E>(self, ex: &'e E) -> BoxFuture<'e, Result<u64, Error>>
    where
        E: Executor + Send + Sync + 'e,
    {
        let compiled = self.compile();
        Box::pin(async move {
            let args = build_arguments(&compiled.binds)?;
            ex.execute(&compiled.sql, args).await
        })
    }
}
