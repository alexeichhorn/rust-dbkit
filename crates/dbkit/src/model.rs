use crate::executor::BoxFuture;
use crate::Executor;
use crate::{ColumnRef, Error, Value};

pub trait ModelValue {
    fn column_value(&self, column: ColumnRef) -> Option<Value>;
}

pub trait JoinedModel: ModelValue + Sized {
    fn joined_columns() -> &'static [ColumnRef];
    fn joined_primary_keys() -> &'static [ColumnRef];

    fn joined_from_row_prefixed(row: &sqlx::postgres::PgRow, prefix: &str) -> Result<Self, sqlx::Error>;

    fn joined_row_has_pk(row: &sqlx::postgres::PgRow, prefix: &str) -> Result<bool, sqlx::Error>;
}

pub trait SetRelation<Rel, ValueOut> {
    fn set_relation(&mut self, _rel: Rel, value: ValueOut) -> Result<(), Error> {
        let _ = value;
        Err(Error::RelationMismatch)
    }
}

pub trait GetRelation<Rel, ValueOut> {
    fn get_relation(&self, _rel: Rel) -> Option<&ValueOut> {
        None
    }

    fn get_relation_mut(&mut self, _rel: Rel) -> Option<&mut ValueOut> {
        None
    }
}

pub trait ModelDelete: Sized {
    fn delete<'e, E>(self, ex: &'e E) -> BoxFuture<'e, Result<u64, Error>>
    where
        E: Executor + Send + Sync + 'e;
}

pub trait LoadRelation<Rel>: Sized {
    type Out;

    fn load_relation<'e, E>(self, rel: Rel, ex: &'e E) -> BoxFuture<'e, Result<Self::Out, Error>>
    where
        E: Executor + Send + Sync + 'e;
}
