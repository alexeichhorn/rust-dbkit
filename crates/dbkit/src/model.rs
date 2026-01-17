use crate::{ColumnRef, Error, Value};
use crate::executor::BoxFuture;
use crate::Executor;

pub trait ModelValue {
    fn column_value(&self, column: ColumnRef) -> Option<Value>;
}

pub trait SetRelation<Rel, ValueOut> {
    fn set_relation(&mut self, _rel: Rel, value: ValueOut) -> Result<(), Error> {
        let _ = value;
        Err(Error::RelationMismatch)
    }
}

pub trait ModelDelete: Sized {
    fn delete<'e, E>(self, ex: &'e mut E) -> BoxFuture<'e, Result<u64, Error>>
    where
        E: Executor + Send + 'e;
}
