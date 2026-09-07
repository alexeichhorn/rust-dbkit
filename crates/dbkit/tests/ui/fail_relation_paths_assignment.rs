#[path = "../support/relation_paths.rs"]
mod relation_paths;

use relation_paths::Record;

fn main() {
    // Filtering through a relation automatically adds its join if not already declared.
    let _ = Record::query().filter(Record::owner.enabled.eq(true));
    // A relation path must not silently become an UPDATE of the base row's column.
    let _ = Record::update().set(Record::owner.enabled, true); //~ E0308
}
