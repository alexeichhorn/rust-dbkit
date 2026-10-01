#[path = "../support/relation_paths.rs"]
mod relation_paths;

use dbkit::prelude::*;
use dbkit::{Database, DbTransaction};
use relation_paths::Record;

fn database(db: Database) {
    let rows = Record::query().stream(&db);
    drop(db); //~ E0505
    drop(rows);
}

async fn transaction(tx: DbTransaction<'_>) {
    let rows = Record::query().stream(&tx);
    tx.commit().await.unwrap(); //~ E0505
    drop(rows);
}

fn main() {}
