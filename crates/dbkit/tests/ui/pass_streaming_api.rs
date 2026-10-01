//@check-pass
#[path = "../support/relation_paths.rs"]
mod relation_paths;

use dbkit::prelude::*;
use dbkit::{Database, DbTransaction, Error, Order};
use futures_util::{Stream, TryStreamExt};
use relation_paths::Record;

fn assert_stream<T>(stream: impl Stream<Item = Result<T, Error>> + Send + Unpin) {
    drop(stream);
}

async fn database(db: &Database) -> Result<(), Error> {
    // The query and bound string may be temporary. Only the executor is borrowed.
    let mut rows = {
        let label = String::from("first");
        Record::query().filter(Record::label.eq(label.as_str())).stream(db)
    };
    while let Some(record) = rows.try_next().await? {
        let _: Record = record;
    }
    drop(rows);

    assert_stream::<Record>(Record::query().stream(db));
    assert_stream::<Record>(Record::query().stream(db.pool()));
    assert_stream::<(i64, Option<String>)>(
        Record::query()
            .select_only()
            .column(Record::id)
            .column(Record::owner.label)
            .order_by(Order::asc(Record::id))
            .into_model::<(i64, Option<String>)>()
            .stream(db),
    );

    Record::query().stream(db).try_for_each(|_| async { Ok(()) }).await?;
    Ok(())
}

async fn transaction(tx: &DbTransaction<'_>) -> Result<(), Error> {
    assert_stream::<Record>(Record::query().for_update().skip_locked().stream(tx));
    assert_stream::<Record>(Record::query().for_update().nowait().stream(tx));
    let rows: Vec<Record> = Record::query().stream(tx).try_collect().await?;
    drop(rows);
    Ok(())
}

fn main() {}
