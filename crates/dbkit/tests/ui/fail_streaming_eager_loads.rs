#[path = "../support/relation_paths.rs"]
mod relation_paths;

use dbkit::prelude::*;
use dbkit::Database;
use relation_paths::{Member, Record};

fn eager_loads(db: &Database) {
    // Eager loading requires batching or grouping; streaming must not silently discard it.
    let _ = Record::query().with(Record::owner.selectin()).stream(db); //~ E0599
    let _ = Record::query().with(Record::owner.joined()).stream(db); //~ E0599
    let _ = Record::query()
        .with(Record::owner.selectin().with(Member::organization.joined()))
        .stream(db); //~ E0599

    // Projecting a loaded query must not bypass the restriction.
    let _ = Record::query()
        .with(Record::owner.selectin())
        .select_only()
        .column(Record::id)
        .into_model::<(i64,)>()
        .stream(db); //~ E0599
}

fn main() {}
