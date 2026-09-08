#[path = "../support/relation_paths.rs"]
mod relation_paths;

use relation_paths::Record;

fn main() {
    let _ = Record::owner.enabled.eq(true);
    let _ = Record::owner.unknown.eq(true); //~ E0609
    let _ = Record::owner.organization.unknown.eq(true); //~ E0609
}
