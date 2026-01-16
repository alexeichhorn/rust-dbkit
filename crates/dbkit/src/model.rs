use crate::{ColumnRef, Error, Value};

pub trait ModelValue {
    fn column_value(&self, column: ColumnRef) -> Option<Value>;
}

pub trait SetRelation<Rel, ValueOut> {
    fn set_relation(&mut self, _rel: Rel, value: ValueOut) -> Result<(), Error> {
        let _ = value;
        Err(Error::RelationMismatch)
    }
}
